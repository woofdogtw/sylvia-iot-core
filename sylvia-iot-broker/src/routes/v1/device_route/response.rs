use serde::Serialize;

#[derive(Serialize)]
pub struct PostDeviceRoute {
    pub data: PostDeviceRouteData,
}

#[derive(Serialize)]
pub struct PostDeviceRouteData {
    #[serde(rename = "routeId")]
    pub route_id: String,
}

#[derive(Serialize)]
pub struct GetDeviceRouteCount {
    pub data: GetCountData,
}

#[derive(Serialize)]
pub struct GetCountData {
    pub count: u64,
}

#[derive(Serialize)]
pub struct GetDeviceRouteList {
    pub data: Vec<GetDeviceRouteData>,
}

#[derive(Serialize)]
pub struct GetDeviceRouteData {
    #[serde(rename = "routeId")]
    pub route_id: String,
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
    #[serde(rename = "networkCode")]
    pub network_code: String,
    #[serde(rename = "networkAddr")]
    pub network_addr: String,
    pub profile: String,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    #[serde(rename = "modifiedAt")]
    pub modified_at: String,
}
