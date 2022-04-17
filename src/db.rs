use crate::{MyError, EmoteCache, CommandSource};

use std::sync::{Arc, Mutex};
use std::time::Duration;

use rand::{self, Rng};
use chrono::{DateTime, Utc};
use sqlx::sqlite::SqlitePool;
use sqlx::Sqlite;
use twitch_irc::message::PrivmsgMessage;


// QR == query result
#[derive(sqlx::FromRow)]
struct StringQR(String);

#[derive(sqlx::FromRow)]
struct I32QR(i32);

#[derive(sqlx::FromRow)]
pub struct I32I32QR(pub i32, pub i32);

#[derive(sqlx::FromRow)]
struct DateTimeQR(DateTime<Utc>);

#[derive(sqlx::FromRow)]
struct ChannelCommandQR(String, String, i32);

#[derive(sqlx::FromRow, Debug)]
pub struct Reminder {
	pub id: i32,
    pub from_user_id: i32,
    pub for_user_id: i32,
    pub raise_timestamp: DateTime<Utc>,
    pub message: String,
}

pub async fn init_db(
    pool: &SqlitePool,
) -> anyhow::Result<()> {
	let mut conn = pool.acquire().await?;
	let sql = include_str!("../assets/sql/init_db.sql");

	sqlx::query::<Sqlite>(&sql)
		.execute(&mut *conn)
		.await?;
	
	Ok(())
}

// create table for current set channel (if it does not exist)
pub async fn try_create_tables_for_channel(
    pool:       &SqlitePool,
    channel_id: i32,
) -> anyhow::Result<()> {
	let mut conn = pool.acquire().await?;
	let sql = include_str!("../assets/sql/channel_tables.sql")
		.replace("{{ CHANNEL_ID }}", &channel_id.to_string());

	sqlx::query::<Sqlite>(&sql)
		.execute(&mut *conn)
		.await?;

	Ok(())
}

// save incoming messages to db
pub async fn log(
    pool: &SqlitePool,
    privmsg: &twitch_irc::message::PrivmsgMessage,
) -> anyhow::Result<()> {
	let mut conn = pool.acquire().await?;
	let channel_id = privmsg.channel_id.parse::<i32>().unwrap();

    let sql = r#"
    INSERT
        INTO CHANNEL_{{ CHANNEL_ID }} 
	        (sender_id, sender_nick, badges, timestamp, message)
        VALUES
	        (?1, ?2, ?3, ?4, ?5)
    "#.replace("{{ CHANNEL_ID }}", &channel_id.to_string());

	sqlx::query::<Sqlite>(&sql)
		.bind(&privmsg.sender.id)
		.bind(&privmsg.sender.name)
		.bind(&privmsg.badges.iter().map(|badge| format!("{}_", badge.name)).collect::<String>())
		.bind(&format!("{}", &privmsg.server_timestamp))
		.bind(&privmsg.message_text)
		.execute(&mut *conn)
		.await?;

	Ok(())
}

// processes message for markov index table entries
pub async fn log_markov(
    pool: &SqlitePool,
	emote_cache_arc: &Arc<Mutex<EmoteCache>>,
    privmsg: &twitch_irc::message::PrivmsgMessage,
) -> anyhow::Result<()> {
	let mut conn = pool.acquire().await?;

	let words = privmsg.message_text.split(' ').collect::<Vec<&str>>();

	// this is kinda tacky but I couldn't get it working otherwise, so idk
	let emote_cache;
	if let Ok(cache) = emote_cache_arc.lock() {
		emote_cache = cache.clone();
	} else {
		return Ok(());
	}

	// process each word (besides the last one)
	for idx in 0..words.len()-1 {
		let word = match format_markov_entry(&privmsg, &emote_cache, words[idx]) {
			Ok(a) => a,
			Err(_) => return Ok(()),
		};
		let succ = match format_markov_entry(&privmsg, &emote_cache, words[idx + 1]) {
			Ok(a) => a,
			Err(_) => return Ok(()),
		};

		if let (Some(w), Some(s)) = (word, succ) {
			let sql = r#"
				INSERT 
					INTO CHANNEL_{{ CHANNEL_ID }}_MARKOV
						(word, succ)
					VALUES
						($1, $2);
			"#.replace("{{ CHANNEL_ID }}", &privmsg.channel_id);
				
			sqlx::query::<Sqlite>(&sql)
				.bind(w)
				.bind(s)
				.execute(&mut *conn)
				.await?;
		}
	}

	Ok(())
}

