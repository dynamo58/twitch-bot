#![allow(unused)]

use crate::TwitchAuth;
use crate::api_models as models;

use std::fmt::Display;

use chrono::{DateTime, Utc};
use reqwest::Client;


// —————————————————————————————————————————
//               Twitch API
// —————————————————————————————————————————

// ref: https://dev.twitch.tv/docs/api/reference#get-users
pub async fn get_twitch_user(
    nick: &str,
    auth: &TwitchAuth,
) -> anyhow::Result<models::UsersResponse> {
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

    Ok(parsed)
}

pub async fn id_from_nick(
    nick: &str,
    auth: &TwitchAuth,
) -> anyhow::Result<Option<i32>> {
    match get_twitch_user(&nick, &auth).await?.data.get(0) {
        Some(data) => return Ok(Some(data.id.parse::<i32>().unwrap())),
        None       => return Ok(None),
    }
}

pub async fn get_acc_creation_date(
    nick: &str,
    auth: &TwitchAuth,
) -> anyhow::Result<Option<DateTime<Utc>>> {
    match get_twitch_user(&nick, &auth).await?.data.get(0) {
        Some(data) => return Ok(Some(data.created_at)),
        None       => return Ok(None),
    }
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

// ref: https://dev.twitch.tv/docs/api/reference#get-users-follows
// get the date of the follow of a user of a channel.... what?
pub async fn get_followage(
    auth: &TwitchAuth,
    channel_id: i32,
    user_id: i32
) -> anyhow::Result<Option<DateTime<Utc>>> {
    let client = Client::new();

    let res = client
        .get(&format!("https://api.twitch.tv/helix/users/follows?to_id={channel_id}&from_id={user_id}"))
        .header("Client-ID", auth.client_id.clone())
        .header("Authorization", format!("Bearer {}", auth.oauth.clone()))
        .send()
        .await?
        .text()
        .await?;

    let parsed: models::TwitchFollowResponse = serde_json::from_str(&res)?;

    match parsed.total {
        0 => Ok(None),
        _ => Ok(Some(parsed.data[0].followed_at)),
    }
}


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

// pub async fn translate(
//     src_lang: &str,
//     target_lang: &str,
//     text: &str,
// ) -> anyhow::Result<String> {
//     todo!()
//     // let client = Client::new();

//     // let res = client
//     //     .get("https://api-free.deepl.com/v2/translate")
//     //     .form(&[
//     //         ("text"       , text),
//     //         ("target_lang", target_lang)
//     //     ])
//     //     .send()
//     //     .await?
//     //     .text()
//     //     .await?;

//     // dbg!(&res);
//     // let info: models::DeelResponse = serde_json::from_str(&res)?;

//     // Ok(info.translations[0].text.clone())
// }

// query wikipedia for an article gist
pub async fn query_wikipedia(
    phrase: &str,
) -> anyhow::Result<Option<models::WikiResponse>> {
    let client = Client::new();

    let res = client
        .get(&format!("https://en.wikipedia.org/w/api.php?action=query&titles={phrase}&prop=extracts&format=json&exintro=1&exsectionformat=plain&explaintext=1"))
        .send()
        .await?
        .text()
        .await?;

    match serde_json::from_str(&res) {
        Ok(w) => return Ok(Some(w)),
        Err(_) => return Ok(None),
    }
}

// query the english dictionary for an entry
pub async fn query_dictionary(
    word: &str,
) -> anyhow::Result<Option<String>> {
    let client = Client::new();

    let res = client
        .get(&format!("https://api.dictionaryapi.dev/api/v2/entries/en/{word}"))
        .send()
        .await?
        .text()
        .await?;

    let parsed: models::DictionaryResponse = match serde_json::from_str(&res) {
        Ok(tr) => tr,
        Err(_) => return Ok(None),
    };

    let pronunciation = match &parsed[0].phonetic {
        Some(p) => p,
        None    => "",
    };
    let definition = &parsed[0].meanings[0].definitions[0].definition;

    Ok(Some(format!("{pronunciation} {definition}")))
}

// query urban dictionary for an entry
pub async fn query_urban_dictionary(
    term: &str,
) -> anyhow::Result<Option<String>> {
    let client = Client::new();

    let res = client
        .get(&format!("https://api.urbandictionary.com/v0/define?term={term}"))
        .send()
        .await?
        .text()
        .await?;

    let parsed: models::UrbanDictionaryResponse = serde_json::from_str(&res)?;

    match parsed.list.len() {
        0 => return Ok(None),
        _ => {
            let term    = &parsed.list[0].word;
            let def     = parsed.list[0].definition.replace("[", "").replace("]", "");
            let example = parsed.list[0].example.replace("[", "").replace("]", "");
            let more_defs_count = match parsed.list.len() {
                0 => "".to_owned(),
                _ => format!("({} more definitions)", parsed.list.len() - 1),
            };

            return Ok(Some(format!("{term} - {def} | Example: {example} {more_defs_count}")));
        },
    }
}

// upload some text to pastebin
pub async fn upload_to_pastebin(
    text: &str,
) -> anyhow::Result<String> {
    let client = Client::new();

    let params = [
        ("api_dev_key",           &std::env::var("PASTEBIN")?[..]),
        ("api_paste_expire_date", "1D"                           ),
        ("api_paste_code",        text                           ),
        ("api_option",            "paste"                        ),
    ];

    let res = client
        .post(&format!("https://pastebin.com/api/api_post.php"))
        .form(&params)
        .send()
        .await?
        .text()
        .await?;
    
    Ok(res)
}

#[derive(Debug)]
pub enum RedditPostRelevancy {
	Hour,
	Day, 
	Week,
	Month,
	Year,
	All,
}

impl RedditPostRelevancy {
	pub fn new_from_vec(v: &Vec<String>) -> Self {
        let options = ["hour", "day", "week", "month", "year", "all"];
        let mut relevancy = Self::Week;

        for i in 0..options.len() {
            if v.contains(&options[i].to_owned()) {
                relevancy = match i {
                    0 => Self::Hour,
                    1 => Self::Day,
                    2 => Self::Week,
                    3 => Self::Month,
                    4 => Self::Year,
                    5 => Self::All,
                    _ => Self::Week
                }
            }
        }

        relevancy
	}

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Hour  => "hour",
            Self::Day   => "day",
            Self::Week  => "week",
            Self::Month => "month",
            Self::Year  => "year",
            Self::All   => "all"
        }
    }
}

