mod commands;
mod db;

use twitch_bot::{Config, CommandSource};
use commands::handle_command;

use sqlx::sqlite::SqlitePool;

use dotenv::dotenv;
use std::env;

use twitch_irc::login::StaticLoginCredentials;
use twitch_irc::TwitchIRCClient;
use twitch_irc::{ClientConfig, SecureTCPTransport};
use twitch_irc::message::ServerMessage;

const DB_PATH: &str = "sqlite:db.db";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
	dotenv().ok();

	let config = match Config::from_config_file() {
		Ok(conf) => conf,
		Err(_) => panic!("Couldn't load config, aborting."),
	};
    let pool = SqlitePool::connect(DB_PATH).await?;

	for channel in &config.channels {
		db::try_create_table(&pool, channel).await?;
	}

	let twitch_nick = env::var("TWITCH_NICK").expect("Twitch nick is missing in .env").clone();
	let twitch_oauth = env::var("TWITCH_OAUTH").expect("Twitch OAuth is missing in .env").clone();

	let client_config: ClientConfig<StaticLoginCredentials> = ClientConfig::new_simple(
		StaticLoginCredentials::new(
			twitch_nick,
			Some(twitch_oauth),
		)
	);
    let (mut incoming_messages, client) =
        TwitchIRCClient::<SecureTCPTransport, StaticLoginCredentials>::new(client_config);

	for channel in &config.channels {
		client.join(channel.into());
	}

    let join_handle = tokio::spawn(async move {
        while let Some(message) = incoming_messages.recv().await {
			if let ServerMessage::Privmsg(privmsg) = message {
				db::log(&pool, &privmsg).await.unwrap();

				if privmsg.message_text.chars().nth(0).unwrap() == config.prefix {
					let cmd_src = CommandSource::from_privmsg(privmsg);
					handle_command(client.clone(), cmd_src).await.unwrap();
				}
			}
        }
    });

    join_handle.await.unwrap();
    Ok(())
}
