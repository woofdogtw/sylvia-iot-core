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
    #[serde(rename = "reqTime")]
    pub req_time: String,
    #[serde(rename = "resTime")]
    pub res_time: String,
    #[serde(rename = "latencyMs")]
    pub latency_ms: i64,
    pub status: i32,
    #[serde(rename = "sourceIp")]
    pub source_ip: String,
    pub method: String,
    pub path: String,
    pub body: Option<Map<String, Value>>,
    #[serde(rename = "userId")]
    pub user_id: String,
    #[serde(rename = "clientId")]
    pub client_id: String,
    #[serde(rename = "errCode")]
    pub err_code: Option<String>,
    #[serde(rename = "errMessage")]
    pub err_message: Option<String>,
}
