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
    pool: &SqlitePool,
    name: &str,
) -> anyhow::Result<()> {
	let mut conn = pool.acquire().await?;
	let sql = include_str!("../assets/sql/channel_tables.sql")
		.replace("{{ CHANNEL_NAME }}", name);

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
	let channel = &privmsg.source.params[0][1..];

    let sql = r#"
    INSERT
        INTO {{ TABLE_NAME }} 
	        (sender_id, sender_nick, badges, timestamp, message)
        VALUES
	        (?1, ?2, ?3, ?4, ?5)
    "#.replace("{{ TABLE_NAME }}", channel);

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
					INTO {{ CHANNEL_NAME }}_MARKOV
						(word, succ) 
					VALUES
						($1, $2);
			"#.replace("{{ CHANNEL_NAME }}", &privmsg.source.params[0][1..].to_owned());
				
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
	pool: &SqlitePool,
	channel: &str,
	word: &str
) -> anyhow::Result<Option<String>> {
	let mut conn = pool.acquire().await?;

	let sql = r#"
		SELECT succ
			FROM {{ CHANNEL_NAME }}_MARKOV
			WHERE
				word=$1
			COLLATE NOCASE;
	"#.replace("{{ CHANNEL_NAME }}", channel);

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
	pool:      &SqlitePool,
	sender_id: i32,
	channel:   &str
) -> anyhow::Result<Option<String>> {
	let mut conn = pool.acquire().await?;

	let sql = r#"
		SELECT message
			FROM {{ CHANNEL_NAME }}
			WHERE
				sender_id=$1
			LIMIT 1;
	"#.replace("{{ CHANNEL_NAME }}", channel);

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
	pool:         &SqlitePool,
	channel_name: &str,
	offliner_id:  i32,
) -> anyhow::Result<()> {
	let mut conn = pool.acquire().await?;

	let sql = r#"
    INSERT INTO
        {{ CHANNEL_NAME }}_OFFLINERS
			(offliner_id, time_s)
        VALUES
	        (?1, 60)
		ON CONFLICT
		DO UPDATE
			SET time_s = time_s + 60
    "#.replace("{{ CHANNEL_NAME }}", channel_name);

	sqlx::query::<Sqlite>(&sql)
		.bind(offliner_id)
		.execute(&mut *conn)
		.await?;
	
	Ok(())
}

pub async fn get_offline_time(
	pool: &SqlitePool,
	channel_name: &str,
	offliner_id: i32,
) -> anyhow::Result<chrono::Duration> {
	let mut conn = pool.acquire().await?;

	let sql = r#"
		SELECT
			time_s
		FROM
			{{ CHANNEL_NAME }}_OFFLINERS
		WHERE
			offliner_id=$1;
	"#.replace("{{ CHANNEL_NAME }}", channel_name);

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
	channel_name: &str,
	cmd_name: &str,
	cmd_type: &str,
	cmd_expr: &str
) -> anyhow::Result<()> {
	let mut conn = pool.acquire().await?;

    let sql = r#"
    INSERT OR REPLACE
        INTO {{ TABLE_NAME }}_COMMANDS
	        (name, type, expression)
        VALUES
	        (?1, ?2, ?3)
    "#.replace("{{ TABLE_NAME }}", channel_name);

	sqlx::query::<Sqlite>(&sql)
		.bind(cmd_name)
		.bind(cmd_type)
		.bind(cmd_expr)
		.execute(&mut *conn)
		.await?;
	
	Ok(())
}

pub async fn get_channel_cmd(
    pool:         &SqlitePool,
    channel_name: &str,
    cmd_name:     &str,
) -> anyhow::Result<Option<(String, String, i32)>> {
	let mut conn = pool.acquire().await?;

	let sql = r#"
		UPDATE {{ CHANNEL_NAME }}_COMMANDS
		SET
			metadata = metadata + 1
		WHERE
			name=?1;
		SELECT type, expression, metadata 
			FROM {{ CHANNEL_NAME }}_COMMANDS
			WHERE
				name=?1
	"#.replace("{{ CHANNEL_NAME }}", channel_name);

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
	pool:         &SqlitePool,
	channel_name: &str,
	cmd_name:     &str,
) -> anyhow::Result<i32> {
	let mut conn = pool.acquire().await?;

	let sql = r#"
		DELETE
			FROM {{ CHANNEL_NAME }}_COMMANDS
			WHERE
				name=?1;
		SELECT changes();
	"#.replace("{{ CHANNEL_NAME }}", channel_name);

	let num_affected: i32 = sqlx::query_as::<Sqlite, I32QR>(&sql)
		.bind(cmd_name)
		.fetch_all(&mut *conn)
		.await?
		[0].0;

	Ok(num_affected)
}
