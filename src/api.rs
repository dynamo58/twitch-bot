#![allow(unused)]

use crate::{TwitchAuth, MyError};
use crate::api_models as models;

use std::borrow::BorrowMut;
use std::fmt::Display;

use chrono::{DateTime, Utc};
use rand::{thread_rng, Rng};
use reqwest::Client;

// —————————————————————————————————————————
//               Twitch API
// —————————————————————————————————————————

// ref: https://dev.twitch.tv/docs/api/reference#get-users
pub async fn get_twitch_user(
    nick: &str,
    auth: &TwitchAuth,
) -> anyhow::Result<models::UsersResponse> {
    let info: models::UsersResponse = Client::new()
        .get(&format!("https://api.twitch.tv/helix/users?login={nick}"))
        .header("Client-ID", auth.client_id.clone())
        .header("Authorization", format!("Bearer {}", auth.oauth.clone()))
        .send()
        .await?
        .json()
        .await?;

    Ok(info)
}

pub async fn id_from_nick(
    nick: &str,
    auth: &TwitchAuth,
) -> anyhow::Result<Option<i32>> {
    match get_twitch_user(nick, auth).await?.data.get(0) {
        Some(data) => Ok(Some(data.id.parse::<i32>().unwrap())),
        None       => Ok(None),
    }
}

pub async fn get_acc_creation_date(
    nick: &str,
    auth: &TwitchAuth,
) -> anyhow::Result<Option<DateTime<Utc>>> {
    match get_twitch_user(nick, auth).await?.data.get(0) {
        Some(data) => Ok(Some(data.created_at)),
        None       => Ok(None),
    }
}

// translates users id to name
// ref: https://dev.twitch.tv/docs/api/reference#get-users
pub async fn nick_from_id(
    user_id: i32,
    auth:    &TwitchAuth,
) -> anyhow::Result<String> {
    let res: models::UsersResponse = Client::new()
        .get(&format!("https://api.twitch.tv/helix/users?id={user_id}"))
        .header("Client-ID", auth.client_id.clone())
        .header("Authorization", format!("Bearer {}", auth.oauth.clone()))
        .send()
        .await?
        .json()
        .await?;

    Ok(res.data[0].login.clone())
}

// gets all viewers present in a twitch stream
pub async fn get_chatters(
    channel_name: &str,
) -> anyhow::Result<Option<Vec<String>>> {
    let mut res: models::ChattersResponse = Client::new()
        .get(&format!("https://tmi.twitch.tv/group/user/{channel_name}/chatters"))
        .send()
        .await?
        .json()
        .await?;

    let mut chatters = vec![];
    
    chatters.append(&mut res.chatters.broadcaster);
    chatters.append(&mut res.chatters.vips);
    chatters.append(&mut res.chatters.moderators);
    chatters.append(&mut res.chatters.staff);
    chatters.append(&mut res.chatters.admins);
    chatters.append(&mut res.chatters.global_mods);
    chatters.append(&mut res.chatters.viewers);

    match chatters.len() {
        0 => Ok(None),
        _ => Ok(Some(chatters)),
    }
}

// gets information about a stream
// ref: https://dev.twitch.tv/docs/api/reference#get-streams
pub async fn get_stream_info(
    auth: &TwitchAuth,
    channel_name: &str,
) -> anyhow::Result<Option<models::StreamsResponse>> {
    let res: models::StreamsResponse = Client::new()
        .get(&format!("https://api.twitch.tv/helix/streams?user_login={channel_name}"))
        .header("Client-ID", auth.client_id.clone())
        .header("Authorization", format!("Bearer {}", auth.oauth.clone()))
        .send()
        .await?
        .json()
        .await?;

    match res.data.len() {
        0 => Ok(None),
        _ => Ok(Some(res))
    }
}

// check if a streamer is live
pub async fn streamer_is_live(
    auth: &TwitchAuth,
    channel_name: &str,
) -> anyhow::Result<bool> {
    match get_stream_info(auth, channel_name).await? {
        Some(_) => Ok(true),
        None    => Ok(false),
    }
}

