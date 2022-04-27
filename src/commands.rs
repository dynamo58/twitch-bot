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
	convert_from_html_entities,
};

use std::sync::{Arc, Mutex};
use std::time::Instant;

use async_recursion::async_recursion;
use chrono::{offset::TimeZone, DateTime, Local, NaiveDateTime, Utc, Duration};
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
	ongoing_trivia_games_arc: Arc<Mutex<crate::OngoingTriviaGames>>,
) -> Option<String> {
	if let Ok(mut cache) = cache_arc.clone().lock() {
		cache.insert(cmd.sender.name.clone(), cmd.sender.id);
	}

	let now = Instant::now();

	let cmd_out = match cmd.cmd.as_str() {
		// standard commands
		"echo"           => echo(&cmd).await,
		"query"          => query(&cmd).await,
		"math"           => query(&cmd).await,
		"ping"           => ping(config).await,
		"decide"         => decide(&cmd).await,
		"8ball"          => decide(&cmd).await,
		"time"           => get_time(&cmd).await,
		"pasta"          => get_rand_pasta().await,
		"markov"         => markov(pool, &cmd).await,
		"newcmd"         => new_cmd(pool, &cmd).await,
		"suggest"        => suggest(pool, &cmd).await,
		"ls"             => find_last_seen(&cmd, auth, config).await,
		"reddit"         => get_reddit_post(&cmd).await,
		"wiki"           => query_wikipedia(&cmd).await,
		"setalias"       => set_alias(pool, &cmd).await,
		"define"         => query_dictionary(&cmd).await,
		"rmalias"        => remove_alias(pool, &cmd).await,
		"random"         => rand_int_from_range(&cmd).await,
		"lurk"           => set_lurk_status(pool, &cmd).await,
		"explain"        => explain(pool, &cmd.args[0]).await,
		"urban"          => query_urban_dictionary(&cmd).await,
		"weather"        => get_weather_report(&cmd.args).await,
		"chatstats"      => get_chatstats(pool, &cmd, auth).await,
		"uptime"         => get_uptime(auth, &cmd, cache_arc).await,
		"accage"         => get_accage(auth, &cmd, cache_arc).await,
		"delcmd"         => remove_channel_command(pool, &cmd).await,
		"followage"      => get_followage(&cmd, auth, cache_arc).await,
		"clearreminders" => clear_reminders(pool, cmd.sender.id).await,
		"rmrm"           => clear_reminders(pool, cmd.sender.id).await,
		"first"          => first_message(pool, auth, cache_arc, &cmd).await,
		"bible"          => get_rand_holy_book_verse(api::HolyBook::Bible).await,
		"quran"          => get_rand_holy_book_verse(api::HolyBook::Quran).await,
		"tanakh"         => get_rand_holy_book_verse(api::HolyBook::Tanakh).await,
		"offlinetime"    => get_offline_time(pool, auth, &cmd, cache_arc).await,
		"remindme"       => add_reminder(pool, auth, cache_arc, &cmd, true).await,
		"remind"         => add_reminder(pool, auth, cache_arc, &cmd, false).await,
		"giveup"         => give_up_trivia(&cmd, auth,ongoing_trivia_games_arc).await,
		"hint"           => give_trivia_hint(&cmd, auth, ongoing_trivia_games_arc).await,
		"wordratio"      => get_word_ratio(pool, auth, &cmd, config.prefix, cache_arc).await,
		"commands"       => get_commands_reference_link(&config.commands_reference_path).await,
		"trivia"         => attempt_start_trivia_game(&cmd, auth, ongoing_trivia_games_arc).await,
		"rose"           => tag_rand_chatter_with_rose(&cmd.channel.name, &config.disregarded_users).await,
		"demultiplex"    => demultiplex(pool, client.clone(), config, auth, cache_arc, &cmd, ongoing_trivia_games_arc).await,
		"bench"          => bench_command(pool, client.clone(), config, auth, cache_arc, &cmd, ongoing_trivia_games_arc).await,
		// special commands
		"pipe"           => pipe(pool, client.clone(), config, auth, cache_arc, &cmd, ongoing_trivia_games_arc).await,
		""               => execute_alias(pool, client.clone(), config, auth, cache_arc, &cmd, ongoing_trivia_games_arc).await,
		_                => try_execute_channel_command(pool, &cmd).await,
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
		pool,
		&cmd,
		now.elapsed(),
		if let Some(s) = &cmd_out {s} else {""}
	).await {
		Ok(_) => (),
		Err(e) => println!("{e}")
	};
	
	if let Some(output) = cmd_out {
		// twitch generally doesn't allow awfully long messages
		let out = {
			if output.len() > 500 {
				(&output[..500]).into()
			} else {
				output
			}
		};

		client.say(cmd.channel.name.to_owned(), out).await.unwrap();
	}

	None
}

