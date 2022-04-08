#![allow(unused)]

use crate::db;
use crate::api;
use crate::{
	CommandSource,
	MyError,
	TwitchAuth,
	TwitchBadge,
	NameIdCache,
	Config,
	fmt_duration,
};

use std::sync::{Arc, Mutex};
use std::time::Instant;

use async_recursion::async_recursion;
use chrono::{Duration, Utc};
use rand::{self, Rng};
use sqlx::sqlite::SqlitePool;
use thiserror::Error as ThisError;

type TwitchClient = twitch_irc::TwitchIRCClient<twitch_irc::transport::tcp::TCPTransport<twitch_irc::transport::tcp::TLS>, twitch_irc::login::StaticLoginCredentials>;

// handle incoming commands
#[async_recursion]
pub async fn handle_command(
	// this looks like an abomination
	// but it is what it is
	pool:       &SqlitePool,
	client:     TwitchClient,
	config:     &Config,
	auth:       &TwitchAuth,
	cache_arc: Arc<Mutex<NameIdCache>>,
	cmd:        CommandSource,
	is_pipe:    bool,
) -> Option<String> {
	let cache_arc2 = cache_arc.clone();
	if let Ok(mut cache) = cache_arc2.lock() {
		cache.insert(cmd.sender.name.to_owned(), cmd.sender.id);
	}

	let now = Instant::now();

	let cmd_out = match cmd.cmd.as_str() {
		// standard commands
		"ping"           => ping(),
		"decide"         => decide(&cmd),
		"echo"           => echo(&cmd.args),
		"say"            => echo(&cmd.args),
		"translate"      => translate(&cmd).await,
		"markov"         => markov(&pool, &cmd).await,
		"newcmd"         => new_cmd(&pool, &cmd).await,
		"suggest"        => suggest(&pool, &cmd).await,
		"reddit"         => get_reddit_post(&cmd).await,
		"wiki"           => query_wikipedia(&cmd).await,
		"define"         => query_dictionary(&cmd).await,
		"setalias"       => set_alias(&pool, &cmd).await,
		"uptime"         => get_uptime(&auth, &cmd).await,
		"accage"         => get_accage(&auth, &cmd).await,
		"rmalias"        => remove_alias(&pool, &cmd).await,
		"followage"      => get_followage(&cmd, &auth).await,
		"urban"          => query_urban_dictionary(&cmd).await,
		"lurk"           => set_lurk_status(&pool, &cmd).await,
		"explain"        => explain(&pool, &cmd.args[0]).await,
		"weather"        => get_weather_report(&cmd.args).await,
		"delcmd"         => remove_channel_command(&pool, &cmd).await,
		"offlinetime"    => get_offline_time(&pool, &auth, &cmd).await,
		"clearreminders" => clear_reminders(&pool, cmd.sender.id).await,
		"rmrm"           => clear_reminders(&pool, cmd.sender.id).await,
		"first"          => first_message(&pool, &auth, cache_arc, &cmd).await,
		"remindme"       => add_reminder(&pool, &auth, cache_arc, &cmd, true).await,
		"wordratio"      => get_word_ratio(&pool, &auth, &cmd, config.prefix).await,
		"remind"         => add_reminder(&pool, &auth, cache_arc, &cmd, false).await,
		"rose"           => tag_rand_chatter_with_rose(&cmd.channel, &config.disregarded_users).await,
		"bench"          => bench_command(&pool, client.clone(), config, &auth, cache_arc, cmd.clone()).await,
		// special commands
		"pipe"           => pipe(&pool, client.clone(), config, &auth, cache_arc, &cmd).await,
		""               => execute_alias(&pool, client.clone(), config, &auth, cache_arc, &cmd).await,
		_                => try_execute_channel_command(&pool, &cmd).await,
	};

	let cmd_out = match cmd_out {
		Ok(content_or_not) => content_or_not,
		Err(e)      => {
			println!("{e}");
			Some("error while processing, sorry PoroSad".into())
		},
	};

	if is_pipe {
		return cmd_out;
	}

	match db::log_command(
		&pool,
		&cmd,
		now.elapsed(),
		if let Some(s) = &cmd_out {s} else {""}
	).await {
		Ok(_) => (),
		Err(e) => println!("{e}")
	};
	
	if let Some(output) = cmd_out {
		client.say(cmd.channel, output.into()).await.unwrap();
	}

	None
}