// fetches all 7tv emotes of specified channel
pub async fn get_7tv_channel_emotes(
    channel_name: &str,
) -> anyhow::Result<Option<Vec<String>>> {
    let res: models::Emotes7TVResponse = Client::new()
        .get(&format!("https://api.7tv.app/v2/users/{channel_name}/emotes"))
        .send()
        .await?
        .json()
        .await?;

    match res.len() {
        0 => Ok(None),
        _ => Ok(Some(res.iter().map(|emote| emote.name.to_string()).collect()))
    }
}

// fetches all 7tv global
pub async fn get_7tv_global_emotes(
) -> anyhow::Result<Option<Vec<String>>> {
    let res: models::GlobalEmotes7TVResponse = Client::new()
        .get("https://api.7tv.app/v2/emotes/global")
        .send()
        .await?
        .json()
        .await?;

    match res.len() {
        0 => Ok(None),
        _ => Ok(Some(res.iter().map(|emote| emote.name.to_string()).collect()))
    }
}

// fetches all bttv emotes of specified channel
pub async fn get_bttv_channel_emotes<T: Display>(
    channel_id: T,
) -> anyhow::Result<Option<Vec<String>>> {
    let res: models::EmotesBttvResponse = Client::new()
        .get(&format!("https://api.betterttv.net/3/cached/users/twitch/{channel_id}"))
        .send()
        .await?
        .json()
        .await?;

    match res.channel_emotes.len() + res.shared_emotes.len() {
        0 => Ok(None),
        _ => return {
            let mut emotes = vec![];

            res.channel_emotes.iter().for_each(|emote| emotes.push(emote.code.to_owned()));
            res.shared_emotes.iter().for_each(|emote| emotes.push(emote.code.to_owned()));

            Ok(Some(emotes))
        }
    }
}

// fetches all bttv global emotes
pub async fn get_bttv_global_emotes(
) -> anyhow::Result<Option<Vec<String>>> {
    let res: models::GlobalEmotesBttvResponse  = Client::new()
        .get("https://api.betterttv.net/3/cached/emotes/global")
        .send()
        .await?
        .json()
        .await?;

    match res.len() {
        0 => Ok(None),
        _ => Ok(Some(res.iter().map(|emote| emote.code.to_owned()).collect())),
    }
}

// fetches all ffz emotes of specified channel
pub async fn get_ffz_channel_emotes<T: Display>(
    channel_id: T,
) -> anyhow::Result<Option<Vec<String>>> {
    let res: Vec<models::EmotesFfzResponse> = Client::new()
        .get(&format!("https://api.betterttv.net/3/cached/frankerfacez/users/twitch/{channel_id}"))
        .send()
        .await?
        .json()
        .await?;

    match res.len() {
        0 => Ok(None),
        _ => Ok(Some(res.iter().map(|emote| emote.code.to_owned()).collect())),
    }
}

// fetches all ffz global emotes
pub async fn get_ffz_global_emotes(
) -> anyhow::Result<Option<Vec<String>>> {
    let res: models::GlobalEmotesFfzResponse = Client::new()
        .get("https://api.betterttv.net/3/cached/frankerfacez/emotes/global")
        .send()
        .await?
        .json()
        .await?;

    match res.len() {
        0 => Ok(None),
        _ => Ok(Some(res.iter().map(|emote| emote.code.to_owned()).collect())),
    }
}

// fetches all channel emotes
pub async fn get_all_channel_emotes<T: Display>(
    channel_id: T
) -> anyhow::Result<Option<Vec<String>>> {
    let res: models::ChannelEmotesResponse = Client::new()
        .get(&format!("https://emotes.adamcy.pl/v1/channel/{channel_id}/emotes/all"))
        .send()
        .await?
        .json()
        .await?;

    match res.len() {
        0 => Ok(None),
        _ => Ok(Some(res.iter().map(|emote| emote.code.to_owned()).collect())),
    }
}

// ref: https://dev.twitch.tv/docs/api/reference#get-users-follows
// get the date of the follow of a user of a channel.... what?
pub async fn get_followage(
    auth: &TwitchAuth,
    channel_id: i32,
    user_id: i32
) -> anyhow::Result<Option<DateTime<Utc>>> {
    let res: models::TwitchFollowResponse = Client::new()
        .get(&format!("https://api.twitch.tv/helix/users/follows?to_id={channel_id}&from_id={user_id}"))
        .header("Client-ID", auth.client_id.clone())
        .header("Authorization", format!("Bearer {}", auth.oauth.clone()))
        .send()
        .await?
        .json()
        .await?;

    match res.total {
        0 => Ok(None),
        _ => Ok(Some(res.data[0].followed_at)),
    }
}