// checks for reminders of a specified user, return & delete them
pub async fn check_for_reminders(
	pool: &SqlitePool,
	user_id: i32,
) -> anyhow::Result<Option<Vec<Reminder>>> {
	let mut conn = pool.acquire().await?;

	// query reminders
	let sql = r#"
		SELECT *
			FROM user_reminders
			WHERE
				for_user_id=?1
			AND raise_timestamp <= DATETIME('NOW');
	"#;

	let reminders: Vec<Reminder> = sqlx::query_as::<Sqlite, Reminder>(&sql)
		.bind(user_id)
		.fetch_all(&mut *conn)
		.await?;

	if reminders.len() == 0 {
		return Ok(None);
	}

	// delete reminders

	let sql = r#"
		DELETE
			FROM user_reminders
			WHERE
				id in (?1);
	"#;

	for r in &reminders {
		sqlx::query::<Sqlite>(&sql)
		.bind(r.id)
		.execute(&mut *conn)
		.await?;
	}

	// return the queried ones
	Ok(Some(reminders))
}


// format the words parsed from the message into format
// acceptible for the db entry
fn format_markov_entry(
	privmsg: &PrivmsgMessage,
	emote_cache: &EmoteCache,
	s: &str,
) -> anyhow::Result<Option<String>> {
    let mut out = s.to_owned();
    let invalid_front_chars = vec![
		'"',
		'\'',
		'«',
		'「',
		'“',
		'‘',
		'(',
		'[',
		'{',
		',',
		'.',
		';',
		' ',
		'⠀' // this is a "blank" braille character
	];
    let invalid_back_chars = vec![
		'"',
		'\'',
		'»',
		'」',
		'”',
		'’',
		')',
		']',
		'}',
		',',
		'.',
		';',
		' ', // space
		'⠀', // this is a "blank" braille character
		'!',
		'?'
	];
    // the invisible braille char

    // shave off all trailing unwanted chars
    while invalid_front_chars.contains(&out.chars().nth(0).ok_or(MyError::OutOfBounds)?) {
        out.remove(0);
    }

    while invalid_back_chars.contains(&out.chars().last().ok_or(MyError::OutOfBounds)?) {
        out.pop();
    }

    // if there is still the blank braille's or it is a link
	// don't remove anything; else remove the formatted word
    if out.contains("⠀") || out.contains("//") || out.contains("www.") || out == "".to_string() { 
        Ok(None)
    } else {
		match emote_cache.self_or_privmsg_has_emote(&privmsg, &out) {
			true  => return Ok(Some(out)),
			false => return Ok(Some(out.to_lowercase()))
		}
        
    }
}

pub async fn clear_users_sent_reminders(
	pool: &SqlitePool,
	user_id: i32,
) -> anyhow::Result<i32> {
	let mut conn = pool.acquire().await?;

	let sql = r#"
		DELETE
			FROM user_reminders
			WHERE
				from_user_id=$1;
		SELECT changes();
	"#;

	let num_affected: i32 = sqlx::query_as::<Sqlite, I32QR>(&sql)
		.bind(user_id)
		.fetch_all(&mut *conn)
		.await?
		[0].0;

		Ok(num_affected)
	} 

