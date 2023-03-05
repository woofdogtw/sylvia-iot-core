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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<Map<String, Value>>,
    #[serde(rename = "userId")]
    pub user_id: String,
    #[serde(rename = "clientId")]
    pub client_id: String,
    #[serde(rename = "errCode", skip_serializing_if = "Option::is_none")]
    pub err_code: Option<String>,
    #[serde(rename = "errMessage", skip_serializing_if = "Option::is_none")]
    pub err_message: Option<String>,
}

#[derive(Serialize)]
pub struct GetListCsvData {
    pub data_id: String,
    pub req_time: String,
    pub res_time: String,
    pub latency_ms: i64,
    pub status: i32,
    pub source_ip: String,
    pub method: String,
    pub path: String,
    pub body: String,
    pub user_id: String,
    pub client_id: String,
    pub err_code: String,
    pub err_message: String,
}
