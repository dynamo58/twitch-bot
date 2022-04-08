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
    pub id: String,
    #[serde(rename = "user_id")]
    pub user_id: String,
    #[serde(rename = "user_login")]
    pub user_login: String,
    #[serde(rename = "user_name")]
    pub user_name: String,
    #[serde(rename = "game_id")]
    pub game_id: String,
    #[serde(rename = "game_name")]
    pub game_name: String,
    #[serde(rename = "type")]
    pub type_field: String,
    pub title: String,
    #[serde(rename = "viewer_count")]
    pub viewer_count: i64,
    #[serde(rename = "started_at")]
    pub started_at: DateTime<Utc>,
    pub language: String,
    #[serde(rename = "thumbnail_url")]
    pub thumbnail_url: String,
    #[serde(rename = "tag_ids")]
    pub tag_ids: Vec<String>,
    #[serde(rename = "is_mature")]
    pub is_mature: bool,
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

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Root2 {
    pub id: String,
    pub name: String,
    pub owner: Owner,
    pub visibility: i64,
    #[serde(rename = "visibility_simple")]
    pub visibility_simple: Vec<String>,
    pub mime: String,
    pub status: i64,
    pub tags: Vec<String>,
    pub width: Vec<i64>,
    pub height: Vec<i64>,
    pub urls: Vec<Vec<String>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Owner {
    pub id: String,
    #[serde(rename = "twitch_id")]
    pub twitch_id: String,
    pub login: String,
    #[serde(rename = "display_name")]
    pub display_name: String,
    pub role: Role,
    #[serde(rename = "profile_picture_id")]
    pub profile_picture_id: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Role {
    pub id: String,
    pub name: String,
    pub position: i64,
    pub color: i64,
    pub allowed: i64,
    pub denied: i64,
    pub default: Option<bool>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EmotesBttvResponse {
    pub id: String,
    pub bots: Vec<String>,
    pub avatar: String,
    pub channel_emotes: Vec<ChannelEmote>,
    pub shared_emotes: Vec<SharedEmote>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChannelEmote {
    pub id: String,
    pub code: String,
    pub image_type: String,
    pub user_id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SharedEmote {
    pub id: String,
    pub code: String,
    pub image_type: String,
    pub user: User,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub id: String,
    pub name: String,
    pub display_name: String,
    pub provider_id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EmotesFfzResponse {
    pub resp: Vec<Resp>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Resp {
    pub id: i64,
    pub user: User_,
    pub code: String,
    pub images: Images_,
    pub image_type: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct User_ {
    pub id: i64,
    pub name: String,
    pub display_name: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Images_ {
    #[serde(rename = "1x")]
    pub n1x: String,
    #[serde(rename = "2x")]
    pub n2x: Option<String>,
    #[serde(rename = "4x")]
    pub n4x: Option<String>,
}

pub type ChannelEmotesResponse = Vec<Root2___>;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Root2___ {
    pub provider: i64,
    pub code: String,
    pub urls: Vec<Url>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Url {
    pub size: String,
    pub url: String,
}

pub type GlobalEmotes7TVResponse = Vec<Root2Globals>;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Root2Globals {
    pub id: String,
    pub name: String,
    pub owner: OwnerGlobals,
    pub visibility: i64,
    #[serde(rename = "visibility_simple")]
    pub visibility_simple: Vec<String>,
    pub mime: String,
    pub status: i64,
    pub tags: Vec<String>,
    pub width: Vec<i64>,
    pub height: Vec<i64>,
    pub urls: Vec<Vec<String>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OwnerGlobals {
    pub id: String,
    #[serde(rename = "twitch_id")]
    pub twitch_id: String,
    pub login: String,
    #[serde(rename = "display_name")]
    pub display_name: String,
    pub role: RoleGlobals,
    #[serde(rename = "profile_picture_id")]
    pub profile_picture_id: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RoleGlobals {
    pub id: String,
    pub name: String,
    pub position: i64,
    pub color: i64,
    pub allowed: i64,
    pub denied: i64,
    pub default: Option<bool>,
}

pub type GlobalEmotesBttvResponse = Vec<Root2GlobalsBttv>;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Root2GlobalsBttv {
    pub id: String,
    pub code: String,
    pub image_type: String,
    pub user_id: String,
}

pub type GlobalEmotesFfzResponse = Vec<Root2FfzGlobals>;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Root2FfzGlobals {
    pub id: i64,
    pub user: UserFfzGlobals,
    pub code: String,
    pub images: ImagesFfzGlobals,
    pub image_type: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserFfzGlobals {
    pub id: i64,
    pub name: String,
    pub display_name: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImagesFfzGlobals {
    #[serde(rename = "1x")]
    pub n1x: String,
    #[serde(rename = "2x")]
    pub n2x: Option<String>,
    #[serde(rename = "4x")]
    pub n4x: Option<String>,
}


// —————————————————————————————————————————
//               Other APIs
// —————————————————————————————————————————

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WttrInResponse {
    #[serde(rename = "current_condition")]
    pub current_condition: Vec<CurrentCondition>,
    #[serde(rename = "nearest_area")]
    pub nearest_area: Vec<NearestArea>,
    pub request: Vec<Request>,
    pub weather: Vec<Weather>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CurrentCondition {
    #[serde(rename = "FeelsLikeC")]
    pub feels_like_c: String,
    #[serde(rename = "FeelsLikeF")]
    pub feels_like_f: String,
    pub cloudcover: String,
    pub humidity: String,
    pub local_obs_date_time: String,
    #[serde(rename = "observation_time")]
    pub observation_time: String,
    pub precip_inches: String,
    #[serde(rename = "precipMM")]
    pub precip_mm: String,
    pub pressure: String,
    pub pressure_inches: String,
    #[serde(rename = "temp_C")]
    pub temp_c: String,
    #[serde(rename = "temp_F")]
    pub temp_f: String,
    pub uv_index: String,
    pub visibility: String,
    pub visibility_miles: String,
    pub weather_code: String,
    pub weather_desc: Vec<WeatherDesc>,
    pub weather_icon_url: Vec<WeatherIconUrl>,
    #[serde(rename = "winddir16Point")]
    pub winddir16point: String,
    pub winddir_degree: String,
    pub windspeed_kmph: String,
    pub windspeed_miles: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LangC {
    pub value: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WeatherDesc {
    pub value: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WeatherIconUrl {
    pub value: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NearestArea {
    pub area_name: Vec<AreaName>,
    pub country: Vec<Country>,
    pub latitude: String,
    pub longitude: String,
    pub population: String,
    pub region: Vec<Region>,
    pub weather_url: Vec<WeatherUrl>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AreaName {
    pub value: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Country {
    pub value: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Region {
    pub value: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WeatherUrl {
    pub value: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Request {
    pub query: String,
    #[serde(rename = "type")]
    pub type_field: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Weather {
    pub astronomy: Vec<Astronomy>,
    pub avgtemp_c: String,
    pub avgtemp_f: String,
    pub date: String,
    pub hourly: Vec<Hourly>,
    pub maxtemp_c: String,
    pub maxtemp_f: String,
    pub mintemp_c: String,
    pub mintemp_f: String,
    pub sun_hour: String,
    #[serde(rename = "totalSnow_cm")]
    pub total_snow_cm: String,
    pub uv_index: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Astronomy {
    #[serde(rename = "moon_illumination")]
    pub moon_illumination: String,
    #[serde(rename = "moon_phase")]
    pub moon_phase: String,
    pub moonrise: String,
    pub moonset: String,
    pub sunrise: String,
    pub sunset: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Hourly {
    #[serde(rename = "DewPointC")]
    pub dew_point_c: String,
    #[serde(rename = "DewPointF")]
    pub dew_point_f: String,
    #[serde(rename = "FeelsLikeC")]
    pub feels_like_c: String,
    #[serde(rename = "FeelsLikeF")]
    pub feels_like_f: String,
    #[serde(rename = "HeatIndexC")]
    pub heat_index_c: String,
    #[serde(rename = "HeatIndexF")]
    pub heat_index_f: String,
    #[serde(rename = "WindChillC")]
    pub wind_chill_c: String,
    #[serde(rename = "WindChillF")]
    pub wind_chill_f: String,
    #[serde(rename = "WindGustKmph")]
    pub wind_gust_kmph: String,
    #[serde(rename = "WindGustMiles")]
    pub wind_gust_miles: String,
    pub chanceoffog: String,
    pub chanceoffrost: String,
    pub chanceofhightemp: String,
    pub chanceofovercast: String,
    pub chanceofrain: String,
    pub chanceofremdry: String,
    pub chanceofsnow: String,
    pub chanceofsunshine: String,
    pub chanceofthunder: String,
    pub chanceofwindy: String,
    pub cloudcover: String,
    pub humidity: String,
    pub precip_inches: String,
    #[serde(rename = "precipMM")]
    pub precip_mm: String,
    pub pressure: String,
    pub pressure_inches: String,
    pub temp_c: String,
    pub temp_f: String,
    pub time: String,
    pub uv_index: String,
    pub visibility: String,
    pub visibility_miles: String,
    pub weather_code: String,
    pub weather_desc: Vec<WeatherDesc2>,
    pub weather_icon_url: Vec<WeatherIconUrl2>,
    #[serde(rename = "winddir16Point")]
    pub winddir16point: String,
    pub winddir_degree: String,
    pub windspeed_kmph: String,
    pub windspeed_miles: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LangC2 {
    pub value: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WeatherDesc2 {
    pub value: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WeatherIconUrl2 {
    pub value: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeelResponse {
    pub translations: Vec<Translation>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Translation {
    #[serde(rename = "detected_source_language")]
    pub detected_source_language: String,
    pub text: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WikiResponse {
    pub batchcomplete: String,
    pub query: Query,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Query {
    pub normalized: Vec<Normalized>,
    pub pages: HashMap<i32, PagesInner>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Normalized {
    pub from: String,
    pub to: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PagesInner {
    pub pageid: i64,
    pub ns: i64,
    pub title: String,
    pub extract: String,
}

pub type DictionaryResponse = Vec<Root2_>;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Root2_ {
    pub word: String,
    pub phonetic: Option<String>,
    pub phonetics: Vec<Phonetic>,
    pub meanings: Vec<Meaning>,
    pub license: License2,
    pub source_urls: Vec<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Phonetic {
    pub text: Option<String>,
    pub audio: Option<String>,
    pub source_url: Option<String>,
    pub license: Option<License>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct License {
    pub name: String,
    pub url: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Meaning {
    pub part_of_speech: String,
    pub definitions: Vec<Definition>,
    pub synonyms: Vec<String>,
    pub antonyms: Vec<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Definition {
    pub definition: String,
    pub synonyms: Vec<String>,
    pub antonyms: Vec<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct License2 {
    pub name: String,
    pub url: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UrbanDictionaryResponse {
    pub list: Vec<List>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct List {
    pub definition: String,
    pub permalink: String,
    #[serde(rename = "thumbs_up")]
    pub thumbs_up: i64,
    #[serde(rename = "sound_urls")]
    pub sound_urls: Vec<String>,
    pub author: String,
    pub word: String,
    pub defid: i64,
    #[serde(rename = "current_vote")]
    pub current_vote: String,
    #[serde(rename = "written_on")]
    pub written_on: String,
    pub example: String,
    #[serde(rename = "thumbs_down")]
    pub thumbs_down: i64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TwitchFollowResponse {
    pub total: i64,
    pub data: Vec<_Daum>,
    pub pagination: _Pagination,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct _Daum {
    #[serde(rename = "from_id")]
    pub from_id: String,
    #[serde(rename = "from_login")]
    pub from_login: String,
    #[serde(rename = "from_name")]
    pub from_name: String,
    #[serde(rename = "to_id")]
    pub to_id: String,
    #[serde(rename = "to_login")]
    pub to_login: String,
    #[serde(rename = "to_name")]
    pub to_name: String,
    #[serde(rename = "followed_at")]
    pub followed_at: DateTime<Utc>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct _Pagination {
    pub cursor: Option<String>,
}

use serde_json::Value;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubredditResponse {
    pub kind: String,
    pub data: Data___,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Data___ {
    pub after: String,
    pub dist: i64,
    pub modhash: String,
    #[serde(rename = "geo_filter")]
    pub geo_filter: String,
    pub children: Vec<Children>,
    pub before: Value,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Children {
    pub kind: String,
    pub data: Data2___,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Data2___ {
    #[serde(rename = "approved_at_utc")]
    pub approved_at_utc: Value,
    pub subreddit: String,
    pub selftext: String,
    #[serde(rename = "author_fullname")]
    pub author_fullname: String,
    pub saved: bool,
    #[serde(rename = "mod_reason_title")]
    pub mod_reason_title: Value,
    pub gilded: i64,
    pub clicked: bool,
    pub title: String,
    #[serde(rename = "link_flair_richtext")]
    pub link_flair_richtext: Vec<LinkFlairRichtext>,
    #[serde(rename = "subreddit_name_prefixed")]
    pub subreddit_name_prefixed: String,
    pub hidden: bool,
    pub pwls: i64,
    #[serde(rename = "link_flair_css_class")]
    pub link_flair_css_class: Option<String>,
    pub downs: i64,
    #[serde(rename = "thumbnail_height")]
    pub thumbnail_height: i64,
    #[serde(rename = "top_awarded_type")]
    pub top_awarded_type: Value,
    #[serde(rename = "hide_score")]
    pub hide_score: bool,
    pub name: String,
    pub quarantine: bool,
    #[serde(rename = "link_flair_text_color")]
    pub link_flair_text_color: String,
    #[serde(rename = "upvote_ratio")]
    pub upvote_ratio: f64,
    #[serde(rename = "author_flair_background_color")]
    pub author_flair_background_color: Option<String>,
    pub ups: i64,
    #[serde(rename = "total_awards_received")]
    pub total_awards_received: i64,
    #[serde(rename = "media_embed")]
    pub media_embed: MediaEmbed,
    #[serde(rename = "thumbnail_width")]
    pub thumbnail_width: i64,
    #[serde(rename = "author_flair_template_id")]
    pub author_flair_template_id: Option<String>,
    #[serde(rename = "is_original_content")]
    pub is_original_content: bool,
    #[serde(rename = "user_reports")]
    pub user_reports: Vec<Value>,
    #[serde(rename = "secure_media")]
    pub secure_media: Option<SecureMedia>,
    #[serde(rename = "is_reddit_media_domain")]
    pub is_reddit_media_domain: bool,
    #[serde(rename = "is_meta")]
    pub is_meta: bool,
    pub category: Value,
    #[serde(rename = "secure_media_embed")]
    pub secure_media_embed: SecureMediaEmbed,
    #[serde(rename = "link_flair_text")]
    pub link_flair_text: String,
    #[serde(rename = "can_mod_post")]
    pub can_mod_post: bool,
    pub score: i64,
    #[serde(rename = "approved_by")]
    pub approved_by: Value,
    #[serde(rename = "is_created_from_ads_ui")]
    pub is_created_from_ads_ui: bool,
    #[serde(rename = "author_premium")]
    pub author_premium: bool,
    pub thumbnail: String,
    pub edited: bool,
    #[serde(rename = "author_flair_css_class")]
    pub author_flair_css_class: Option<String>,
    #[serde(rename = "author_flair_richtext")]
    pub author_flair_richtext: Vec<AuthorFlairRichtext>,
    pub gildings: Gildings,
    #[serde(rename = "post_hint")]
    pub post_hint: String,
    #[serde(rename = "content_categories")]
    pub content_categories: Value,
    #[serde(rename = "is_self")]
    pub is_self: bool,
    #[serde(rename = "subreddit_type")]
    pub subreddit_type: String,
    pub created: f64,
    #[serde(rename = "link_flair_type")]
    pub link_flair_type: String,
    pub wls: i64,
    #[serde(rename = "removed_by_category")]
    pub removed_by_category: Value,
    #[serde(rename = "banned_by")]
    pub banned_by: Value,
    #[serde(rename = "author_flair_type")]
    pub author_flair_type: String,
    pub domain: String,
    #[serde(rename = "allow_live_comments")]
    pub allow_live_comments: bool,
    #[serde(rename = "selftext_html")]
    pub selftext_html: Value,
    pub likes: Value,
    #[serde(rename = "suggested_sort")]
    pub suggested_sort: Value,
    #[serde(rename = "banned_at_utc")]
    pub banned_at_utc: Value,
    #[serde(rename = "url_overridden_by_dest")]
    pub url_overridden_by_dest: String,
    #[serde(rename = "view_count")]
    pub view_count: Value,
    pub archived: bool,
    #[serde(rename = "no_follow")]
    pub no_follow: bool,
    #[serde(rename = "is_crosspostable")]
    pub is_crosspostable: bool,
    pub pinned: bool,
    #[serde(rename = "over_18")]
    pub over_18: bool,
    pub preview: Preview,
    #[serde(rename = "all_awardings")]
    pub all_awardings: Vec<AllAwarding>,
    pub awarders: Vec<Value>,
    #[serde(rename = "media_only")]
    pub media_only: bool,
    #[serde(rename = "link_flair_template_id")]
    pub link_flair_template_id: String,
    #[serde(rename = "can_gild")]
    pub can_gild: bool,
    pub spoiler: bool,
    pub locked: bool,
    #[serde(rename = "author_flair_text")]
    pub author_flair_text: Option<String>,
    #[serde(rename = "treatment_tags")]
    pub treatment_tags: Vec<Value>,
    pub visited: bool,
    #[serde(rename = "removed_by")]
    pub removed_by: Value,
    #[serde(rename = "mod_note")]
    pub mod_note: Value,
    pub distinguished: Value,
    #[serde(rename = "subreddit_id")]
    pub subreddit_id: String,
    #[serde(rename = "author_is_blocked")]
    pub author_is_blocked: bool,
    #[serde(rename = "mod_reason_by")]
    pub mod_reason_by: Value,
    #[serde(rename = "num_reports")]
    pub num_reports: Value,
    #[serde(rename = "removal_reason")]
    pub removal_reason: Value,
    #[serde(rename = "link_flair_background_color")]
    pub link_flair_background_color: String,
    pub id: String,
    #[serde(rename = "is_robot_indexable")]
    pub is_robot_indexable: bool,
    #[serde(rename = "report_reasons")]
    pub report_reasons: Value,
    pub author: String,
    #[serde(rename = "discussion_type")]
    pub discussion_type: Value,
    #[serde(rename = "num_comments")]
    pub num_comments: i64,
    #[serde(rename = "send_replies")]
    pub send_replies: bool,
    #[serde(rename = "whitelist_status")]
    pub whitelist_status: String,
    #[serde(rename = "contest_mode")]
    pub contest_mode: bool,
    #[serde(rename = "mod_reports")]
    pub mod_reports: Vec<Value>,
    #[serde(rename = "author_patreon_flair")]
    pub author_patreon_flair: bool,
    #[serde(rename = "author_flair_text_color")]
    pub author_flair_text_color: Option<String>,
    pub permalink: String,
    #[serde(rename = "parent_whitelist_status")]
    pub parent_whitelist_status: String,
    pub stickied: bool,
    pub url: String,
    #[serde(rename = "subreddit_subscribers")]
    pub subreddit_subscribers: i64,
    #[serde(rename = "created_utc")]
    pub created_utc: f64,
    #[serde(rename = "num_crossposts")]
    pub num_crossposts: i64,
    pub media: Option<Media>,
    #[serde(rename = "is_video")]
    pub is_video: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkFlairRichtext {
    pub e: String,
    pub t: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaEmbed {
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SecureMedia {
    #[serde(rename = "reddit_video")]
    pub reddit_video: RedditVideo,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RedditVideo {
    #[serde(rename = "bitrate_kbps")]
    pub bitrate_kbps: i64,
    #[serde(rename = "fallback_url")]
    pub fallback_url: String,
    pub height: i64,
    pub width: i64,
    #[serde(rename = "scrubber_media_url")]
    pub scrubber_media_url: String,
    #[serde(rename = "dash_url")]
    pub dash_url: String,
    pub duration: i64,
    #[serde(rename = "hls_url")]
    pub hls_url: String,
    #[serde(rename = "is_gif")]
    pub is_gif: bool,
    #[serde(rename = "transcoding_status")]
    pub transcoding_status: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SecureMediaEmbed {
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthorFlairRichtext {
    pub a: Option<String>,
    pub e: String,
    pub u: Option<String>,
    pub t: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Gildings {
    #[serde(rename = "gid_1")]
    pub gid_1: Option<i64>,
    #[serde(rename = "gid_2")]
    pub gid_2: Option<i64>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Preview {
    pub images: Vec<Image>,
    pub enabled: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Image {
    pub source: Source,
    pub resolutions: Vec<Resolution>,
    pub variants: Variants,
    pub id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Source {
    pub url: String,
    pub width: i64,
    pub height: i64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Resolution {
    pub url: String,
    pub width: i64,
    pub height: i64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Variants {
    pub gif: Option<Gif>,
    pub mp4: Option<Mp4>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Gif {
    pub source: Source2,
    pub resolutions: Vec<Resolution2>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Source2 {
    pub url: String,
    pub width: i64,
    pub height: i64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Resolution2 {
    pub url: String,
    pub width: i64,
    pub height: i64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Mp4 {
    pub source: Source3,
    pub resolutions: Vec<Resolution3>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Source3 {
    pub url: String,
    pub width: i64,
    pub height: i64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Resolution3 {
    pub url: String,
    pub width: i64,
    pub height: i64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AllAwarding {
    #[serde(rename = "giver_coin_reward")]
    pub giver_coin_reward: Value,
    #[serde(rename = "subreddit_id")]
    pub subreddit_id: Value,
    #[serde(rename = "is_new")]
    pub is_new: bool,
    #[serde(rename = "days_of_drip_extension")]
    pub days_of_drip_extension: Value,
    #[serde(rename = "coin_price")]
    pub coin_price: i64,
    pub id: String,
    #[serde(rename = "penny_donate")]
    pub penny_donate: Value,
    #[serde(rename = "award_sub_type")]
    pub award_sub_type: String,
    #[serde(rename = "coin_reward")]
    pub coin_reward: i64,
    #[serde(rename = "icon_url")]
    pub icon_url: String,
    #[serde(rename = "days_of_premium")]
    pub days_of_premium: Option<i64>,
    #[serde(rename = "tiers_by_required_awardings")]
    pub tiers_by_required_awardings: Value,
    #[serde(rename = "resized_icons")]
    pub resized_icons: Vec<ResizedIcon>,
    #[serde(rename = "icon_width")]
    pub icon_width: i64,
    #[serde(rename = "static_icon_width")]
    pub static_icon_width: i64,
    #[serde(rename = "start_date")]
    pub start_date: Value,
    #[serde(rename = "is_enabled")]
    pub is_enabled: bool,
    #[serde(rename = "awardings_required_to_grant_benefits")]
    pub awardings_required_to_grant_benefits: Value,
    pub description: String,
    #[serde(rename = "end_date")]
    pub end_date: Value,
    #[serde(rename = "subreddit_coin_reward")]
    pub subreddit_coin_reward: i64,
    pub count: i64,
    #[serde(rename = "static_icon_height")]
    pub static_icon_height: i64,
    pub name: String,
    #[serde(rename = "resized_static_icons")]
    pub resized_static_icons: Vec<ResizedStaticIcon>,
    #[serde(rename = "icon_format")]
    pub icon_format: Option<String>,
    #[serde(rename = "icon_height")]
    pub icon_height: i64,
    #[serde(rename = "penny_price")]
    pub penny_price: Option<i64>,
    #[serde(rename = "award_type")]
    pub award_type: String,
    #[serde(rename = "static_icon_url")]
    pub static_icon_url: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResizedIcon {
    pub url: String,
    pub width: i64,
    pub height: i64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResizedStaticIcon {
    pub url: String,
    pub width: i64,
    pub height: i64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Media {
    #[serde(rename = "reddit_video")]
    pub reddit_video: RedditVideo2,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RedditVideo2 {
    #[serde(rename = "bitrate_kbps")]
    pub bitrate_kbps: i64,
    #[serde(rename = "fallback_url")]
    pub fallback_url: String,
    pub height: i64,
    pub width: i64,
    #[serde(rename = "scrubber_media_url")]
    pub scrubber_media_url: String,
    #[serde(rename = "dash_url")]
    pub dash_url: String,
    pub duration: i64,
    #[serde(rename = "hls_url")]
    pub hls_url: String,
    #[serde(rename = "is_gif")]
    pub is_gif: bool,
    #[serde(rename = "transcoding_status")]
    pub transcoding_status: String,
}