async fn get_commands_reference_link(link: &str) -> anyhow::Result<Option<String>> {
	Ok(Some(format!("🛠️ {link}")))
}


// ping -> pong
async fn ping(
	config: &Config,
)
-> anyhow::Result<Option<String>> {
	let mut out = String::from("Pong!");

	if let Ok(startup_time) = std::env::var("STARTUP_TIME") {
		let naive_startup = NaiveDateTime::parse_from_str(&startup_time[..startup_time.len()-17], "%Y-%m-%d %H:%M:%S").unwrap();
		let parsed_startup: DateTime<Local> = Local.from_local_datetime(&naive_startup).unwrap();
		let dur = chrono::Local::now() - parsed_startup;
		
		out.push_str(&format!(" | uptime: {}", fmt_duration(dur, false)));
	}

	if let Some(url) = &config.github_repo_api_path {
		let repo = api::get_github_repo_info(url).await?;

		let dur_since_update = Utc::now() - repo.pushed_at;

		out.push_str(&format!(" | last update: {} ago", fmt_duration(dur_since_update, false)));
	}

	Ok(Some(out))
}

// say whatever caller said
async fn echo(
	cmd: &CommandSource,
) -> anyhow::Result<Option<String>> {
	if cmd.sender.is_mvb() {
		Ok(Some(cmd.args.join(" ")))
	} else {
		Ok(Some("❌ requires MVB privileges | E4".into()))
	}
}

// get age of specified account (or called)
async fn get_accage(
	twitch_auth: &TwitchAuth,
	cmd:         &CommandSource,
	name_id_cache_arc: Arc<Mutex<NameIdCache>>,
) -> anyhow::Result<Option<String>> {
	let (user, _) = match cmd.user_channel_info_from_args(twitch_auth, name_id_cache_arc).await {
		Ok(a) => a,
		Err(e) => return Ok(Some(e.to_string())),
	};

	match api::get_acc_creation_date(&user.name, twitch_auth).await? {
		Some(date) => {
			let duration = (Utc::now() - date).num_days();
			let years = duration as f32 / 365.2425;

			if years > 0.5 {
				return Ok(Some(format!("⏱️ {}'s account is {:.2} years old", user.name, years)));
			} else {
				return Ok(Some(format!("⏱️ {}'s account is {duration} days old", user.name)));
			}
		},
		None       => Ok(Some("❌ user not found".into())),
	}
}

// allows for user to add a new alias for themselves
async fn set_alias(
	pool: &SqlitePool,
	cmd: &CommandSource,
) -> anyhow::Result<Option<String>> {
	let alias = match cmd.args.get(0) {
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
	ongoing_trivia_games_arc: Arc<Mutex<crate::OngoingTriviaGames>>,
) -> anyhow::Result<Option<String>> {
	let alias = match cmd.args.get(0) {
		Some(a) => a.clone(),
		None => return Ok(Some("❌ missing alias name".into())),
	};

	let alias_cmd = match db::get_alias_cmd(pool, cmd.sender.id, &alias).await? {
		Some(alias) => alias
			.split(' ')
			.map(|a| a.to_string())
			.collect::<Vec<String>>(),
		None => return Ok(Some("❌ alias not recognized".into())),
	};

	let new_cmd = CommandSource {
		cmd: match alias_cmd.get(0) {
			Some(a) => a[1..].to_owned(),
			None => return Ok(Some("❌ alias faulty".into())),
		},
		args: match alias_cmd.get(1) {
			Some(_) => alias_cmd[1..].to_vec(),
			None => vec![],
		},
		channel: cmd.channel.clone(),
		sender: cmd.sender.clone(),
		timestamp: cmd.timestamp,
	};

	handle_command(pool, client, config, auth, cache_arc, new_cmd, false, ongoing_trivia_games_arc.clone()).await;

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
		0 => Ok(Some("❌ no such alias".into())),
		_ => Ok(Some("✅ alias removed".into())),
	}
}

// parse the incoming duration identifying string
// expected input: (xh,xm) 
fn parse_duration_to_hm(s: &str) -> anyhow::Result<(i64, i64)> {
	let hrs  = s[s.find('(').ok_or(MyError::NotFound)?+1..s.find('h').ok_or(MyError::NotFound)?].to_owned().parse()?;
	let mins = s[s.find(',').ok_or(MyError::NotFound)?+1..s.find('m').ok_or(MyError::NotFound)?].to_owned().parse()?;

	Ok((hrs, mins))
} 

