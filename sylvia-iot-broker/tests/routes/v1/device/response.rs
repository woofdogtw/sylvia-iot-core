use serde::Deserialize;
use serde_json::{Map, Value};

#[derive(Deserialize)]
pub struct PostDevice {
    pub data: PostDeviceData,
}

#[derive(Deserialize)]
pub struct PostDeviceData {
    #[serde(rename = "deviceId")]
    pub device_id: String,
}

#[derive(Deserialize)]
pub struct GetDeviceCount {
    pub data: GetDeviceCountData,
}

#[derive(Deserialize)]
pub struct GetDeviceCountData {
    pub count: usize,
}

#[derive(Debug, Deserialize)]
pub struct GetDeviceList {
    pub data: Vec<GetDeviceListData>,
}

#[derive(Debug, Deserialize)]
pub struct GetDeviceListData {
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
    pub name: String,
    pub info: Map<String, Value>,
}

#[derive(Debug, Deserialize)]
pub struct GetDevice {
    pub data: GetDeviceListData,
}
