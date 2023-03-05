use serde::Serialize;
use serde_json::{Map, Value};

#[derive(Serialize)]
pub struct GetCount {
    pub data: GetCountData,
}

#[derive(Serialize)]
pub struct GetCountData {
    pub count: u64,
}

#[derive(Serialize)]
pub struct GetList {
    pub data: Vec<GetListData>,
}

#[derive(Serialize)]
pub struct GetListData {
    #[serde(rename = "dataId")]
    pub data_id: String,
    pub proc: String,
    #[serde(rename = "unitCode")]
    pub unit_code: Option<String>,
    #[serde(rename = "networkCode")]
    pub network_code: String,
    #[serde(rename = "networkAddr")]
    pub network_addr: String,
    #[serde(rename = "unitId", skip_serializing_if = "Option::is_none")]
    pub unit_id: Option<String>,
    #[serde(rename = "deviceId", skip_serializing_if = "Option::is_none")]
    pub device_id: Option<String>,
    pub time: String,
    pub data: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extension: Option<Map<String, Value>>,
}

#[derive(Serialize)]
pub struct GetListCsvData {
    pub data_id: String,
    pub proc: String,
    pub unit_code: String,
    pub network_code: String,
    pub network_addr: String,
    pub unit_id: String,
    pub device_id: String,
    pub time: String,
    pub data: String,
    pub extension: String,
}