// get age of specified account (or called)
async fn get_accage(
	auth: &TwitchAuth,
	cmd: &CommandSource, 
) -> anyhow::Result<Option<String>> {
	let user_name = match cmd.args.get(0) {
		Some(nick) => &nick,
		None       => &cmd.sender.name
	};

	match api::get_acc_creation_date(user_name, auth).await? {
		Some(date) => {
			let duration = (Utc::now() - date).num_days();
			let years = duration as f32 / 365.24;

			if years > 0.5 {
				return Ok(Some(format!("⏱️ {user_name}'s account is {:.2} years old", years)));
			} else {
				return Ok(Some(format!("⏱️ {user_name}'s account is {duration} days old")));
			}
		},
		None       => return Ok(Some("❌ user not found".into())),
	}
}

// allows for user to add a new alias for themselves
async fn set_alias(
	pool: &SqlitePool,
	cmd: &CommandSource,
) -> anyhow::Result<Option<String>> {
	let alias = match &cmd.args.get(0) {
		Some(a) => a.clone(),
		None => return Ok(Some("❌ no alias name provided".into()))
	};

	let alias_cmd = match cmd.args.get(1) {
		Some(_) => cmd.args[1..].join(" "),
		None => return Ok(Some("❌ no alias command provided".into())),
	};

	db::set_alias(pool, cmd.sender.id, &alias, &alias_cmd).await?;

	Ok(Some("✅ alias created".into()))
}

// run user's alias
async fn execute_alias(
	pool: &SqlitePool,
	client: TwitchClient,
	config: &Config,
	auth: &TwitchAuth,
	cache_arc: Arc<Mutex<NameIdCache>>,
	cmd: &CommandSource,
) -> anyhow::Result<Option<String>> {
	let alias = match &cmd.args.get(0).clone() {
		Some(a) => a.clone(),
		None => return Ok(Some("❌ missing alias name".into())),
	};

	let alias_cmd = match db::get_alias_cmd(pool, cmd.sender.id, &alias).await? {
		Some(alias) => alias
			.split(' ')
			.map(|a| a.clone().to_string())
			.collect::<Vec<String>>(),
		None => return Ok(Some("❌ alias not recognized".into())),
	};

	let new_cmd = CommandSource {
		cmd: match alias_cmd.get(0) {
			Some(a) => a[1..].to_owned(),
			None => return Ok(Some("❌ alias faulty".into())),
		},
		args: match alias_cmd.get(1) {
			Some(_) => alias_cmd[1..].into_iter().map(|x| x.clone().to_string()).collect::<Vec<String>>(),
			None => vec![],
		},
		channel: cmd.channel.clone(),
		sender: cmd.sender.clone(),
		timestamp: cmd.timestamp.clone(),
	};

	handle_command(pool, client, config, auth, cache_arc, new_cmd, false).await;

	Ok(None)
}

// allows caller to remove an alias of theirs
async fn remove_alias(
	pool: &SqlitePool,
	cmd: &CommandSource,
) -> anyhow::Result<Option<String>> {
	let alias = match cmd.args.get(0) {
		Some(a) => a.to_owned(),
		None => return Ok(Some("❌ no alias provided".into()))
	};

	match db::remove_alias(pool, cmd.sender.id, &alias).await? {
		0 => return Ok(Some("❌ no such alias".into())),
		_ => return Ok(Some("✅ alias removed".into())),
	}
}

// parse the incoming duration identifying string
// expected input: (xh,xm) 
fn parse_duration_to_hm(s: &String) -> anyhow::Result<(i64, i64)> {
	let hrs  = s[s.find('(').ok_or(MyError::NotFound)?+1..s.find('h').ok_or(MyError::NotFound)?].to_owned().parse()?;
	let mins = s[s.find(',').ok_or(MyError::NotFound)?+1..s.find('m').ok_or(MyError::NotFound)?].to_owned().parse()?;

	Ok((hrs, mins))
} 

