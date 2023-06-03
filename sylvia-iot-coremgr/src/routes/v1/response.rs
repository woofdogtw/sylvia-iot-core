use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

#[derive(Deserialize)]
pub struct GetTokenInfo {
    pub data: TokenInfo,
}

#[derive(Deserialize)]
pub struct TokenInfo {
    #[serde(rename = "userId")]
    pub user_id: String,
    pub account: String,
    pub name: String,
    pub roles: HashMap<String, bool>,
    #[serde(rename = "clientId")]
    pub client_id: String,
    pub scopes: Vec<String>,
}

#[derive(Deserialize)]
pub struct GetUnit {
    pub data: Unit,
}

/// The unit response that is from broker GET API and **ONLY be serialized for CSV**.
#[derive(Deserialize, Serialize)]
pub struct Unit {
    #[serde(rename = "unitId")]
    pub unit_id: String,
    pub code: String,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    #[serde(rename = "modifiedAt")]
    pub modified_at: String,
    #[serde(rename = "ownerId")]
    pub owner_id: String,
    #[serde(rename = "memberIds", skip_serializing)]
    pub member_ids: Vec<String>,
    #[serde(rename = "memberIds")]
    pub member_ids_str: Option<String>,
    pub name: String,
    #[serde(skip_serializing)]
    pub info: Map<String, Value>,
    #[serde(rename(serialize = "info"))]
    pub info_str: Option<String>,
}

#[derive(Deserialize, Serialize)]
pub struct PostApplication {
    pub data: PostApplicationData,
}

#[derive(Deserialize, Serialize)]
pub struct PostApplicationData {
    #[serde(rename = "applicationId")]
    pub application_id: String,
    pub password: Option<String>,
}

#[derive(Deserialize, Serialize)]
pub struct GetApplication {
    pub data: GetApplicationData,
}

#[derive(Deserialize, Serialize)]
pub struct GetApplicationData {
    #[serde(rename = "applicationId")]
    pub application_id: String,
    pub code: String,
    #[serde(rename = "unitId")]
    pub unit_id: String,
    #[serde(rename = "unitCode")]
    pub unit_code: String,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    #[serde(rename = "modifiedAt")]
    pub modified_at: String,
    #[serde(rename = "hostUri")]
    pub host_uri: String,
    pub name: String,
    pub info: Map<String, Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ttl: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub length: Option<usize>,
}

#[derive(Serialize)]
pub struct GetApplicationStats {
    pub data: GetApplicationStatsData,
}

#[derive(Serialize)]
pub struct GetApplicationStatsData {
    pub uldata: Stats,
    #[serde(rename = "dldataResp")]
    pub dldata_resp: Stats,
    #[serde(rename = "dldataResult")]
    pub dldata_result: Stats,
}

#[derive(Deserialize, Serialize)]
pub struct PostNetwork {
    pub data: PostNetworkData,
}

#[derive(Deserialize, Serialize)]
pub struct PostNetworkData {
    #[serde(rename = "networkId")]
    pub network_id: String,
    pub password: Option<String>,
}

#[derive(Deserialize, Serialize)]
pub struct GetNetwork {
    pub data: GetNetworkData,
}

#[derive(Deserialize, Serialize)]
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ttl: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub length: Option<usize>,
}

#[derive(Serialize)]
pub struct GetNetworkStats {
    pub data: GetNetworkStatsData,
}

#[derive(Serialize)]
pub struct GetNetworkStatsData {
    pub dldata: Stats,
    pub ctrl: Stats,
}

#[derive(Default, Serialize)]
pub struct Stats {
    pub consumers: usize,
    pub messages: usize,
    #[serde(rename = "publishRate")]
    pub publish_rate: f64,
    #[serde(rename = "deliverRate")]
    pub deliver_rate: f64,
}

#[derive(Deserialize)]
pub struct GetDevice {
    pub data: Device,
}

#[derive(Deserialize)]
pub struct Device {
    #[serde(rename = "deviceId")]
    pub device_id: String,
    #[serde(rename = "unitId")]
    pub unit_id: String,
    #[serde(rename = "unitCode")]
    pub unit_code: Option<String>,
    #[serde(rename = "networkId")]
    pub network_id: String,
    #[serde(rename = "networkCode")]
    pub network_code: String,
    #[serde(rename = "networkAddr")]
    pub network_addr: String,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    #[serde(rename = "modifiedAt")]
    pub modified_at: String,
    pub name: String,
    pub info: Map<String, Value>,
}
