use crate::{CommandSource, MyError};
use crate::db;

use std::path::Path;

use chrono::Duration;
use sqlx::sqlite::SqlitePool;

type TwitchClient = twitch_irc::TwitchIRCClient<twitch_irc::transport::tcp::TCPTransport<twitch_irc::transport::tcp::TLS>, twitch_irc::login::StaticLoginCredentials>;

// handle incoming commands
pub async fn handle_command(
	pool: &SqlitePool,
	client: TwitchClient,
	cmd: CommandSource
) -> anyhow::Result<()> {
	let cmd_out = match cmd.cmd.as_str() {
		"ping" => ping()?,
		"markov" => markov(&pool, &cmd).await?,
		"explain" => explain(&cmd.args[0])?,
		"echo" => echo(&cmd.args)?,
		"remind" => add_reminder(&pool, &cmd, false).await?,
		"remindme" => add_reminder(&pool, &cmd, true).await?,
		_ => None,
	};

	if let Some(output) = cmd_out {
		client.say(cmd.channel, output.into()).await.unwrap();
	}

	Ok(())
}

// \(xh,xm\) \[text\] 

fn parse_duration_to_hm(s: &String) -> anyhow::Result<(i64, i64)> {
	let hrs  = s[s.find('(').ok_or(MyError::NotFound)?+1..s.find('h').ok_or(MyError::NotFound)?].to_owned().parse()?;
	let mins = s[s.find(',').ok_or(MyError::NotFound)?+1..s.find('m').ok_or(MyError::NotFound)?].to_owned().parse()?;

	Ok((hrs, mins))
} 

async fn add_reminder(
	pool: &SqlitePool,
	cmd: &CommandSource,
	is_for_self: bool,
) -> anyhow::Result<Option<String>> {
	let (h, m) = match parse_duration_to_hm(&cmd.args[0]) {
		Ok(a) => a,
		Err(_) => return Ok(Some("Bad time formatting.".into()))
	};

	let remind_time = cmd.timestamp + Duration::hours(h) + Duration::minutes(m);
	
	let to_user_name = match is_for_self {
		true => &cmd.sender.name,
		false => match cmd.args.get(1).ok_or(MyError::OutOfBounds) {
			Ok(a) => a,
			Err(_) => return Ok(Some("No name provided.".into())),
		}
	};

	let message = match cmd.args.get(2).ok_or(MyError::OutOfBounds) {
		Ok(_) => cmd.args[2..].join(" "),
		Err(_) => return Ok(Some("No message provided.".into())),
	};

	let reminder = db::Reminder {
		// dummy
		id: 0,
		from_user_name: cmd.sender.name.clone(),
		for_user_name: to_user_name.clone(),
		raise_timestamp: remind_time,
		message: message,
	};

	db::insert_reminder(pool, &reminder).await?;

	Ok(Some("Reminder set successfully.".into()))
}

// async fn remove_reminder(
// 	pool: &SqlitePool,
// 	id: i32,
// ) -> anyhow::Result<Option<String>> {

// }

fn ping()
-> anyhow::Result<Option<String>> {
	Ok(Some("pong".into()))
}

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
		0 => return Ok(Some("Insufficient args".into())),
		// if number of rounds isn't set, set to default
		1 => {rounds = 7},
		// else parse both arguments
		_ => 
			match cmd.args[1].parse::<usize>() {
				Ok(num) => {rounds = num},
				Err(_)  => return Ok(Some("Invalid length argument, use a positive integer.".into())),
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
		return Ok(Some("That word has not been indexed yet | E1".into()));
	}

	Ok(Some(output.join(" ")))
}

fn explain (error_code: &str) -> anyhow::Result<Option<String>> {
	let err_explanation = std::fs::read_to_string(Path::new(&format!("assets/explanations/{}.txt", error_code)));

	match err_explanation {
		Ok(expl) => return Ok(Some(expl)),
		Err(_) => return Ok(Some("No such error code".into()))
	}
}