// add a reminder for someone
async fn add_reminder(
	pool: &SqlitePool,
	auth: &TwitchAuth,
	cache_arc: Arc<Mutex<NameIdCache>>,
	cmd: &CommandSource,
	is_for_self: bool,
) -> anyhow::Result<Option<String>> {
	let (h, m) = match parse_duration_to_hm(&cmd.args[0]) {
		Ok(a) => a,
		Err(_) => return Ok(Some("❌ bad time formatting".into()))
	};

	let remind_time = cmd.timestamp + Duration::hours(h) + Duration::minutes(m);
	
	let to_user_name = match is_for_self {
		true => &cmd.sender.name,
		false => match cmd.args.get(1).ok_or(MyError::OutOfBounds) {
			Ok(a) => a,
			Err(_) => return Ok(Some("❌ no name provided".into())),
		}
	};

	let start_idx = if is_for_self { 1 } else { 2 };
	let message = match cmd.args.get(start_idx).ok_or(MyError::OutOfBounds) {
		Ok(_)  => cmd.args[start_idx..].join(" "),
		Err(_) => return Ok(Some("❌ no message provided".into())),
	};

	let mut for_user_id: Option<i32> = None;
	if let Ok(cache) = cache_arc.lock() {
		match cache.get(to_user_name) {
			Some(id) => { for_user_id = Some(*id); },
			None     => (), 
		};
	}

	match for_user_id {
		Some(_) => (),
		None    => match api::id_from_nick(to_user_name, auth).await? {
			Some(id) => {
				if let Ok(mut cache) = cache_arc.lock() {
					cache.insert(to_user_name.to_string(), id);
				}

				for_user_id = Some(id); 
			},
			None     => return Ok(Some("❌ user nonexistant".into()))
		}
	}

	let for_user_id = for_user_id.unwrap();

	let reminder = db::Reminder {
		id: 0, // dummy
		from_user_id: cmd.sender.id,
		raise_timestamp: remind_time,
		for_user_id,
		message,
	};

	db::insert_reminder(pool, &reminder).await?;

	Ok(Some("✅ set successfully".into()))
}

// clears reminders user has sent out
async fn clear_reminders(
	pool: &SqlitePool,
	user_id: i32,
) -> anyhow::Result<Option<String>> {
	let delete_count = db::clear_users_sent_reminders(pool, user_id).await?;

	if delete_count == 0 {
		return Ok(Some("❌ no reminders set, nothing happened".into()));
	} else {
		return Ok(Some(format!("✅ cleared {delete_count} reminders")));
	}
}

// ping -> pong
fn ping()
-> anyhow::Result<Option<String>> {
	Ok(Some("pong".into()))
}

// say whatever caller said
fn echo(args: &Vec<String>)
-> anyhow::Result<Option<String>> {
	Ok(Some(args.join(" ")))
}

// return a markov chain of words
async fn markov(
	pool: &SqlitePool,
	cmd: &CommandSource
) -> anyhow::Result<Option<String>> {
	let rounds: usize;
	match cmd.args.len() {
		// if no arguments are supplied, return immediately
		0 => return Ok(Some("❌ insufficient args".into())),
		// if number of rounds isn't set, set to default
		1 => {rounds = 7},
		// else parse both arguments
		_ => 
			match cmd.args[1].parse::<usize>() {
				Ok(num) => {rounds = num},
				Err(_)  => return Ok(Some("❌ expected positive integer".into())),
		}
	}

	let mut output: Vec<String> = vec![cmd.args[0].clone()];
	let mut seed: String = cmd.args[0].clone();

	for _ in 0..rounds-1 {
		let succ = match db::get_rand_markov_succ(pool, &cmd.channel, &seed).await {
		Ok(Some(successor)) => successor,
			Ok(None) => continue,
			Err(e) => {println!("{e}");break},
		};

		seed = succ.clone();
		output.push(succ.to_owned());
	}

	if output.len() == 1 {
		return Ok(Some("❌ word not indexed yet | E1".into()));
	}

	Ok(Some(format!("🔮 {}", output.join(" "))))
}

// show additional information about a spec. error
async fn explain (
	pool: &SqlitePool,
	error_code: &str,
) -> anyhow::Result<Option<String>> {

	match db::get_explanation(pool, error_code).await? {
		Some(expl) => return Ok(Some(expl)),
		None => return Ok(Some("❌ no such explanation".into()))
	}
}

