use serde::Serialize;

use crate::libs::network::{
    DhcpLease, LanConf4, WanConf, WanConn4, WifiApInfo, WlanConf, WwanConf,
};

#[derive(Serialize)]
pub struct GetWan {
    pub data: Vec<GetWanData>,
}

#[derive(Serialize)]
pub struct GetWanData {
    #[serde(rename = "wanId")]
    pub wan_id: String,
    pub conf: WanConf,
    pub conn4: WanConn4,
}

#[derive(Serialize)]
pub struct GetLan {
    pub data: GetLanData,
}

#[derive(Serialize)]
pub struct GetLanData {
    pub conf4: LanConf4,
}

#[derive(Serialize)]
pub struct GetLanLeases {
    pub data: Vec<DhcpLease>,
}

#[derive(Serialize)]
pub struct GetWlan {
    pub data: GetWlanData,
}

#[derive(Serialize)]
pub struct GetWlanData {
    pub enable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conf: Option<WlanConf>,
}

#[derive(Serialize)]
pub struct GetWwan {
    pub data: GetWwanData,
}

#[derive(Serialize)]
pub struct GetWwanData {
    pub enable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conf: Option<WwanConf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conn4: Option<WanConn4>,
}

#[derive(Serialize)]
pub struct GetWwanList {
    pub data: Vec<WifiApInfo>,
}
