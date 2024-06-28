use std::collections::HashMap;

use chrono::{SubsecRound, Utc};
use laboratory::{expect, SpecContext};
use mongodb::bson::{doc, DateTime, Document};
use serde::{Deserialize, Serialize};

use sylvia_iot_broker::models::Model;

use super::{super::common::dldata_buffer as common_test, TestState, STATE};

/// MongoDB schema.
#[derive(Debug, Deserialize, Serialize)]
struct Schema {
    #[serde(rename = "dataId")]
    data_id: String,
    #[serde(rename = "unitId")]
    unit_id: String,
    #[serde(rename = "unitCode")]
    unit_code: String,
    #[serde(rename = "applicationId")]
    application_id: String,
    #[serde(rename = "applicationCode")]
    application_code: String,
    #[serde(rename = "networkId")]
    network_id: String,
    #[serde(rename = "networkAddr")]
    network_addr: String,
    #[serde(rename = "deviceId")]
    device_id: String,
    #[serde(rename = "createdAt")]
    created_at: DateTime,
    #[serde(rename = "expiredAt")]
    expired_at: DateTime,
}

const COL_NAME: &'static str = "dldataBuffer";

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
    let model = state.mongodb.as_ref().unwrap().dldata_buffer();

    let result = runtime.block_on(async { model.init().await });
    expect(result.is_ok()).to_equal(true)
}

/// Test `get()`.
pub fn get(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let conn = state.mongodb.as_ref().unwrap().get_connection();
    let model = state.mongodb.as_ref().unwrap().dldata_buffer();

    let now = Utc::now().trunc_subsecs(3);
    let item = Schema {
        data_id: "data_id_get".to_string(),
        unit_id: "unit_id_get".to_string(),
        unit_code: "unit_code_get".to_string(),
        application_id: "application_id_get".to_string(),
        application_code: "application_code_get".to_string(),
        network_id: "network_id_get".to_string(),
        network_addr: "network_addr_get".to_string(),
        device_id: "device_id_get".to_string(),
        created_at: now.into(),
        expired_at: now.into(),
    };
    if let Err(e) =
        runtime.block_on(async { conn.collection::<Schema>(COL_NAME).insert_one(item).await })
    {
        return Err(format!("insert_one() error: {}", e));
    }

    match runtime.block_on(async { model.get("data_id_not_exist").await }) {
        Err(e) => return Err(format!("model.get() error: {}", e)),
        Ok(data) => match data {
            None => (),
            Some(_) => return Err(format!("should not get not-exist one")),
        },
    };

    let data = match runtime.block_on(async { model.get("data_id_get").await }) {
        Err(e) => return Err(format!("model.get() error: {}", e)),
        Ok(data) => match data {
            None => return Err("should get one".to_string()),
            Some(data) => data,
        },
    };
    expect(data.data_id).to_equal("data_id_get".to_string())?;
    expect(data.unit_id).to_equal("unit_id_get".to_string())?;
    expect(data.unit_code).to_equal("unit_code_get".to_string())?;
    expect(data.application_id).to_equal("application_id_get".to_string())?;
    expect(data.application_code).to_equal("application_code_get".to_string())?;
    expect(data.network_id).to_equal("network_id_get".to_string())?;
    expect(data.network_addr).to_equal("network_addr_get".to_string())?;
    expect(data.device_id).to_equal("device_id_get".to_string())?;
    expect(data.created_at).to_equal(now)?;
    expect(data.expired_at).to_equal(now)
}

/// Test `add()`.
pub fn add(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().dldata_buffer();

    common_test::add(runtime, model)
}

/// Test `add()` with duplicate key.
pub fn add_dup(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().dldata_buffer();

    common_test::add_dup(runtime, model)
}

/// Test `del()` by specifying a data ID.
pub fn del_by_data_id(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().dldata_buffer();

    common_test::del_by_data_id(runtime, model)
}

/// Test `del()` twice.
pub fn del_twice(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().dldata_buffer();

    common_test::del_twice(runtime, model)
}

/// Test `del()` by specifying a unit ID.
pub fn del_by_unit_id(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().dldata_buffer();

    common_test::del_by_unit_id(runtime, model)
}

/// Test `del()` by specifying a pair of unit ID and data ID.
pub fn del_by_unit_data(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().dldata_buffer();

    common_test::del_by_unit_data(runtime, model)
}

/// Test `del()` by specifying a application ID.
pub fn del_by_application_id(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().dldata_buffer();

    common_test::del_by_application_id(runtime, model)
}

/// Test `del()` by specifying a network ID.
pub fn del_by_network_id(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().dldata_buffer();

    common_test::del_by_network_id(runtime, model)
}

/// Test `del()` by specifying a device ID.
pub fn del_by_device_id(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().dldata_buffer();

    common_test::del_by_device_id(runtime, model)
}

/// Test `del()` by specifying network addresses.
pub fn del_by_network_addrs(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().dldata_buffer();

    common_test::del_by_network_addrs(runtime, model)
}

/// Test `count()`.
pub fn count(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().dldata_buffer();

    common_test::count(runtime, model)
}

/// Test `list()`.
pub fn list(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().dldata_buffer();

    common_test::list(runtime, model)
}

/// Test `list()` with sorting.
pub fn list_sort(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().dldata_buffer();

    common_test::list_sort(runtime, model)
}

/// Test `list()` with offset/limit.
pub fn list_offset_limit(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().dldata_buffer();

    common_test::list_offset_limit(runtime, model)
}

/// Test `list()` with cursors.
pub fn list_cursor(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().dldata_buffer();

    common_test::list_cursor(runtime, model)
}
