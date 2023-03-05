use serde::Deserialize;
use serde_json::{Map, Value};

#[derive(Deserialize)]
pub struct PostNetwork {
    pub data: PostNetworkData,
}

#[derive(Deserialize)]
pub struct PostNetworkData {
    #[serde(rename = "networkId")]
    pub network_id: String,
}

#[derive(Deserialize)]
pub struct GetNetworkCount {
    pub data: GetNetworkCountData,
}

#[derive(Deserialize)]
pub struct GetNetworkCountData {
    pub count: usize,
}

#[derive(Debug, Deserialize)]
pub struct GetNetworkList {
    pub data: Vec<GetNetworkListData>,
}

#[derive(Debug, Deserialize)]
pub struct GetNetworkListData {
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

#[derive(Debug, Deserialize)]
pub struct GetNetwork {
    pub data: GetNetworkListData,
}
