use serde::Deserialize;
use serde_json::{Map, Value};

#[derive(Deserialize)]
pub struct UnitIdPath {
    pub unit_id: String,
}

#[derive(Deserialize)]
pub struct UserIdPath {
    pub user_id: String,
}

#[derive(Deserialize)]
pub struct PostUnitBody {
    pub data: PostUnitData,
}

#[derive(Deserialize)]
pub struct PostUnitData {
    pub code: String,
    #[serde(rename = "ownerId")]
    pub owner_id: Option<String>,
    pub name: Option<String>,
    pub info: Option<Map<String, Value>>,
}

#[derive(Deserialize)]
pub struct GetUnitCountQuery {
    pub owner: Option<String>,
    pub member: Option<String>,
    pub contains: Option<String>,
}

#[derive(Clone, Deserialize)]
pub struct GetUnitListQuery {
    pub owner: Option<String>,
    pub member: Option<String>,
    pub contains: Option<String>,
    pub offset: Option<u64>,
    pub limit: Option<u64>,
    pub sort: Option<String>,
    pub format: Option<ListFormat>,
}

#[derive(Deserialize)]
pub struct PatchUnitBody {
    pub data: PatchUnitData,
}

#[derive(Deserialize)]
pub struct PatchUnitData {
    #[serde(rename = "ownerId")]
    pub owner_id: Option<String>,
    #[serde(rename = "memberIds")]
    pub member_ids: Option<Vec<String>>,
    pub name: Option<String>,
    pub info: Option<Map<String, Value>>,
}

#[derive(Clone, Deserialize, PartialEq)]
pub enum ListFormat {
    #[serde(rename = "array")]
    Array,
    #[serde(rename = "data")]
    Data,
}
