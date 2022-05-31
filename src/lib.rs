pub mod commands;
pub mod db;
pub mod api;
pub mod api_models;
pub mod background;
pub mod constants;

use std::{collections::HashMap, fs::read_to_string};
use std::sync::{Arc, Mutex};
use std::path::Path;
use constants::*;

use colored::*;
use chrono::{DateTime, Utc};
use rand::seq::SliceRandom;
use serde::{Serialize, Deserialize};
use thiserror::Error;
use twitch_irc::message::PrivmsgMessage;


#[derive(Error, Debug)]
pub enum MyError {
	#[error("index out of bounds")]
	OutOfBounds,
	#[error("item not found")]
	NotFound,
	#[error("Thread error")]
	ThreadError,
	#[error("Insufficient privileges | requires to be mod/vip/broadcaster")]
	InsufficientPrivileges,
	#[error("❌ missing a parameter | add {0}=\"something\" into the command")]
	MissingHardParameter(String),
	#[error("❌ parameter `{0}` has bad type, expected a {1}")]
	BadHardArgumentType(String, String),
	#[error("❌ missing positional argument | position {0}, argument type: {1}")]
	MissingPositionalArgument(u8, String),
	#[error("❌ an internal error has occured, sorry PoroSad")]
	Internal,
	#[error("❌ an unknown error has occured, sorry PoroSad")]
	Unknown,
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
	Partner,
}

// twitch authentification credentials
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TwitchAuth {
	pub client_id: String,
	pub oauth:     String,
	pub nick:      String,
}

