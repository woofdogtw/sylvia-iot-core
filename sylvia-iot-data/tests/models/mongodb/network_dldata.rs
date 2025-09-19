use std::{collections::HashMap, error::Error as StdError};

use async_trait::async_trait;
use futures::TryStreamExt;
use laboratory::{SpecContext, expect};
use mongodb::{
    Database,
    bson::{self, DateTime, Document, doc},
};
use serde::Deserialize;

use sylvia_iot_data::models::{Model, network_dldata::NetworkDlData};

use super::{super::common::network_dldata as common_test, STATE, TestState};

struct Db<'a> {
    conn: &'a Database,
}

/// MongoDB schema.
#[derive(Deserialize)]
struct Schema {
    #[serde(rename = "dataId")]
    pub data_id: String,
    pub proc: DateTime,
    #[serde(rename = "pub")]
    pub publish: DateTime,
    pub resp: Option<DateTime>,
    pub status: i32,
    #[serde(rename = "unitId")]
    pub unit_id: String,
    #[serde(rename = "deviceId")]
    pub device_id: String,
    #[serde(rename = "networkCode")]
    pub network_code: String,
    #[serde(rename = "networkAddr")]
    pub network_addr: String,
    pub profile: String,
    pub data: String,
    pub extension: Option<Document>,
}

const COL_NAME: &'static str = "networkDlData";

pub fn after_each_fn(state: &mut HashMap<&'static str, TestState>) -> () {
    let state = state.get_mut(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let conn = state.mongodb.as_ref().unwrap().get_connection();
    let _ = runtime.block_on(async {
        conn.collection::<Schema>(COL_NAME)
            .delete_many(Document::new())
            .await
    });
}

/// Test table initialization.
pub fn init(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().network_dldata();

    let result = runtime.block_on(async { model.init().await });
    expect(result.is_ok()).to_equal(true)
}

/// Test `add()`.
pub fn add(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().network_dldata();
    let conn = state.mongodb.as_ref().unwrap().get_connection();

    common_test::add(runtime, model, &Db { conn })
}

/// Test `add()` with duplicate key.
pub fn add_dup(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().network_dldata();

    common_test::add_dup(runtime, model)
}

/// Test `del()` by specifying unit ID.
pub fn del_by_unit(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().network_dldata();
    let conn = state.mongodb.as_ref().unwrap().get_connection();

    common_test::del_by_unit(runtime, model, &Db { conn })
}

/// Test `del()` twice.
pub fn del_twice(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().network_dldata();

    common_test::del_twice(runtime, model)
}

/// Test `del()` by specifying device ID.
pub fn del_by_device_id(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().network_dldata();
    let conn = state.mongodb.as_ref().unwrap().get_connection();

    common_test::del_by_device_id(runtime, model, &Db { conn })
}

/// Test `del()` by specifying processed time.
pub fn del_by_proc(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().network_dldata();
    let conn = state.mongodb.as_ref().unwrap().get_connection();

    common_test::del_by_proc(runtime, model, &Db { conn })
}

/// Test `update()` status with zero.
pub fn update_with_zero(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().network_dldata();
    let conn = state.mongodb.as_ref().unwrap().get_connection();

    common_test::update_with_zero(runtime, model, &Db { conn })
}

/// Test `update()` status with positive.
pub fn update_with_positive(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().network_dldata();
    let conn = state.mongodb.as_ref().unwrap().get_connection();

    common_test::update_with_positive(runtime, model, &Db { conn })
}

/// Test `update()` status with negative.
pub fn update_with_negative(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().network_dldata();
    let conn = state.mongodb.as_ref().unwrap().get_connection();

    common_test::update_with_negative(runtime, model, &Db { conn })
}

/// Test `update()` with a non-exist condition.
pub fn update_not_exist(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().network_dldata();

    common_test::update_not_exist(runtime, model)
}

/// Test `count()`.
pub fn count(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().network_dldata();

    common_test::count(runtime, model)
}

/// Test `list()`.
pub fn list(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().network_dldata();

    common_test::list(runtime, model)
}

/// Test `list()` with sorting.
pub fn list_sort(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().network_dldata();

    common_test::list_sort(runtime, model)
}

/// Test `list()` with offset/limit.
pub fn list_offset_limit(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().network_dldata();

    common_test::list_offset_limit(runtime, model)
}

/// Test `list()` with cursors.
pub fn list_cursor(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().network_dldata();

    common_test::list_cursor(runtime, model)
}

#[async_trait]
impl<'a> common_test::Db for Db<'a> {
    async fn get(&self, data_id: &str) -> Result<Option<NetworkDlData>, Box<dyn StdError>> {
        let mut filter = Document::new();
        filter.insert("dataId", data_id);
        let mut cursor = self
            .conn
            .collection::<Schema>(COL_NAME)
            .find(filter)
            .await?;
        if let Some(item) = cursor.try_next().await? {
            return Ok(Some(NetworkDlData {
                data_id: item.data_id,
                proc: item.proc.into(),
                publish: item.publish.into(),
                resp: match item.resp {
                    None => None,
                    Some(resp) => Some(resp.into()),
                },
                status: item.status,
                unit_id: item.unit_id,
                device_id: item.device_id,
                network_code: item.network_code,
                network_addr: item.network_addr,
                profile: item.profile,
                data: item.data,
                extension: match item.extension {
                    None => None,
                    Some(extension) => Some(bson::deserialize_from_document(extension)?),
                },
            }));
        }
        Ok(None)
    }
}
