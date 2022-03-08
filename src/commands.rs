use crate::{CommandSource, MyError};
use crate::db;
use crate::twitch_api;

use async_recursion::async_recursion;
use chrono::{Duration, Utc};
use sqlx::sqlite::SqlitePool;

type TwitchClient = twitch_irc::TwitchIRCClient<twitch_irc::transport::tcp::TCPTransport<twitch_irc::transport::tcp::TLS>, twitch_irc::login::StaticLoginCredentials>;

// handle incoming commands
#[async_recursion]
pub async fn handle_command(
	pool: &SqlitePool,
	client: TwitchClient,
	cmd: CommandSource
) -> anyhow::Result<()> {
	let cmd_out = match cmd.cmd.as_str() {
		"ping"           => ping(),
		"echo"           => echo(&cmd.args),
		"markov"         => markov(&pool, &cmd).await,
		"suggest"        => suggest(&pool, &cmd).await,
		"setalias"       => set_alias(&pool, &cmd).await,
		"rmalias"        => remove_alias(&pool, &cmd).await,
		"first"          => first_message(&pool, &cmd).await,
		"explain"        => explain(&pool, &cmd.args[0]).await,
		"remindme"       => add_reminder(&pool, &cmd, true).await,
		"remind"         => add_reminder(&pool, &cmd, false).await,
		"clearreminders" => clear_reminders(&pool, cmd.sender.id).await,
		"rmrm"           => clear_reminders(&pool, cmd.sender.id).await,
		"$"              => execute_alias(&pool, client.clone(), &cmd).await,
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

// allows for user to add a new alias for themselves
async fn set_alias(
	pool: &SqlitePool,
	cmd: &CommandSource,
) -> anyhow::Result<Option<String>> {
	let alias = match cmd.args.get(0) {
		Some(a) => a,
		None => return Ok(Some("❌ no alias name provided".into()))
	};

	let alias_cmd = match cmd.args.get(1) {
		Some(_) => cmd.args[1..].join(" "),
		None => return Ok(Some("❌ no alias command provided".into())),
	};

	db::set_alias(pool, &cmd.sender.name, &alias, &alias_cmd).await?;

	Ok(Some("✅ alias created".into()))
}

// run user's alias
async fn execute_alias(
	pool: &SqlitePool,
	client: TwitchClient,
	cmd: &CommandSource,
) -> anyhow::Result<Option<String>> {
	let alias = match cmd.args.get(0).clone() {
		Some(a) => a,
		None => return Ok(Some("❌ missing alias name".into())),
	};
	let owner = &cmd.sender.name;

	let alias_cmd = match db::get_alias_cmd(pool, &owner, &alias).await? {
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

	handle_command(pool, client, new_cmd).await?;

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

	match db::remove_alias(pool, &cmd.sender.name, &alias).await? {
		0 => return Ok(Some("❌ no such alias".into())),
		_ => return Ok(Some("✅ alias removed".into())),
	}

	todo!()
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
		Ok(_) => cmd.args[start_idx..].join(" "),
		Err(_) => return Ok(Some("❌ no message provided".into())),
	};

	let reminder = db::Reminder {
		// dummy
		id: 0,
		from_user_id: cmd.sender.id,
		// TODO: translate name to id
		for_user_id: todo!(),
		raise_timestamp: remind_time,
		message: message,
	};

	db::insert_reminder(pool, &reminder).await?;

	Ok(Some("✅ set successfully.".into()))
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

	let mut output: Vec<String> = vec![cmd.args[0].clone().to_lowercase()];
	let mut seed: String = cmd.args[0].clone().to_lowercase();

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
	pool: &SqlitePool,
	cmd:  &CommandSource,
) -> anyhow::Result<Option<String>> {
	let sender_id = match &cmd.args.get(0) {
		Some(name) => twitch_api::id_from_nick(),
		None => cmd.sender.id,
	};

	let channel = match &cmd.args.get(1) {
		Some(c) => c.clone(),
		None =>    &cmd.channel,
	};

	let message = db::get_first_message(pool, sender_id, channel).await?;

	match message {
		Some(msg) => return Ok(Some(msg)),
		None => return Ok(Some("❌ nothing found | E2".into())),
	}
}

pub async fn suggest(
	pool: &SqlitePool,
	cmd: &CommandSource,
) -> anyhow::Result<Option<String>> {
	let text = match &cmd.args.get(0) {
		Some(_) => cmd.args.join(" "),
		None => return Ok(Some("❌ no message".into())),
	};

	let sender_id = cmd.sender.id.parse::<i32>().unwrap();

	db::save_suggestion(pool, sender_id, &cmd.sender.name, &text, Utc::now()).await?;

	Ok(Some("✅ suggestion saved".into()))
}
