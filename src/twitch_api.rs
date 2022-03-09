use crate::TwitchAuth;

use reqwest::Client;
use serde::{Deserialize};


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
    // this is missing for some reason?
    // pub email: String,
    #[serde(rename = "created_at")]
    pub created_at: String,
}

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

    let parsed: UsersResponse = serde_json::from_str(&res)?;

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

    let parsed: UsersResponse = serde_json::from_str(&res)?;
    let nick = &parsed.data[0].login;

    Ok(nick.to_owned())
}