// add a reminder for someone
async fn add_reminder(
	pool:        &SqlitePool,
	auth:        &TwitchAuth,
	cache_arc:   Arc<Mutex<NameIdCache>>,
	cmd:         &CommandSource,
	is_for_self: bool,
) -> anyhow::Result<Option<String>> {
	if cmd.args.is_empty() {
		return Ok(Some("❌ insufficient args".into()));
	}

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
		if let Some(id) = cache.get(to_user_name) {
			for_user_id = Some(*id);
		}
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
		Ok(Some("❌ no reminders set, nothing happened".into()))
	} else {
		Ok(Some(format!("✅ cleared {delete_count} reminders")))
	}
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

	// clamp the number of rounds to be <= 100
	let rounds = {
		if rounds > 100 { 100 } else { rounds }
	};

	let mut output: Vec<String> = vec![cmd.args[0].clone()];
	let mut seed: String = cmd.args[0].clone();

	for _ in 0..rounds-1 {
		let succ = match db::get_rand_markov_succ(pool, cmd.channel.id, &seed).await {
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
		Some(expl) => Ok(Some(expl)),
		None =>       Ok(Some("❌ no such explanation".into()))
	}
}

// returns the first (logged) message of a user
async fn first_message(
	pool:              &SqlitePool,
	twitch_auth:       &TwitchAuth,
	name_id_cache_arc: Arc<Mutex<NameIdCache>>,
	cmd:               &CommandSource,
) -> anyhow::Result<Option<String>> {
	let (user, channel) = match cmd.user_channel_info_from_args(twitch_auth, name_id_cache_arc).await {
		Ok(a) => a,
		Err(e) => return Ok(Some(e.to_string())),
	};
	let message = db::get_first_message(pool, user.id, channel.id).await?;

	match message {
		Some(msg) => Ok(Some(msg)),
		None      => Ok(Some("❌ nothing found | E2".into())),
	}
}

// user can leave a suggestion, that will
// get saved into the database
async fn suggest(
	pool: &SqlitePool,
	cmd:  &CommandSource,
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
	channel_name:      &str,
	disregarded_users: &[String],
) -> anyhow::Result<Option<String>> {
	let chatters = match api::get_chatters(channel_name).await? {
		Some(chatters) => chatters,
		None           => return Ok(Some("❌ no users in the chatroom".into())),
	};

	let mut rand_chatter = "".to_string();

	while rand_chatter.is_empty() {
		let try_rand_chatter = chatters[rand::thread_rng().gen_range(0..chatters.len())].clone();
	
		if !disregarded_users.contains(&try_rand_chatter.to_lowercase()) {
			rand_chatter = try_rand_chatter;
		}
	}
	
	Ok(Some(format!("@{rand_chatter} PeepoGlad 🌹")))
}

// get weather report from wttr.in API
async fn get_weather_report(
	args: &[String],
) -> anyhow::Result<Option<String>> {
	if args.is_empty() {
		return Ok(Some("❌ no location provided".into()));
	}

	let location = args.join(" ");

	match api::get_weather_report(&location).await? {
		Some(r) => Ok(Some(r)),
		None    => Ok(Some("❌ location not identified".into())),
	}
}

// get uptime of a stream
async fn get_uptime(
	auth:              &TwitchAuth,
	cmd:               &CommandSource,
	name_id_cache_arc: Arc<Mutex<NameIdCache>>,
) -> anyhow::Result<Option<String>> {
	let channel_name = match cmd.args.get(0) {
		Some(nick) => nick,
		None       => &cmd.channel.name,
	};

	let info = match api::get_stream_info(auth, channel_name).await? {
		Some(i) => i,
		None    => return Ok(Some("❌ streamer not live".into())),
	};
	let duration = Utc::now() - info.data[0].started_at;
	
	let formatted = fmt_duration(duration, false);

	Ok(Some(format!("⏱️ {channel_name} has been live for {formatted}")))
}

// the language identifiers
// expected input: (l1,l2) 
fn parse_langs(s: &str) -> anyhow::Result<(&str, &str)> {
	Ok((
		&s[s.find('(').ok_or(MyError::NotFound)?+1..s.find(',').ok_or(MyError::NotFound)?],
		&s[s.find(',').ok_or(MyError::NotFound)?+1..s.find(')').ok_or(MyError::NotFound)?],
	))
} 

