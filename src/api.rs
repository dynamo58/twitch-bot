use crate::TwitchAuth;
use crate::api_models as models;

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

    let info: Option<models::StreamsResponse> = match serde_json::from_str(&res) {
        Ok(i) => Some(i),
        Err(_)   => None,
    };

    Ok(info)
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
