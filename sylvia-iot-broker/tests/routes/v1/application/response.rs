use serde::Deserialize;
use serde_json::{Map, Value};

#[derive(Deserialize)]
pub struct PostApplication {
    pub data: PostApplicationData,
}

#[derive(Deserialize)]
pub struct PostApplicationData {
    #[serde(rename = "applicationId")]
    pub application_id: String,
}

#[derive(Deserialize)]
pub struct GetApplicationCount {
    pub data: GetApplicationCountData,
}

#[derive(Deserialize)]
pub struct GetApplicationCountData {
    pub count: usize,
}

#[derive(Debug, Deserialize)]
pub struct GetApplicationList {
    pub data: Vec<GetApplicationListData>,
}

#[derive(Debug, Deserialize)]
pub struct GetApplicationListData {
    #[serde(rename = "applicationId")]
    pub application_id: String,
    pub code: String,
    #[serde(rename = "unitId")]
    pub unit_id: String,
    #[serde(rename = "unitCode")]
    pub unit_code: String,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    #[serde(rename = "modifiedAt")]
    pub modified_at: String,
    #[serde(rename = "hostUri")]
    pub host_uri: String,
    pub name: String,
    pub info: Map<String, Value>,
}

#[derive(Debug, Deserialize)]
pub struct GetApplication {
    pub data: GetApplicationListData,
}
