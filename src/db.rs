use rand::{self, Rng};

use sqlx::sqlite::SqlitePool;
use sqlx::Sqlite;

use thiserror::Error;

// QR == query result
#[derive(sqlx::FromRow)]
struct StringQR(String);

pub async fn init_db(pool: &SqlitePool) -> anyhow::Result<()> {
	let mut conn = pool.acquire().await?;
	let sql = include_str!("../assets/sql/init_db.sql");

	sqlx::query::<Sqlite>(&sql)
		.execute(&mut *conn)
		.await?;
	
	Ok(())
}

// create table for current set channel (if it does not exist)
pub async fn try_create_tables_for_channel(pool: &SqlitePool, name: &str) -> anyhow::Result<()> {
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
pub async fn log(pool: &SqlitePool, privmsg: &twitch_irc::message::PrivmsgMessage) -> anyhow::Result<()> {
	let mut conn = pool.acquire().await?;
	let channel = &privmsg.source.params[0][1..];
	let sql = include_str!("../assets/sql/log_message.sql")
		.replace("{{ TABLE_NAME }}", channel);

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

pub async fn log_markov
(pool: &SqlitePool, privmsg: &twitch_irc::message::PrivmsgMessage)
-> anyhow::Result<()> {
	let mut conn = pool.acquire().await?;

	let words = privmsg.message_text.split(' ').collect::<Vec<&str>>();

	for idx in 0..words.len()-1 {
		let word = format_markov_entry(words[idx])?;
		let succ = format_markov_entry(words[idx + 1])?;

        if let (Some(w), Some(s)) = (word, succ) {
            let sql = &format!(
				"INSERT INTO {}_MARKOV (word, succ) VALUES ($1, $2);",
				privmsg.source.params[0][1..].to_owned()
			);

            sqlx::query::<Sqlite>(sql)
                .bind(w)
                .bind(s)
                .execute(&mut *conn)
                .await?;
        }
	}

	Ok(())
}

    
#[derive(Error, Debug)]
enum MyError {
	#[error("index out of bounds")]
	OutOfBounds,
}

// format the words parsed from the message into format
// acceptible for the db entry
fn format_markov_entry(s: &str) -> anyhow::Result<Option<String>> {
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
        out.chars().next();
    }

    while invalid_back_chars.contains(&out.chars().last().ok_or(MyError::OutOfBounds)?) {
        out.pop();
    }

    // if there is still the blank braille's, 
    if out.contains("⠀") { 
        Ok(None)
    } else {
        Ok(Some(out.to_lowercase()))
    }
}

pub async fn get_rand_markov_succ(
	pool: &SqlitePool,
	channel: &str,
	word: &str
) -> anyhow::Result<Option<String>> {
	let mut conn = pool.acquire().await?;

	let sql = &format!("SELECT succ from {channel}_MARKOV WHERE word=$1;");

	let succs: Vec<String> = sqlx::query_as::<Sqlite, StringQR>(sql)
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
