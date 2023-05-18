use serde::Deserialize;
use serde_json::{Map, Value};

#[derive(Deserialize)]
pub struct DeviceIdPath {
    pub device_id: String,
}

#[derive(Deserialize)]
pub struct PostDeviceBody {
    pub data: PostDeviceData,
}

#[derive(Deserialize)]
pub struct PostDeviceData {
    #[serde(rename = "unitId")]
    pub unit_id: String,
    #[serde(rename = "networkId")]
    pub network_id: String,
    #[serde(rename = "networkAddr")]
    pub network_addr: String,
    pub profile: Option<String>,
    pub name: Option<String>,
    pub info: Option<Map<String, Value>>,
}

#[derive(Deserialize)]
pub struct PostDeviceBulkBody {
    pub data: PostDeviceBulkData,
}

#[derive(Deserialize)]
pub struct PostDeviceBulkData {
    #[serde(rename = "unitId")]
    pub unit_id: String,
    #[serde(rename = "networkId")]
    pub network_id: String,
    #[serde(rename = "networkAddrs")]
    pub network_addrs: Vec<String>,
    pub profile: Option<String>,
}

#[derive(Deserialize)]
pub struct PostDeviceRangeBody {
    pub data: PostDeviceRangeData,
}

#[derive(Deserialize)]
pub struct PostDeviceRangeData {
    #[serde(rename = "unitId")]
    pub unit_id: String,
    #[serde(rename = "networkId")]
    pub network_id: String,
    #[serde(rename = "startAddr")]
    pub start_addr: String,
    #[serde(rename = "endAddr")]
    pub end_addr: String,
    pub profile: Option<String>,
}

#[derive(Deserialize)]
pub struct GetDeviceCountQuery {
    pub unit: Option<String>,
    pub network: Option<String>,
    pub addr: Option<String>,
    pub profile: Option<String>,
    pub contains: Option<String>,
}

#[derive(Clone, Deserialize)]
pub struct GetDeviceListQuery {
    pub unit: Option<String>,
    pub network: Option<String>,
    pub addr: Option<String>,
    pub profile: Option<String>,
    pub contains: Option<String>,
    pub offset: Option<u64>,
    pub limit: Option<u64>,
    pub sort: Option<String>,
    pub format: Option<ListFormat>,
}

#[derive(Deserialize)]
pub struct PatchDeviceBody {
    pub data: PatchDeviceData,
}

#[derive(Deserialize)]
pub struct PatchDeviceData {
    #[serde(rename = "networkId")]
    pub network_id: Option<String>,
    #[serde(rename = "networkAddr")]
    pub network_addr: Option<String>,
    pub profile: Option<String>,
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
