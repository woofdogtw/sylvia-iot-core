use serde::Serialize;
use serde_json::{Map, Value};

#[derive(Debug, Serialize)]
pub struct PostUnit {
    pub data: PostUnitData,
}

#[derive(Debug, Serialize)]
pub struct PostUnitData {
    pub code: String,
    #[serde(rename = "ownerId", skip_serializing_if = "Option::is_none")]
    pub owner_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub info: Option<Map<String, Value>>,
}

#[derive(Debug, Default, Serialize)]
pub struct GetUnitCount {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub member: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contains: Option<String>,
}

#[derive(Debug, Default, Serialize)]
pub struct GetUnitList {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub member: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contains: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort: Option<String>, // this will be fill from sort_vec automatically.
    #[serde(skip_serializing)]
    pub sort_vec: Option<Vec<(&'static str, bool)>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
}

#[derive(Debug, Default, Serialize)]
pub struct PatchUnit {
    pub data: PatchUnitData,
}

#[derive(Debug, Default, Serialize)]
pub struct PatchUnitData {
    #[serde(rename = "ownerId", skip_serializing_if = "Option::is_none")]
    pub owner_id: Option<String>,
    #[serde(rename = "memberIds", skip_serializing_if = "Option::is_none")]
    pub member_ids: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub info: Option<Map<String, Value>>,
}
