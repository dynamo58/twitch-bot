pub mod commands;
pub mod db;

use std::path::Path;

use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
	pub channels: Vec<String>,
	pub disregarded_users: Vec<String>,
	pub prefix: char,
	pub index_markov: bool,
}

impl Config {
	pub fn from_config_file() -> anyhow::Result<Config> {
		let json = std::fs::read_to_string(Path::new("assets/config.json"))?;
		let deserialized: Config = serde_json::from_str(&json)?;

		Ok(deserialized)
	}
}

pub enum TwitchStatus {
	Broadcaster,
	Admin,
	GlobalMod,
	Mod,
	Staff,
	Subscriber,
	Vip,
}

pub struct Sender {
	pub id: String,
	pub name: String,
}

pub struct CommandSource {
	pub cmd: String,
	pub args: Vec<String>,
	pub sender: Sender,
	pub channel: String,
	pub statuses: Vec<TwitchStatus>,
}

impl CommandSource {
	pub fn from_privmsg(privmsg: twitch_irc::message::PrivmsgMessage) -> Self {
		let mut args: Vec<String> = privmsg.message_text.split(" ").map(|arg| arg.to_owned()).collect();
		let cmd = args[0].to_lowercase()[1..].to_owned();
		args = args[1..].to_owned();

		let sender = Sender {
			id: privmsg.sender.id,
			name: privmsg.sender.name
		};

		let badges: Vec<TwitchStatus> = privmsg.badges.into_iter().map(|badge| match badge.name.as_str() {
			"admin" => TwitchStatus::Admin,
			"broadcaster" => TwitchStatus::Broadcaster,
			"global_mod" => TwitchStatus::GlobalMod,
			"moderator" => TwitchStatus::Mod,
			"staff" => TwitchStatus::Staff,
			"subscriber" => TwitchStatus::Subscriber,
			"vip" => TwitchStatus::Vip,
			_ => {println!("{badge:#?}");unreachable!()},
		})
		.collect();

		Self {
			cmd: cmd,
			args: args,
			sender: sender,
			channel: privmsg.source.params[0][1..].to_owned(),
			statuses: badges,
		}
	}
}

// pub type CommandOutput<'a> = Option<(String, &'a str)>;