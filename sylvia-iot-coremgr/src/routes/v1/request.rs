use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

#[derive(Deserialize, Serialize)]
pub struct PostApplicationBody {
    pub data: PostApplicationData,
}

#[derive(Deserialize, Serialize)]
pub struct PostApplicationData {
    pub code: String,
    #[serde(rename = "unitId")]
    pub unit_id: String,
    #[serde(rename = "hostUri")]
    pub host_uri: String,
    pub name: Option<String>,
    pub info: Option<Map<String, Value>>,
    #[serde(skip_serializing)]
    pub ttl: Option<usize>,
    #[serde(skip_serializing)]
    pub length: Option<usize>,
}

#[derive(Deserialize, Serialize)]
pub struct PatchApplicationBody {
    pub data: PatchApplicationData,
}

#[derive(Default, Deserialize, Serialize)]
pub struct PatchApplicationData {
    #[serde(rename = "hostUri", skip_serializing_if = "Option::is_none")]
    pub host_uri: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub info: Option<Map<String, Value>>,
    #[serde(skip_serializing)]
    pub ttl: Option<usize>,
    #[serde(skip_serializing)]
    pub length: Option<usize>,
    #[serde(skip_serializing)]
    pub password: Option<String>,
}

#[derive(Deserialize)]
pub struct PostApplicationDlDataBody {
    pub data: PostApplicationDlData,
}

#[derive(Deserialize)]
pub struct PostApplicationDlData {
    #[serde(rename = "deviceId")]
    pub device_id: String,
    pub payload: String,
}

#[derive(Deserialize, Serialize)]
pub struct PostNetworkBody {
    pub data: PostNetworkData,
}

#[derive(Deserialize, Serialize)]
pub struct PostNetworkData {
    pub code: String,
    #[serde(rename = "unitId")]
    pub unit_id: Option<String>,
    #[serde(rename = "hostUri")]
    pub host_uri: String,
    pub name: Option<String>,
    pub info: Option<Map<String, Value>>,
    #[serde(skip_serializing)]
    pub ttl: Option<usize>,
    #[serde(skip_serializing)]
    pub length: Option<usize>,
}

#[derive(Deserialize, Serialize)]
pub struct PatchNetworkBody {
    pub data: PatchNetworkData,
}

#[derive(Default, Deserialize, Serialize)]
pub struct PatchNetworkData {
    #[serde(rename = "hostUri", skip_serializing_if = "Option::is_none")]
    pub host_uri: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub info: Option<Map<String, Value>>,
    #[serde(skip_serializing)]
    pub ttl: Option<usize>,
    #[serde(skip_serializing)]
    pub length: Option<usize>,
    #[serde(skip_serializing)]
    pub password: Option<String>,
}

#[derive(Deserialize)]
pub struct PostNetworkUlDataBody {
    pub data: PostNetworkUlData,
}

#[derive(Deserialize)]
pub struct PostNetworkUlData {
    #[serde(rename = "deviceId")]
    pub device_id: String,
    pub payload: String,
}
