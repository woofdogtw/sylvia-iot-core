use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::{Map, Value};

#[derive(Deserialize)]
pub struct UserIdPath {
    pub user_id: String,
}

#[derive(Deserialize)]
pub struct PatchUserBody {
    pub data: PatchUserData,
}

#[derive(Deserialize)]
pub struct PatchUserData {
    pub password: Option<String>,
    pub name: Option<String>,
    pub info: Option<Map<String, Value>>,
}

#[derive(Deserialize)]
pub struct PostAdminUserBody {
    pub data: PostAdminUserData,
    #[serde(rename = "expiredAt")]
    pub expired_at: Option<DateTime<Utc>>,
}

#[derive(Deserialize)]
pub struct PostAdminUserData {
    pub account: String,
    pub password: String,
    pub name: Option<String>,
    pub info: Option<Map<String, Value>>,
}

#[derive(Deserialize)]
pub struct GetAdminUserCountQuery {
    pub account: Option<String>,
    pub contains: Option<String>,
}

#[derive(Clone, Deserialize)]
pub struct GetAdminUserListQuery {
    pub account: Option<String>,
    pub contains: Option<String>,
    pub fields: Option<String>,
    pub offset: Option<u64>,
    pub limit: Option<u64>,
    pub sort: Option<String>,
    pub format: Option<ListFormat>,
}

#[derive(Deserialize)]
pub struct PatchAdminUserBody {
    pub data: Option<PatchAdminUserData>,
    pub disable: Option<bool>,
}

#[derive(Deserialize)]
pub struct PatchAdminUserData {
    #[serde(rename = "verifiedAt")]
    pub verified_at: Option<String>,
    pub roles: Option<HashMap<String, bool>>,
    pub password: Option<String>,
    pub name: Option<String>,
    pub info: Option<Map<String, Value>>,
}

#[derive(Clone, Deserialize, PartialEq)]
pub enum ListFormat {
    #[serde(rename = "array")]
    Array,
    #[serde(rename = "data")]
    Data,
}
