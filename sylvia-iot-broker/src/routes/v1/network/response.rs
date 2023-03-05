use serde::Serialize;
use serde_json::{Map, Value};

#[derive(Serialize)]
pub struct PostNetwork {
    pub data: PostNetworkData,
}

#[derive(Serialize)]
pub struct PostNetworkData {
    #[serde(rename = "networkId")]
    pub network_id: String,
}

#[derive(Serialize)]
pub struct GetNetworkCount {
    pub data: GetCountData,
}

#[derive(Serialize)]
pub struct GetCountData {
    pub count: u64,
}

#[derive(Serialize)]
pub struct GetNetworkList {
    pub data: Vec<GetNetworkData>,
}

#[derive(Serialize)]
pub struct GetNetwork {
    pub data: GetNetworkData,
}

#[derive(Serialize)]
pub struct GetNetworkData {
    #[serde(rename = "networkId")]
    pub network_id: String,
    pub code: String,
    #[serde(rename = "unitId")]
    pub unit_id: Option<String>,
    #[serde(rename = "unitCode")]
    pub unit_code: Option<String>,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    #[serde(rename = "modifiedAt")]
    pub modified_at: String,
    #[serde(rename = "hostUri")]
    pub host_uri: String,
    pub name: String,
    pub info: Map<String, Value>,
}