// go into AFK state
async fn set_lurk_status(
	pool: &SqlitePool,
	cmd:  &CommandSource,
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
	cache_arc:  Arc<Mutex<NameIdCache>>,
	cmd:        &CommandSource,
	ongoing_trivia_games_arc: Arc<Mutex<crate::OngoingTriviaGames>>,
) -> anyhow::Result<Option<String>> {
	let new_cmd = CommandSource {
		cmd: match cmd.args.get(0) {
			Some(a) => a[1..].to_owned(),
			None => return Ok(Some("❌ no command provided".into())),
		},
		args: match cmd.args.get(1) {
			Some(_) => cmd.args[1..].to_vec(),
			None => vec![],
		},
		channel: cmd.channel.clone(),
		sender: cmd.sender.clone(),
		timestamp: cmd.timestamp,
	};

	let now = Instant::now();
	handle_command(pool, client, config, auth, cache_arc, new_cmd, true, ongoing_trivia_games_arc.clone()).await;
	Ok(Some(format!("📡 {} ms", now.elapsed().as_millis())))
}

// get the time a user has spent in an offline chat
async fn get_offline_time(
	pool: &SqlitePool,
	twitch_auth: &TwitchAuth,
	cmd:  &CommandSource,
	name_id_cache_arc: Arc<Mutex<NameIdCache>>,
) -> anyhow::Result<Option<String>> {
	let (user, channel) = match cmd.user_channel_info_from_args(twitch_auth, name_id_cache_arc).await {
		Ok(a) => a,
		Err(e) => return Ok(Some(e.to_string())),
	};

	let t = db::get_offline_time(pool, channel.id, user.id).await?;
	Ok(Some(format!("{} has spent {} in {}'s offline chat!", user.name, channel.name, fmt_duration(t, false))))
}

// get the abstract from a wikipedia page
async fn query_wikipedia(
	cmd: &CommandSource,
) -> anyhow::Result<Option<String>> {
	let title = &cmd.args.join(" ");

	match api::query_wikipedia(title).await? {
		Some(mut w) => {
			if let Some((_, page)) = w.query.pages.iter_mut().next() {
				let abs = page
					.extract
					.split('.').collect::<Vec<&str>>()[0];

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
	cmd:         &CommandSource,
	twitch_auth: &TwitchAuth,
	name_id_cache_arc: Arc<Mutex<NameIdCache>>,
) -> anyhow::Result<Option<String>> {
	let (user, channel) = match cmd.user_channel_info_from_args(twitch_auth, name_id_cache_arc).await {
		Ok(a) => a,
		Err(e) => return Ok(Some(e.to_string())),
	};

	match api::get_followage(twitch_auth, channel.id, user.id).await? {
		Some(date) => {
			let duration = Utc::now() - date;
			let years = duration.num_days() as f32 / 365.2425;

			if years > 0.5 {
				return Ok(Some(format!("⏱️ {} has been following {} for {years:.2} years", user.name, channel.name)));
			} else {
				return Ok(Some(format!("⏱️ {} has been following {} for {}", user.name, channel.name, fmt_duration(duration, false))));
			}
		},
		None       => return Ok(Some(format!("❌ {} does not follow {}", user.name, channel.name))),
	}
}

async fn new_cmd(
	pool: &SqlitePool,
	cmd:  &CommandSource,
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

	db::new_cmd(pool, cmd.channel.id, cmd_name, cmd_type, &cmd_expr).await?;

	Ok(Some("🔧 command created successfully".into()))
}

pub async fn try_execute_channel_command(
	pool: &SqlitePool,
	cmd:  &CommandSource,
) -> anyhow::Result<Option<String>> {
	let cmd_name = cmd.cmd.as_str();

	let (cmd_type, cmd_expr, cmd_meta) = match db::get_channel_cmd(pool, cmd.channel.id, cmd_name).await? {
		Some(cmd) => cmd,
		None => return Ok(None),
	};

	let mut out = cmd_expr;

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
	cmd:  &CommandSource,
) -> anyhow::Result<Option<String>> {
	let cmd_name = match cmd.args.get(0) {
		Some(a) => a,
		None => return Ok(Some("❌ no command name provided".into()))
	};

	match db::remove_channel_command(pool, cmd.channel.id, cmd_name).await? {
		0 => Ok(Some("❌ no such command existed".into())),
		_ => Ok(Some("✅ removed successfully".into())),
	}
}

pub async fn get_word_ratio(
	pool:   &SqlitePool,
	auth:   &TwitchAuth,
	cmd:    &CommandSource,
	cmd_prefix: char,
	name_id_cache_arc: Arc<Mutex<NameIdCache>>,
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
				db::get_word_ratio(pool, cmd.channel.id, user_id, word, cmd_prefix).await? * 100.,
			)
		)
	)
}

