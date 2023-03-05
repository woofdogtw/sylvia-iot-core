use serde::Serialize;

#[derive(Serialize)]
pub struct GetDlDataBufferCount {
    pub data: GetCountData,
}

#[derive(Serialize)]
pub struct GetCountData {
    pub count: u64,
}

#[derive(Serialize)]
pub struct GetDlDataBufferList {
    pub data: Vec<GetDlDataBufferData>,
}

#[derive(Serialize)]
pub struct GetDlDataBufferData {
    #[serde(rename = "dataId")]
    pub data_id: String,
    #[serde(rename = "unitId")]
    pub unit_id: String,
    #[serde(rename = "applicationId")]
    pub application_id: String,
    #[serde(rename = "applicationCode")]
    pub application_code: String,
    #[serde(rename = "deviceId")]
    pub device_id: String,
    #[serde(rename = "networkId")]
    pub network_id: String,
    #[serde(rename = "networkAddr")]
    pub network_addr: String,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    #[serde(rename = "expiredAt")]
    pub expired_at: String,
}
