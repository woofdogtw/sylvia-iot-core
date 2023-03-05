use serde::Serialize;

#[derive(Serialize)]
pub struct PostNetworkRoute {
    pub data: PostNetworkRouteData,
}

#[derive(Serialize)]
pub struct PostNetworkRouteData {
    #[serde(rename = "routeId")]
    pub route_id: String,
}

#[derive(Serialize)]
pub struct GetNetworkRouteCount {
    pub data: GetCountData,
}

#[derive(Serialize)]
pub struct GetCountData {
    pub count: u64,
}

#[derive(Serialize)]
pub struct GetNetworkRouteList {
    pub data: Vec<GetNetworkRouteData>,
}

#[derive(Serialize)]
pub struct GetNetworkRouteData {
    #[serde(rename = "routeId")]
    pub route_id: String,
    #[serde(rename = "unitId")]
    pub unit_id: String,
    #[serde(rename = "applicationId")]
    pub application_id: String,
    #[serde(rename = "applicationCode")]
    pub application_code: String,
    #[serde(rename = "networkId")]
    pub network_id: String,
    #[serde(rename = "networkCode")]
    pub network_code: String,
    #[serde(rename = "createdAt")]
    pub created_at: String,
}
