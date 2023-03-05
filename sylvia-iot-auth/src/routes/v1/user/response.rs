use std::collections::HashMap;

use serde::Serialize;
use serde_json::{Map, Value};

#[derive(Serialize)]
pub struct GetUser {
    pub data: GetUserData,
}

#[derive(Serialize)]
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

#[derive(Serialize)]
pub struct PostAdminUser {
    pub data: PostAdminUserData,
}

#[derive(Serialize)]
pub struct PostAdminUserData {
    #[serde(rename = "userId")]
    pub user_id: String,
}

#[derive(Serialize)]
pub struct GetAdminUserCount {
    pub data: GetCountData,
}

#[derive(Serialize)]
pub struct GetCountData {
    pub count: u64,
}

#[derive(Serialize)]
pub struct GetAdminUserList {
    pub data: Vec<GetAdminUserData>,
}

#[derive(Serialize)]
pub struct GetAdminUser {
    pub data: GetAdminUserData,
}

#[derive(Serialize)]
pub struct GetAdminUserData {
    #[serde(rename = "userId")]
    pub user_id: String,
    pub account: String,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    #[serde(rename = "modifiedAt")]
    pub modified_at: String,
    #[serde(rename = "verifiedAt")]
    pub verified_at: Option<String>,
    #[serde(rename = "expiredAt", skip_serializing_if = "Option::is_none")]
    pub expired_at: Option<Value>,
    #[serde(rename = "disabledAt", skip_serializing_if = "Option::is_none")]
    pub disabled_at: Option<Value>,
    pub roles: HashMap<String, bool>,
    pub name: String,
    pub info: Map<String, Value>,
}
