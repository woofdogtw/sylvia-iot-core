use serde::Deserialize;

#[derive(Deserialize)]
pub struct PostDeviceRoute {
    pub data: PostDeviceRouteData,
}

#[derive(Deserialize)]
pub struct PostDeviceRouteData {
    #[serde(rename = "routeId")]
    pub route_id: String,
}

#[derive(Deserialize)]
pub struct GetDeviceRouteCount {
    pub data: GetDeviceRouteCountData,
}

#[derive(Deserialize)]
pub struct GetDeviceRouteCountData {
    pub count: usize,
}

#[derive(Debug, Deserialize)]
pub struct GetDeviceRouteList {
    pub data: Vec<GetDeviceRouteListData>,
}

#[derive(Debug, Deserialize)]
pub struct GetDeviceRouteListData {
    #[serde(rename = "routeId")]
    pub route_id: String,
    #[serde(rename = "unitId")]
    pub _unit_id: String,
    #[serde(rename = "applicationId")]
    pub _application_id: String,
    #[serde(rename = "applicationCode")]
    pub _application_code: String,
    #[serde(rename = "deviceId")]
    pub _device_id: String,
    #[serde(rename = "networkId")]
    pub _network_id: String,
    #[serde(rename = "networkCode")]
    pub network_code: String,
    #[serde(rename = "networkAddr")]
    pub network_addr: String,
    #[serde(rename = "profile")]
    pub _profile: String,
    #[serde(rename = "createdAt")]
    pub _created_at: String,
    #[serde(rename = "modifiedAt")]
    pub _modified_at: String,
}
