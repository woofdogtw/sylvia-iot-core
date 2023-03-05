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
    pub unit_id: String,
    #[serde(rename = "applicationId")]
    pub application_id: String,
    #[serde(rename = "applicationCode")]
    pub application_code: String,
    #[serde(rename = "networkId")]
    pub network_id: String,
    #[serde(rename = "deviceId")]
    pub device_id: String,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    #[serde(rename = "expiredAt")]
    pub expired_at: String,
}
