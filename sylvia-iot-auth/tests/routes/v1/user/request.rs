use std::collections::HashMap;

use serde::Serialize;
use serde_json::{Map, Value};

#[derive(Debug, Serialize)]
pub struct PatchUser {
    pub data: PatchUserData,
}

#[derive(Debug, Serialize)]
pub struct PatchUserData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub info: Option<Map<String, Value>>,
}

#[derive(Debug, Serialize)]
pub struct PostAdminUser {
    pub data: PostAdminUserData,
    #[serde(rename = "expiredAt", skip_serializing_if = "Option::is_none")]
    pub expired_at: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PostAdminUserData {
    pub account: String,
    pub password: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub info: Option<Map<String, Value>>,
}

#[derive(Debug, Default, Serialize)]
pub struct GetAdminUserCount {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contains: Option<String>,
}

#[derive(Debug, Default, Serialize)]
pub struct GetAdminUserList {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contains: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fields: Option<String>, // this will be fill from fields_vec automatically.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort: Option<String>, // this will be fill from sort_vec automatically.
    #[serde(skip_serializing)]
    pub fields_vec: Option<Vec<&'static str>>,
    #[serde(skip_serializing)]
    pub sort_vec: Option<Vec<(&'static str, bool)>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
}

#[derive(Debug, Default, Serialize)]
pub struct PatchAdminUser {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<PatchAdminUserData>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disable: Option<bool>,
}

#[derive(Debug, Default, Serialize)]
pub struct PatchAdminUserData {
    #[serde(rename = "verifiedAt", skip_serializing_if = "Option::is_none")]
    pub verified_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub roles: Option<HashMap<String, bool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub info: Option<Map<String, Value>>,
}
