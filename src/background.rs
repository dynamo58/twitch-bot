use crate::{api, db, Config, TwitchAuth, NameIdCache};

use std::sync::{Mutex, Arc};

use sqlx::SqlitePool;

pub async fn check_for_offliners(
	pool: &SqlitePool,
	config: &Config,
	twitch_auth: &TwitchAuth,
	cache_arc: &Arc<Mutex<NameIdCache>>,
) -> anyhow::Result<u16> {
	let mut count = 0;

	for channel_name in &config.channels {
		if (api::get_stream_info(twitch_auth, channel_name).await?).is_none() {
			if let Some(offliners) = api::get_chatters(channel_name).await? {
				for offliner in &offliners {
					if config.disregarded_users.contains(&offliner.to_lowercase()) {
						continue
					}
					count += 1;

					let mut _offliner_id: Option<i32> = None;
					let mut _channel_id : Option<i32> = None;

					if let Ok(cache) = cache_arc.lock() {
						if let Some(id) = cache.get(offliner) {
							_offliner_id = Some(*id);
						}

						if let Some(id) = cache.get(offliner) {
							_offliner_id = Some(*id);
						}
					}

					if _channel_id.is_none() {
						_channel_id = Some(api::id_from_nick(channel_name, twitch_auth).await?.unwrap());
					}

					if _offliner_id.is_none() {
						_offliner_id = Some(api::id_from_nick(offliner, twitch_auth).await?.unwrap());
					}

					db::add_offliner_minute(pool, _channel_id.unwrap(), _offliner_id.unwrap()).await?;
				}
			}
		}
	}

	Ok(count)
}

pub async fn clear_name_id_cache(
	name_id_cache_arc: &Arc<Mutex<NameIdCache>>,
) -> anyhow::Result<usize> {
	let mut num = 0;

	if let Ok(mut cache) = name_id_cache_arc.lock() {
		num = cache.len();
		(*cache).clear();
	}

	Ok(num)
}
