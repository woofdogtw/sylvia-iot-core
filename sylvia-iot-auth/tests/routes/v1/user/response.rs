use std::collections::HashMap;

use serde::Deserialize;
use serde_json::{Map, Value};
use serde_with;

#[derive(Deserialize)]
pub struct GetUser {
    pub data: GetUserData,
}

#[derive(Deserialize)]
pub struct GetUserData {
    pub account: String,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    #[serde(rename = "modifiedAt")]
    pub modified_at: String,
    #[serde(rename = "verifiedAt")]
    pub verified_at: Option<String>,
    pub roles: HashMap<String, bool>,
    pub name: String,
    pub info: Map<String, Value>,
}

#[derive(Deserialize)]
pub struct PostAdminUser {
    pub data: PostAdminUserData,
}

#[derive(Deserialize)]
pub struct PostAdminUserData {
    #[serde(rename = "userId")]
    pub user_id: String,
}

#[derive(Deserialize)]
pub struct GetAdminUserCount {
    pub data: GetAdminUserCountData,
}

#[derive(Deserialize)]
pub struct GetAdminUserCountData {
    pub count: usize,
}

#[derive(Debug, Deserialize)]
pub struct GetAdminUserList {
    pub data: Vec<GetAdminUserListData>,
}

#[derive(Debug, Deserialize)]
pub struct GetAdminUserListData {
    #[serde(rename = "userId")]
    pub user_id: String,
    pub account: String,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    #[serde(rename = "modifiedAt")]
    pub modified_at: String,
    #[serde(rename = "verifiedAt")]
    pub verified_at: Option<String>,
    #[serde(
        rename = "expiredAt",
        default,
        with = "serde_with::rust::double_option"
    )]
    pub expired_at: Option<Option<String>>,
    #[serde(
        rename = "disabledAt",
        default,
        with = "serde_with::rust::double_option"
    )]
    pub disabled_at: Option<Option<String>>,
    pub roles: HashMap<String, bool>,
    pub name: String,
    pub info: Map<String, Value>,
}

#[derive(Debug, Deserialize)]
pub struct GetAdminUser {
    pub data: GetAdminUserListData,
}
