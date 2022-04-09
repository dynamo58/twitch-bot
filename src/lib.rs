pub mod commands;
pub mod db;
pub mod api;
pub mod api_models;
pub mod background;

use std::{collections::HashMap, fs::read_to_string};
use std::path::Path;


use colored::*;
use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};
use thiserror::Error;
use twitch_irc::message::PrivmsgMessage;


// some custom errors (ad hoc)
#[derive(Error, Debug)]
pub enum MyError {
	#[error("index out of bounds")]
	OutOfBounds,
	#[error("item not found")]
	NotFound,
}

// All the statuses one can have in Twitch chat
#[derive(Clone, PartialEq)]
pub enum TwitchBadge {
	Broadcaster,
	Admin,
	GlobalMod,
	Mod,
	Staff,
	Subscriber,
	Vip,
	Premium,
	GlitchCon2020,
	Unrecognized,
	GLHFPledge,
	Bits,
	BitsCharity,
}

// twitch authentification credentials
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TwitchAuth {
	pub client_id: String,
	pub oauth: String,
	pub nick: String,
}

impl TwitchAuth {
	pub fn from_dotenv() -> anyhow::Result<TwitchAuth> {
		let oauth     = std::env::var("TWITCH_OAUTH")?;
		let client_id = std::env::var("TWITCH_CLIENT_ID")?;
		let nick      = std::env::var("TWITCH_NICK")?;

		Ok(TwitchAuth { client_id, oauth, nick })
	}
}

// config that directs how the bot works
// gets set up during runtime
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
	pub channels: Vec<String>,
	pub disregarded_users: Vec<String>,
	pub prefix: char,
	pub index_markov: bool,
	pub track_offliners: bool,
}

impl Config {
	// parse from config file at `assets/config.json`
	pub fn from_config_file() -> anyhow::Result<Config> {
		let json: String = read_to_string(Path::new("assets/config.json"))?;
		
		let mut config: Config = serde_json::from_str(&json)?;

		config.disregarded_users = config.disregarded_users
			.iter()
			.map(|user| user.to_lowercase())
			.collect();

		Ok(config)
	}
}

#[derive(Clone)]
pub struct Sender {
	pub id: i32,
	pub name: String,
	pub statuses: Vec<TwitchBadge>,
}

// the only info which is important and
// which the bot works with
#[derive(Clone)]
pub struct CommandSource {
	pub cmd: String,
	pub args: Vec<String>,
	pub sender: Sender,
	pub channel: String,
	pub timestamp: DateTime<Utc>,
}

impl CommandSource {
	// parse new from twitch_irc::message::PrivmsgMessage
	pub fn from_privmsg(privmsg: twitch_irc::message::PrivmsgMessage) -> Self {
		let mut args: Vec<String> = privmsg.message_text
			.split(" ")
			.map(|arg| arg.to_owned())
			.collect();
		let cmd = args[0].to_lowercase()[1..].to_owned();
		args = args[1..].to_owned();

		// parse badges
		let badges: Vec<TwitchBadge> = privmsg.badges
			.into_iter()
			.map(|badge| match badge.name.as_str() {
				"admin"         => TwitchBadge::Admin,
				"broadcaster"   => TwitchBadge::Broadcaster,
				"global_mod"    => TwitchBadge::GlobalMod,
				"moderator"     => TwitchBadge::Mod,
				"staff"         => TwitchBadge::Staff,
				"subscriber"    => TwitchBadge::Subscriber,
				"vip"           => TwitchBadge::Vip,
				"premium"       => TwitchBadge::Premium,
				"glitchcon2020" => TwitchBadge::GlitchCon2020,
				"glhf-pledge"   => TwitchBadge::GLHFPledge,
				"bits"          => TwitchBadge::Bits,
				"bits-charity"  => TwitchBadge::BitsCharity,
				_ => {
					println!(
						"{} Encountered unrecognized badge: {}",
						"WARN   ".bright_red().bold(),
						badge.name.bold()
					);
					
					TwitchBadge::Unrecognized
				}
		})
		.collect();

		let sender = Sender {
			id: privmsg.sender.id.parse::<i32>().unwrap(),
			name: privmsg.sender.name,
			statuses: badges,
		};

		Self {
			cmd: cmd,
			args: args,
			sender: sender,
			channel: privmsg.source.params[0][1..].to_owned(),
			timestamp: privmsg.server_timestamp,
		}
	}
}

