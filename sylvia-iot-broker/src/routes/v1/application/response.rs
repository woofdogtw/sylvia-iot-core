use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

#[derive(Serialize)]
pub struct PostApplication {
    pub data: PostApplicationData,
}

#[derive(Serialize)]
pub struct PostApplicationData {
    #[serde(rename = "applicationId")]
    pub application_id: String,
}

#[derive(Serialize)]
pub struct GetApplicationCount {
    pub data: GetCountData,
}

#[derive(Serialize)]
pub struct GetCountData {
    pub count: u64,
}

#[derive(Serialize)]
pub struct GetApplicationList {
    pub data: Vec<GetApplicationData>,
}

#[derive(Serialize)]
pub struct GetApplication {
    pub data: GetApplicationData,
}

#[derive(Deserialize, Serialize)]
pub struct GetApplicationData {
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