// returns the first (logged) message of a user
async fn first_message(
	pool:      &SqlitePool,
	auth:      &TwitchAuth,
	cache_arc: Arc<Mutex<NameIdCache>>,
	cmd:       &CommandSource,
) -> anyhow::Result<Option<String>> {
	let sender_id = match &cmd.args.get(0) {
		Some(name) => {
			let mut _id: Option<i32> = None;

			if let Ok(cache) = cache_arc.lock() {
				match cache.get(*name) {
					Some(id) => { _id = Some(*id); },
					None     => (), 
				};
			}

			match _id {
				Some(id) => id,
				None     => match api::id_from_nick(name, auth).await? {
					Some(id) => {
						if let Ok(mut cache) = cache_arc.lock() {
							cache.insert(name.to_string(), id);
						}
						
						id
					},
					None     => return Ok(Some(format!("user {} nonexistant", name))),
				},
			}
		},
		None => cmd.sender.id,
	};

	let channel = match &cmd.args.get(1) {
		Some(c) => c.clone(),
		None =>    &cmd.channel,
	};

	let message = db::get_first_message(pool, sender_id, channel).await?;

	match message {
		Some(msg) => return Ok(Some(msg)),
		None      => return Ok(Some("❌ nothing found | E2".into())),
	}
}

// user can leave a suggestion, that will
// get saved into the database
async fn suggest(
	pool: &SqlitePool,
	cmd: &CommandSource,
) -> anyhow::Result<Option<String>> {
	let text = match &cmd.args.get(0) {
		Some(_) => cmd.args.join(" "),
		None    => return Ok(Some("❌ no message".into())),
	};

	db::save_suggestion(
		pool,
		cmd.sender.id,
		&cmd.sender.name,
		&text,
		Utc::now()
	).await?;

	Ok(Some("✅ suggestion saved".into()))
}

// give a rose to a random chatter in the channel
async fn tag_rand_chatter_with_rose(
	channel_name: &str,
	disregarded_users: &Vec<String>,
) -> anyhow::Result<Option<String>> {
	let chatters = match api::get_chatters(channel_name).await? {
		Some(chatters) => chatters,
		None           => return Ok(Some("❌ no users in the chatroom".into())),
	};

	let mut rand_chatter = "".to_string();

	while rand_chatter.len() == 0 {
		let try_rand_chatter = chatters[rand::thread_rng().gen_range(0..chatters.len())].clone();
	
		if !disregarded_users.contains(&try_rand_chatter.to_lowercase()) {
			rand_chatter = try_rand_chatter;
		}
	}
	
	Ok(Some(format!("@{rand_chatter} PeepoGlad 🌹")))
}

// get weather report from wttr.in API
async fn get_weather_report(
	args: &Vec<String>,
) -> anyhow::Result<Option<String>> {
	if args.len() == 0 {
		return Ok(Some("❌ no location provided".into()));
	}

	let location = args.join(" ");

	match api::get_weather_report(&location).await? {
		Some(r) => return Ok(Some(r)),
		None    => return Ok(Some("❌ location not identified".into())),
	}
}

// get uptime of a stream
async fn get_uptime(
	auth: &TwitchAuth,
	cmd: &CommandSource,
) -> anyhow::Result<Option<String>> {
	let channel = match &cmd.args.get(0) {
		Some(a) => a,
		None    => &cmd.channel,
	};

	let info = match api::get_stream_info(auth, &channel).await? {
		Some(i) => i,
		None    => return Ok(Some("❌ streamer not live".into())),
	};
	let duration = Utc::now() - info.data[0].started_at;
	
	let formatted = fmt_duration(duration);

	Ok(Some(format!("⏱️ {channel} has been live for {formatted}")))
}

// the language identifiers
// expected input: (l1,l2) 
fn parse_langs(s: &String) -> anyhow::Result<(&str, &str)> {
	Ok((
		&s[s.find('(').ok_or(MyError::NotFound)?+1..s.find(',').ok_or(MyError::NotFound)?],
		&s[s.find(',').ok_or(MyError::NotFound)?+1..s.find(')').ok_or(MyError::NotFound)?],
	))
} 

// translate a phrase
async fn translate(
	cmd: &CommandSource,
) -> anyhow::Result<Option<String>> {
	let langs = match cmd.args.get(0) {
		Some(l) => l,
		None    => return Ok(Some("❌ insufficient args".into())),
	};

	let text = match &cmd.args.get(1) {
		Some(_) => cmd.args[1..].join(" "),
		None    => return Ok(Some("❌ insufficient args".into())),
	};

	let (src_lang, target_lang) = match parse_langs(langs) {
		Ok(ls) => ls,
		Err(_) => return Ok(Some("❌ bad formatting".into())),
	};

	Ok(Some(api::translate(src_lang, target_lang, &text).await?))
}

