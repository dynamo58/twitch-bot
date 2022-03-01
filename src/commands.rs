use crate::CommandSource;
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
		"explain" => explain(cmd.arg[1]),
		_ => None,
	};

	if let Some(output) = cmd_out {
		client.say(cmd.channel, output.into()).await.unwrap();
	}

	Ok(())
}

fn ping() -> anyhow::Result<Option<String>> {
	Ok(Some("pong".into()))
}

// return a markov chain of words
async fn markov(
	pool: &SqlitePool,
	cmd: &CommandSource
) -> anyhow::Result<Option<String>> {
	let mut output: Vec<String> = vec![cmd.args[0].clone()];
	let mut seed = cmd.args[0].clone();
	let rounds = &cmd.args[1].parse::<usize>()?;

	for _ in 0..*rounds-1 {
		let succ = match db::get_rand_markov_succ(pool, &cmd.channel, &seed).await {
			Ok(successor) => successor,
			Err(_) => break,
		};

		seed = succ.clone();
		output.push(succ.to_owned());
	}

	if output.len() == 1 {
		Ok(Some("That word has not been indexed yet | E1".into()))
	}

	Ok(Some(output.join(" ")))
}

fn explain (error_code: &str) -> anyhow::Result<Option<String>> {
	let err_explanation = fs::read_to_string(Path::new(&format!("assets/explanations/{}.txt", error_code)))?;
	// TODO: !!

	Ok(err_explanation)
}