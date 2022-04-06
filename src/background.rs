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
		if let None = api::get_stream_info(&twitch_auth, channel_name).await? {
			if let Some(offliners) = api::get_chatters(channel_name).await? {
				for offliner in &offliners {
					if config.disregarded_users.contains(&offliner.to_lowercase()) {
						continue
					}
					count += 1;

					let mut _id: Option<i32> = None;

					if let Ok(cache) = cache_arc.lock() {
						match cache.get(offliner) {
							Some(id) => { _id = Some(*id); },
							None     => (), 
						};
					}

					match _id {
						Some(id) => {
							db::add_offliner_minute(&pool, channel_name, id).await?;
						},
						None     => match api::id_from_nick(offliner, twitch_auth).await? {
							Some(id) => {
								if let Ok(mut cache) = cache_arc.lock() {
									cache.insert(offliner.to_string(), id);
								}
								
								db::add_offliner_minute(&pool, channel_name, id).await?;
							},
							None     => { continue },
						},
					}
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
