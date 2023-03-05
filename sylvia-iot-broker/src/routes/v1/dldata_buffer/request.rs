use serde::Deserialize;

#[derive(Deserialize)]
pub struct DataIdPath {
    pub data_id: String,
}

#[derive(Deserialize)]
pub struct GetDlDataBufferCountQuery {
    pub unit: Option<String>,
    pub application: Option<String>,
    pub network: Option<String>,
    pub device: Option<String>,
}

#[derive(Clone, Deserialize)]
pub struct GetDlDataBufferListQuery {
    pub unit: Option<String>,
    pub application: Option<String>,
    pub network: Option<String>,
    pub device: Option<String>,
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
