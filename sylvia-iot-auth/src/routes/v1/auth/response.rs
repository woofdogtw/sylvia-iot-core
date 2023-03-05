use std::collections::HashMap;

use serde::Serialize;

#[derive(Serialize)]
pub struct GetTokenInfo {
    pub data: GetTokenInfoData,
}

#[derive(Serialize)]
pub struct GetTokenInfoData {
    #[serde(rename = "userId")]
    pub user_id: String,
    pub account: String,
    pub roles: HashMap<String, bool>,
    pub name: String,
    #[serde(rename = "clientId")]
    pub client_id: String,
    pub scopes: Vec<String>,
}
