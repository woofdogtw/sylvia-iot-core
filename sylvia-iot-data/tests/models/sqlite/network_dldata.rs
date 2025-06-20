use std::{collections::HashMap, error::Error as StdError};

use async_trait::async_trait;
use chrono::{TimeZone, Utc};
use laboratory::{SpecContext, expect};
use sql_builder::{SqlBuilder, quote};
use sqlx::SqlitePool;

use sylvia_iot_data::models::{Model, network_dldata::NetworkDlData};

use super::{super::common::network_dldata as common_test, STATE, TestState};

struct Db<'a> {
    conn: &'a SqlitePool,
}

/// SQLite schema.
#[derive(sqlx::FromRow)]
struct Schema {
    pub data_id: String,
    /// i64 as time tick from Epoch in milliseconds.
    pub proc: i64,
    /// i64 as time tick from Epoch in milliseconds.
    #[sqlx(rename = "pub")]
    pub publish: i64,
    /// i64 as time tick from Epoch in milliseconds.
    pub resp: Option<i64>,
    pub status: i32,
    pub unit_id: String,
    /// use empty string as NULL.
    pub device_id: String,
    /// use empty string as NULL.
    pub network_code: String,
    /// use empty string as NULL.
    pub network_addr: String,
    pub profile: String,
    pub data: String,
    pub extension: String,
}

const TABLE_NAME: &'static str = "network_dldata";
const FIELDS: &'static [&'static str] = &[
    "data_id",
    "proc",
    "pub",
    "resp",
    "status",
    "unit_id",
    "device_id",
    "network_code",
    "network_addr",
    "profile",
    "data",
    "extension",
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
    let model = state.sqlite.as_ref().unwrap().application_dldata();

    let result = runtime.block_on(async { model.init().await });
    expect(result.is_ok()).to_equal(true)
}

/// Test `add()`.
pub fn add(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().network_dldata();
    let conn = state.sqlite.as_ref().unwrap().get_connection();

    common_test::add(runtime, model, &Db { conn })
}

/// Test `add()` with duplicate key.
pub fn add_dup(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().network_dldata();

    common_test::add_dup(runtime, model)
}

/// Test `del()` by specifying unit ID.
pub fn del_by_unit(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().network_dldata();
    let conn = state.sqlite.as_ref().unwrap().get_connection();

    common_test::del_by_unit(runtime, model, &Db { conn })
}

/// Test `del()` twice.
pub fn del_twice(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().network_dldata();

    common_test::del_twice(runtime, model)
}

/// Test `del()` by specifying device ID.
pub fn del_by_device_id(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().network_dldata();
    let conn = state.sqlite.as_ref().unwrap().get_connection();

    common_test::del_by_device_id(runtime, model, &Db { conn })
}

/// Test `del()` by specifying processed time.
pub fn del_by_proc(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().network_dldata();
    let conn = state.sqlite.as_ref().unwrap().get_connection();

    common_test::del_by_proc(runtime, model, &Db { conn })
}

/// Test `update()` status with zero.
pub fn update_with_zero(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().network_dldata();
    let conn = state.sqlite.as_ref().unwrap().get_connection();

    common_test::update_with_zero(runtime, model, &Db { conn })
}

/// Test `update()` status with positive.
pub fn update_with_positive(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().network_dldata();
    let conn = state.sqlite.as_ref().unwrap().get_connection();

    common_test::update_with_positive(runtime, model, &Db { conn })
}

/// Test `update()` status with negative.
pub fn update_with_negative(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().network_dldata();
    let conn = state.sqlite.as_ref().unwrap().get_connection();

    common_test::update_with_negative(runtime, model, &Db { conn })
}

/// Test `update()` with a non-exist condition.
pub fn update_not_exist(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().network_dldata();

    common_test::update_not_exist(runtime, model)
}

/// Test `count()`.
pub fn count(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().network_dldata();

    common_test::count(runtime, model)
}

/// Test `list()`.
pub fn list(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().network_dldata();

    common_test::list(runtime, model)
}

/// Test `list()` with sorting.
pub fn list_sort(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().network_dldata();

    common_test::list_sort(runtime, model)
}

/// Test `list()` with offset/limit.
pub fn list_offset_limit(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().network_dldata();

    common_test::list_offset_limit(runtime, model)
}

/// Test `list()` with cursors.
pub fn list_cursor(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().network_dldata();

    common_test::list_cursor(runtime, model)
}

#[async_trait]
impl<'a> common_test::Db for Db<'a> {
    async fn get(&self, data_id: &str) -> Result<Option<NetworkDlData>, Box<dyn StdError>> {
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

        Ok(Some(NetworkDlData {
            data_id: row.data_id,
            proc: Utc.timestamp_nanos(row.proc * 1000000),
            publish: Utc.timestamp_nanos(row.publish * 1000000),
            resp: match row.resp {
                None => None,
                Some(resp) => Some(Utc.timestamp_nanos(resp * 1000000)),
            },
            status: row.status,
            unit_id: row.unit_id,
            device_id: row.device_id,
            network_code: row.network_code,
            network_addr: row.network_addr,
            profile: row.profile,
            data: row.data,
            extension: match row.extension.len() {
                0 => None,
                _ => serde_json::from_str(row.extension.as_str())?,
            },
        }))
    }
}
