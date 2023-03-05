use serde::Serialize;

#[derive(Debug, Default, Serialize)]
pub struct GetCount {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tfield: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tstart: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tend: Option<i64>,
}

#[derive(Debug, Default, Serialize)]
pub struct GetList {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tfield: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tstart: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tend: Option<i64>,
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
