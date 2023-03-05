use serde::Deserialize;
use serde_with;

#[derive(Deserialize)]
pub struct ClientIdPath {
    pub client_id: String,
}

#[derive(Deserialize)]
pub struct UserIdPath {
    pub user_id: String,
}

#[derive(Deserialize)]
pub struct PostClientBody {
    pub data: PostClientData,
    pub credentials: Option<bool>,
}

#[derive(Deserialize)]
pub struct PostClientData {
    #[serde(rename = "redirectUris")]
    pub redirect_uris: Vec<String>,
    pub scopes: Vec<String>,
    #[serde(rename = "userId")]
    pub user_id: Option<String>,
    pub name: String,
    pub image: Option<String>,
}

#[derive(Deserialize)]
pub struct GetClientCountQuery {
    pub user: Option<String>,
}

#[derive(Clone, Deserialize)]
pub struct GetClientListQuery {
    pub user: Option<String>,
    pub offset: Option<u64>,
    pub limit: Option<u64>,
    pub sort: Option<String>,
    pub format: Option<ListFormat>,
}

#[derive(Deserialize)]
pub struct PatchClientBody {
    pub data: Option<PatchClientData>,
    #[serde(rename = "regenSecret")]
    pub regen_secret: Option<bool>,
}

#[derive(Deserialize)]
pub struct PatchClientData {
    #[serde(rename = "redirectUris")]
    pub redirect_uris: Option<Vec<String>>,
    pub scopes: Option<Vec<String>>,
    pub name: Option<String>,
    #[serde(default, with = "serde_with::rust::double_option")]
    pub image: Option<Option<String>>,
}

#[derive(Clone, Deserialize, PartialEq)]
pub enum ListFormat {
    #[serde(rename = "array")]
    Array,
    #[serde(rename = "data")]
    Data,
}