// get a random successor of specified word from the markov index table
pub async fn get_rand_markov_succ(
	pool:       &SqlitePool,
	channel_id: i32,
	word:       &str
) -> anyhow::Result<Option<String>> {
	let mut conn = pool.acquire().await?;

	let sql = r#"
		SELECT succ
			FROM CHANNEL_{{ CHANNEL_ID }}_MARKOV
			WHERE
				word=$1
			COLLATE NOCASE;
	"#.replace("{{ CHANNEL_ID }}", &channel_id.to_string());


	let succs: Vec<String> = sqlx::query_as::<Sqlite, StringQR>(&sql)
		.bind(word)
		.fetch_all(&mut *conn)
		.await?
		.iter()
		.map(|succ| succ.0.clone())
		.collect();
	
	if succs.len() < 1 {
		return Ok(None);
	}

	let rand_succ = succs[rand::thread_rng().gen_range(0..succs.len())].clone();

	Ok(rand_succ.into())
}

// insert a reminder for a user
pub async fn insert_reminder(
    pool: &SqlitePool,
    reminder: &Reminder,
) -> anyhow::Result<()> {
	let mut conn = pool.acquire().await?;

    let sql = r#"
        INSERT 
            INTO user_reminders 
                (from_user_id, for_user_id, raise_timestamp, message)
            VALUES
                (?1, ?2, ?3, ?4);
    "#;

	sqlx::query::<Sqlite>(&sql)
		.bind(reminder.from_user_id)
		.bind(reminder.for_user_id)
		.bind(
			&format!(
				"{}",
				reminder.raise_timestamp
					.format("%Y-%m-%d %H:%M:%S")
					.to_string()
			)
		)
		.bind(&reminder.message)
		.execute(&mut *conn)
		.await?;
    
    Ok(())
}

pub async fn log_command(
	pool: &SqlitePool,
	cmd: &CommandSource,
	execution_time: Duration,
	output: &str,
) -> anyhow::Result<()> {
	let mut conn = pool.acquire().await?;

	let ser_args = serde_json::to_string(&cmd.args)?;

    let sql = r#"
        INSERT 
            INTO command_history
                (sender_id, sender_name, command, args, execution_time_s, output, timestamp)
            VALUES
                (?1, ?2, ?3, ?4, ?5, ?6, ?7);
    "#;
	sqlx::query::<Sqlite>(&sql)
		.bind(cmd.sender.id)
		.bind(&cmd.sender.name)
		.bind(&cmd.cmd)
		.bind(ser_args)
		.bind(execution_time.as_secs_f64())
		.bind(output)
		.bind(
			&format!(
				"{}",
				Utc::now()
					.format("%Y-%m-%d %H:%M:%S")
					.to_string()
			)
		)
		.execute(&mut *conn)
		.await?;
    
    Ok(())
}

pub async fn get_explanation(
	pool: &SqlitePool,
	code: &str,
) -> anyhow::Result<Option<String>> {
	let mut conn = pool.acquire().await?;

	let sql = r#"
		SELECT
			message
			FROM
				explanations
			WHERE
				code=?1;
	"#;

	let messages: Vec<String> = sqlx::query_as::<Sqlite, StringQR>(&sql)
		.bind(code)
		.fetch_all(&mut *conn)
		.await?
		.iter()
		.map(|succ| succ.0.clone())
		.collect();

	if messages.len() == 0 {
		return Ok(Some("no such error code".into()));
	} else {
		return Ok(Some(messages[0].clone()));
	}
}

// set an alias for the user
pub async fn set_alias<'a>(
    pool:      &SqlitePool,
	owner_id:  i32,
    alias:     &'a str,
	alias_cmd: &'a str
) -> anyhow::Result<()> {
	let mut conn = pool.acquire().await?;

	let sql = r#"
        INSERT 
            INTO user_aliases
                (owner_id, alias, alias_cmd)
            VALUES
                (?1, ?2, ?3);
    "#;

	sqlx::query::<Sqlite>(&sql)
		.bind(owner_id)
		.bind(alias)
		.bind(alias_cmd)
		.execute(&mut *conn)
		.await?;
    
    Ok(())
}

