// https://github.com/dynamo58/lovcen/blob/master/src/utils/twitch.js
// https://crates.io/crates/reqwest
    // https://docs.rs/reqwest/0.11.9/reqwest/header/index.html


use reqwest::Client;
use serde::{Serialize, Deserialize};

// use reqwest::header::ACCEPT;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsersResponse {
    pub data: Vec<UsersResponseData>,
}

#[derive(Deserialize)]
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
    pub email: String,
    #[serde(rename = "created_at")]
    pub created_at: String,
}

// translates users nickname to id
// ref: https://dev.twitch.tv/docs/api/reference#get-users
pub async fn id_from_nick(
    nick: &str,
    client_id: &str,
    auth: &str
) -> anyhow::Result<String> {
    let client = Client::new();

    let res = client
        .get(&format!("https://api.twitch.tv/helix/users?login={nick}"))
        .header("Client-ID", client_id)
        .header("Authorization", auth)
        .send()
        .await?;

    let id = res.json::<APIResponse>()
        .await
        .data[0].id

    println!("{:#?}", resp);
    Ok(())
}