// parses the args into a list of (comma-separated) decisions,
// choses one of them at random and returns it
async fn decide(
	cmd: &CommandSource,
) -> anyhow::Result<Option<String>> {
	match cmd.args.len() {
		0 => Ok(Some("❌ no options provided".into())),
		_ => {
			let mut options: Vec<String> = cmd.args
				.join(" ")
				.split(',')
				.map(|a| a.to_owned())
				.collect();
			
			if options.len() == 1 {
				options = cmd.args
				.join(" ")
				.split("or")
				.map(|a| a.to_owned())
				.collect();
			}

			// if the text user sent doesn't have any 'or's, then
			// try to see if the message starts with 'is' or 'does'
			// if so, process it as a yes/no
			if options.is_empty() || options.len() == 1 {
				match cmd.args[0].to_lowercase().as_str() {
					"is"     => (),
					"does"   => (),
					"will"   => (),
					"should" => (),
					"do"     => (),
					_ => return Ok(Some("❌ prompt not recognized".into()))
				}

				match rand::thread_rng().gen_range(0..2) {
					0 => return Ok(Some("🎱 No, I don't think so".into())),
					_ => return Ok(Some("🎱 Yes, I do think so".into())),
				}
			}

			let rand_opt = options[
				rand::thread_rng().gen_range(0..options.len())
			].clone();

			Ok(Some(format!("🎱 I choose... {rand_opt}")))
		}
	}
}

// chain commands via |
async fn pipe(
	pool:      &SqlitePool,
	client:    TwitchClient,
	config:    &Config,
	auth:      &TwitchAuth,
	cache_arc: Arc<Mutex<NameIdCache>>,
	cmd:       &CommandSource,
	ongoing_trivia_games_arc: Arc<Mutex<crate::OngoingTriviaGames>>,
) -> anyhow::Result<Option<String>> {
	// the command is supposed to be of the form
	// $pipe <command1 + command1 args> | <command2 + command3 args> | ...
	// therefore we parse the command into each individual commands and
	// execute them one by one
	let commands: Vec<String> = cmd.args
		.join(" ")
		.split('|')
		.map(|a| a.trim().to_owned())
		.collect();

	if commands.len() < 2 {
		return Ok(Some("❌ no command to pipe".into()));
	}

	let mut temp_output = String::new();
	for (i, _cmd) in commands.iter().enumerate() {
		let trimmed_cmd: Vec<String> = _cmd
			.trim()
			.to_string()
			.split(' ')
			.map(|a| a.to_owned())
			.collect();

		let new_cmd = CommandSource {
			cmd: match trimmed_cmd.get(0) {
				Some(a) => a[1..].to_owned(),
				None    => return Ok(Some(format!("❌ {}th pipe faulty", i+1))),
			},
			args: match trimmed_cmd.get(1) {
				Some(_) => trimmed_cmd[1..].to_vec(),
				None    => vec![],
			},
			channel:   cmd.channel.clone(),
			sender:    cmd.sender.clone(),
			timestamp: cmd.timestamp,
		};

		// these are some special ad hoc commands
		// that may only be used in pipes
		match _cmd.as_str() {
			"pastebin"  => { temp_output = api::upload_to_pastebin(&temp_output).await?; continue },
			"lower"     => { temp_output = temp_output.to_lowercase()                  ; continue },
			"upper"     => { temp_output = temp_output.to_uppercase()                  ; continue },
			"stdout"    => {                                                           ; continue },
			"/dev/null" => { temp_output = "".to_string()                              ; continue },
			"devnull"   => { temp_output = "".to_owned()                               ; continue },
			// "pm"        => { temp_output = format!("/w {} {temp_output}", cmd.sender.name); continue },
			_           => (),
		}

		if let Some(output) = handle_command(pool, client.clone(), config, auth, cache_arc.clone(), new_cmd, true, ongoing_trivia_games_arc.clone()).await {
			temp_output = output;
		} else {
			temp_output = "".to_owned();
		}
	}

	Ok(Some(temp_output))
}