// —————————————————————————————————————————
//               Other APIs
// —————————————————————————————————————————

pub async fn get_weather_report(
    location: &str,
) -> anyhow::Result<Option<String>> {
    let weather: models::WttrInResponse = Client::new()
        .get(&format!("https://wttr.in/{location}?format=j1"))
        .send()
        .await?
        .json()
        .await?;

    let dir = &weather.current_condition[0].winddir16point;

    let temp     = format!("🌡️ {}°C", weather.current_condition[0].temp_c);
    let humid    = format!("🌫️ {}%", weather.current_condition[0].humidity);
    let pressure = format!("🔽 {}hPa", weather.current_condition[0].pressure);
    let precip   = format!("💧 {}mm", weather.current_condition[0].precip_mm);
    let wind     = format!("💨 {}km/h {dir}", weather.current_condition[0].windspeed_kmph);

    // this might cause at some point cause issues
    let area = &weather.nearest_area[0].area_name[0].value;
    let country = &weather.nearest_area[0].country[0].value;

    Ok(Some(format!("Weather in {area}, {country}: {temp}, {humid}, {pressure}, {precip}, {wind}")))
}

// query wikipedia for an article gist
pub async fn query_wikipedia(
    phrase: &str,
) -> anyhow::Result<Option<models::WikiResponse>> {
    let res = Client::new()
        .get(&format!("https://en.wikipedia.org/w/api.php?action=query&titles={phrase}&prop=extracts&format=json&exintro=1&exsectionformat=plain&explaintext=1"))
        .send()
        .await?
        .json::<models::WikiResponse>()
        .await;

    match res {
        Ok(w) =>  Ok(Some(w)),
        Err(_) => Ok(None),
    }
}

// query the english dictionary for an entry
pub async fn query_dictionary(
    word: &str,
) -> anyhow::Result<Option<String>> {
    let res = Client::new()
        .get(&format!("https://api.dictionaryapi.dev/api/v2/entries/en/{word}"))
        .send()
        .await?
        .json::<models::DictionaryResponse>()
        .await;


    let res = match res {
        Ok(a) => a,
        Err(_) => return Ok(None),
    };

    let pronunciation = match &res[0].phonetic {
        Some(p) => p,
        None    => "",
    };
    let definition = &res[0].meanings[0].definitions[0].definition;

    Ok(Some(format!("{pronunciation} {definition}")))
}

// query urban dictionary for an entry
pub async fn query_urban_dictionary(
    term: &str,
) -> anyhow::Result<Option<String>> {
    let res: models::UrbanDictionaryResponse = Client::new()
        .get(&format!("https://api.urbandictionary.com/v0/define?term={term}"))
        .send()
        .await?
        .json()
        .await?;

    match res.list.len() {
        0 => Ok(None),
        _ => {
            let term    = &res.list[0].word;
            let def     = res.list[0].definition.replace('[', "").replace(']', "");
            let example = res.list[0].example.replace('[', "").replace(']', "");
            let more_defs_count = match res.list.len() {
                0 => "".to_owned(),
                _ => format!("({} more definitions)", res.list.len() - 1),
            };

            return Ok(Some(format!("{term} - {def} | Example: {example} {more_defs_count}")));
        },
    }
}