// insert a reminder for a user
pub async fn get_alias_cmd(
    pool:     &SqlitePool,
	owner_id: i32,
    alias:    &str,
) -> anyhow::Result<Option<String>> {
	let mut conn = pool.acquire().await?;

	let sql = r#"
		SELECT
			alias_cmd
			FROM
				user_aliases
			WHERE
				owner_id=?1
			AND
				alias=?2;
	"#;

	// length should be 1 || 0
	let aliases: Vec<String> = sqlx::query_as::<Sqlite, StringQR>(&sql)
		.bind(owner_id)
		.bind(alias)
		.fetch_all(&mut *conn)
		.await?
		.iter()
		.map(|a| a.0.clone())
		.collect();
	
	if aliases.len() == 0 {
		return Ok(None);
	} else {
		return Ok(Some(aliases[0].to_owned()));
	}
}

// remove a specified alias
pub async fn remove_alias<'a>(
	pool: &SqlitePool,
	owner_id: i32,
	alias: &'a str
) -> anyhow::Result<i32> {
	let mut conn = pool.acquire().await?;

	let sql = r#"
		DELETE
			FROM user_aliases
			WHERE
				owner_id=?1
			AND
				alias=?2;
		SELECT changes();
	"#;

	let num_affected: i32 = sqlx::query_as::<Sqlite, I32QR>(&sql)
		.bind(owner_id)
		.bind(alias)
		.fetch_all(&mut *conn)
		.await?
		[0].0;

	Ok(num_affected)
}

pub async fn get_first_message(
	pool:       &SqlitePool,
	sender_id:  i32,
	channel_id: i32,
) -> anyhow::Result<Option<String>> {
	let mut conn = pool.acquire().await?;

	let sql = r#"
		SELECT message
			FROM CHANNEL_{{ CHANNEL_ID }}
			WHERE
				sender_id=$1
			LIMIT 1;
	"#.replace("{{ CHANNEL_ID }}", &channel_id.to_string());

	let messages: Vec<String> = sqlx::query_as::<Sqlite, StringQR>(&sql)
		.bind(sender_id)
		.fetch_all(&mut *conn)
		.await?
		.iter()
		.map(|succ| succ.0.clone())
		.collect();
	
	if messages.len() < 1 {
		return Ok(None);
	} else {
		return Ok(Some(messages[0].clone()));
	}
}

pub async fn save_suggestion(
	pool: &SqlitePool,
	sender_id: i32,
	sender_name: &str,
	text: &str,
	dt: DateTime<Utc>,
) -> anyhow::Result<()> {
	let mut conn = pool.acquire().await?;

	let sql = r#"
        INSERT 
            INTO user_feedback
                (sender_id, sender_name, message, time)
            VALUES
                (?1, ?2, ?3, ?4);
    "#;

	sqlx::query::<Sqlite>(&sql)
		.bind(sender_id)
		.bind(sender_name)
		.bind(text)
		.bind(dt.format("%Y-%m-%d %H:%M:%S").to_string())
		.execute(&mut *conn)
		.await?;

    Ok(())
}

pub async fn set_lurk_status(
	pool:      &SqlitePool,
	sender_id: i32,
	timestamp: DateTime<Utc>,
) -> anyhow::Result<()> {
	let mut conn = pool.acquire().await?;
	
	let sql = r#"
		INSERT 
			INTO lurkers
				(lurker_id, timestamp)
			VALUES
				(?1, ?2);
	"#;

	sqlx::query::<Sqlite>(&sql)
		.bind(sender_id)
		.bind(timestamp.format("%Y-%m-%d %H:%M:%S").to_string())
		.execute(&mut *conn)
		.await?;
	
	Ok(())
}

