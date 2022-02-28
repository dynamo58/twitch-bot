use crate::CommandSource;
use crate::db;

use sqlx::sqlite::SqlitePool;

type TwitchClient = twitch_irc::TwitchIRCClient<twitch_irc::transport::tcp::TCPTransport<twitch_irc::transport::tcp::TLS>, twitch_irc::login::StaticLoginCredentials>;

pub async fn handle_command(
	pool: &SqlitePool,
	client: TwitchClient,
	cmd: CommandSource
) -> anyhow::Result<()> {
	let cmd_out = match cmd.cmd.as_str() {
		"ping" => ping()?,
		"markov" => markov(&pool, &cmd).await?,
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

async fn markov(
	pool: &SqlitePool,
	cmd: &CommandSource
) -> anyhow::Result<Option<String>> {
	let mut output: Vec<String> = vec![cmd.args[0].clone()];
	let mut seed = cmd.args[0].clone();
	let rounds = &cmd.args[1].parse::<usize>()?;

	// TODO:
		// secure if number of rounds is to big for the dataset
		
		// convert to lowercase ascii; remove whitespaces + ... + dots and shit

		// provide feedback to user if there's invalid input / 
		// bot unable to generate

	for _ in 0..*rounds-1 {
		let succ = db::get_rand_markov_succ(pool, &cmd.channel, &seed).await?;

		seed = succ.clone();
		output.push(succ.to_owned());
	}

	Ok(Some(output.join(" ")))
}