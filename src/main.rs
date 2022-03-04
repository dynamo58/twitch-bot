mod commands;
mod db;

use twitch_bot::{Config, CommandSource, Sender, MyError};
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
    let pool = SqlitePool::connect(DB_PATH)
		.await
		.expect("Database connection could not be established, aborting.");

	db::init_db(&pool)
		.await
		.expect("Database could not be set up, aborting.");

	// create database tables for channels in config
	// (if they do not already exist)
	for channel in &config.channels {
		db::try_create_tables_for_channel(&pool, channel)
			.await
			.expect(&format!("Could not connect to channel {}, aborting", channel));
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
				if config.disregarded_users.contains(&privmsg.sender.login) {
					return;
				}

				// log chat messages into database
				// (messages by the bot itself are not here,
				//	, so that's taken care off)
				match db::log(&pool, &privmsg).await {
					Ok(_) => (),
					Err(e) => println!("{e}"),
				}

				// check if user has any reminders set for him
				let reminders =  db::check_for_reminders(&pool, &privmsg.sender.login).await.unwrap();

				if let Some(rs) = reminders {
					for r in &rs {
						client.clone().say(privmsg.source.params[0][1..].to_owned(), format!("@{}, you've got a reminder from {}: {}", r.for_user_name, r.from_user_name, r.message));
					}
				}


				// if message is a command, handle it
				if privmsg.message_text.chars().nth(0).unwrap() == config.prefix {
					let cmd_src = CommandSource::from_privmsg(privmsg);
					handle_command(&pool, client.clone(), cmd_src).await.unwrap();
				} else {
					// index for markov if enabled by config
					if config.index_markov {
						match db::log_markov(&pool, &privmsg).await {
							Ok(_) => (),
							Err(e) => println!("{e}"),
						};
					}
				}
			}
        }
    });

    join_handle.await.unwrap();
    Ok(())
}
