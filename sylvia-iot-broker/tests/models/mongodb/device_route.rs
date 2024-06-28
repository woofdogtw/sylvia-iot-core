use std::collections::HashMap;

use chrono::{SubsecRound, Utc};
use laboratory::{expect, SpecContext};
use mongodb::bson::{doc, DateTime, Document};
use serde::{Deserialize, Serialize};

use sylvia_iot_broker::models::Model;

use super::{super::common::device_route as common_test, TestState, STATE};

/// MongoDB schema.
#[derive(Debug, Deserialize, Serialize)]
struct Schema {
    #[serde(rename = "routeId")]
    route_id: String,
    #[serde(rename = "unitId")]
    unit_id: String,
    #[serde(rename = "unitCode")]
    unit_code: String,
    #[serde(rename = "applicationId")]
    application_id: String,
    #[serde(rename = "applicationCode")]
    application_code: String,
    #[serde(rename = "deviceId")]
    device_id: String,
    #[serde(rename = "networkId")]
    network_id: String,
    #[serde(rename = "networkCode")]
    network_code: String,
    #[serde(rename = "networkAddr")]
    network_addr: String,
    profile: String,
    #[serde(rename = "createdAt")]
    created_at: DateTime,
    #[serde(rename = "modifiedAt")]
    modified_at: DateTime,
}

const COL_NAME: &'static str = "deviceRoute";

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
    let model = state.mongodb.as_ref().unwrap().device_route();

    let result = runtime.block_on(async { model.init().await });
    expect(result.is_ok()).to_equal(true)
}

/// Test `get()`.
pub fn get(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let conn = state.mongodb.as_ref().unwrap().get_connection();
    let model = state.mongodb.as_ref().unwrap().device_route();

    let now = Utc::now().trunc_subsecs(3);
    let item = Schema {
        route_id: "route_id_get".to_string(),
        unit_id: "unit_id_get".to_string(),
        unit_code: "unit_code_get".to_string(),
        application_id: "application_id_get".to_string(),
        application_code: "application_code_get".to_string(),
        device_id: "device_id_get".to_string(),
        network_id: "network_id_get".to_string(),
        network_code: "network_code_get".to_string(),
        network_addr: "network_addr_get".to_string(),
        profile: "profile_get".to_string(),
        created_at: now.into(),
        modified_at: now.into(),
    };
    if let Err(e) =
        runtime.block_on(async { conn.collection::<Schema>(COL_NAME).insert_one(item).await })
    {
        return Err(format!("insert_one() error: {}", e));
    }

    match runtime.block_on(async { model.get("route_id_not_exist").await }) {
        Err(e) => return Err(format!("model.get() error: {}", e)),
        Ok(route) => match route {
            None => (),
            Some(_) => return Err(format!("should not get not-exist one")),
        },
    };

    let route = match runtime.block_on(async { model.get("route_id_get").await }) {
        Err(e) => return Err(format!("model.get() error: {}", e)),
        Ok(route) => match route {
            None => return Err("should get one".to_string()),
            Some(route) => route,
        },
    };
    expect(route.route_id).to_equal("route_id_get".to_string())?;
    expect(route.unit_id).to_equal("unit_id_get".to_string())?;
    expect(route.unit_code).to_equal("unit_code_get".to_string())?;
    expect(route.application_id).to_equal("application_id_get".to_string())?;
    expect(route.application_code).to_equal("application_code_get".to_string())?;
    expect(route.device_id).to_equal("device_id_get".to_string())?;
    expect(route.network_id).to_equal("network_id_get".to_string())?;
    expect(route.network_code).to_equal("network_code_get".to_string())?;
    expect(route.network_addr).to_equal("network_addr_get".to_string())?;
    expect(route.profile).to_equal("profile_get".to_string())?;
    expect(route.created_at).to_equal(now)?;
    expect(route.modified_at).to_equal(now)
}

/// Test `add()`.
pub fn add(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().device_route();

    common_test::add(runtime, model)
}

/// Test `add()` with duplicate key.
pub fn add_dup(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().device_route();

    common_test::add_dup(runtime, model)
}

/// Test `add_bulk()`.
pub fn add_bulk(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().device_route();

    common_test::add_bulk(runtime, model)
}

/// Test `del()` by specifying a route ID.
pub fn del_by_route_id(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().device_route();

    common_test::del_by_route_id(runtime, model)
}

/// Test `del()` twice.
pub fn del_twice(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().device_route();

    common_test::del_twice(runtime, model)
}

/// Test `del()` by specifying a unit ID.
pub fn del_by_unit_id(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().device_route();

    common_test::del_by_unit_id(runtime, model)
}

/// Test `del()` by specifying a pair of unit ID and route ID.
pub fn del_by_unit_route(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().device_route();

    common_test::del_by_unit_route(runtime, model)
}

/// Test `del()` by specifying an application ID.
pub fn del_by_application_id(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().device_route();

    common_test::del_by_application_id(runtime, model)
}

/// Test `del()` by specifying a network ID.
pub fn del_by_network_id(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().device_route();

    common_test::del_by_network_id(runtime, model)
}

/// Test `del()` by specifying a device ID.
pub fn del_by_device_id(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().device_route();

    common_test::del_by_device_id(runtime, model)
}

/// Test `del()` by specifying network addresses.
pub fn del_by_network_addrs(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().device_route();

    common_test::del_by_network_addrs(runtime, model)
}

/// Test `update()`.
pub fn update(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().device_route();

    common_test::update(runtime, model)
}

/// Test `update()` with a non-exist condition.
pub fn update_not_exist(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().device_route();

    common_test::update_not_exist(runtime, model)
}

/// Test `update()` with invalid update content.
pub fn update_invalid(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().device_route();

    common_test::update_invalid(runtime, model)
}

/// Test `count()`.
pub fn count(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().device_route();

    common_test::count(runtime, model)
}

/// Test `list()`.
pub fn list(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().device_route();

    common_test::list(runtime, model)
}

/// Test `list()` with sorting.
pub fn list_sort(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().device_route();

    common_test::list_sort(runtime, model)
}

/// Test `list()` with offset/limit.
pub fn list_offset_limit(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().device_route();

    common_test::list_offset_limit(runtime, model)
}

/// Test `list()` with cursors.
pub fn list_cursor(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().device_route();

    common_test::list_cursor(runtime, model)
}
