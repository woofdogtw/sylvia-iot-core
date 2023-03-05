use serde::Deserialize;

#[derive(Deserialize)]
pub struct RouteIdPath {
    pub route_id: String,
}

#[derive(Deserialize)]
pub struct PostDeviceRouteBody {
    pub data: PostDeviceRouteData,
}

#[derive(Deserialize)]
pub struct PostDeviceRouteData {
    #[serde(rename = "deviceId")]
    pub device_id: String,
    #[serde(rename = "applicationId")]
    pub application_id: String,
}

#[derive(Deserialize)]
pub struct PostDeviceRouteBulkBody {
    pub data: PostDeviceRouteBulkData,
}

#[derive(Deserialize)]
pub struct PostDeviceRouteBulkData {
    #[serde(rename = "applicationId")]
    pub application_id: String,
    #[serde(rename = "networkId")]
    pub network_id: String,
    #[serde(rename = "networkAddrs")]
    pub network_addrs: Vec<String>,
}

#[derive(Deserialize)]
pub struct PostDeviceRouteRangeBody {
    pub data: PostDeviceRouteRangeData,
}

#[derive(Deserialize)]
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

#[derive(Deserialize)]
pub struct GetDeviceRouteCountQuery {
    pub unit: Option<String>,
    pub application: Option<String>,
    pub network: Option<String>,
    pub device: Option<String>,
}

#[derive(Clone, Deserialize)]
pub struct GetDeviceRouteListQuery {
    pub unit: Option<String>,
    pub application: Option<String>,
    pub network: Option<String>,
    pub device: Option<String>,
    pub offset: Option<u64>,
    pub limit: Option<u64>,
    pub sort: Option<String>,
    pub format: Option<ListFormat>,
}

#[derive(Clone, Deserialize, PartialEq)]
pub enum ListFormat {
    #[serde(rename = "array")]
    Array,
    #[serde(rename = "data")]
    Data,
}
