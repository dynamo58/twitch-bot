use std::collections::HashMap;

use serde::{Deserialize};
use chrono::{DateTime, Utc};
// use tracing_subscriber::registry::Data;

// —————————————————————————————————————————
//               Twitch API
// —————————————————————————————————————————

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct UsersResponse {
    pub data: Vec<UsersResponseData>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct UsersResponseData {
    pub id: String,
    pub login: String,
    #[serde(rename = "display_name")]
    pub display_name: String,
    #[serde(rename = "type")]
    pub type_field: String,
    #[serde(rename = "broadcaster_type")]
    pub broadcaster_type: String,
    pub description: String,
    #[serde(rename = "profile_image_url")]
    pub profile_image_url: String,
    #[serde(rename = "offline_image_url")]
    pub offline_image_url: String,
    #[serde(rename = "view_count")]
    pub view_count: i64,
	// this field is in the documentation,
	// but missing in the actual response
	// ... the state of the Twitch API smh my head
    // pub email: String,
    #[serde(rename = "created_at")]
    pub created_at: DateTime<Utc>,
}

#[derive(Default, Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChattersResponse {
    #[serde(rename = "_links")]
    pub links: Links,
    #[serde(rename = "chatter_count")]
    pub chatter_count: i64,
    pub chatters: Chatters,
}

#[derive(Default, Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Links {
}

#[derive(Default, Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Chatters {
    pub broadcaster: Vec<String>,
    pub vips: Vec<String>,
    pub moderators: Vec<String>,
    pub staff: Vec<String>,
    pub admins: Vec<String>,
    #[serde(rename = "global_mods")]
    pub global_mods: Vec<String>,
    pub viewers: Vec<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamsResponse {
    pub data: Vec<Daum>,
    pub pagination: Pagination,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Daum {
    #[serde(rename = "started_at")]
    pub started_at: DateTime<Utc>,
}

#[derive(Default, Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Pagination {
    pub cursor: Option<String>,
}

// —————————————————————————————————————————
//               Twitch-related APIs
// —————————————————————————————————————————

pub type Emotes7TVResponse = Vec<Root2>;

#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Root2 {
    pub name: String,
}

#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct EmotesBttvResponse {
    pub channel_emotes: Vec<ChannelEmote>,
    pub shared_emotes: Vec<SharedEmote>,
}

#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ChannelEmote {
    pub id: String,
    pub code: String,
    pub image_type: String,
    pub user_id: String,
}

#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SharedEmote {
    pub id: String,
    pub code: String,
    pub image_type: String,
    pub user: User,
}

#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub id: String,
    pub name: String,
    pub display_name: String,
    pub provider_id: String,
}

#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct EmotesFfzResponse {
    pub resp: Vec<Resp>,
}

#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Resp {
    pub code: String,
}

#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct User_ {
    pub id: i64,
    pub name: String,
    pub display_name: String,
}

pub type ChannelEmotesResponse = Vec<Root2___>;

#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Root2___ {
    pub code: String,
}

pub type GlobalEmotes7TVResponse = Vec<Root2Globals>;

#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Root2Globals {
    pub name: String,
}

pub type GlobalEmotesBttvResponse = Vec<Root2GlobalsBttv>;

#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Root2GlobalsBttv {
    pub code: String,
}

pub type GlobalEmotesFfzResponse = Vec<Root2FfzGlobals>;

#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Root2FfzGlobals {
    pub code: String,
}

#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserFfzGlobals {
    pub name: String,
    pub display_name: String,
}

// —————————————————————————————————————————
//               Other APIs
// —————————————————————————————————————————

#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct WttrInResponse {
    #[serde(rename = "current_condition")]
    pub current_condition: Vec<CurrentCondition>,
    #[serde(rename = "nearest_area")]
    pub nearest_area: Vec<NearestArea>,
}

#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CurrentCondition {
    pub humidity: String,
    #[serde(rename = "precipMm")]
    pub precip_mm: String,
    pub pressure: String,
    #[serde(rename = "temp_C")]
    pub temp_c: String,
    pub winddir16point: String,
    pub winddir_degree: String,
    pub windspeed_kmph: String,
}


#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct NearestArea {
    pub area_name: Vec<AreaName>,
    pub country: Vec<Country>,
}

#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AreaName {
    pub value: String,
}

#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Country {
    pub value: String,
}

#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DeelResponse {
    pub translations: Vec<Translation>,
}

#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Translation {
    #[serde(rename = "detected_source_language")]
    pub detected_source_language: String,
    pub text: String,
}

#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct WikiResponse {
    pub query: Query,
}

#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Query {
    pub pages: HashMap<i32, PagesInner>,
}

#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PagesInner {
    pub pageid: i64,
    pub ns: i64,
    pub title: String,
    pub extract: String,
}

pub type DictionaryResponse = Vec<Root2_>;

#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Root2_ {
    pub word: String,
    pub phonetic: Option<String>,
    pub meanings: Vec<Meaning>,
}

#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Phonetic {
    pub text: Option<String>,
    pub audio: Option<String>,
    pub source_url: Option<String>,
    pub license: Option<License>,
}

#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct License {
    pub name: String,
    pub url: String,
}

#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Meaning {
    pub part_of_speech: String,
    pub definitions: Vec<Definition>,
    pub synonyms: Vec<String>,
    pub antonyms: Vec<String>,
}

#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Definition {
    pub definition: String,
    pub synonyms: Vec<String>,
    pub antonyms: Vec<String>,
}

#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UrbanDictionaryResponse {
    pub list: Vec<List>,
}

#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct List {
    pub definition: String,
    pub word: String,
    pub example: String,
}

#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TwitchFollowResponse {
    pub data: Vec<_Daum>,
    pub total: i64,
}

#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct _Daum {
    #[serde(rename = "followed_at")]
    pub followed_at: DateTime<Utc>,
}

#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SubredditResponse {
    pub data: Data___,
}

#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Data___ {
    pub children: Vec<Children>,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Children {
    pub data: Data2___,
}

#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Data2___ {
    pub selftext: String,
    pub title: String,
    pub url: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IPGeolocationResponse {
    pub timezone: String,
    #[serde(rename = "timezone_offset")]
    pub timezone_offset: i64,
    #[serde(rename = "timezone_offset_with_dst")]
    pub timezone_offset_with_dst: i64,
    #[serde(rename = "date_time")]
    pub date_time: String,
    #[serde(rename = "is_dst")]
    pub is_dst: bool,
}
