use serde::Deserialize;
use serde_json::{Map, Value};

#[derive(Deserialize)]
pub struct NetworkIdPath {
    pub network_id: String,
}

#[derive(Deserialize)]
pub struct PostNetworkBody {
    pub data: PostNetworkData,
}

#[derive(Deserialize)]
pub struct PostNetworkData {
    pub code: String,
    #[serde(rename = "unitId")]
    pub unit_id: Option<String>,
    #[serde(rename = "hostUri")]
    pub host_uri: String,
    pub name: Option<String>,
    pub info: Option<Map<String, Value>>,
}

#[derive(Deserialize)]
pub struct GetNetworkCountQuery {
    pub unit: Option<String>,
    pub contains: Option<String>,
}

#[derive(Clone, Deserialize)]
pub struct GetNetworkListQuery {
    pub unit: Option<String>,
    pub contains: Option<String>,
    pub offset: Option<u64>,
    pub limit: Option<u64>,
    pub sort: Option<String>,
    pub format: Option<ListFormat>,
}

#[derive(Deserialize)]
pub struct PatchNetworkBody {
    pub data: PatchNetworkData,
}

#[derive(Deserialize)]
pub struct PatchNetworkData {
    #[serde(rename = "hostUri")]
    pub host_uri: Option<String>,
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
