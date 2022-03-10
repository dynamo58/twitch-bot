use serde::Deserialize;

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
    pub created_at: String,
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

// —————————————————————————————————————————
//               Other APIs
// —————————————————————————————————————————

#[derive(Default, Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WttrInResponse {
    #[serde(rename = "current_condition")]
    pub current_condition: Vec<CurrentCondition>,
    #[serde(rename = "nearest_area")]
    pub nearest_area: Vec<NearestArea>,
    pub request: Vec<Request>,
    pub weather: Vec<Weather>,
}

#[derive(Default, Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CurrentCondition {
    #[serde(rename = "FeelsLikeC")]
    pub feels_like_c: String,
    #[serde(rename = "FeelsLikeF")]
    pub feels_like_f: String,
    pub cloudcover: String,
    pub humidity: String,
    #[serde(rename = "lang_cs")]
    pub lang_cs: Vec<LangC>,
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

#[derive(Default, Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LangC {
    pub value: String,
}

#[derive(Default, Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WeatherDesc {
    pub value: String,
}

#[derive(Default, Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WeatherIconUrl {
    pub value: String,
}

#[derive(Default, Debug, Clone, PartialEq, Deserialize)]
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

#[derive(Default, Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AreaName {
    pub value: String,
}

#[derive(Default, Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Country {
    pub value: String,
}

#[derive(Default, Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Region {
    pub value: String,
}

#[derive(Default, Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WeatherUrl {
    pub value: String,
}

#[derive(Default, Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Request {
    pub query: String,
    #[serde(rename = "type")]
    pub type_field: String,
}

#[derive(Default, Debug, Clone, PartialEq, Deserialize)]
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

#[derive(Default, Debug, Clone, PartialEq, Deserialize)]
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

#[derive(Default, Debug, Clone, PartialEq, Deserialize)]
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
    #[serde(rename = "lang_cs")]
    pub lang_cs: Vec<LangC2>,
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

#[derive(Default, Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LangC2 {
    pub value: String,
}

#[derive(Default, Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WeatherDesc2 {
    pub value: String,
}

#[derive(Default, Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WeatherIconUrl2 {
    pub value: String,
}
