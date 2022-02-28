mod commands;
mod db;

use twitch_bot::{Config, CommandSource};
use commands::handle_command;
use sqlx::sqlite::SqlitePool;
use dotenv::dotenv;

// use tracing::{info, error, warn};
// use tracing_subscriber;

use twitch_irc::login::StaticLoginCredentials;
use twitch_irc::{ClientConfig, SecureTCPTransport, TwitchIRCClient};
use twitch_irc::message::ServerMessage;

// use tokio_cron_scheduler::{JobScheduler, JobToRun, Job};

const DB_PATH: &str = "sqlite:db.db";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
	// load environment variables from `.env` file
	dotenv().ok();
	// instantiate traces
	// tracing_subscriber::fmt::init();

	// instantiate running config from `config.json`
	let config = match Config::from_config_file() {
		Ok(conf) => conf,
		Err(_) => panic!("Couldn't load config, aborting."),
	};
	// instantiate database connection pool
    let pool = SqlitePool::connect(DB_PATH).await?;

	// create database tables for channels in config
	// (if they do not already exist)
	for channel in &config.channels {
		db::try_create_tables_for_channel(&pool, channel).await?;
	}

	let twitch_nick = std::env::var("TWITCH_NICK").expect("Twitch nick is missing in .env").clone();
	let twitch_oauth = std::env::var("TWITCH_OAUTH").expect("Twitch OAuth is missing in .env").clone();

	// instantiate Twitch client
	let client_config: ClientConfig<StaticLoginCredentials> = ClientConfig::new_simple(
		StaticLoginCredentials::new(
			twitch_nick,
			Some(twitch_oauth),
		)
	);
    let (mut incoming_messages, client) =
        TwitchIRCClient::<SecureTCPTransport, StaticLoginCredentials>::new(client_config);

	// join all channels in config
	for channel in &config.channels {
		client.join(channel.into());
	}

    let join_handle = tokio::spawn(async move {
		// capture incoming messages
        while let Some(message) = incoming_messages.recv().await {
			// privmsg == chat message
			if let ServerMessage::Privmsg(privmsg) = message {
				// log chat messages into database

				// TODO: 
				// check whether these include messages of the bot itself,
				// if so, get rid of them
				db::log(&pool, &privmsg).await.unwrap();
				db::log_markov(&pool, &privmsg).await.unwrap();

				// if message is a command, handle it
				if privmsg.message_text.chars().nth(0).unwrap() == config.prefix {
					let cmd_src = CommandSource::from_privmsg(privmsg);
					handle_command(&pool, client.clone(), cmd_src).await.unwrap();
				}
			}
        }
    });

    join_handle.await.unwrap();
    Ok(())
}
