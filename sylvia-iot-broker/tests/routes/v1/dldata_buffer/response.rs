use serde::Deserialize;

#[derive(Deserialize)]
pub struct GetDlDataBufferCount {
    pub data: GetDlDataBufferCountData,
}

#[derive(Deserialize)]
pub struct GetDlDataBufferCountData {
    pub count: usize,
}

#[derive(Debug, Deserialize)]
pub struct GetDlDataBufferList {
    pub data: Vec<GetDlDataBufferListData>,
}

#[derive(Debug, Deserialize)]
pub struct GetDlDataBufferListData {
    #[serde(rename = "dataId")]
    pub data_id: String,
    #[serde(rename = "unitId")]
    pub _unit_id: String,
    #[serde(rename = "applicationId")]
    pub application_id: String,
    #[serde(rename = "applicationCode")]
    pub _application_code: String,
    #[serde(rename = "networkId")]
    pub _network_id: String,
    #[serde(rename = "deviceId")]
    pub _device_id: String,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    #[serde(rename = "expiredAt")]
    pub _expired_at: String,
}