pub type NameIdCache = HashMap<String, i32>;

#[derive(Clone)]
pub struct EmoteCache {
	// channels the bot is joined to
	// with the emotes they have enabled (7tv, bttv, ffz)
	pub channels: HashMap<String, Vec<String>>,
	// 7tv, bttv and ffz global emotes
	pub globals: Vec<String>,
	// all the other Twitch emotes (globals and channel emotes)
	// have to be processed from the Privmsg directly
}

pub type Cashe = EmoteCache;

impl EmoteCache {
	pub async fn init(
		config: &Config,
		auth:   &TwitchAuth,
	) -> anyhow::Result<Self> {
		let mut channels: HashMap<String, Vec<String>> = HashMap::new();
		let mut globals: Vec<String> = vec![];
		
		for channel_name in &config.channels {
			let channel_id = api::id_from_nick(channel_name, &auth)
				.await?
				.ok_or(MyError::NotFound)?;
			
			let channel_emotes = api::get_all_channel_emotes(channel_id)
				.await?;
			
			if let Some(emotes) = channel_emotes {
				channels.insert(channel_name.to_string(), emotes);
			}
		}

		api::get_7tv_global_emotes()
			.await?
			.ok_or(MyError::NotFound)?
			.iter().for_each(|emote_name| globals.push(emote_name.to_owned()));

		api::get_bttv_global_emotes()
			.await?
			.ok_or(MyError::NotFound)?
			.iter().for_each(|emote_name| globals.push(emote_name.to_owned()));

		api::get_ffz_global_emotes()
			.await?
			.ok_or(MyError::NotFound)?
			.iter().for_each(|emote_name| globals.push(emote_name.to_owned()));

		Ok(Self {
			channels,
			globals,
		})
	}

	pub fn self_or_privmsg_has_emote(
		&self,
		privmsg:      &PrivmsgMessage,
		emote_name:   &String,
	) -> bool {
		let channel_name = &privmsg.source.params[0][1..];
		let channel_emotes = match self.channels.get(channel_name) {
			Some(emotes) => emotes,
			None => return false,
		};

		channel_emotes.contains(emote_name) ||
		self.globals.contains(emote_name) ||
		privmsg.emotes
			.iter()
			.map(|emote| emote.code.to_owned())
			.collect::<String>()
			.contains(emote_name)
	}

	pub async fn renew(
		&mut self,
		config: &Config,
		auth: &TwitchAuth,
	) -> anyhow::Result<()> {
		match Self::init(&config, &auth).await {
			Ok(new_cache) => {
				self.channels = new_cache.channels;
				self.globals = new_cache.globals;
				println!(
					"{}   Renewed the emote cache.",
					"INFO   ".blue().bold()
				);
			},
			Err(_) => println!(
				"{}   Couldn't renew emote cache, keeping it the same.",
				"ERROR  ".red().bold()
			),
		}

		Ok(())
	}
}

pub fn fmt_duration(dur: chrono::Duration) -> String {
	let mut out = String::new();
	let num_sec = dur.num_seconds() as f32;

	if num_sec == 0.0 {
		return "no time".into();
	}
	
	let yrs = (num_sec / 31557082.0).floor();
	let mts = ((num_sec - (yrs * 31557082.0)) / 2629757.0).floor();
	let dys = ((num_sec - (yrs * 31557082.0) - (mts * 2629757.0)) / 86400.0).floor();
	let hrs = ((num_sec - (yrs * 31557082.0) - (mts * 2629757.0) - (dys * 86400.0)) / 3600.0).floor();
	let mns = ((num_sec - (yrs * 31557082.0) - (mts * 2629757.0) - (dys * 86400.0) - (hrs * 3600.0)) / 60.0).floor();
	let scs = dur.num_seconds() % 60;

	if yrs > 0.0 {
		out.push_str(&format!("{yrs} years, "));
	}

	if mts > 0.0 {
		out.push_str(&format!("{mts} months, "));
	}

	if dys > 0.0 {
		out.push_str(&format!("{dys} days, "));
	}

	if hrs > 0.0 {
		out.push_str(&format!("{hrs} hours, "));
	}

	if mns > 0.0 {
		out.push_str(&format!("{mns} minutes, "));
	}

	if scs > 0 {
		out.push_str(&format!("{scs} seconds, "));
	}

	out.pop();
	out.pop();
	out
}