// checks whether a user is currently lurking, if so, return the time duration
pub async fn is_lurker(
	pool: &SqlitePool,
	sender_id: i32,
) -> anyhow::Result<Option<chrono::Duration>> {
	let mut conn = pool.acquire().await?;

	// query lurkers
	let sql = r#"
		SELECT timestamp
			FROM lurkers
			WHERE
				lurker_id=?1;
	"#;

	let lurkers: Vec<DateTimeQR> = sqlx::query_as::<Sqlite, DateTimeQR>(&sql)
		.bind(sender_id)
		.fetch_all(&mut *conn)
		.await?;

	let lurker_timestamp = match lurkers.len() {
		0 => return Ok(None),
		_ => lurkers[0].0
	};

	// remove from lurkers
	let sql = r#"
		DELETE
			FROM lurkers
			WHERE
				lurker_id=?1;
	"#;

	sqlx::query::<Sqlite>(&sql)
		.bind(sender_id)
		.execute(&mut *conn)
		.await?;

	// return duration of lurk
	Ok(Some(Utc::now() - lurker_timestamp))
}

// adds a minute to a chatter's offline time
pub async fn add_offliner_minute(
	pool:        &SqlitePool,
	channel_id:  i32,
	offliner_id: i32,
) -> anyhow::Result<()> {
	let mut conn = pool.acquire().await?;

	let sql = r#"
    INSERT INTO
        CHANNEL_{{ CHANNEL_ID }}_OFFLINERS
			(offliner_id, time_s)
        VALUES
	        (?1, 60)
		ON CONFLICT
		DO UPDATE
			SET time_s = time_s + 60
    "#.replace("{{ CHANNEL_ID }}", &channel_id.to_string());

	sqlx::query::<Sqlite>(&sql)
		.bind(offliner_id)
		.execute(&mut *conn)
		.await?;
	
	Ok(())
}

pub async fn get_offline_time(
	pool:        &SqlitePool,
	channel_id:  i32,
	offliner_id: i32,
) -> anyhow::Result<chrono::Duration> {
	let mut conn = pool.acquire().await?;

	let sql = r#"
		SELECT
			time_s
		FROM
			CHANNEL_{{ CHANNEL_ID }}_OFFLINERS
		WHERE
			offliner_id=$1;
	"#.replace("{{ CHANNEL_ID }}", &channel_id.to_string());

	let offliners_secs = sqlx::query_as::<Sqlite, I32QR>(&sql)
		.bind(offliner_id)
		.fetch_all(&mut *conn)
		.await?;

	match offliners_secs.get(0) {
		Some(a) => return Ok(chrono::Duration::seconds(a.0 as i64)),
		None    => return Ok(chrono::Duration::seconds(0         )),
	}
}

pub async fn new_cmd(
	pool: &SqlitePool,
	channel_id: i32,
	cmd_name:   &str,
	cmd_type:   &str,
	cmd_expr:   &str
) -> anyhow::Result<()> {
	let mut conn = pool.acquire().await?;

    let sql = r#"
    INSERT OR REPLACE
        INTO CHANNEL_{{ CHANNEL_ID }}_COMMANDS
	        (name, type, expression)
        VALUES
	        (?1, ?2, ?3)
    "#.replace("{{ CHANNEL_ID }}", &channel_id.to_string());

	sqlx::query::<Sqlite>(&sql)
		.bind(cmd_name)
		.bind(cmd_type)
		.bind(cmd_expr)
		.execute(&mut *conn)
		.await?;
	
	Ok(())
}

pub async fn get_channel_cmd(
    pool:       &SqlitePool,
    channel_id: i32,
    cmd_name:   &str,
) -> anyhow::Result<Option<(String, String, i32)>> {
	let mut conn = pool.acquire().await?;

	let sql = r#"
		UPDATE CHANNEL_{{ CHANNEL_ID }}_COMMANDS
		SET
			metadata = metadata + 1
		WHERE
			name=?1;
		SELECT type, expression, metadata 
			FROM CHANNEL_{{ CHANNEL_ID }}_COMMANDS
			WHERE
				name=?1
	"#.replace("{{ CHANNEL_ID }}", &channel_id.to_string());

	let cmds: Vec<ChannelCommandQR> = sqlx::query_as::<Sqlite, ChannelCommandQR>(&sql)
		.bind(cmd_name)
		.fetch_all(&mut *conn)
		.await?;

	if cmds.len() == 0 {
		return Ok(None);
	} else {
		return Ok(Some((cmds[0].0.clone(), cmds[0].1.clone(), cmds[0].2)));
	}
}