// go into AFK state
async fn set_lurk_status(
	pool: &SqlitePool,
	cmd: &CommandSource,
) -> anyhow::Result<Option<String>> {
	let sender_name = &cmd.sender.name;
	let sender_id = cmd.sender.id;
	let timestamp = cmd.timestamp;

	db::set_lurk_status(pool, sender_id, timestamp).await?;

	Ok(Some(format!("{sender_name} is now AFK")))
}

// measure how long a command takes
// (requires bot to be vip/mod/...)
async fn bench_command(
	pool:       &SqlitePool,
	client:     TwitchClient,
	config:     &Config,
	auth:       &TwitchAuth,
	cache_arc: Arc<Mutex<NameIdCache>>,
	cmd:        CommandSource,
) -> anyhow::Result<Option<String>> {
	let new_cmd = CommandSource {
		cmd: match cmd.args.get(0) {
			Some(a) => a[1..].to_owned(),
			None => return Ok(Some("❌ no command provided".into())),
		},
		args: match cmd.args.get(1) {
			Some(_) => cmd.args[1..].into_iter().map(|x| x.clone().to_string()).collect::<Vec<String>>(),
			None => vec![],
		},
		channel: cmd.channel.clone(),
		sender: cmd.sender.clone(),
		timestamp: cmd.timestamp.clone(),
	};

	let now = Instant::now();
	handle_command(pool, client, config, auth, cache_arc, new_cmd, true).await;
	Ok(Some(format!("📡 {} ms", now.elapsed().as_millis())))
}

// get the time a user has spent in an offline chat
async fn get_offline_time(
	pool: &SqlitePool,
	auth: &TwitchAuth,
	cmd:  &CommandSource,
) -> anyhow::Result<Option<String>> {
	let channel_name: &str;
	let offliner_id: i32;
	let offliner_name: &str;
	
	match cmd.args.len() {
		0 => {
			channel_name = &cmd.channel;
			offliner_id = cmd.sender.id;
			offliner_name = &cmd.sender.name;
		},
		1 => {
			channel_name = &cmd.channel;
			offliner_name = &cmd.args[0];
			offliner_id = match api::id_from_nick(&cmd.args[0], auth).await? {
				Some(id) => id,
				None     => return Ok(Some(format!("❌ user {offliner_name} does not exist")))
			}
		},
		_ => {
			channel_name = &cmd.args[1];
			offliner_name = &cmd.args[0];
			offliner_id = match api::id_from_nick(&cmd.args[0], auth).await? {
				Some(id) => id,
				None     => return Ok(Some(format!("❌ user {offliner_name} does not exist")))
			}
		},
	};

	let t = db::get_offline_time(pool, channel_name, offliner_id).await?;
	Ok(Some(format!("{offliner_name} has spent {} in {channel_name}'s offline chat!", fmt_duration(t))))
}

// get the abstract from a wikipedia page
async fn query_wikipedia(
	cmd: &CommandSource,
) -> anyhow::Result<Option<String>> {
	let title = &cmd.args.join(" ");

	match api::query_wikipedia(title).await? {
		Some(mut w) => {
			for (_, page) in w.query.pages.iter_mut() {
				let abs = page
					.extract
					.split(".").collect::<Vec<&str>>()[0];

				return Ok(Some(abs.to_owned()));
			}
		},
		None => return Ok(Some("❌ Article not found.".into())),
	};
	todo!()
}

// get a (english only) word definition
async fn query_dictionary(
	cmd: &CommandSource,
) -> anyhow::Result<Option<String>> {
	let word = match cmd.args.get(0) {
		Some(w) => w,
		None    => return Ok(Some("❌ No word provided".into())),
	};
	
	match api::query_dictionary(word).await? {
		Some(def) => Ok(Some(def)),
		None      => Ok(Some("❌ word not found".into()))
	}
}

// get a definiton from urbandictionary
async fn query_urban_dictionary(
	cmd: &CommandSource,
) -> anyhow::Result<Option<String>> {
	let term = match cmd.args.len() {
		0 => return Ok(Some("❌ no term provided".into())),
		_ => cmd.args.join(" "),
	};

	match api::query_urban_dictionary(&term).await? {
		Some(ud) => Ok(Some(ud)),
		None     => Ok(Some("❌ not found".into())),
	}
}