impl TwitchAuth {
	pub fn from_env() -> anyhow::Result<TwitchAuth> {
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
	pub channels:                Vec<String>,
	pub disregarded_users:       Vec<String>,
	pub commands_reference_path: String,
	pub github_repo_api_path:    Option<String>,
	pub index_markov:            bool,
	pub track_offliners:         bool,
	pub prefix:                  char,
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

// the sender of a message
#[derive(Clone)]
pub struct Sender {
	pub id: i32,
	pub name: String,
	pub statuses: Vec<TwitchBadge>,
}

impl Sender {
	// checks whether a certain user is either mod/vip/broadcaster 
	pub fn is_mvb(&self) -> bool {
		self.statuses.contains(&TwitchBadge::Vip) ||
		self.statuses.contains(&TwitchBadge::Mod) || 
		self.statuses.contains(&TwitchBadge::Broadcaster) 
	}
}

// the channel a message is posted in
#[derive(Clone)]
pub struct Channel {
	pub id: i32,
	pub name: String,
}

// the only info which is important and
// which the bot works with
#[derive(Clone)]
pub struct CommandSource {
	pub cmd:       String,
	pub args:      Vec<String>,
	pub sender:    Sender,
	pub channel:   Channel,
	pub timestamp: DateTime<Utc>,
	pub is_pipe:   bool,
}

impl CommandSource {
	// parse new from twitch_irc::message::PrivmsgMessage
	pub fn from_privmsg(privmsg: twitch_irc::message::PrivmsgMessage) -> Self {
		let mut args: Vec<String> = privmsg.message_text
			.split(' ')
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
				"partner"       => TwitchBadge::Partner,
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
			cmd,
			args,
			sender,
			channel: Channel {
				name: privmsg.source.params[0][1..].to_owned(),
				id: privmsg.channel_id.parse::<i32>().unwrap(),
			},
			timestamp: privmsg.server_timestamp,
			is_pipe: false,
		}
	}

	// get the channel id, channel name, user id and user name
	// from by infering the command target
	pub async fn user_channel_info_from_args(
		&self,
		twitch_auth:       &TwitchAuth,
		name_id_cache_arc: Arc<Mutex<NameIdCache>>
	) -> Result<(Channel, Channel), UserChannelParseError> {
		match self.args.len() {
			// if 0 args are supplied:
			//     the command sender is the target user,
			//     the channel from which command is ran is the target channel
			0 => {
				let user = Channel {
					name: self.sender.name.clone(),
					id:   self.sender.id,
				};

				let channel = self.channel.clone();

				Ok((user, channel))
			},

			// if 1 arg is supplied:
			//     the first arg is the target user,
			//     the channel from which command is ran is the target channel
			1 => {
				let user_name = self.args[0].clone();
				let mut user_id: Option<i32> = None;

				if let Ok(cache) = name_id_cache_arc.lock() {
					if let Some(id) = cache.get(&user_name) {
						user_id = Some(*id);
					}
				}

				if user_id.is_none() {
					user_id = Some(api::id_from_nick(&user_name, twitch_auth)
						.await.ok().ok_or(UserChannelParseError::Unknown)?
						.ok_or_else(|| UserChannelParseError::UserNotFound(user_name.clone()))?); 
				}
				
				let user = Channel {
					name: user_name,
					id:   user_id.unwrap(),
				};
				let channel = self.channel.clone();

				Ok((user, channel))
			}

			// if 2 (or more) args are supplied:
			//     the first arg is the target user
			//     the second arg is the target channel
			_ => {
				let user_name = self.args[0].clone();
				let mut user_id: Option<i32> = None;
				
				let channel_name = self.args[1].clone();
				let mut channel_id: Option<i32> = None;
				
				if let Ok(cache) = name_id_cache_arc.lock() {
					if let Some(id) = cache.get(&user_name) {
						user_id = Some(*id);
					}

					if let Some(id) = cache.get(&channel_name) {
						channel_id = Some(*id);
					}
				}
				
				if user_id.is_none() {
					user_id = Some(api::id_from_nick(&user_name, twitch_auth)
						.await.ok().ok_or(UserChannelParseError::Unknown)?
						.ok_or_else(|| UserChannelParseError::UserNotFound(user_name.clone()))?);
				}

				if channel_id.is_none() {
					channel_id = Some(api::id_from_nick(&channel_name, twitch_auth)
						.await
						.ok()
						.ok_or(UserChannelParseError::Unknown)?
						.ok_or_else(|| UserChannelParseError::ChannelNotFound(channel_name.clone()))?);
				}

				let user = Channel {
					id:   user_id.unwrap(),
					name: user_name,
				};

				let channel = Channel {
					id: channel_id.unwrap(),
					name: channel_name,
				};

				Ok((user, channel))
			},
		}
	}
}

#[derive(Debug, Error)]
pub enum UserChannelParseError {
	#[error("💢 User `{0}` was not found")]
	UserNotFound(String),
	#[error("💢 Channel `{0}` was not found")]
	ChannelNotFound(String),
	#[error("💢 Unknown error has occured")]
    Unknown,
}

// Is used to cache the emotes of the channel
// in order not to overwhelm the APIs;
// emotes are (as of know) used only to
// decide whether to convert to lowercase
// when saving markov entries into the database
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

impl EmoteCache {
	pub async fn init(
		config: &Config,
		auth:   &TwitchAuth,
	) -> anyhow::Result<Self> {
		let mut channels: HashMap<String, Vec<String>> = HashMap::new();
		let mut globals: Vec<String> = vec![];
		
		for channel_name in &config.channels {
			let channel_id = api::id_from_nick(channel_name, auth)
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
		emote_name:   &str,
	) -> bool {
		let channel_name = &privmsg.source.params[0][1..];
		let channel_emotes = match self.channels.get(channel_name) {
			Some(emotes) => emotes,
			None => return false,
		};

		channel_emotes.contains(&emote_name.to_owned()) ||
		self.globals.contains(&emote_name.to_owned()) ||
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
		match Self::init(config, auth).await {
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

// store the users' Twitch ID
pub type NameIdCache = HashMap<String, i32>;

#[derive(Clone, Debug)]
pub enum HookMatchType {
	Exact,
	Substring,
}

impl std::str::FromStr for HookMatchType {
	type Err = MyError;
	
	fn from_str(s: &str) -> Result<Self, Self::Err> {
		match s.to_lowercase().as_str() {
			"exact"  => Ok(Self::Exact),
			"substr" => Ok(Self::Substring),
			_        => Err(MyError::NotFound),
		}
	}
}

impl std::string::ToString for HookMatchType {
	fn to_string(&self) -> String {
		match self {
			Self::Exact     => "exact".to_owned(),
			Self::Substring => "substr".to_owned(),
		}
	}
}



#[derive(Clone, Debug)]
pub struct MessageHook {
	pub capture_string: String, 
	pub content:        String,
	pub h_type:         HookMatchType,
}

#[derive(Debug)]
pub struct ChannelSpecifics {
	pub hooks:               Vec<MessageHook>,
	pub ongoing_trivia_game: Option<TriviaGameInfo>, 
}

pub type ChannelSpecificsCache = HashMap<String, ChannelSpecifics>;

// converts html entities to actual chars (only some selected ones, not all!!) 
pub fn convert_from_html_entities(s: String) -> String {
    s
        .replace("&nbsp;", " ")
        .replace("&#039;", "'")
        .replace("&quot;", "\"")
		.replace("&Delta;", "d")
		.replace("&deg;", " degrees")
		.replace("&rsquo;", "’")
		.replace("&hellip;", "…")
		.replace("&rdquo;", "”")
		.replace("&pi;", "π")
}

pub fn convert_to_html_encoding(s: String) -> String {
	s
		.replace('%',  "%25") // this one has to be always first!
		.replace(' ',  "%20")
		.replace('&',  "%26")
		.replace('\'',  "%27")
		.replace('(',  "%28")
		.replace(')',  "%29")
        .replace('/',  "%2F")
		.replace('*',  "%2A")
		.replace('+',  "%2B")
		.replace(',',  "%2C")
		.replace('-',  "%2D")
		.replace('.',  "%2E")
		.replace('/',  "%2F")
		.replace(':',  "%3A")
		.replace('<',  "%3C")
		.replace('=',  "%3D")
		.replace('>',  "%3E")
		.replace('?',  "%3F")
		.replace('@',  "%40")
		.replace('[',  "%5B")
		.replace('\\', "%5C")
		.replace(']',  "%5D")
		.replace('^',  "%5E")
		.replace('_',  "%5F")
		.replace('`',  "%60")
		.replace('{',  "%7B")
		.replace('|',  "%7C")
		.replace('}',  "%7D")
		.replace('~',  "%7E")
		.replace('€',  "%E2%82%AC")
		.replace('‚',  "%E2%80%9A")
		.replace('„',  "%E2%80%9E")
		.replace('ˆ',  "%CB%86")
		.replace('‘',  "%E2%80%98")
		.replace('’',  "%E2%80%99")
		.replace('“',  "%E2%80%9C")
		.replace('”',  "%E2%80%9D")
}

#[derive(Clone, Debug)]
pub struct TriviaGameInfo {
	pub question: String,
	pub correct_answer: String,
	pub wrong_answers:  Vec<String>,
}

impl TriviaGameInfo {
	pub fn from_api_object(s: crate::api_models::TriviaQuestion) -> Self {
		Self {
			question: s.question,
			correct_answer: s.correct_answer,
			wrong_answers: s.incorrect_answers
		}
	}

	pub fn shuffled_answers(&self) -> Vec<&String> {
		let mut answers = vec![
			&self.correct_answer,
			&self.wrong_answers[0],
			&self.wrong_answers[1],
			&self.wrong_answers[2],
		];

		answers.shuffle(&mut rand::thread_rng());
		answers
	}
}

// format a duration into a string
#[allow(non_snake_case)]
pub fn fmt_duration(dur: chrono::Duration, long_format: bool) -> String {
	let num_sec = dur.num_seconds() as f32;
	if num_sec == 0.0 {
		return "no time".into();
	}
	
	let yrs = (num_sec / SECONDS_IN_YEAR).floor();
	let dys = ((num_sec - (yrs * SECONDS_IN_YEAR)) / SECONDS_IN_DAY).floor();
	let hrs = ((num_sec - (yrs * SECONDS_IN_YEAR) - (dys * SECONDS_IN_DAY)) / SECONDS_IN_HOUR).floor();
	let mns = ((num_sec - (yrs * SECONDS_IN_YEAR) - (dys * SECONDS_IN_DAY) - (hrs * SECONDS_IN_HOUR)) / SECONDS_IN_MINUTE).floor();
	let scs = dur.num_seconds() % 60;

	let SECONDS = if long_format { " seconds" } else { "s" };
	let MINUTES = if long_format { " minutes" } else { "m" };
	let HOURS   = if long_format { " hours"   } else { "h" };
	let DAYS    = if long_format { " days"    } else { "d" };
	let YEARS   = if long_format { " years"   } else { "y" };
	let mut out = String::new();

	let mut out_len: u8 = 0;

	if yrs > 0.0 {
		out.push_str(&format!("{yrs}{YEARS}, "));
		out_len += 1;
	}

	if dys > 0.0 {
		out.push_str(&format!("{dys}{DAYS}, "));
		out_len += 1;
	}

	if (hrs > 0.0) && (out_len <= 2) {
		out.push_str(&format!("{hrs}{HOURS}, "));
		out_len += 1;
	}

	if (mns > 0.0) && (out_len <= 2) {
		out.push_str(&format!("{mns}{MINUTES}, "));
		out_len += 1;
	}

	if (scs > 0) && (out_len <= 2) {
		out.push_str(&format!("{scs}{SECONDS}, "));
	}

	out.pop();
	out.pop();
	out
}

pub fn factorial(n: u128) -> u128 {
	if n == 0 || n == 1 {
		return 1;
	}

	n * factorial(n-1)
}

pub fn binomial_p_exact(n_of_tries: u128, n_of_successess: u128, p_of_success: f64) -> f64 {
	(factorial(n_of_tries)/(factorial(n_of_tries - n_of_successess) * factorial(n_of_successess))) as f64 *
    p_of_success.powf(n_of_successess as f64)                                                    *
    (1. -p_of_success).powf((n_of_tries - n_of_successess) as f64)
}

pub fn binomial_p_exact_or_less(n_of_tries: u128, n_of_successess: u128, p_of_success: f64) -> f64 {
	let mut total_p = 0.;
	
	for i in 0..=n_of_successess {
		total_p += binomial_p_exact(n_of_tries, i, p_of_success);
	}

	total_p
}