pub async fn remove_channel_command(
	pool:       &SqlitePool,
	channel_id: i32,
	cmd_name:   &str,
) -> anyhow::Result<i32> {
	let mut conn = pool.acquire().await?;

	let sql = r#"
		DELETE
			FROM CHANNEL_{{ CHANNEL_ID }}_COMMANDS
			WHERE
				name=?1;
		SELECT changes();
	"#.replace("{{ CHANNEL_ID }}", &channel_id.to_string());

	let num_affected: i32 = sqlx::query_as::<Sqlite, I32QR>(&sql)
		.bind(cmd_name)
		.fetch_all(&mut *conn)
		.await?
		[0].0;

	Ok(num_affected)
}

pub async fn get_word_ratio(
	pool:       &SqlitePool,
	channel_id: i32,
	user_id:    i32,
	word:       &str,
	cmd_prefix: char,
) -> anyhow::Result<f32> {
	let mut conn = pool.acquire().await?;

	let with_word_count;
	let total_count;

	// get the count of rows which contain `word`
	let sql = r#"
		SELECT COUNT(*)
			FROM CHANNEL_{{ CHANNEL_ID }}
			WHERE
				sender_id=?1
			AND 
				message LIKE '%?2%'
			AND
				message NOT LIKE '?3%'
	"#
		.replace("{{ CHANNEL_ID }}", &channel_id.to_string())
		// the sqlx templating does not work,
		// so i am manually replacing it here
		.replace("?2", word)
		.replace("?3", &cmd_prefix.to_string());

	with_word_count = sqlx::query_as::<Sqlite, I32QR>(&sql)
		.bind(user_id)
		.fetch_all(&mut *conn)
		.await?[0].0;

	// get total message count
	let sql = r#"
		SELECT COUNT(*)
			FROM CHANNEL_{{ CHANNEL_ID }}
		WHERE
			sender_id=?1
		AND
			message NOT LIKE '?2%';
	"#
		.replace("{{ CHANNEL_ID }}", &channel_id.to_string())
		.replace("?2", &cmd_prefix.to_string());

	total_count = sqlx::query_as::<Sqlite, I32QR>(&sql)
		.bind(user_id)
		.fetch_all(&mut *conn)
		.await?[0].0;
	
	Ok(with_word_count as f32 / total_count as f32)
}

pub enum ChatStatPeriod {
	ThisStream,
	Last24Hours,
	Alltime,
}

impl ChatStatPeriod {
	#[allow(dead_code)]
	pub fn from_vec(v: &Vec<String>) -> Self {
		let args = v.join(" ").to_lowercase();
		let opts = ["stream", "this stream", "24", "last24hours", "all", "alltime"];

		// the default one
		let mut out_idx = 4;

        for opt in &opts {
            if args.contains(opt) {
                out_idx = opts.iter().position(|r| r == opt).unwrap();
                break;
            }
        }

		match out_idx {
			_x @ 0..=1 => Self::ThisStream,
			_x @ 2..=3 => Self::Last24Hours,
			_ => Self::Alltime,
		}
	}

	pub fn from_str(s: &str) -> Self {
		match s.to_lowercase().as_str() {
			"all"      => Self::Alltime,
			"alltime"  => Self::Alltime,
			"all_time" => Self::Alltime,
			"stream"      => Self::ThisStream,
			"thisstream"  => Self::ThisStream,
			"this_stream" => Self::ThisStream,
			"24"            => Self::Last24Hours,
			"last24hours"   => Self::Last24Hours,
			"last_24_hours" => Self::Last24Hours,
			_ => Self::Alltime,
		}
	}
}

pub enum ChatStatsMode {
	Top(u8),           // the number of users
	One(i32),          // the twitch user_id of the one
	WordCount(String), // the phrase, that is being searched for
}