#[derive(Debug)]
pub enum RedditPostType {
	MostUpvotes,
	Random,
}

impl RedditPostType {
	pub fn new_from_vec(v: &Vec<String>) -> Self {
        let options = ["upvotes", "random"];
        let mut post_type = Self::Random;

        for i in 0..options.len() {
            if v.contains(&options[i].to_owned()) {
                post_type = match i {
                    0 => Self::MostUpvotes,
                    1 => Self::Random,
                    _ => Self::Random,
                }
            }
        }

        post_type
	}
}

#[derive(PartialEq, Eq)]
pub enum AdditionalRedditParameter {
    HasMedia,
}

impl AdditionalRedditParameter {
    pub fn new_from_vec(v: &Vec<String>) -> Vec<Self> {
        let mut out = Vec::new();
        
        if v.contains(&"media".to_owned()) || v.contains(&"--has-media".to_owned()) {
            out.push(AdditionalRedditParameter::HasMedia);
        }

        out
    }
}

pub async fn get_reddit_posts(
    subreddit:  &str,
    relevancy:  RedditPostRelevancy,
) -> anyhow::Result<models::SubredditResponse> {
    let relevancy_str = relevancy.as_str();

    let client = Client::new();

    let res = client
        .get(&format!("https://www.reddit.com/r/{subreddit}/top.json?t=${relevancy_str}"))
        .send()
        .await?
        .text()
        .await?;

    let parsed: models::SubredditResponse = serde_json::from_str(&res)?;
    Ok(parsed)
}
