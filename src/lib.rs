pub mod commands;
pub mod db;

use std::collections::HashMap;
use std::path::Path;

use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};
use thiserror::Error;

// some custom errors (ad hoc)
#[derive(Error, Debug)]
pub enum MyError {
	#[error("index out of bounds")]
	OutOfBounds,
	#[error("item not found")]
	NotFound,
}

// twitch authentification credentials
#[derive(Serialize, Deserialize, Debug)]
pub struct TwitchAuth {
	pub client_id: String,
	pub auth: String,
	pub twitch_nick: String,
}

// config that directs how the bot works
// gets set up during runtime
#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
	pub channels: Vec<String>,
	pub disregarded_users: Vec<String>,
	pub prefix: char,
	pub index_markov: bool,
	pub auth: TwitchAuth,
}

impl Config {
	// parse from config file at `assets/config.json`
	pub fn new() -> anyhow::Result<Config> {
		let json: String = std::fs::read_to_string(Path::new("assets/config.json"))?;

		let oauth = std::env::var("TWITCH_OAUTH")
			.expect("Twitch OAuth is missing in .env");
		let client_id = std::env::var("TWITCH_CLIENT_ID")
			.expect("Twitch Client-ID is missing in .env");
		let twitch_nick = std::env::var("TWITCH_NICK")
			.expect("Twitch nick is missing in .env").clone();
		
		
		let json = format!(
			"{},
			\"auth\": {{
				\"client_id\": \"{client_id}\",
				\"oauth\": \"Bearer: {oauth}\",
				\"twitch_nick\": \"{twitch_nick}\"
			}} }}", &json[..json.len()-2]
		)

		let mut config: Config = serde_json::from_str(&json)?;

		config.disregarded_users = config.disregarded_users.iter().map(|user| user.to_lowercase()).collect(); 

		Ok(config)
	
	
	}
}

// All the statuses one can have in Twitch chat
#[derive(Clone)]
pub enum TwitchStatus {
	Broadcaster,
	Admin,
	GlobalMod,
	Mod,
	Staff,
	Subscriber,
	Vip,
	Premium,
}

#[derive(Clone)]
pub struct Sender {
	pub id: i32,
	pub name: String,
}

// the only info which is important and
// which the bot works with
pub struct CommandSource {
	pub cmd: String,
	pub args: Vec<String>,
	pub sender: Sender,
	pub channel: String,
	pub statuses: Vec<TwitchStatus>,
	pub timestamp: DateTime<Utc>,
}

impl CommandSource {
	// parse new from twitch_irc::message::PrivmsgMessage
	pub fn from_privmsg(privmsg: twitch_irc::message::PrivmsgMessage) -> Self {
		let mut args: Vec<String> = privmsg.message_text.split(" ").map(|arg| arg.to_owned()).collect();
		let cmd = args[0].to_lowercase()[1..].to_owned();
		args = args[1..].to_owned();

		let sender = Sender {
			                                    // will always be valid
			id: privmsg.sender.id.parse::<i32>().unwrap(),
			name: privmsg.sender.name
		};

		// parse badges
		let badges: Vec<TwitchStatus> = privmsg.badges.into_iter().map(|badge| match badge.name.as_str() {
			"admin" => TwitchStatus::Admin,
			"broadcaster" => TwitchStatus::Broadcaster,
			"global_mod" => TwitchStatus::GlobalMod,
			"moderator" => TwitchStatus::Mod,
			"staff" => TwitchStatus::Staff,
			"subscriber" => TwitchStatus::Subscriber,
			"vip" => TwitchStatus::Vip,
			"premium" => TwitchStatus::Premium,
			_ => {println!("{}", badge.name); unreachable!()}
		})
		.collect();

		Self {
			cmd: cmd,
			args: args,
			sender: sender,
			channel: privmsg.source.params[0][1..].to_owned(),
			statuses: badges,
			timestamp: privmsg.server_timestamp,
		}
	}
}

type UserNameIdCache: HashMap<String, i32> = HashMap::new();
