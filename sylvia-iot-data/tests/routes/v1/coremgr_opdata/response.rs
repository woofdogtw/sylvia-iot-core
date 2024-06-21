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
    pub _latency_ms: i64,
    #[serde(rename = "status")]
    pub _status: i32,
    #[serde(rename = "sourceIp")]
    pub _source_ip: String,
    #[serde(rename = "method")]
    pub _method: String,
    #[serde(rename = "path")]
    pub _path: String,
    #[serde(rename = "body")]
    pub _body: Option<Map<String, Value>>,
    #[serde(rename = "userId")]
    pub _user_id: String,
    #[serde(rename = "clientId")]
    pub _client_id: String,
    #[serde(rename = "errCode")]
    pub _err_code: Option<String>,
    #[serde(rename = "errMessage")]
    pub _err_message: Option<String>,
}
