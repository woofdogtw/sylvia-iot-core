use serde::Deserialize;

#[derive(Deserialize)]
pub struct PostClient {
    pub data: PostClientData,
}

#[derive(Deserialize)]
pub struct PostClientData {
    #[serde(rename = "clientId")]
    pub client_id: String,
}

#[derive(Deserialize)]
pub struct GetClientCount {
    pub data: GetClientCountData,
}

#[derive(Deserialize)]
pub struct GetClientCountData {
    pub count: usize,
}

#[derive(Debug, Deserialize)]
pub struct GetClientList {
    pub data: Vec<GetClientListData>,
}

#[derive(Debug, Deserialize)]
pub struct GetClientListData {
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
    #[serde(rename = "userId")]
    pub user_id: Option<String>,
    pub name: String,
    pub image: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct GetClient {
    pub data: GetClientListData,
}
