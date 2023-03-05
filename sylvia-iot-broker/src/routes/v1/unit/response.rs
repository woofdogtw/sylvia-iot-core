use serde::Serialize;
use serde_json::{Map, Value};

#[derive(Serialize)]
pub struct PostUnit {
    pub data: PostUnitData,
}

#[derive(Serialize)]
pub struct PostUnitData {
    #[serde(rename = "unitId")]
    pub unit_id: String,
}

#[derive(Serialize)]
pub struct GetUnitCount {
    pub data: GetCountData,
}

#[derive(Serialize)]
pub struct GetCountData {
    pub count: u64,
}

#[derive(Serialize)]
pub struct GetUnitList {
    pub data: Vec<GetUnitData>,
}

#[derive(Serialize)]
pub struct GetUnit {
    pub data: GetUnitData,
}

#[derive(Serialize)]
pub struct GetUnitData {
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
