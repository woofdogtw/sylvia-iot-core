use serde::Deserialize;
use serde_json::{Map, Value};

#[derive(Deserialize)]
pub struct PostUnit {
    pub data: PostUnitData,
}

#[derive(Deserialize)]
pub struct PostUnitData {
    #[serde(rename = "unitId")]
    pub unit_id: String,
}

#[derive(Deserialize)]
pub struct GetUnitCount {
    pub data: GetUnitCountData,
}

#[derive(Deserialize)]
pub struct GetUnitCountData {
    pub count: usize,
}

#[derive(Debug, Deserialize)]
pub struct GetUnitList {
    pub data: Vec<GetUnitListData>,
}

#[derive(Debug, Deserialize)]
pub struct GetUnitListData {
    #[serde(rename = "unitId")]
    pub unit_id: String,
    pub code: String,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    #[serde(rename = "modifiedAt")]
    pub modified_at: String,
    #[serde(rename = "ownerId")]
    pub owner_id: String,
    #[serde(rename = "memberIds")]
    pub member_ids: Vec<String>,
    pub name: String,
    pub info: Map<String, Value>,
}

#[derive(Debug, Deserialize)]
pub struct GetUnit {
    pub data: GetUnitListData,
}
