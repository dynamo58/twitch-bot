use crate::TwitchAuth;
use crate::api_models as models;

use std::fmt::Display;

use reqwest::Client;


// —————————————————————————————————————————
//               Twitch API
// —————————————————————————————————————————

// translates users nickname to id
// ref: https://dev.twitch.tv/docs/api/reference#get-users
pub async fn id_from_nick(
    nick: &str,
    auth: &TwitchAuth,
) -> anyhow::Result<Option<i32>> {
    let client = Client::new();

    let res = client
        .get(&format!("https://api.twitch.tv/helix/users?login={nick}"))
        .header("Client-ID", auth.client_id.clone())
        .header("Authorization", format!("Bearer {}", auth.oauth.clone()))
        .send()
        .await?
        .text()
        .await?;

    let parsed: models::UsersResponse = serde_json::from_str(&res)?;

    match parsed.data.get(0) {
        Some(data) => return Ok(Some(data.id.parse::<i32>().unwrap())),
        None => return Ok(None),
    };
}

// translates users id to name
// ref: https://dev.twitch.tv/docs/api/reference#get-users
pub async fn nick_from_id(
    user_id: i32,
    auth: TwitchAuth,
) -> anyhow::Result<String> {
    let client = Client::new();

    let res = client
        .get(&format!("https://api.twitch.tv/helix/users?id={user_id}"))
        .header("Client-ID", auth.client_id.clone())
        .header("Authorization", format!("Bearer {}", auth.oauth.clone()))
        .send()
        .await?
        .text()
        .await?;

    let parsed: models::UsersResponse = serde_json::from_str(&res)?;
    let nick = &parsed.data[0].login;

    Ok(nick.to_owned())
}

// gets all viewers present in a twitch stream
pub async fn get_chatters(
    channel_name: &str,
) -> anyhow::Result<Option<Vec<String>>> {
    let client = Client::new();

    let res = client
        .get(&format!("https://tmi.twitch.tv/group/user/{channel_name}/chatters"))
        .send()
        .await?
        .text()
        .await?;

    let mut parsed: models::ChattersResponse = serde_json::from_str(&res)?;
    let mut chatters = vec![];
    
    // there must be a better way to do this?
    // ... none come to mind

    chatters.append(&mut parsed.chatters.broadcaster);
    chatters.append(&mut parsed.chatters.vips);
    chatters.append(&mut parsed.chatters.moderators);
    chatters.append(&mut parsed.chatters.staff);
    chatters.append(&mut parsed.chatters.admins);
    chatters.append(&mut parsed.chatters.global_mods);
    chatters.append(&mut parsed.chatters.viewers);

    match chatters.len() {
        0 => return Ok(None),
        _ => return Ok(Some(chatters)),
    }
}

// gets information about a stream
// ref: https://dev.twitch.tv/docs/api/reference#get-streams
pub async fn get_stream_info(
    auth: &TwitchAuth,
    channel_name: &str,
) -> anyhow::Result<Option<models::StreamsResponse>> {
    let client = Client::new();

    let res = client
        .get(&format!("https://api.twitch.tv/helix/streams?user_login={channel_name}"))
        .header("Client-ID", auth.client_id.clone())
        .header("Authorization", format!("Bearer {}", auth.oauth.clone()))
        .send()
        .await?
        .text()
        .await?;

    let info: models::StreamsResponse = serde_json::from_str(&res)?;

    match info.data.len() {
        0 => return Ok(None),
        _ => return Ok(Some(info))
    }
}

// fetches all 7tv emotes of specified channel
pub async fn get_7tv_channel_emotes(
    channel_name: &str,
) -> anyhow::Result<Option<Vec<String>>> {
    let client = Client::new();

    let res = client
        .get(&format!("https://api.7tv.app/v2/users/{channel_name}/emotes"))
        .send()
        .await?
        .text()
        .await?;

    let parsed: models::Emotes7TVResponse = serde_json::from_str(&res)?;

    match parsed.len() {
        0 => return Ok(None),
        _ => return Ok(Some(parsed.iter().map(|emote| emote.name.to_string()).collect()))
    }
}

// fetches all 7tv global
pub async fn get_7tv_global_emotes(
) -> anyhow::Result<Option<Vec<String>>> {
    let client = Client::new();

    let res = client
        .get("https://api.7tv.app/v2/emotes/global")
        .send()
        .await?
        .text()
        .await?;

    let parsed: models::GlobalEmotes7TVResponse = serde_json::from_str(&res)?;

    match parsed.len() {
        0 => return Ok(None),
        _ => return Ok(Some(parsed.iter().map(|emote| emote.name.to_string()).collect()))
    }
}

