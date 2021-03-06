mod api;
mod api_models;
mod commands;
mod db;
mod background;

use twitch_bot::{
	Config,
	Channel,
	MyError,
	EmoteCache,
	TwitchAuth, 
	TwitchBadge,
	NameIdCache,
	CommandSource,
	TriviaGameInfo,
	ChannelSpecifics,
	ChannelSpecificsCache,
	MessageHook,
	HookMatchType,
	fmt_duration,
	convert_from_html_entities,
	convert_to_html_encoding,
	binomial_p_exact,
	binomial_p_exact_or_less,
};
use background as bg;
use commands::handle_command;

use std::sync::{Arc, Mutex};

use colored::*;
use chrono::Local;
use dotenv::dotenv;
use sqlx::sqlite::SqlitePool;
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



	// load all of the credentials and configurations
	let config = Config::from_config_file()
		.unwrap_or_else(|_| panic!("{}   Couldn't load config, aborting.", "ERROR  ".red().bold()));
	let auth = TwitchAuth::from_env()
		.unwrap_or_else(|_| panic!("{}   Couldn't load Twitch credentials from .env", "ERROR  ".red().bold()));

	println!("{}   Obtained credentials and config from local files", "INFO   ".blue().bold());



	// this will hold cached names and ids of users
	// to prevent flooding the Twitch API too much
	let name_id_cache = Arc::new(Mutex::new(NameIdCache::new()));

	let emote_cache = Arc::new(Mutex::new({
		match EmoteCache::init(&config, &auth).await {
			Ok(e) => e,
			Err(e) => panic!("{}", e),
		}
	}));

	// holds channel-specific information crucial for runtime
	let channel_specifics_arc = Arc::new(Mutex::new(ChannelSpecificsCache::new()));


	
	// instantiate database connection pool
    let pool = SqlitePool::connect(DB_PATH)
		.await
		.unwrap_or_else(|_| panic!("{}   Database connection could not be established, aborting.", "ERROR  ".red().bold()));



	// create all of that stuff necessary
	// to be present in database
	db::init_db(&pool)
		.await
		.unwrap_or_else(|_| panic!("{}   Database could not be set up, aborting.", "ERROR  ".red().bold()));

	// create database tables for channels in config
	// (if they do not already exist)
	let mut ids: Vec<i32> =  vec![];
	for channel in &config.channels {
		let channel_id = api::id_from_nick(channel, &auth)
			.await?
			.unwrap_or_else(||
				panic!(
					"{}   Channel \"{}\" wasn't found, aborting",
               		"ERROR  ".red().bold(),
               		channel.bold()
				)
			);

		db::try_create_tables_for_channel(&pool, channel_id)
			.await
			.unwrap_or_else(|_|
				panic!("{}   Could not create tables for channel \"{}\", aborting",
                    "ERROR  ".red().bold(),
                    channel.bold(),
				)
			);
		
		ids.push(channel_id);
	}
	println!("{}   Created tables in db", "INFO   ".blue().bold());


	if let Ok(mut cache) = channel_specifics_arc.lock() {
		for id in ids {
			let hooks = match db::get_channel_hooks(&pool, id).await? {
				Some(hs) => hs,
				None     => vec![],
			};

			cache.insert(id.to_string(), ChannelSpecifics {
				hooks,
				ongoing_trivia_game: None,
			});
		}
	} else {
		panic!(
			"{}   Could not initialize game specifics, aborting",
            "ERROR  ".red().bold()
		)
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



	// set up tasks running periodcally in thebackground
	{
		let _pool = pool.clone();
		let _config = config.clone();
		let _auth = auth.clone();
		let _name_id_cache = name_id_cache.clone();
		
		if config.track_offliners {
			tokio::spawn(async move {
				loop {
					match bg::check_for_offliners(&_pool, &_config, &_auth, &_name_id_cache).await {
						Ok(num)  => println!("{}   Checked for offliners ({} occurences)", "INFO   ".blue().bold(), format!("{}", num).bold()),
						Err(e)   => println!("{}   Error checking for offliners; err: {e}", "ERROR    ".red().bold()),
					}
					std::thread::sleep(std::time::Duration::from_secs(60));
				}
			});
		}

		let _name_id_cache = name_id_cache.clone();

		tokio::spawn(async move {
			loop {
				match bg::clear_name_id_cache(&_name_id_cache).await {
					Ok(num) => println!("{}   Cleared name-id cache ({} items)", "INFO   ".blue().bold(), num),
					Err(e)  => println!("{}   Error clearing name-id cache; err: {e}", "ERROR    ".red().bold()),
				}

				std::thread::sleep(std::time::Duration::from_secs(15 * 60));
			}
		});
	}
	println!("{}   Set up scheduled tasks", "INFO   ".blue().bold());



	// handle incoming messages
    let message_listener_handle = {
		let auth = auth.clone();
		let nameid_cache_arc = name_id_cache.clone();
		let emote_cache_arc = emote_cache.clone();
		let channel_specifics_arc = channel_specifics_arc.clone();

		tokio::spawn(async move {
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


					if let Some(duration) = db::is_lurker(
						&pool,
						privmsg.sender.id.parse::<i32>().unwrap()
					).await.unwrap() {
						client.say(
							privmsg.source.params[0][1..].to_owned(),
							format!("{} is no longer AFK ({})", privmsg.sender.name, fmt_duration(duration, false)),
						).await.unwrap();
					};

					// check if user has any reminders set for them
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
								api::nick_from_id(r.from_user_id, &auth)
								.await
								.unwrap()
							};
							
							let for_user = api::nick_from_id(r.for_user_id, &auth)
								.await
								.unwrap();
							
							client.clone().say(
								privmsg.source.params[0][1..].to_owned(),
								format!("@{for_user} ???????? {from_user}: {}", r.message)
							).await.unwrap();
						}
					}

					// if message is a command, handle it
					if privmsg.message_text.chars().next().unwrap() == config.prefix {
						let cmd_src = CommandSource::from_privmsg(privmsg.clone());
						handle_command(
							&pool,
							client.clone(),
							&config,
							&auth,
							nameid_cache_arc.clone(),
							cmd_src,
							channel_specifics_arc.clone()
						).await;
					} else {
						// index for markov if enabled by config
						if config.index_markov {
							db::log_markov(&pool, &emote_cache_arc, &privmsg).await.unwrap();
						}

						let channel_id = &privmsg.source.tags.0.get("room-id");
						if let Some(Some(room_id)) = channel_id {
							let mut correct = false;
							if let Ok(mut cache) = channel_specifics_arc.lock() {
								if
									(*cache).get(room_id).is_some() &&
									(*cache).get(room_id).unwrap().ongoing_trivia_game.is_some()
								{
									let trivia_info = &(*cache)
										.get(room_id)
										.unwrap()
										.ongoing_trivia_game
										.clone();

									if let Some(ti) = trivia_info {
										if ti.correct_answer.to_lowercase() == privmsg.message_text.to_lowercase() {
											(*cache).remove(room_id);
											correct = true;
										}
									}
								}
							}

							if correct {
								client.say(
									privmsg.source.params[0][1..].to_owned(),
									format!("@{} Correct!", privmsg.sender.name),
								).await.unwrap();
							}
						}
					}

					// check if message doesn't match any of the channel hooks
					let mut matches                      = false;
					let mut match_phrase: Option<String> = None;
					if let Ok(cache) = channel_specifics_arc.lock() {
						if (*cache).get(&privmsg.channel_id).is_some() {
							let hooks = (*cache).get(&privmsg.channel_id).unwrap().hooks.clone();
							
							for hook in &hooks {
								match hook.h_type {
									crate::HookMatchType::Substring => {
										if privmsg.message_text.to_lowercase().contains(&hook.capture_string.to_lowercase()) {
											matches = true;
											match_phrase = Some(hook.content.clone());
										}
									},
									crate::HookMatchType::Exact     => {
										if privmsg.message_text == hook.capture_string {
											matches = true;
											match_phrase = Some(hook.content.clone());
										}
									},
								}
							}
						}
					}

					if matches {
						client.say(
							privmsg.source.params[0][1..].to_owned(),
							match_phrase.unwrap(),
						).await.unwrap();
					}
				}
			}
		})
	};

	let t = format!("{}", Local::now());
	std::env::set_var("STARTUP_TIME", &t);

	println!(
		"{}   Bot is now running!\n          Local time is {}\n\n",
		"SUCCESS".green().bold(),
		&t[..t.len()-17]
	);

    message_listener_handle.await.unwrap();

    Ok(())
}