// upload some text to pastebin
pub async fn upload_to_pastebin(
    text: &str,
) -> anyhow::Result<String> {
    let params = [
        ("api_dev_key"          , &std::env::var("PASTEBIN_API_KEY")?[..]),
        ("api_paste_expire_date", "1D"                                   ),
        ("api_paste_code"       , text                                   ),
        ("api_option"           , "paste"                                ),
    ];

    let res = Client::new()
        .post("https://pastebin.com/api/api_post.php")
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
	pub fn new_from_vec(v: &[String]) -> Self {
        let options = ["hour", "day", "week", "month", "year", "all", "alltime"];
        let mut relevancy = Self::Week;

        for (i, opt) in options.iter().enumerate() {
            if v.contains(&opt.to_string()) {
                relevancy = match i {
                    0 => Self::Hour,
                    1 => Self::Day,
                    2 => Self::Week,
                    3 => Self::Month,
                    4 => Self::Year,
                    5 => Self::All,
                    6 => Self::All,
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
	pub fn new_from_vec(v: &[String]) -> Self {
        let options = ["upvotes", "random"];
        let mut post_type = Self::Random;

        for (i, opt) in options.iter().enumerate() {
            if v.contains(&opt.to_string()) {
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
    pub fn new_from_vec(v: &[String]) -> Vec<Self> {
        let mut out = Vec::new();
        
        if v.contains(&"media".to_owned()) || v.contains(&"--has-media".to_owned()) {
            out.push(AdditionalRedditParameter::HasMedia);
        }

        out
    }
}

// get reddit posts from a sub
pub async fn get_reddit_posts(
    subreddit:  &str,
    relevancy:  &RedditPostRelevancy,
) -> anyhow::Result<models::SubredditResponse> {
    let relevancy_str = relevancy.as_str();

    let res: models::SubredditResponse = Client::new()
        .get(&format!("https://www.reddit.com/r/{subreddit}/top.json?limit=30&t=${relevancy_str}"))
        .send()
        .await?
        .json()
        .await?;

    Ok(res)
}

// get the time in a location
pub async fn get_time(
    location: &str
) -> anyhow::Result<Option<String>> {
    let api_key = &std::env::var("IPGEOLOCATION_API_KEY")?[..];
    let res = Client::new()
        .get(&format!("https://api.ipgeolocation.io/timezone?apiKey={api_key}&location={location}"))
        .send()
        .await?
        .json::<models::IPGeolocationResponse>()
        .await;
    
    let res = match res {
        Ok(p) => p,
        Err(_) => return Ok(None),
    };

    let gmt_offset = {
        if res.is_dst {
            format!("GMT+{}", res.timezone_offset_with_dst)
        } else {
            format!("GMT+{}", res.timezone_offset)
        }
    };

    Ok(Some(format!("{}, (timezone {} {gmt_offset})", res.date_time, res.timezone)))
}

pub enum HolyBook {
    Quran,
    Bible,
    Tanakh,
}

impl std::str::FromStr for HolyBook {
    type Err = MyError;

    fn from_str(s: &str) -> anyhow::Result<Self, Self::Err> {
        match &s.to_lowercase()[..] {
            "quran"  => Ok(Self::Quran),
            "bible"  => Ok(Self::Bible),
            "tanakh" => Ok(Self::Tanakh),
            _        => Err(MyError::NotFound),
        }
    }
}

// get a random verse from tanakh / bible / quran (curr devotionalium.com api)
pub async fn get_rand_holy_book_verse(
    book_kind: HolyBook,
) -> anyhow::Result<models::HolyBook> {
    let rand_year : u16 = rand::thread_rng().gen_range(1000..2023);
    let rand_month: u16 = rand::thread_rng().gen_range(1..13);
    let rand_day  : u16 = rand::thread_rng().gen_range(1..29);

    let res: models::DevotionaliumResponse = Client::new()
        .get(&format!("https://devotionalium.com/api/v2?date={rand_year}-{rand_month}-{rand_day}"))
        .send()
        .await?
        .json()
        .await?;
    
    match book_kind {
        HolyBook::Bible  => Ok(res.bible),
        HolyBook::Tanakh => Ok(res.tanakh),
        HolyBook::Quran  => Ok(res.quran),
    }
}

// https://opentdb.com/api_config.php
#[derive(PartialEq)]
pub enum TriviaCategory {
    Any,
    GeneralKnowledge,
    EntertainmentBoardGames,
    EntertainmentBooks,
    EntertainmentCartoonAndAnimations,
    EntertainmentComics,
    EntertainmentFilm,
    EntertainmentJapaneseAnimeAndSaga,
    EntertainmentMusic,
    EntertainmentMusicalsAndTheatres,
    EntertainmentTelevision,
    EntertainmentVideoGames,
    ScienceAndNature,
    ScienceComputers,
    ScienceGadgets,
    ScienceMathematics,
    Mythology,
    Sports,
    Geography,
    History,
    Politics,
    Art,
    Celebrities,
    Animals,
    Vehicles,
}

impl TriviaCategory {
    pub fn from_vec(v: &[String]) -> Self {
        let args = v.join(" ").to_lowercase();
        let cats = ["any category", "general knowledge", "board games", "books", "cartoons", "comics", "film", "anime", "music", "musical", "musicals", "theatre", "television", "games", "video games", "science", "cs", "computer science", "gadgets", "math", "mathematics", "mythology", "sport", "sports", "geography", "geo", "history", "politics", "art", "celebrities", "animals", "vehicles"];

        // the default index
        let mut cat_idx = 0;

        for cat in &cats {
            if args.contains(cat) {
                cat_idx = cats.iter().position(|r| r == cat).unwrap();
                break;
            }
        }

        // this is rather stupid but no other way to do it i guess
        // and not bored enough to do an ad hoc macro
        match cat_idx {
            1  => Self::GeneralKnowledge,
            2  => Self::EntertainmentBoardGames,
            3  => Self::EntertainmentBooks,
            4  => Self::EntertainmentCartoonAndAnimations,
            5  => Self::EntertainmentComics,
            6  => Self::EntertainmentFilm,
            7  => Self::EntertainmentJapaneseAnimeAndSaga,
            8  => Self::EntertainmentMusic,
            9  => Self::EntertainmentMusicalsAndTheatres,
            10 => Self::EntertainmentMusicalsAndTheatres,
            11 => Self::EntertainmentMusicalsAndTheatres,
            12 => Self::EntertainmentTelevision,
            13 => Self::EntertainmentVideoGames,
            14 => Self::EntertainmentVideoGames,
            15 => Self::ScienceAndNature,
            16 => Self::ScienceComputers,
            17 => Self::ScienceComputers,
            18 => Self::ScienceGadgets,
            19 => Self::ScienceMathematics,
            20 => Self::ScienceMathematics,
            21 => Self::Mythology,
            22 => Self::Sports,
            23 => Self::Sports,
            24 => Self::Geography,
            25 => Self::Geography,
            26 => Self::History,
            27 => Self::Politics,
            28 => Self::Art,
            29 => Self::Celebrities,
            30 => Self::Animals,
            31 => Self::Vehicles,
            _ => Self::Any,
        }
    }

    pub fn to_opentdb_index(&self) -> &'static str {
        match self {
            // this is literally cringe
            // the fact that they have this as "any"
            // means that i cannot just simply use
            // an enum associated value, NO NO NO
            Self::Any                               => "any", // like why can't this just be 0 ...
            Self::GeneralKnowledge                  => "9",
            Self::EntertainmentBooks                => "10",
            Self::EntertainmentFilm                 => "11",
            Self::EntertainmentMusic                => "12",
            Self::EntertainmentMusicalsAndTheatres  => "13",
            Self::EntertainmentTelevision           => "14",
            Self::EntertainmentVideoGames           => "15",
            Self::EntertainmentBoardGames           => "16",
            Self::ScienceAndNature                  => "17",
            Self::ScienceComputers                  => "18",
            Self::ScienceMathematics                => "19",
            Self::Mythology                         => "20",
            Self::Sports                            => "21",
            Self::Geography                         => "22",
            Self::History                           => "23",
            Self::Politics                          => "24",
            Self::Art                               => "25",
            Self::Celebrities                       => "26",
            Self::Animals                           => "27",
            Self::Vehicles                          => "28",
            Self::EntertainmentComics               => "29",
            Self::ScienceGadgets                    => "30",
            Self::EntertainmentJapaneseAnimeAndSaga => "31",
            Self::EntertainmentCartoonAndAnimations => "32",
        }
    }
}

#[derive(PartialEq)]
pub enum TriviaDifficulty {
    Any,
    Easy,
    Medium,
    Hard,
}

impl TriviaDifficulty {
    pub fn from_vec(v: &[String]) -> Self {
        let args = v.join(" ").to_lowercase();
        let cats = ["any difficulty", "easy", "medium", "hard"];
        
        // the default difficulty
        let mut cat_idx = 0;

        for cat in &cats {
            if args.contains(cat) {
                cat_idx = cats.iter().position(|r| r == cat).unwrap();
                break;
            }
        }

        match cat_idx {
            1 => Self::Easy,
            2 => Self::Medium,
            3 => Self::Hard,
            _ => Self::Any,
        }
    }

    pub fn to_opentdb_index(&self) -> &'static str {
        match self {
            Self::Any    => "any",
            Self::Easy   => "easy",
            Self::Medium => "medium",
            Self::Hard   => "hard",
        }
    }
}

#[derive(PartialEq)]
pub enum TriviaType {
    Any,
    Multiple,
    TrueFalse,
}

impl TriviaType {
    pub fn from_vec(v: &[String]) -> Self {
        let args = v.join(" ").to_lowercase();
        let cats = ["any type", "multiple", "true false"];

        // the default index
        let mut cat_idx = 1;

        for cat in &cats {
            if args.contains(cat) {
                cat_idx = cats.iter().position(|r| r == cat).unwrap();
                break;
            }
        }

        // this is rather stupid but no other way to do it i guess
        // and not bored enough to do an ad hoc macro
        match cat_idx {
            0 => Self::Any,
            2 => Self::TrueFalse,
            _ => Self::Multiple,
        }
    }

    pub fn to_opentdb_index(&self) -> &'static str {
        match self {
            Self::Any       => "any",
            Self::Multiple  => "multiple",
            Self::TrueFalse => "boolean",
        }
    }
}

// get a trivia question from opentdb
pub async fn fetch_trivia_question(
    cat:   TriviaCategory,
    diff:  TriviaDifficulty,
    ttype: TriviaType,
) -> anyhow::Result<models::TriviaQuestion> {
    let cat = {
        if cat == TriviaCategory::Any {
            "".into()
        } else {
            format!("&category={}", cat.to_opentdb_index())
        }
    };
    let diff = {
        if diff == TriviaDifficulty::Any {
            "".into()
        } else {
            format!("&difficulty={}", diff.to_opentdb_index())
        }
    };
    let ttype = {
        if ttype == TriviaType::Any {
            "".into()
        } else {
            format!("&type={}", ttype.to_opentdb_index())
        }
    };

    let res: models::TriviaResponse = Client::new()
        .get(&format!("https://opentdb.com/api.php?amount=1{cat}{diff}{ttype}"))
        .send()
        .await?
        .json()
        .await?;
    
    Ok(res.results[0].clone())
}

// query a question (currently WolframAlpha)
pub async fn query_generic(
    query: &str,
) -> anyhow::Result<Option<String>> {
    let formatted_query = crate::convert_to_html_encoding(query.to_owned());
    let appid = &std::env::var("WOLFRAMALPHA_APPID")?[..];

    let res: models::WolframAlphaResponse = Client::new()
        .get(&format!("http://api.wolframalpha.com/v2/query?input={formatted_query}&appid={appid}&output=json"))
        .send()
        .await?
        .json()
        .await?;

    if let Some(pods) = res.queryresult.pods {
        let main_pod: Vec<models::Pod> = pods
            .into_iter()    
            .filter(|p| p.primary == Some(true))
            .collect();
        
        if main_pod.is_empty() {
            return Ok(None);
        }
         
        if let Some(subpods) = &main_pod[0].subpods {
            let answer = subpods[0].plaintext.clone();
            return Ok(Some(answer));
        }
    } 

    Ok(None)
}

// get information about a github repository
pub async fn get_github_repo_info(
    url: &str,
) -> anyhow::Result<models::GitHubRepoResponse> {
    // yes, it has to be wrapped like this
    Ok(Client::new()
        .get(url)
        .header("Accept", "application/vnd.github.v3+json")
        .header("User-Agent", "dynamo58")
        .send()
        .await?
        .json::<models::GitHubRepoResponse>()
        .await?)
}

// get some words of wisdom from  inspirebot.me
pub async fn get_inspire_image()
-> anyhow::Result<String> {
    Ok(Client::new()
        .get("https://inspirobot.me/api?generate=true")
        .send()
        .await?
        .text()
        .await?
    )
}