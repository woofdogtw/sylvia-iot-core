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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extension: Option<Map<String, Value>>,
}

#[derive(Serialize)]
pub struct GetListCsvData {
    pub data_id: String,
    pub proc: String,
    pub publish: String,
    pub unit_code: String,
    pub network_code: String,
    pub network_addr: String,
    pub unit_id: String,
    pub device_id: String,
    pub time: String,
    pub data: String,
    pub extension: String,
}
