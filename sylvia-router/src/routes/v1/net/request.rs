use serde::Deserialize;

use crate::libs::network::{LanConf4, WanConf, WlanConf, WwanConf};

#[derive(Deserialize)]
pub struct WanIdPath {
    pub wan_id: String,
}

#[derive(Deserialize)]
pub struct PutWanBody {
    pub data: WanConf,
}

#[derive(Deserialize)]
pub struct PutLanBody {
    pub data: PutLanData,
}

#[derive(Deserialize)]
pub struct PutLanData {
    pub conf4: LanConf4,
}

#[derive(Deserialize)]
pub struct PutWlanBody {
    pub data: PutWlanData,
}

#[derive(Deserialize)]
pub struct PutWlanData {
    pub enable: bool,
    pub conf: Option<WlanConf>,
}

#[derive(Deserialize)]
pub struct PutWwanBody {
    pub data: PutWwanData,
}

#[derive(Deserialize)]
pub struct PutWwanData {
    pub enable: bool,
    pub conf: Option<WwanConf>,
}

#[derive(Deserialize)]
pub struct GetWwanListQuery {
    pub rescan: Option<bool>,
}