// fetch a post from reddit
async fn get_reddit_post(
	cmd: &CommandSource,
) -> anyhow::Result<Option<String>> {
	let subr = match cmd.args.len() {
		0 => return Ok(Some("❌ no subreddit provided".into())),
		_ => {
			let mut s = cmd.args[0].clone();

			if &s[0..2] == "r/" {
				s = s[2..].to_string()
			}

			s
		}
	};

	let relevancy  = api::RedditPostRelevancy::new_from_vec(&cmd.args);
	let post_type  = api::RedditPostType::new_from_vec(&cmd.args);
	let add_params = api::AdditionalRedditParameter::new_from_vec(&cmd.args);

	let mut posts = api::get_reddit_posts(&subr, &relevancy)
		.await?
		.data
		.children;

	match posts.len() {
        0 => return Ok(Some(format!("❌ r/{subr} has no posts in in selection \'{}\'", relevancy.as_str()))),
		_ => {

			if add_params.contains(&api::AdditionalRedditParameter::HasMedia) {
				posts = posts
					.into_iter()
					.filter(|post|
						post.data.url.contains(".png")  ||
						post.data.url.contains(".jpg")  ||
						post.data.url.contains(".gif")  ||
						post.data.url.contains(".webp") ||
						post.data.url.contains(".webm") ||
						post.data.url.contains(".mp4")
					)
					.collect();
				
				if posts.is_empty() {
					return Ok(Some(format!("❌ no post containing media in selection \'{}\'", relevancy.as_str())));
				}
			}

			match post_type {
				api::RedditPostType::MostUpvotes => {
					let title    = &posts[0].data.title;
					let selftext = match &posts[0].data.selftext[..] {
						"" => format!(": {}", &posts[0].data.selftext),
						_  => "".into(),
					};
					let url      = &posts[0].data.url;

					return Ok(Some(format!("{title}{selftext} [ {url} ]")));
				},
				api::RedditPostType::Random => {
					let rand_post = posts[rand::thread_rng().gen_range(0..posts.len())].clone();

					let title    = rand_post.data.title;
					let selftext = match &rand_post.data.selftext[..] {
						"" => format!(": {}", rand_post.data.selftext),
						_  => "".into(),
					};
					let url      = rand_post.data.url;

					return Ok(Some(format!("{title}{selftext} [ {url} ]")));
				},
			}
		},
    }
}

// get local time of a specified location
pub async fn get_time(
	cmd: &CommandSource,
) -> anyhow::Result<Option<String>> {
	let location = match cmd.args.len() {
		0       => return Ok(Some("❌ no location provided".into())),
		_       => cmd.args.join(" "),
	};

	match api::get_time(&location).await? {
		Some(a) => Ok(Some(a)),
		None    => Ok(Some("❌ The location was not found".into()))
	}
}

// get a random verse from the Quran / the Bible / the Tanakh
pub async fn get_rand_holy_book_verse(
	book_kind: api::HolyBook,
) -> anyhow::Result<Option<String>> {
	let holy_book = api::get_rand_holy_book_verse(book_kind).await?;

	let book        = holy_book.book;
	let text        = holy_book.text;
	let book_number = match holy_book.book_number {
		Some(b) => format!(", book {b}"),
		None    => "".into(),
	};
	let chapter     = holy_book.chapter;

	Ok(Some(format!("({book}{book_number} ch. {chapter}) {text}")))
}

// start a trivia game (if one is not going on)
pub async fn attempt_start_trivia_game(
	cmd:                      &CommandSource,
	twitch_auth:              &TwitchAuth,
	ongoing_trivia_games_arc: Arc<Mutex<crate::OngoingTriviaGames>>,
) -> anyhow::Result<Option<String>, anyhow::Error> {
	let channel_id = cmd.channel.id;
	
	let cat = api::TriviaCategory::from_vec(&cmd.args);
	let dif = api::TriviaDifficulty::from_vec(&cmd.args);
	let typ = api::TriviaType::from_vec(&cmd.args);
	
	let question = api::fetch_trivia_question(cat, dif, typ).await?;
	let fmt_answer = {
		let q = convert_from_html_entities(question.question);
		let c = convert_from_html_entities(question.correct_answer);
		let w = question
			.incorrect_answers
			.iter()
			.map(|a| convert_from_html_entities(a.to_owned()))
			.collect::<Vec<String>>();

		crate::TriviaGameInfo {
			question:       q,
			correct_answer: c,
			wrong_answers:  w,
		}
	};


	if let Ok(mut cache) = ongoing_trivia_games_arc.lock() {
		if (*cache).get(&channel_id.to_string()).is_some() {
			return Ok(Some("❌ there is currently a game going on!".into()));
		}

		(*cache).insert(channel_id.to_string(), fmt_answer.clone());

		Ok(Some(convert_from_html_entities(fmt_answer.question)))
	} else {
		Ok(Some("An internal error has occured".into()))
	}
}

