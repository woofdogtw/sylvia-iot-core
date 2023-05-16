use serde::Serialize;
use serde_json::{Map, Value};

#[derive(Serialize)]
pub struct PostDevice {
    pub data: PostDeviceData,
}

#[derive(Serialize)]
pub struct PostDeviceData {
    #[serde(rename = "deviceId")]
    pub device_id: String,
}

#[derive(Serialize)]
pub struct GetDeviceCount {
    pub data: GetCountData,
}

#[derive(Serialize)]
pub struct GetCountData {
    pub count: u64,
}

#[derive(Serialize)]
pub struct GetDeviceList {
    pub data: Vec<GetDeviceData>,
}

#[derive(Serialize)]
pub struct GetDevice {
    pub data: GetDeviceData,
}

#[derive(Serialize)]
pub struct GetDeviceData {
    #[serde(rename = "deviceId")]
    pub device_id: String,
    #[serde(rename = "unitId")]
    pub unit_id: String,
    #[serde(rename = "unitCode")]
    pub unit_code: Option<String>,
    #[serde(rename = "networkId")]
    pub network_id: String,
    #[serde(rename = "networkCode")]
    pub network_code: String,
    #[serde(rename = "networkAddr")]
    pub network_addr: String,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    #[serde(rename = "modifiedAt")]
    pub modified_at: String,
    pub profile: String,
    pub name: String,
    pub info: Map<String, Value>,
}
