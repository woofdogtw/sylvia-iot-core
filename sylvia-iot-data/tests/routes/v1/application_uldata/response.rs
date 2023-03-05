use serde::Deserialize;
use serde_json::{Map, Value};

#[derive(Deserialize)]
pub struct GetCount {
    pub data: GetCountData,
}

#[derive(Deserialize)]
pub struct GetCountData {
    pub count: usize,
}

#[derive(Debug, Deserialize)]
pub struct GetList {
    pub data: Vec<GetListData>,
}

#[derive(Debug, Deserialize)]
pub struct GetListData {
    #[serde(rename = "dataId")]
    pub data_id: String,
    pub proc: String,
    #[serde(rename = "pub")]
    pub publish: String,
    #[serde(rename = "unitCode")]
    pub unit_code: Option<String>,
    #[serde(rename = "networkCode")]
    pub network_code: String,
    #[serde(rename = "networkAddr")]
    pub network_addr: String,
    #[serde(rename = "unitId")]
    pub unit_id: String,
    #[serde(rename = "deviceId")]
    pub device_id: String,
    pub time: String,
    pub data: String,
    pub extension: Option<Map<String, Value>>,
}
