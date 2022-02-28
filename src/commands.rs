type TwitchClient = twitch_irc::TwitchIRCClient<twitch_irc::transport::tcp::TCPTransport<twitch_irc::transport::tcp::TLS>, twitch_irc::login::StaticLoginCredentials>;
use crate::CommandSource;

async fn handle_command
(client: TwitchClient, cmd: CommandSource)
-> anyhow::Result<()> {
	let cmd_out = match cmd.cmd.as_str() {
		"ping" => ping(),
		"markov" => markov(client, cmd)
		_ => None,
	};

	if let Some(output) = cmd_out {
		client.say(cmd.channel, output.into()).await.unwrap();
	}

	Ok(())
}

fn ping() -> Option<&'static str> {
	Some("pong")
}

async fn markov
(client: TwitchClient, cmd: CommandSource)
-> anyhow::Result<Option<&'static str>> {
	
}