use std::{collections::HashMap, error::Error as StdError};

use async_trait::async_trait;
use chrono::{TimeZone, Utc};
use laboratory::{expect, SpecContext};
use sql_builder::{quote, SqlBuilder};
use sqlx::SqlitePool;

use sylvia_iot_data::models::{coremgr_opdata::CoremgrOpData, Model};

use super::{super::common::coremgr_opdata as common_test, TestState, STATE};

struct Db<'a> {
    conn: &'a SqlitePool,
}

/// SQLite schema.
#[derive(sqlx::FromRow)]
struct Schema {
    pub data_id: String,
    /// i64 as time tick from Epoch in milliseconds.
    pub req_time: i64,
    /// i64 as time tick from Epoch in milliseconds.
    pub res_time: i64,
    pub latency_ms: i64,
    pub status: i32,
    pub source_ip: String,
    pub method: String,
    pub path: String,
    /// use empty string as NULL.
    pub body: String,
    pub user_id: String,
    pub client_id: String,
    /// use empty string as NULL.
    pub err_code: String,
    /// use empty string as NULL.
    pub err_message: String,
}

const TABLE_NAME: &'static str = "coremgr_opdata";
const FIELDS: &'static [&'static str] = &[
    "data_id",
    "req_time",
    "res_time",
    "latency_ms",
    "status",
    "source_ip",
    "method",
    "path",
    "body",
    "user_id",
    "client_id",
    "err_code",
    "err_message",
];

pub fn after_each_fn(state: &mut HashMap<&'static str, TestState>) -> () {
    let state = state.get_mut(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let conn = state.sqlite.as_ref().unwrap().get_connection();
    let sql = SqlBuilder::delete_from(TABLE_NAME).sql().unwrap();
    let _ = runtime.block_on(async { sqlx::query(sql.as_str()).execute(conn).await });
}

/// Test table initialization.
pub fn init(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().coremgr_opdata();

    let result = runtime.block_on(async { model.init().await });
    expect(result.is_ok()).to_equal(true)
}

/// Test `add()`.
pub fn add(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().coremgr_opdata();
    let conn = state.sqlite.as_ref().unwrap().get_connection();

    common_test::add(runtime, model, &Db { conn })
}

/// Test `add()` with duplicate key.
pub fn add_dup(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().coremgr_opdata();

    common_test::add_dup(runtime, model)
}

/// Test `del()` by specifying user ID.
pub fn del_by_user(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().coremgr_opdata();
    let conn = state.sqlite.as_ref().unwrap().get_connection();

    common_test::del_by_user(runtime, model, &Db { conn })
}

/// Test `del()` twice.
pub fn del_twice(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().coremgr_opdata();

    common_test::del_twice(runtime, model)
}

/// Test `del()` by specifying client ID.
pub fn del_by_client(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().coremgr_opdata();
    let conn = state.sqlite.as_ref().unwrap().get_connection();

    common_test::del_by_client(runtime, model, &Db { conn })
}

/// Test `del()` by specifying request time.
pub fn del_by_req(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().coremgr_opdata();
    let conn = state.sqlite.as_ref().unwrap().get_connection();

    common_test::del_by_req(runtime, model, &Db { conn })
}

/// Test `count()`.
pub fn count(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().coremgr_opdata();

    common_test::count(runtime, model)
}

/// Test `list()`.
pub fn list(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().coremgr_opdata();

    common_test::list(runtime, model)
}

/// Test `list()` with sorting.
pub fn list_sort(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().coremgr_opdata();

    common_test::list_sort(runtime, model)
}

/// Test `list()` with offset/limit.
pub fn list_offset_limit(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().coremgr_opdata();

    common_test::list_offset_limit(runtime, model)
}

/// Test `list()` with cursors.
pub fn list_cursor(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().coremgr_opdata();

    common_test::list_cursor(runtime, model)
}

#[async_trait]
impl<'a> common_test::Db for Db<'a> {
    async fn get(&self, data_id: &str) -> Result<Option<CoremgrOpData>, Box<dyn StdError>> {
        let sql = SqlBuilder::select_from(TABLE_NAME)
            .fields(FIELDS)
            .and_where_eq("data_id", quote(data_id))
            .sql()?;

        let result: Result<Schema, sqlx::Error> =
            sqlx::query_as(sql.as_str()).fetch_one(self.conn).await;

        let row = match result {
            Err(e) => match e {
                sqlx::Error::RowNotFound => return Ok(None),
                _ => return Err(Box::new(e)),
            },
            Ok(row) => row,
        };

        Ok(Some(CoremgrOpData {
            data_id: row.data_id,
            req_time: Utc.timestamp_nanos(row.req_time * 1000000),
            res_time: Utc.timestamp_nanos(row.res_time * 1000000),
            latency_ms: row.latency_ms,
            status: row.status,
            source_ip: row.source_ip,
            method: row.method,
            path: row.path,
            body: match row.body.len() {
                0 => None,
                _ => serde_json::from_str(row.body.as_str())?,
            },
            user_id: row.user_id,
            client_id: row.client_id,
            err_code: match row.err_code.len() {
                0 => None,
                _ => Some(row.err_code),
            },
            err_message: match row.err_message.len() {
                0 => None,
                _ => Some(row.err_message),
            },
        }))
    }
}
