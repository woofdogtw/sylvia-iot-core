use serde::Deserialize;

#[derive(Deserialize)]
pub struct RouteIdPath {
    pub route_id: String,
}

#[derive(Deserialize)]
pub struct PostNetworkRouteBody {
    pub data: PostNetworkRouteData,
}

#[derive(Deserialize)]
pub struct PostNetworkRouteData {
    #[serde(rename = "networkId")]
    pub network_id: String,
    #[serde(rename = "applicationId")]
    pub application_id: String,
}

#[derive(Deserialize)]
pub struct GetNetworkRouteCountQuery {
    pub unit: Option<String>,
    pub application: Option<String>,
    pub network: Option<String>,
}

#[derive(Clone, Deserialize)]
pub struct GetNetworkRouteListQuery {
    pub unit: Option<String>,
    pub application: Option<String>,
    pub network: Option<String>,
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
