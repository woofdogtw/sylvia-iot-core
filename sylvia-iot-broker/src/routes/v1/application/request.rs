use serde::Deserialize;
use serde_json::{Map, Value};

#[derive(Deserialize)]
pub struct ApplicationIdPath {
    pub application_id: String,
}

#[derive(Deserialize)]
pub struct PostApplicationBody {
    pub data: PostApplicationData,
}

#[derive(Deserialize)]
pub struct PostApplicationData {
    pub code: String,
    #[serde(rename = "unitId")]
    pub unit_id: String,
    #[serde(rename = "hostUri")]
    pub host_uri: String,
    pub name: Option<String>,
    pub info: Option<Map<String, Value>>,
}

#[derive(Deserialize)]
pub struct GetApplicationCountQuery {
    pub unit: Option<String>,
    pub contains: Option<String>,
}

#[derive(Clone, Deserialize)]
pub struct GetApplicationListQuery {
    pub unit: Option<String>,
    pub contains: Option<String>,
    pub offset: Option<u64>,
    pub limit: Option<u64>,
    pub sort: Option<String>,
    pub format: Option<ListFormat>,
}

#[derive(Deserialize)]
pub struct PatchApplicationBody {
    pub data: PatchApplicationData,
}

#[derive(Deserialize)]
pub struct PatchApplicationData {
    #[serde(rename = "hostUri")]
    pub host_uri: Option<String>,
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