async fn get_followage(
	cmd: &CommandSource,
	twitch_auth: &TwitchAuth,
) -> anyhow::Result<Option<String>> {
	let (channel_name, channel_id) = match cmd.args.len() {
		0 => (&cmd.channel, api::id_from_nick(&cmd.channel, twitch_auth).await?),
		1 => (&cmd.channel, api::id_from_nick(&cmd.channel, twitch_auth).await?),
		_ => (&cmd.args[1], api::id_from_nick(&cmd.args[1], twitch_auth).await?),
	};

	let channel_id = match channel_id {
		Some(id) => id,
		None => return Ok(Some(format!("❌ channel {channel_name} does not exist"))),
	};

	let (user_name, user_id) = match cmd.args.len() {
		0 => (&cmd.sender.name, Some(cmd.sender.id)),
		_ => (&cmd.args[0], api::id_from_nick(&cmd.args[0], twitch_auth).await?),
	};

	let user_id = match user_id {
		Some(id) => id,
		None => return Ok(Some(format!("❌ user {user_name} does not exist"))),
	}; 

	match api::get_followage(twitch_auth, channel_id, user_id).await? {
		Some(date) => {
			let duration = (Utc::now() - date).num_days();
			let years = duration as f32 / 365.24;

			if years > 0.5 {
				return Ok(Some(format!("⏱️ {user_name} has been following {channel_name} for {years:.2} years")));
			} else {
				return Ok(Some(format!("⏱️ {user_name} has been following {channel_name} for {duration} days")));
			}
		},
		None       => return Ok(Some(format!("❌ {user_name} does not follow {channel_name}"))),
	}
}

async fn new_cmd(
	pool: &SqlitePool,
	cmd: &CommandSource,
) -> anyhow::Result<Option<String>> {
	if !(
		cmd.sender.statuses.contains(&TwitchBadge::Broadcaster) ||
		cmd.sender.statuses.contains(&TwitchBadge::Mod)         ||
		cmd.sender.statuses.contains(&TwitchBadge::Vip)
	) {
		return Ok(Some("❌ not high enough status".into()));
	}

	let cmd_name = match cmd.args.get(0) {
		Some(name) => name,
		None       => return Ok(Some("❌ no name provided".into())),
	};

	let cmd_type = match cmd.args.get(1) {
		Some(type_) => type_,
		None       => return Ok(Some("❌ no type provided".into())),
	};

	if !(
		cmd_type == "paste" ||
		cmd_type == "templ" ||
		cmd_type == "incr"
	) {
		return Ok(Some("❌ command type not recognized".into()));
	}

	let cmd_expr = match cmd.args.get(2) {
		Some(name) => cmd.args[2..].join(" "),
		None       => return Ok(Some("❌ no expression provided".into())),
	};

	db::new_cmd(pool, &cmd.channel, cmd_name, cmd_type, &cmd_expr).await?;

	Ok(Some("🔧 command created successfully".into()))
}

pub async fn try_execute_channel_command(
	pool: &SqlitePool,
	cmd: &CommandSource,
) -> anyhow::Result<Option<String>> {
	let cmd_name = cmd.cmd.as_str();

	let (cmd_type, cmd_expr, cmd_meta) = match db::get_channel_cmd(pool, &cmd.channel, cmd_name).await? {
		Some(cmd) => cmd,
		None => return Ok(Some("❌ command not recognized".into())),
	};

	let mut out = cmd_expr.clone();

	if (cmd_type == "templ") {
		for i in 0..cmd.args.len() {
			out = out.replace(&format!("{{{}}}", i+1), &cmd.args[i]);
		}

		return Ok(Some(out));
	}

	if (cmd_type == "paste") {
		return Ok(Some(out));
	}

	if (cmd_type == "incr") {
		return Ok(Some(out.replace(&"{}", &format!("{cmd_meta}"))));
	}

	// unreachable unless some obscure error occures
	Ok(None)
}

pub async fn remove_channel_command(
	pool: &SqlitePool,
	cmd: &CommandSource,
) -> anyhow::Result<Option<String>> {
	let cmd_name = match cmd.args.get(0) {
		Some(a) => a,
		None => return Ok(Some("❌ no command name provided".into()))
	};

	match db::remove_channel_command(pool, &cmd.channel, cmd_name).await? {
		0 => return Ok(Some("❌ no such command existed".into())),
		_ => return Ok(Some("✅ removed successfully".into())),
	}
}

