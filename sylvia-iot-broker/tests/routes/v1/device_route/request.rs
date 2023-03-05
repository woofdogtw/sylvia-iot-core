use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct PostDeviceRoute {
    pub data: PostDeviceRouteData,
}

#[derive(Debug, Serialize)]
pub struct PostDeviceRouteData {
    #[serde(rename = "deviceId")]
    pub device_id: String,
    #[serde(rename = "applicationId")]
    pub application_id: String,
}

#[derive(Debug, Serialize)]
pub struct PostDeviceRouteBulk {
    pub data: PostDeviceRouteBulkData,
}

#[derive(Debug, Serialize)]
pub struct PostDeviceRouteBulkData {
    #[serde(rename = "applicationId")]
    pub application_id: String,
    #[serde(rename = "networkId")]
    pub network_id: String,
    #[serde(rename = "networkAddrs")]
    pub network_addrs: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct PostDeviceRouteRange {
    pub data: PostDeviceRouteRangeData,
}

#[derive(Debug, Serialize)]
pub struct PostDeviceRouteRangeData {
    #[serde(rename = "applicationId")]
    pub application_id: String,
    #[serde(rename = "networkId")]
    pub network_id: String,
    #[serde(rename = "startAddr")]
    pub start_addr: String,
    #[serde(rename = "endAddr")]
    pub end_addr: String,
}

#[derive(Debug, Default, Serialize)]
pub struct GetDeviceRouteCount {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub application: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device: Option<String>,
}

#[derive(Debug, Default, Serialize)]
pub struct GetDeviceRouteList {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub application: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort: Option<String>, // this will be fill from sort_vec automatically.
    #[serde(skip_serializing)]
    pub sort_vec: Option<Vec<(&'static str, bool)>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
}
