use rand::{self, Rng};

use sqlx::sqlite::SqlitePool;
use sqlx::Sqlite;

// QR == query result
#[derive(sqlx::FromRow)]
struct StringQR(String);

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

	for idx in 0..words.len() {
		let word = words[idx];
		let succ = if idx == words.len() - 1 { "" } else { words[idx + 1] };

		let sql = &format!("INSERT INTO {}_MARKOV VALUES ($1, $2);", privmsg.source.params[0][1..].to_owned());

		sqlx::query::<Sqlite>(sql)
		    .bind(word)
			.bind(succ)
		    .execute(&mut *conn)
		    .await?;
	}

	Ok(())
}

pub async fn get_rand_markov_succ<'a>(pool: &SqlitePool, channel: &str, word: &str) -> anyhow::Result<String> {
	let mut conn = pool.acquire().await?;

	let sql = &format!("SELECT succ from {channel}_MARKOV WHERE word=$1;");

	let succs: Vec<String> = sqlx::query_as::<Sqlite, StringQR>(sql)
		.bind(word)
		.fetch_all(&mut *conn)
		.await?
		.iter()
		.map(|succ| succ.0.clone())
		.collect();
	
	let rand_succ = succs[rand::thread_rng().gen_range(0..succs.len())].clone();

	Ok(rand_succ.into())
}

// fn test(s: &str) {}
