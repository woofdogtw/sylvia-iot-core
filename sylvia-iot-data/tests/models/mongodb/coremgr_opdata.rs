use std::{collections::HashMap, error::Error as StdError};

use async_trait::async_trait;
use futures::TryStreamExt;
use laboratory::{SpecContext, expect};
use mongodb::{
    Database,
    bson::{self, DateTime, Document, doc},
};
use serde::Deserialize;

use sylvia_iot_data::models::{Model, coremgr_opdata::CoremgrOpData};

use super::{super::common::coremgr_opdata as common_test, STATE, TestState};

struct Db<'a> {
    conn: &'a Database,
}

/// MongoDB schema.
#[derive(Deserialize)]
struct Schema {
    #[serde(rename = "dataId")]
    pub data_id: String,
    #[serde(rename = "reqTime")]
    pub req_time: DateTime,
    #[serde(rename = "resTime")]
    pub res_time: DateTime,
    #[serde(rename = "latencyMs")]
    pub latency_ms: i64,
    pub status: i32,
    #[serde(rename = "sourceIp")]
    pub source_ip: String,
    pub method: String,
    pub path: String,
    pub body: Option<Document>,
    #[serde(rename = "userId")]
    pub user_id: String,
    #[serde(rename = "clientId")]
    pub client_id: String,
    #[serde(rename = "errCode")]
    pub err_code: Option<String>,
    #[serde(rename = "errMessage")]
    pub err_message: Option<String>,
}

const COL_NAME: &'static str = "coremgrOpData";

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
    let model = state.mongodb.as_ref().unwrap().coremgr_opdata();

    let result = runtime.block_on(async { model.init().await });
    expect(result.is_ok()).to_equal(true)
}

/// Test `add()`.
pub fn add(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().coremgr_opdata();
    let conn = state.mongodb.as_ref().unwrap().get_connection();

    common_test::add(runtime, model, &Db { conn })
}

/// Test `add()` with duplicate key.
pub fn add_dup(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().coremgr_opdata();

    common_test::add_dup(runtime, model)
}

/// Test `del()` by specifying user ID.
pub fn del_by_user(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().coremgr_opdata();
    let conn = state.mongodb.as_ref().unwrap().get_connection();

    common_test::del_by_user(runtime, model, &Db { conn })
}

/// Test `del()` twice.
pub fn del_twice(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().coremgr_opdata();

    common_test::del_twice(runtime, model)
}

/// Test `del()` by specifying client ID.
pub fn del_by_client(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().coremgr_opdata();
    let conn = state.mongodb.as_ref().unwrap().get_connection();

    common_test::del_by_client(runtime, model, &Db { conn })
}

/// Test `del()` by specifying request time.
pub fn del_by_req(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().coremgr_opdata();
    let conn = state.mongodb.as_ref().unwrap().get_connection();

    common_test::del_by_req(runtime, model, &Db { conn })
}

/// Test `count()`.
pub fn count(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().coremgr_opdata();

    common_test::count(runtime, model)
}

/// Test `list()`.
pub fn list(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().coremgr_opdata();

    common_test::list(runtime, model)
}

/// Test `list()` with sorting.
pub fn list_sort(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().coremgr_opdata();

    common_test::list_sort(runtime, model)
}

/// Test `list()` with offset/limit.
pub fn list_offset_limit(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().coremgr_opdata();

    common_test::list_offset_limit(runtime, model)
}

/// Test `list()` with cursors.
pub fn list_cursor(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().coremgr_opdata();

    common_test::list_cursor(runtime, model)
}

#[async_trait]
impl<'a> common_test::Db for Db<'a> {
    async fn get(&self, data_id: &str) -> Result<Option<CoremgrOpData>, Box<dyn StdError>> {
        let mut filter = Document::new();
        filter.insert("dataId", data_id);
        let mut cursor = self
            .conn
            .collection::<Schema>(COL_NAME)
            .find(filter)
            .await?;
        if let Some(item) = cursor.try_next().await? {
            return Ok(Some(CoremgrOpData {
                data_id: item.data_id,
                req_time: item.req_time.into(),
                res_time: item.res_time.into(),
                latency_ms: item.latency_ms,
                status: item.status,
                source_ip: item.source_ip,
                method: item.method,
                path: item.path,
                body: match item.body {
                    None => None,
                    Some(body) => bson::from_document(body)?,
                },
                user_id: item.user_id,
                client_id: item.client_id,
                err_code: item.err_code,
                err_message: item.err_message,
            }));
        }
        Ok(None)
    }
}
