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
    pub _unit_code: Option<String>,
    #[serde(rename = "networkCode")]
    pub _network_code: String,
    #[serde(rename = "networkAddr")]
    pub _network_addr: String,
    #[serde(rename = "unitId")]
    pub _unit_id: String,
    #[serde(rename = "deviceId")]
    pub _device_id: String,
    pub time: String,
    #[serde(rename = "profile")]
    pub _profile: String,
    #[serde(rename = "data")]
    pub _data: String,
    #[serde(rename = "extension")]
    pub _extension: Option<Map<String, Value>>,
}