pub async fn get_word_ratio(
	pool:   &SqlitePool,
	auth:   &TwitchAuth,
	cmd:    &CommandSource,
	cmd_prefix: char,
) -> anyhow::Result<Option<String>> {
	let (user_name, user_id, word) = match cmd.args.len() {
		0 => return Ok(Some("❌ no word provided".into())),
		1 => (&cmd.sender.name, cmd.sender.id, &cmd.args[0]),
		_ => {
			let user_id = api::id_from_nick(&cmd.args[0], auth).await?;

			if let Some(id) = user_id {
				(&cmd.args[0], id, &cmd.args[1])
			} else {
				return Ok(Some("❌ user does not exist".into()));
			}
		}
	};

	Ok(
		Some(
			format!(
				"{:.2}% of tracked {user_name}'s messages in this channel contain the word {word}",
				db::get_word_ratio(pool, &cmd.channel, user_id, word, cmd_prefix).await? * 100.,
			)
		)
	)
}

// parses the args into a list of (comma-separated) decisions,
// choses one of them at random and returns it
fn decide(
	cmd: &CommandSource,
) -> anyhow::Result<Option<String>> {
	match cmd.args.len() {
		0 => return Ok(Some("❌ no options provided".into())),
		_ => {
			let options: Vec<String> = cmd.args
				.join(" ")
				.split(",")
				.map(|a| a.to_owned())
				.collect();
			
			let rand_opt = options[
					rand::thread_rng()
						.gen_range(0..options.len())
			].clone();

			return Ok(Some(format!("🎱 I choose... {rand_opt}")));
		}
	}
}

// chain commands via |
async fn pipe(
	pool: &SqlitePool,
	client: TwitchClient,
	config: &Config,
	auth: &TwitchAuth,
	cache_arc: Arc<Mutex<NameIdCache>>,
	cmd: &CommandSource,
) -> anyhow::Result<Option<String>> {
	let commands: Vec<String> = cmd.args
		.join(" ")
		.split("|")
		.map(|a| a.trim().to_owned())
		.collect();

	if commands.len() < 2 {
		return Ok(Some("❌ no command to pipe".into()));
	}

	let mut temp_output = String::new();
	for i in 0..commands.len() {
	
		if i == commands.len()-1 {
			let final_pipe_output = match commands[i].to_lowercase().as_str() {
				"pastebin" => api::upload_to_pastebin(&temp_output).await?,
				"lower"  => temp_output.to_lowercase(),
				"upper"  => temp_output.to_uppercase(),
				"stdout"   => temp_output,
				_          => return Ok(Some(format!("❌ final pipe command not matched"))),
			};

			temp_output = final_pipe_output;
			break;
		}

		let trimmed_cmd: Vec<String> = commands[i]
			.trim()
			.to_string()
			.split(" ")
			.map(|a| a.to_owned())
			.collect();

		let new_cmd = CommandSource {
			cmd: match trimmed_cmd.get(0) {
				Some(a) => a[1..].to_owned(),
				None => return Ok(Some(format!("❌ {} th pipe faulty", i+1))),
			},
			args: match trimmed_cmd.get(1) {
				Some(_) => trimmed_cmd[1..].into_iter().map(|x| x.clone().to_string()).collect::<Vec<String>>(),
				None => vec![],
			},
			channel:   cmd.channel.clone(),
			sender:    cmd.sender.clone(),
			timestamp: cmd.timestamp.clone(),
		};

		if let Some(output) = handle_command(pool, client.clone(), config, auth, cache_arc.clone(), new_cmd, true).await {
			temp_output = output;
		}
	}

	Ok(Some(temp_output))
}

pub enum RedditRelevancy  {
	Hour,
	Day, 
	Week,
	Month,
	Year,
	All,
}

pub enum RedditPostType {
	MostUpvotes,
	Random,
}

async fn get_reddit_post(
	cmd: &CommandSource,
) -> anyhow::Result<Option<String> {
	let subr = match cmd.args.len() {
		0 => return Ok(Some("❌ no subreddit provided")),
		1 => {
			let mut s = cmd.args[0].clone();

			if &s[0..2] == "r/" {
				s = s[2..]
			}

			s
		}
	}

	todo!()
}
