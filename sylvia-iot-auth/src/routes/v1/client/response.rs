use serde::Serialize;

#[derive(Serialize)]
pub struct PostClient {
    pub data: PostClientData,
}

#[derive(Serialize)]
pub struct PostClientData {
    #[serde(rename = "clientId")]
    pub client_id: String,
}

#[derive(Serialize)]
pub struct GetClientCount {
    pub data: GetCountData,
}

#[derive(Serialize)]
pub struct GetCountData {
    pub count: u64,
}

#[derive(Serialize)]
pub struct GetClientList {
    pub data: Vec<GetClientData>,
}

#[derive(Serialize)]
pub struct GetClient {
    pub data: GetClientData,
}

#[derive(Serialize)]
pub struct GetClientData {
    #[serde(rename = "clientId")]
    pub client_id: String,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    #[serde(rename = "modifiedAt")]
    pub modified_at: String,
    #[serde(rename = "clientSecret")]
    pub client_secret: Option<String>,
    #[serde(rename = "redirectUris")]
    pub redirect_uris: Vec<String>,
    pub scopes: Vec<String>,
    #[serde(rename = "userId", skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
    pub name: String,
    pub image: Option<String>,
}