// fetches all bttv emotes of specified channel
pub async fn get_bttv_channel_emotes<T: Display>(
    channel_id: T,
) -> anyhow::Result<Option<Vec<String>>> {
    let client = Client::new();

    let res = client
        .get(&format!("https://api.betterttv.net/3/cached/users/twitch/{channel_id}"))
        .send()
        .await?
        .text()
        .await?;

    let parsed: models::EmotesBttvResponse = serde_json::from_str(&res)?;

    match parsed.channel_emotes.len() + parsed.shared_emotes.len() {
        0 => return Ok(None),
        _ => return {
            let mut emotes = vec![];

            parsed.channel_emotes.iter().for_each(|emote| emotes.push(emote.code.to_owned()));
            parsed.shared_emotes.iter().for_each(|emote| emotes.push(emote.code.to_owned()));

            Ok(Some(emotes))
        }
    }
}

// fetches all bttv global emotes
pub async fn get_bttv_global_emotes(
) -> anyhow::Result<Option<Vec<String>>> {
    let client = Client::new();

    let res = client
        .get("https://api.betterttv.net/3/cached/emotes/global")
        .send()
        .await?
        .text()
        .await?;

    let parsed: models::GlobalEmotesBttvResponse = serde_json::from_str(&res)?;

    match parsed.len() {
        0 => return Ok(None),
        _ => return Ok(Some(parsed.iter().map(|emote| emote.code.to_owned()).collect())),
    }
}

// fetches all ffz emotes of specified channel
pub async fn get_ffz_channel_emotes<T: Display>(
    channel_id: T,
) -> anyhow::Result<Option<Vec<String>>> {
    let client = Client::new();

    let res = client
        .get(&format!("https://api.betterttv.net/3/cached/frankerfacez/users/twitch/{channel_id}"))
        .send()
        .await?
        .text()
        .await?;

    let res = format!("{{\"resp\": {res}}}");
    println!("{res:#?}");
    let parsed: models::EmotesFfzResponse = serde_json::from_str(&res)?;

    match parsed.resp.len() {
        0 => return Ok(None),
        _ => return Ok(Some(parsed.resp.iter().map(|emote| emote.code.to_owned()).collect())),
    }
}

// fetches all ffz global emotes
pub async fn get_ffz_global_emotes(
) -> anyhow::Result<Option<Vec<String>>> {
    let client = Client::new();

    let res = client
        .get("https://api.betterttv.net/3/cached/frankerfacez/emotes/global")
        .send()
        .await?
        .text()
        .await?;

    let parsed: models::GlobalEmotesFfzResponse = serde_json::from_str(&res)?;

    match parsed.len() {
        0 => return Ok(None),
        _ => return Ok(Some(parsed.iter().map(|emote| emote.code.to_owned()).collect())),
    }
}

// fetches all channel emotes
pub async fn get_all_channel_emotes<T: Display>(
    channel_id: T
) -> anyhow::Result<Option<Vec<String>>> {
    let client = Client::new();

    let res = client
        .get(&format!("https://emotes.adamcy.pl/v1/channel/{channel_id}/emotes/all"))
        .send()
        .await?
        .text()
        .await?;

    let parsed: models::ChannelEmotesResponse = serde_json::from_str(&res)?;

    match parsed.len() {
        0 => return Ok(None),
        _ => return Ok(Some(parsed.iter().map(|emote| emote.code.to_owned()).collect())),
    }
}



// https://dev.twitch.tv/docs/api/reference#get-users-follows

// —————————————————————————————————————————
//               Other APIs
// —————————————————————————————————————————

pub async fn get_weather_report(
    location: &str,
) -> anyhow::Result<Option<String>> {
    let client = Client::new();

    let res = client
        .get(&format!("https://wttr.in/{location}?format=j1"))
        .send()
        .await?
        .text()
        .await?;

    let weather: models::WttrInResponse = serde_json::from_str(&res)?;
    let dir = &weather.current_condition[0].winddir16point;

    let temp     = format!("🌡️ {}°C", weather.current_condition[0].temp_c);
    let humid    = format!("🌫️ {}%", weather.current_condition[0].humidity);
    let pressure = format!("🔽 {}hPa", weather.current_condition[0].pressure);
    let precip   = format!("💧 {}mm", weather.current_condition[0].precip_mm);
    let wind     = format!("💨 {}km/h {dir}", weather.current_condition[0].windspeed_kmph);

    // this might cause at some point cause issues
    let area = &weather.nearest_area[0].area_name[0].value;
    let country = &weather.nearest_area[0].country[0].value;

    return Ok(Some(format!("Weather in {area}, {country}: {temp}, {humid}, {pressure}, {precip}, {wind}")));
}
