use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct PostNetworkRoute {
    pub data: PostNetworkRouteData,
}

#[derive(Debug, Serialize)]
pub struct PostNetworkRouteData {
    #[serde(rename = "networkId")]
    pub network_id: String,
    #[serde(rename = "applicationId")]
    pub application_id: String,
}

#[derive(Debug, Default, Serialize)]
pub struct GetNetworkRouteCount {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub application: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network: Option<String>,
}

#[derive(Debug, Default, Serialize)]
pub struct GetNetworkRouteList {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub application: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network: Option<String>,
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
