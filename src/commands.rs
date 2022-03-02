use crate::{CommandSource, Sender};
use crate::db;

use std::path::Path;

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
		// TODO: fix this possible runtime error; cba rn
		"explain" => explain(&cmd.args[1])?,
		"echo" => echo(&cmd.args[1])?,
		"remindme" => add_reminder(&cmd.sender),
		_ => None,
	};

	if let Some(output) = cmd_out {
		client.say(cmd.channel, output.into()).await.unwrap();
	}

	Ok(())
}

fn add_reminder(user: &Sender)
-> anyhow::Result<Option<String>> {
	
}

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
