mod db;
mod commands;
mod api;
mod api_models;

use twitch_bot::{Config, CommandSource, MyError, TwitchAuth, NameIdCache};
use commands::handle_command;

use std::sync::{Arc, Mutex},

use dotenv::dotenv;
use sqlx::sqlite::SqlitePool;
use tokio_cron_scheduler::{JobScheduler, JobToRun, Job};
use tracing::{info, /* error, warn */};
use twitch_irc::login::StaticLoginCredentials;
use twitch_irc::{ClientConfig, SecureTCPTransport, TwitchIRCClient};
use twitch_irc::message::ServerMessage;

// path to database file
// for now, it is a singular file - 
// that might be subject to change
// in the future, idk
const DB_PATH: &str = "sqlite:db.db";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
	// load environment variables from `.env` file
	dotenv().ok();
	
	// init tracing subscriber
	// tracing_subscriber::fmt::init();
	
	// this will hold cached names and ids of users
	// to prevent flooding the Twitch API too much
	let mut name_id_cache = NameIdCache::new()
	
	// holding on to the ids would be dumb
	// it would make the amount of memory used
	// go up indefinitely during runtime + 
	// one might as well not use ids for user
	// identification at all and go by names
	// instead
	// therefore the cache is to be cleared
	// every, say, 5 minutes
	let mut sched = Arc::new(Mutex::new(JobScheduler::new()));

	// the format of these is as follows:
	// sec   min   hour   day of month   month   day of week   year
	// *     *     *      *              *       *             *
	sched.add(Job::new("* 1/5 * * * *", |uuid, l| {
		let cache_arc = name_id_cache.clone()
        if let Ok(mut cache) = cache.lock() {
			*cache.clear();
        };
    }).unwrap());

	// load all of the credentials and configurations
	let config = Config::from_config_file()
		.expect("Couldn't load config, aborting.");
	let auth = TwitchAuth::from_dotenv()
		.expect("Couldn't load Twitch credentials from .env");

	// instantiate database connection pool
    let pool = SqlitePool::connect(DB_PATH)
		.await
		.expect("Database connection could not be established, aborting.");

	// create all of that stuff necessary
	// to be present in database
	db::init_db(&pool)
		.await
		.expect("Database could not be set up, aborting.");

	// create database tables for channels in config
	// (if they do not already exist)
	for channel in &config.channels {
		db::try_create_tables_for_channel(&pool, channel)
			.await
			.expect(&format!("Could not create tables for channel \"{channel}\", aborting"));
	}

	// instantiate Twitch client
	let client_config: ClientConfig<StaticLoginCredentials> = ClientConfig::new_simple(
		StaticLoginCredentials::new(
			auth.nick.clone(),
			Some(auth.oauth.clone()),
		)
	);
    let (mut incoming_messages, client) =
        TwitchIRCClient::<SecureTCPTransport, StaticLoginCredentials>::new(client_config);

	// join all channels in config
	for channel in &config.channels {
		client.join(channel.into());
		info!("joined channel {channel}");
	}

    let message_listener_handle = {
		let auth = auth.clone();
		let cache_arc = Arc::clone(name_id_cache);

		tokio::spawn(async move {
			// capture incoming messages
			while let Some(message) = incoming_messages.recv().await {
				// privmsg == chat message
				if let ServerMessage::Privmsg(privmsg) = message {
					
					if config.disregarded_users.contains(&privmsg.sender.login) {
						continue;
					}
					
					// log chat messages into database
					// (messages by the bot itself are not here,
					//	, so that's taken care off)
					match db::log(&pool, &privmsg).await {
						Ok(_) => (),
						Err(e) => println!("{e}"),
					};
	
					// check if user has any reminders set for him
					let reminders = 
						db::check_for_reminders(
							&pool,
							privmsg.sender.id.parse::<i32>().unwrap(),
						).await.unwrap();
	
					if let Some(rs) = reminders {
						for r in &rs {
							let from_user = if r.from_user_id == r.for_user_id {
								"yourself".into()
							} else {
								api::nick_from_id(r.from_user_id, auth.clone())
								.await
								.unwrap()
							};
							
							let for_user = api::nick_from_id(r.for_user_id, auth.clone())
								.await
								.unwrap();
							
							client.clone().say(
								privmsg.source.params[0][1..].to_owned(),
								format!("@{for_user} 🔔🗨👤 {from_user}: {}", r.message)
							).await.unwrap();
						}
					}
	
					// if message is a command, handle it
					if privmsg.message_text.chars().nth(0).unwrap() == config.prefix {
						let cmd_src = CommandSource::from_privmsg(privmsg);
						handle_command(&pool, client.clone(), &config, &auth, cache_arc, cmd_src).await.unwrap();
					} else {
						// index for markov if enabled by config
						if config.index_markov {
							db::log_markov(&pool, &privmsg).await.unwrap();
						}
					}
				}
			}
		})
	};

	sched.start().await;
    message_listener_handle.await.unwrap();
    Ok(())
}
