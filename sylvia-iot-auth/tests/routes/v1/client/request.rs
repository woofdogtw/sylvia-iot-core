use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct PostClient {
    pub data: PostClientData,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub credentials: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct PostClientData {
    #[serde(rename = "redirectUris")]
    pub redirect_uris: Vec<String>,
    pub scopes: Vec<String>,
    #[serde(rename = "userId", skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
    pub name: String,
    pub image: Option<String>,
}

#[derive(Debug, Default, Serialize)]
pub struct GetClientCount {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
}

#[derive(Debug, Default, Serialize)]
pub struct GetClientList {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
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

#[derive(Debug, Default, Serialize)]
pub struct PatchClient {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<PatchClientData>,
    #[serde(rename = "regenSecret", skip_serializing_if = "Option::is_none")]
    pub regen_secret: Option<bool>,
}

#[derive(Debug, Default, Serialize)]
pub struct PatchClientData {
    #[serde(rename = "redirectUris", skip_serializing_if = "Option::is_none")]
    pub redirect_uris: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scopes: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<Option<String>>,
}
