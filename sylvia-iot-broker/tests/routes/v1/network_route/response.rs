use serde::Deserialize;

#[derive(Deserialize)]
pub struct PostNetworkRoute {
    pub data: PostNetworkRouteData,
}

#[derive(Deserialize)]
pub struct PostNetworkRouteData {
    #[serde(rename = "routeId")]
    pub route_id: String,
}

#[derive(Deserialize)]
pub struct GetNetworkRouteCount {
    pub data: GetNetworkRouteCountData,
}

#[derive(Deserialize)]
pub struct GetNetworkRouteCountData {
    pub count: usize,
}

#[derive(Debug, Deserialize)]
pub struct GetNetworkRouteList {
    pub data: Vec<GetNetworkRouteListData>,
}

#[derive(Debug, Deserialize)]
pub struct GetNetworkRouteListData {
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