// if there is a game going on in the chatroom, give it up
pub async fn give_up_trivia(
	cmd:                      &CommandSource,
	twitch_auth:              &TwitchAuth,
	ongoing_trivia_games_arc: Arc<Mutex<crate::OngoingTriviaGames>>,
) -> anyhow::Result<Option<String>> {
	let channel_id = cmd.channel.id;

	if let Ok(mut cache) = ongoing_trivia_games_arc.lock() {
		if let Some(trivia_info) = (*cache).get(&channel_id.to_string()) {
			let c = trivia_info.clone();
			(*cache).remove(&channel_id.to_string());
			return Ok(Some(format!("So bad LUL | The answer was \'{}\'", c.correct_answer)));
		}
		
		return Ok(Some("❌ there was no game going on LUL".into()));
	}
	
	Ok(Some("An internal error has occured".into()))
}

// get an answer to "any" question
pub async fn query(
	cmd: &CommandSource,
) -> anyhow::Result<Option<String>> {
	let query = match cmd.args.len() {
		0 => return Ok(Some("❌ no query in command arguments".into())),
		_ => cmd.args.join(" "),
	};

	let result = match api::query_generic(&query).await? {
		Some(s) => s,
		None    => return Ok(Some("❌ no answer could be found".into())),
	};

	Ok(Some(result))
}

// used to execute a command multiple times
pub async fn demultiplex(
	pool:      &SqlitePool,
	client:    TwitchClient,
	config:    &Config,
	auth:      &TwitchAuth,
	cache_arc: Arc<Mutex<NameIdCache>>,
	cmd:       &CommandSource,
	ongoing_trivia_games_arc: Arc<Mutex<crate::OngoingTriviaGames>>,
) -> anyhow::Result<Option<String>> {
	if !cmd.sender.is_mvb() {
		return Ok(Some("❌ requires MVB privileges | E4".into()))
	}

	let rounds;
	let new_args;

	match cmd.args.len() {
		0 => return Ok(Some("❌ insufficient args".into())),
		1 => return Ok(Some("❌ missing actual command".into())),
		_ => {
			match cmd.args[0].parse::<u8>() {
				Ok(n)  => {
					// clamp the number of iterations to be 1 <=< 10
					if n < 1 {
						return Ok(Some("❌ first arg should be a positive integer".into()));
					}

					rounds = if n < 51 { n } else { 50 };
					new_args = &cmd.args[1..];
				},
				Err(_) => return Ok(Some("❌ first arg should be a positive integer".into())),
			};
		}
	};

	let new_cmd = CommandSource {
		cmd: match new_args.get(0) {
			Some(a) => new_args[0][1..].to_owned(),
			None => return Ok(Some("❌ alias faulty".into())),
		},
		args: match new_args.get(1) {
			Some(_) => new_args[1..].to_vec(),
			None => vec![],
		},
		channel: cmd.channel.clone(),
		sender: cmd.sender.clone(),
		timestamp: cmd.timestamp,
	};

	let mut final_output = String::new();
	for _ in 0..rounds {
		let temp_out = handle_command(
			pool,
			client.clone(),
			config,
			auth, cache_arc.clone(),
			new_cmd.clone(),
			true,
			ongoing_trivia_games_arc.clone()
		).await;

		if let Some(o) = temp_out {
			final_output.push(' ');
			final_output.push_str(&o);
		}
	}

	Ok(Some(final_output))
}

pub async fn rand_int_from_range(
	cmd: &CommandSource,
) -> anyhow::Result<Option<String>> {
	let (min, max) = match cmd.args.len() {
		0 => return Ok(Some("❌ number expected".into())),
		1 => {
			let n = match cmd.args[0].parse::<i16>() {
				Ok(n)  => n,
				Err(_) => return Ok(Some("❌ number expected".into()))
			};

			(1, n)
		}
		_ => {
			let n1 = match cmd.args[0].parse::<i16>() {
				Ok(n)  => n,
				Err(_) => return Ok(Some("❌ number expected".into()))
			};

			let n2 = match cmd.args[1].parse::<i16>() {
				Ok(n)  => n,
				Err(_) => return Ok(Some("❌ number expected".into()))
			};

			(n1, n2)
		}
	};

	let number = rand::thread_rng()
		.gen_range(min..=max)
		.to_string();

	Ok(Some(number))
}

