use serde::Serialize;
use serde_json::{Map, Value};

#[derive(Debug, Serialize)]
pub struct PostApplication {
    pub data: PostApplicationData,
}

#[derive(Debug, Serialize)]
pub struct PostApplicationData {
    pub code: String,
    #[serde(rename = "unitId")]
    pub unit_id: String,
    #[serde(rename = "hostUri")]
    pub host_uri: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub info: Option<Map<String, Value>>,
}

#[derive(Debug, Default, Serialize)]
pub struct GetApplicationCount {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contains: Option<String>,
}

#[derive(Debug, Default, Serialize)]
pub struct GetApplicationList {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
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
pub struct PatchApplication {
    pub data: PatchApplicationData,
}

#[derive(Debug, Default, Serialize)]
pub struct PatchApplicationData {
    #[serde(rename = "hostUri", skip_serializing_if = "Option::is_none")]
    pub host_uri: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub info: Option<Map<String, Value>>,
}