impl ChatStatsMode {
	pub async fn from_cmd(cmd: &CommandSource, twitch_auth: &crate::TwitchAuth) -> anyhow::Result<Self> {
		let args = cmd.args.join(" ").to_lowercase();
		let opts = ["top", "out", "wordcount"];

		// the default one
		let mut out_idx = 0;

        for opt in &opts {
            if args.contains(opt) {
                out_idx = opts.iter().position(|r| r == opt).unwrap();
                break;
            }
        }

		match out_idx {
			0 => {
				// again, the default one
				let mut top_count = 3;

				if let Some(num) = cmd.args.get(2) {
					if let Ok(parsed_num) = num.parse::<u8>() {
						// clamp the number to be 1 <= x <= 5
						if parsed_num > 0 && parsed_num < 6 {
							top_count = parsed_num
						}
					}
				}

				Ok(Self::Top(top_count))
			},
			1 => {
				// again again, the default one
				let mut user_id = cmd.sender.id;

				if let Some(user_name) = cmd.args.get(2) {
					if let Some(id) = crate::api::id_from_nick(user_name, twitch_auth).await? {
						user_id = id;
					}
				}

				Ok(Self::One(user_id))
			},
			2 => {

				let phrase = match cmd.args.get(2) {
					Some(_) => cmd.args[2..].join(" "),
					None    => "".to_owned(),
				};

				Ok(Self::WordCount(phrase))
			},
			_ => Ok(Self::Top(3)),
		}
	}
}

pub async fn get_channel_chat_stats(
	pool:        &SqlitePool,
	channel:     &crate::Channel,
	twitch_auth: &crate::TwitchAuth,
	period:      ChatStatPeriod,
	mode:        ChatStatsMode,
) -> anyhow::Result<Vec<I32I32QR>> {
	let mut conn = pool.acquire().await?;

	let period_clause = match period {
		ChatStatPeriod::Last24Hours => {
			let yesterday = Utc::now() - chrono::Duration::days(1);
			let clause    = format!(">= \"{}\"", yesterday.format("%Y-%m-%d %H:%M:%S"));

			clause
		},
		ChatStatPeriod::Alltime     => "LIKE \"%\"".to_owned(),
		ChatStatPeriod::ThisStream  => {
			let stream_info = crate::api::get_stream_info(twitch_auth, &channel.name).await?.ok_or(MyError::NotFound)?;
			let clause = format!(">= \"{}\"", stream_info.data[0].started_at);

			clause
		},
	};

	let (mode_clause, limit) = match mode {
		ChatStatsMode::One(id) => {
			let clause = format!("AND WHERE sender_id = {id}");

			(clause, 10)
		},
		ChatStatsMode::Top(num) => {
			// is clamped from before
			("".to_owned(), num)
		},
		ChatStatsMode::WordCount(s) => {
			let clause = format!("AND message LIKE \"%{s}%\"");
		
			(clause, 1)
		},
	};

	// get the count of rows which contain `word`
	let sql = r#"
		SELECT sender_id, COUNT(*) AS cnt
			FROM CHANNEL_{{ CHANNEL_ID }}
				WHERE
					timestamp {{ PERIOD_CLAUSE }}
				{{ MODE_CLAUSE }}
				GROUP BY
					sender_id
				ORDER BY
					cnt DESC
				LIMIT {{ LIMIT_NUM }};
	"#
		.replace("{{ CHANNEL_ID }}"   , &channel.id.to_string())
		.replace("{{ PERIOD_CLAUSE }}", &period_clause)
		.replace("{{ MODE_CLAUSE }}"  , &mode_clause)
		.replace("{{ LIMIT_NUM }}"    , &limit.to_string());

	let rows: Vec<I32I32QR> = sqlx::query_as::<Sqlite, I32I32QR>(&sql)
		.fetch_all(&mut *conn)
		.await?;
		
	Ok(rows)
} 