#[allow(non_ascii_idents)]
pub async fn get_rand_pasta()
-> anyhow::Result<Option<String>> {
	let raw: String = std::fs::read_to_string(
		std::path::Path::new("assets/copypastas.json")
	)?;
		
	let pastas: crate::api_models::CopypastaFileJSON = serde_json::from_str(&raw)?;
	let pastas = pastas.pastas;
	let rand_pasta = pastas[rand::thread_rng().gen_range(0..pastas.len())]
		.text
		.clone();

	Ok(Some((&rand_pasta[..]).to_owned()))
}
use std::str::FromStr;
// get the chat statistics of a channel
pub async fn get_chatstats(
	pool:        &SqlitePool,
	cmd:         &CommandSource,
	twitch_auth: &TwitchAuth,
) -> anyhow::Result<Option<String>> {
	let (period, mode) = match cmd.args.len() {
		0 => {
			let period = db::ChatStatPeriod::Alltime;
			let mode   = db::ChatStatsMode::Top(3);

			(period, mode)
		},
		1 => {
			let period = db::ChatStatPeriod::from_str(&cmd.args[0]).unwrap();
			let mode   = db::ChatStatsMode::Top(3);

			(period, mode)
		}
		_ => {
			let period = db::ChatStatPeriod::from_str(&cmd.args[0]).unwrap();
			let mode   = db::ChatStatsMode::from_cmd(cmd, twitch_auth).await?;

			(period, mode)
		}
	};

	if let db::ChatStatPeriod::ThisStream = period {
		if !api::streamer_is_live(twitch_auth, &cmd.channel.name).await? {
			return Ok(Some("❌ streamer is not live".into()))
		}
	}

	let stats = db::get_channel_chat_stats(pool, &cmd.channel, twitch_auth, period, mode).await?;

	let mut out = String::new();
	let mut place: u8 = 1;
	for stat in stats {
		let user_id = stat.0;
		let count = stat.1;

		let user_name = api::nick_from_id(user_id, twitch_auth).await?;

		out.push_str(&format!(" {place}. {user_name} ({count})"));
		place += 1;
	}

	Ok(Some(out))
}

async fn give_trivia_hint(
	cmd:                      &CommandSource,
	twitch_auth:              &TwitchAuth,
	ongoing_trivia_games_arc: Arc<Mutex<crate::OngoingTriviaGames>>,
) -> anyhow::Result<Option<String>> {
	let channel_id = cmd.channel.id;

	if let Ok(mut cache) = ongoing_trivia_games_arc.lock() {
		if let Some(trivia_info) = (*cache).get(&channel_id.to_string()) {
			let c = trivia_info
				.shuffled_answers()
				.iter()
				.map(|a| a.to_string())
				.collect::<Vec<String>>()
				.join("\", \"");
			return Ok(Some(format!("The options are: \"{}\"", c)));
		}

		return Ok(Some("❌ there is no game going on FeelsDankMan".into()));
	}
	
	Ok(Some("❌ an internal error has occured".into()))
}

// find when and where was specified user last seen
pub fn find_last_seen(
	cmd:         &CommandSource,
	twitch_auth: &TwitchAuth,
	config:      &Config
) -> anyhow::Result<Option<String>> {
	let (target_user_name, target_user_id) = match cmd.args.len() {
		0 => return Ok(Some("❌ provide a user that you want to find".into())),
		_ => {
			let user_name = &cmd.args[0];
			let user_id = api::id_from_nick(user_name, twitch_auth).await?;

			match user_id {
				Some(id) => (user_name, id)
				None     => return Ok(Some(format!("❌ user \'{user_name}\' doesn't exist found")))
			}
		}
	}

	let mut latest_timestamp: Option<DateTime> = None;
	let mut found_in_channel: Option<String>   = None;
	for channel_name in config.channels {
		let channel_id = api::id_from_nick(channel_name, twitch_auth).await?;
		let latest = db::latest_message_date(channel_id, target_user_id).await?;

		if Some(timestamp) = latest {
			if timestamp > latest_timestamp {
				latest_timestamp = Some(timestamp);
				found_in_channel = Some(channel_name.clone());
			}
		}
	}
	
	let duration = fmt_duration(Utc::now() - latest_timestamp, false)

	match latest_timestamp {
		Some(tm) => Ok(Some(format!("⌛ {target_user_name} was last seen {duration} in {found_in_channel}"))),
		None     => Ok(Some(format!("❌ {target_user_name} not found in records"))),
	}
}
