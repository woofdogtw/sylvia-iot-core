use serde::Deserialize;

#[derive(Clone, Deserialize)]
pub struct GetCountQuery {
    pub unit: Option<String>,
    pub device: Option<String>,
    pub network: Option<String>,
    pub addr: Option<String>,
    pub tfield: Option<String>,
    pub tstart: Option<i64>,
    pub tend: Option<i64>,
}

#[derive(Clone, Deserialize)]
pub struct GetListQuery {
    pub unit: Option<String>,
    pub device: Option<String>,
    pub network: Option<String>,
    pub addr: Option<String>,
    pub tfield: Option<String>,
    pub tstart: Option<i64>,
    pub tend: Option<i64>,
    pub offset: Option<u64>,
    pub limit: Option<u64>,
    pub sort: Option<String>,
    pub format: Option<ListFormat>,
}

#[derive(Clone, Deserialize, PartialEq)]
pub enum ListFormat {
    #[serde(rename = "array")]
    Array,
    #[serde(rename = "csv")]
    Csv,
    #[serde(rename = "data")]
    Data,
}
