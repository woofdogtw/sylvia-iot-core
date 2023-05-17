use serde::Serialize;
use serde_json::{Map, Value};

#[derive(Debug, Serialize)]
pub struct PostDevice {
    pub data: PostDeviceData,
}

#[derive(Debug, Serialize)]
pub struct PostDeviceData {
    #[serde(rename = "unitId")]
    pub unit_id: String,
    #[serde(rename = "networkId")]
    pub network_id: String,
    #[serde(rename = "networkAddr")]
    pub network_addr: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub info: Option<Map<String, Value>>,
}

#[derive(Debug, Serialize)]
pub struct PostDeviceBulk {
    pub data: PostDeviceBulkData,
}

#[derive(Debug, Serialize)]
pub struct PostDeviceBulkData {
    #[serde(rename = "unitId")]
    pub unit_id: String,
    #[serde(rename = "networkId")]
    pub network_id: String,
    #[serde(rename = "networkAddrs")]
    pub network_addrs: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PostDeviceRange {
    pub data: PostDeviceRangeData,
}

#[derive(Debug, Serialize)]
pub struct PostDeviceRangeData {
    #[serde(rename = "unitId")]
    pub unit_id: String,
    #[serde(rename = "networkId")]
    pub network_id: String,
    #[serde(rename = "startAddr")]
    pub start_addr: String,
    #[serde(rename = "endAddr")]
    pub end_addr: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile: Option<String>,
}

#[derive(Debug, Default, Serialize)]
pub struct GetDeviceCount {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub addr: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contains: Option<String>,
}

#[derive(Debug, Default, Serialize)]
pub struct GetDeviceList {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub addr: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile: Option<String>,
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
pub struct PatchDevice {
    pub data: PatchDeviceData,
}

#[derive(Debug, Default, Serialize)]
pub struct PatchDeviceData {
    #[serde(rename = "networkId", skip_serializing_if = "Option::is_none")]
    pub network_id: Option<String>,
    #[serde(rename = "networkAddr", skip_serializing_if = "Option::is_none")]
    pub network_addr: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub info: Option<Map<String, Value>>,
}
