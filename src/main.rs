mod db;
mod commands;
mod api;
mod api_models;

use twitch_bot::{Config, CommandSource, MyError, TwitchAuth, NameIdCache, EmoteCache};
use commands::handle_command;

use std::sync::{Arc, Mutex};
use colored::*;
use chrono::Local;
use dotenv::dotenv;
use tokio::{self, sync::Mutex as TokioMutex};
use sqlx::sqlite::SqlitePool;
use tokio_cron_scheduler::{JobScheduler, Job};
// use tracing::{info, error, warn};
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

	// load all of the credentials and configurations
	let config = Config::from_config_file()
		.expect(&format!("{}   Couldn't load config, aborting.", "ERROR  ".red().bold()));
	let auth = TwitchAuth::from_dotenv()
		.expect(&format!("{}   Couldn't load Twitch credentials from .env", "ERROR  ".red().bold()));

	println!("{}   Obtained credentials and config from local files", "INFO   ".blue().bold());
	
	// this will hold cached names and ids of users
	// to prevent flooding the Twitch API too much
	let name_id_cache = Arc::new(Mutex::new(NameIdCache::new()));
	let name_id_cache_arc = name_id_cache.clone();
	
	let emote_cache = Arc::new(TokioMutex::new({
		match EmoteCache::init(&config, &auth).await {
			Ok(e) => e,
			Err(e) => panic!("{}", e),
		}
			
	}));

	// holding on to the data forever would be dumb;
	// it would make the amount of memory used
	// go up indefinitely during runtime + 
	// in the case of name-id cache,
	// one might as well not use ids for user
	// identification at all and go by names instead;
	// therefore the cache is to be cleared
	// every now and then
	let mut sched = JobScheduler::new();

	// the format of these is as follows:
	// sec   min   hour   day of month   month   day of week   year
	// *     *     *      *              *       *             *
	sched.add(Job::new("0 1/15 * * * *", move |_, _| {
        if let Ok(mut cache) = name_id_cache_arc.lock() {
			let num = cache.len();
			(*cache).clear();
			println!("{}   Cleared name-id cache ({} items)", "INFO   ".blue().bold(), num);
        }
    }).unwrap())
		.expect(&format!("{}   Setting up a scheduled task failed, but why?", "ERROR  ".red().bold()));
	
	// I was extensively trying to get the emote cache
	// to renew periodically via the tokio_cron_scheduler
	// API, but I could not figure it out, so I will leave
	// it blank for now....................................
	
	println!("{}   Set up scheduled tasks", "INFO   ".blue().bold());

	// instantiate database connection pool
    let pool = SqlitePool::connect(DB_PATH)
		.await
		.expect(&format!("{}   Database connection could not be established, aborting.", "ERROR  ".red().bold()));

	// create all of that stuff necessary
	// to be present in database
	db::init_db(&pool)
		.await
		.expect(&format!("{}   Database could not be set up, aborting.", "ERROR  ".red().bold()));

	// create database tables for channels in config
	// (if they do not already exist)
	for channel in &config.channels {
		db::try_create_tables_for_channel(&pool, channel)
			.await
			.expect(
				&format!(
					"{}   Could not create tables for channel \"{}\", aborting",
					"ERROR  ".red().bold(),
					channel.bold()
				)
			);
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
		println!("{}   Joined #{}", "INFO   ".blue().bold(), channel.bold());
	}

    let message_listener_handle = {
		let auth = auth.clone();
		let cache_arc = name_id_cache.clone();

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
						Err(e) => println!("{}   Uncaught error; message: {e}", "ERROR    ".red().bold()),
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
						handle_command(&pool, client.clone(), &config, &auth, cache_arc.clone(), cmd_src).await.unwrap();
					} else {
						// index for markov if enabled by config
						if config.index_markov {
							db::log_markov(&pool, &emote_cache, &privmsg).await.unwrap();
						}
					}
				}
			}
		})
	};

	let t = format!("{}", Local::now());
	println!(
		"{}   Bot is now running!\n          Local time is {}",
		"SUCCESS".green().bold(),
		&t[..t.len()-17]
	);
	std::mem::drop(t);
	sched.start().await.unwrap();
    message_listener_handle.await.unwrap();
    Ok(())
}
