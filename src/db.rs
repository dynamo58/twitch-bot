use rand::{self, Rng};

use chrono::{DateTime, Utc};

use sqlx::sqlite::SqlitePool;
use sqlx::Sqlite;

use crate::MyError;

// QR == query result
#[derive(sqlx::FromRow)]
struct StringQR(String);

#[derive(sqlx::FromRow)]
struct I32QR(i32);

#[derive(sqlx::FromRow, Debug)]
pub struct Reminder {
	pub id: i32,
    pub from_user_id: i32,
    pub for_user_id: i32,
    pub raise_timestamp: DateTime<Utc>,
    pub message: String
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
	let sql = include_str!("../assets/sql/channel_table.sql")
		.replace("{{ TABLE_NAME }}", name);

	sqlx::query::<Sqlite>(&sql)
		.execute(&mut *conn)
		.await?;

	let sql = include_str!("../assets/sql/markov_index_table.sql")
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
		// TODO: this is utterly fucking retarded
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
    privmsg: &twitch_irc::message::PrivmsgMessage
) -> anyhow::Result<()> {
	let mut conn = pool.acquire().await?;

	let words = privmsg.message_text.split(' ').collect::<Vec<&str>>();

	// process each word (besides the last one)
	for idx in 0..words.len()-1 {
		let word = match format_markov_entry(words[idx]) {
			Ok(a) => a,
			Err(_) => return Ok(()),
		};
		let succ = match format_markov_entry(words[idx + 1]) {
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
fn format_markov_entry(s: &str)
-> anyhow::Result<Option<String>> {
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
        Ok(Some(out.to_lowercase()))
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
				word=$1;
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
