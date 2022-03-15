use crate::{CommandSource, MyError, TwitchAuth, NameIdCache, Config};
use crate::db;
use crate::api;

use std::sync::{Arc, Mutex};

use async_recursion::async_recursion;
use chrono::{Duration, Utc};
use rand::{self, Rng};
use sqlx::sqlite::SqlitePool;

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
) -> anyhow::Result<()> {
	let cache_arc2 = cache_arc.clone();
	if let Ok(mut cache) = cache_arc2.lock() {
		cache.insert(cmd.sender.name.to_owned(), cmd.sender.id);
	}

	let cmd_out = match cmd.cmd.as_str() {
		"ping"           => ping(),
		"echo"           => echo(&cmd.args),
		"say"            => echo(&cmd.args),
		"markov"         => markov(&pool, &cmd).await,
		"suggest"        => suggest(&pool, &cmd).await,
		"setalias"       => set_alias(&pool, &cmd).await,
		"rmalias"        => remove_alias(&pool, &cmd).await,
		"explain"        => explain(&pool, &cmd.args[0]).await,
		"weather"        => get_weather_report(&cmd.args).await,
		"clearreminders" => clear_reminders(&pool, cmd.sender.id).await,
		"rmrm"           => clear_reminders(&pool, cmd.sender.id).await,
		"rose"           => tag_rand_chatter_with_rose(&cmd.channel, &config.disregarded_users).await,
		"first"          => first_message(&pool, &auth, cache_arc, &cmd).await,
		"remindme"       => add_reminder(&pool, &auth, cache_arc, &cmd, true).await,
		"remind"         => add_reminder(&pool, &auth, cache_arc, &cmd, false).await,
		"uptime"         => get_uptime(&auth, &cmd).await,
		"accage"         => get_accage(&auth, &cmd).await
		_ if (cmd.cmd.as_str() == &config.prefix.to_string()) => execute_alias(&pool, client.clone(), config, &auth, cache_arc, &cmd).await,
		_ => Ok(None),
	};

	let cmd_out = match cmd_out {
		Ok(content) => content,
		Err(e)      => {
			println!("{e}");
			Some("error while processing, sorry PoroSad".into())
		},
	};

	if let Some(output) = cmd_out {
		client.say(cmd.channel, output.into()).await.unwrap();
	}

	Ok(())
}

// get age of specified account (or called)
async fn get_accage(
	auth: &TwitchAuth,
	cmd: &CommandSource, 
) -> anyhow::Result<Option<String>> {
	let user_name = match cmd.args.get(0) {
		Some(nick) => &nick,
		None       => &cmd.sender.name
	}

	match db::get_acc_creation_date(auth, user_name).await? {
		Some(date) => {
			let duration = (Utc::now() - date).num_days();
			let years = (duration as f32 / 365.24);

			if years > 0.5 {
				return Ok(Some(format!("⏱️ {user_name}'s account is {:.2} years old", years)));
			} else {
				return Ok(Some(format!("⏱️ {user_name}'s account is {duration} days old")));
			}
		},
		None       => return Some("❌ user not found"),
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
		statuses: cmd.statuses.clone(),
		timestamp: cmd.timestamp.clone(),
	};

	handle_command(pool, client, config, auth, cache_arc, new_cmd).await?;

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

	Ok(Some(output.join(" ")))
}

// show additional information about a spec. error
pub async fn explain (
	pool: &SqlitePool,
	error_code: &str,
) -> anyhow::Result<Option<String>> {

	match db::get_explanation(pool, error_code).await? {
		Some(expl) => return Ok(Some(expl)),
		None => return Ok(Some("❌ no such explanation".into()))
	}
}

// returns the first message of user
pub async fn first_message(
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

pub async fn suggest(
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

pub async fn tag_rand_chatter_with_rose(
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
	
		if !disregarded_users.contains(&try_rand_chatter) {
			rand_chatter = try_rand_chatter;
		}
	}
	
	Ok(Some(format!("@{rand_chatter} PeepoGlad 🌹")))
}

pub async fn get_weather_report(
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
pub async fn get_uptime(
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
	let duration = (Utc::now() - info.data[0].started_at).num_seconds();
	
	let hrs = (duration as f32 / 3600.0) as u8;
	let mins = ((duration % 3600) as f32 / 60.0) as u8;
	let secs = (duration % 60) as u8;

	let mut formatted = String::new();

	if hrs > 0 {
		formatted.push_str(&format!("{hrs} hrs, "));
	}
	if mins > 0 {
		formatted.push_str(&format!("{mins} mins, "));
	}

	formatted.push_str(&format!("{secs} secs"));

	Ok(Some(format!("⏱️ {channel} has been live for {formatted}")))
}
