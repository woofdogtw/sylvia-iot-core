use std::collections::HashMap;

use chrono::{SubsecRound, Utc};
use laboratory::{expect, SpecContext};
use serde_json::{Map, Value};
use sql_builder::{quote, SqlBuilder};

use sylvia_iot_broker::models::{
    device::{QueryCond, QueryOneCond},
    Model,
};

use super::{super::common::device as common_test, TestState, STATE};

const TABLE_NAME: &'static str = "device";
const FIELDS: &'static [&'static str] = &[
    "device_id",
    "unit_id",
    "unit_code",
    "network_id",
    "network_code",
    "network_addr",
    "created_at",
    "modified_at",
    "profile",
    "name",
    "info",
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
    let model = state.sqlite.as_ref().unwrap().device();

    let result = runtime.block_on(async { model.init().await });
    expect(result.is_ok()).to_equal(true)
}

/// Test `get()` by specifying a device ID.
pub fn get_by_device_id(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let conn = state.sqlite.as_ref().unwrap().get_connection();
    let model = state.sqlite.as_ref().unwrap().device();

    let now = Utc::now().trunc_subsecs(3);
    let sql = match SqlBuilder::insert_into(TABLE_NAME)
        .fields(FIELDS)
        .values(&vec![
            quote("device_id_get_none"),
            quote("unit_id_get"),
            quote(""),
            quote("network_id_get_none"),
            quote("network_code_get_none"),
            quote("network_addr_get_none"),
            now.timestamp_millis().to_string(),
            now.timestamp_millis().to_string(),
            quote(""),
            quote(""),
            quote("{}"),
        ])
        .sql()
    {
        Err(e) => return Err(format!("sql() none error: {}", e.to_string())),
        Ok(sql) => sql,
    };
    if let Err(e) = runtime.block_on(async { sqlx::query(&sql).execute(conn).await }) {
        return Err(format!("insert_into() none error: {}", e.to_string()));
    }

    let cond = QueryCond {
        device_id: Some("device_id_not_exist"),
        ..Default::default()
    };
    match runtime.block_on(async { model.get(&cond).await }) {
        Err(e) => return Err(format!("model.get() error: {}", e)),
        Ok(device) => match device {
            None => (),
            Some(_) => return Err(format!("should not get not-exist one")),
        },
    };

    let cond = QueryCond {
        device_id: Some("device_id_get_none"),
        ..Default::default()
    };
    let device = match runtime.block_on(async { model.get(&cond).await }) {
        Err(e) => return Err(format!("model.get() none error: {}", e)),
        Ok(device) => match device {
            None => return Err("should get none one".to_string()),
            Some(device) => device,
        },
    };
    expect(device.device_id).to_equal("device_id_get_none".to_string())?;
    expect(device.unit_id).to_equal("unit_id_get".to_string())?;
    expect(device.unit_code).to_equal(None)?;
    expect(device.network_id).to_equal("network_id_get_none".to_string())?;
    expect(device.network_code).to_equal("network_code_get_none".to_string())?;
    expect(device.network_addr).to_equal("network_addr_get_none".to_string())?;
    expect(device.created_at).to_equal(now)?;
    expect(device.modified_at).to_equal(now)?;
    expect(device.profile).to_equal("".to_string())?;
    expect(device.name).to_equal("".to_string())?;
    expect(device.info).to_equal(Map::<String, Value>::new())?;

    let sql = match SqlBuilder::insert_into(TABLE_NAME)
        .fields(FIELDS)
        .values(&vec![
            quote("device_id_get_some"),
            quote("unit_id_get"),
            quote("unit_code_get"),
            quote("network_id_get_some"),
            quote("network_code_get_some"),
            quote("network_addr_get_some"),
            now.timestamp_millis().to_string(),
            now.timestamp_millis().to_string(),
            quote("profile_get"),
            quote("name_get"),
            quote("{\"boolean\":false,\"string\":\"string\",\"number\":1,\"object\":{\"array\":[\"array\"]}}"),
        ])
        .sql()
    {
        Err(e) => return Err(format!("sql() some error: {}", e.to_string())),
        Ok(sql) => sql,
    };
    if let Err(e) = runtime.block_on(async { sqlx::query(&sql).execute(conn).await }) {
        return Err(format!("insert_into() some error: {}", e.to_string()));
    }

    let cond = QueryCond {
        device_id: Some("device_id_get_some"),
        ..Default::default()
    };
    let device = match runtime.block_on(async { model.get(&cond).await }) {
        Err(e) => return Err(format!("model.get() some error: {}", e)),
        Ok(device) => match device {
            None => return Err("should get some one".to_string()),
            Some(device) => device,
        },
    };
    expect(device.device_id).to_equal("device_id_get_some".to_string())?;
    expect(device.unit_id).to_equal("unit_id_get".to_string())?;
    expect(device.unit_code).to_equal(Some("unit_code_get".to_string()))?;
    expect(device.network_id).to_equal("network_id_get_some".to_string())?;
    expect(device.network_code).to_equal("network_code_get_some".to_string())?;
    expect(device.network_addr).to_equal("network_addr_get_some".to_string())?;
    expect(device.created_at).to_equal(now)?;
    expect(device.modified_at).to_equal(now)?;
    expect(device.profile).to_equal("profile_get".to_string())?;
    expect(device.name).to_equal("name_get".to_string())?;

    match device.info.get("boolean") {
        Some(Value::Bool(v)) => match *v {
            false => (),
            true => return Err("wrong info.boolean value".to_string()),
        },
        _ => return Err("wrong info.boolean type".to_string()),
    }
    match device.info.get("string") {
        Some(Value::String(v)) => match v.as_str() {
            "string" => (),
            _ => return Err("wrong info.string value".to_string()),
        },
        _ => return Err("wrong info.string type".to_string()),
    }
    match device.info.get("number") {
        Some(Value::Number(v)) => match v.as_i64() {
            Some(1) => (),
            _ => return Err("wrong info.number value".to_string()),
        },
        _ => return Err("wrong info.number type".to_string()),
    }
    match device.info.get("object") {
        Some(Value::Object(v)) => {
            if v.len() != 1 {
                return Err("wrong info.object key length not 1".to_string());
            }
            match v.get("array") {
                Some(Value::Array(v)) => {
                    if v.len() != 1 {
                        return Err("wrong info.object.array length not 1".to_string());
                    }
                    match v.get(0) {
                        Some(Value::String(v)) => match v.as_str() {
                            "array" => (),
                            _ => return Err("wrong info.object.array content value".to_string()),
                        },
                        _ => return Err("wrong info.object.array content type".to_string()),
                    }
                }
                _ => return Err("wrong info.object.array type".to_string()),
            }
        }
        _ => return Err("wrong info.object type".to_string()),
    }
    Ok(())
}

/// Test `get()` by specifying a pair of unit ID and device ID.
pub fn get_by_unit_device(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let conn = state.sqlite.as_ref().unwrap().get_connection();
    let model = state.sqlite.as_ref().unwrap().device();

    let now = Utc::now().trunc_subsecs(3);
    let device_id_get = "device_id_get";
    let device_id_not_get = "device_id_not_get";
    let device_id_get_other = "device_id_get_other";
    let sql = match SqlBuilder::insert_into(TABLE_NAME)
        .fields(FIELDS)
        .values(&vec![
            quote(device_id_get),
            quote("unit_id_get"),
            quote(""),
            quote("network_id_get"),
            quote("network_code_get"),
            quote("network_addr_get"),
            now.timestamp_millis().to_string(),
            now.timestamp_millis().to_string(),
            quote("profile_get"),
            quote("name_get"),
            quote("{}"),
        ])
        .sql()
    {
        Err(e) => return Err(format!("sql() get error: {}", e.to_string())),
        Ok(sql) => sql,
    };
    if let Err(e) = runtime.block_on(async { sqlx::query(&sql).execute(conn).await }) {
        return Err(format!("insert_into() get error: {}", e.to_string()));
    }
    let sql = match SqlBuilder::insert_into(TABLE_NAME)
        .fields(FIELDS)
        .values(&vec![
            quote(device_id_not_get),
            quote("unit_id_get"),
            quote(""),
            quote("network_id_get"),
            quote("network_code_not_get"),
            quote("network_addr_not_get"),
            now.timestamp_millis().to_string(),
            now.timestamp_millis().to_string(),
            quote("profile_get"),
            quote("name_get"),
            quote("{}"),
        ])
        .sql()
    {
        Err(e) => return Err(format!("sql() not-get error: {}", e.to_string())),
        Ok(sql) => sql,
    };
    if let Err(e) = runtime.block_on(async { sqlx::query(&sql).execute(conn).await }) {
        return Err(format!("insert_into() not-get error: {}", e.to_string()));
    }
    let sql = match SqlBuilder::insert_into(TABLE_NAME)
        .fields(FIELDS)
        .values(&vec![
            quote(device_id_get_other),
            quote("unit_id_get_other"),
            quote(""),
            quote("network_id_get"),
            quote("network_code_get_other"),
            quote("network_addr_get_other"),
            now.timestamp_millis().to_string(),
            now.timestamp_millis().to_string(),
            quote("profile_get"),
            quote("name_get"),
            quote("{}"),
        ])
        .sql()
    {
        Err(e) => return Err(format!("sql() other error: {}", e.to_string())),
        Ok(sql) => sql,
    };
    if let Err(e) = runtime.block_on(async { sqlx::query(&sql).execute(conn).await }) {
        return Err(format!("insert_into() other error: {}", e.to_string()));
    }

    let cond = QueryCond {
        device_id: Some(device_id_get),
        unit_id: Some("unit_id_get"),
        ..Default::default()
    };
    let device = match runtime.block_on(async { model.get(&cond).await }) {
        Err(e) => return Err(format!("model.get() error: {}", e)),
        Ok(device) => match device {
            None => return Err("should get one".to_string()),
            Some(device) => device,
        },
    };
    if device.device_id.as_str() != device_id_get {
        return Err("get wrong device".to_string());
    }

    let cond = QueryCond {
        device_id: Some(device_id_get_other),
        unit_id: Some("unit_id_get"),
        ..Default::default()
    };
    match runtime.block_on(async { model.get(&cond).await }) {
        Err(e) => return Err(format!("model.get() other error: {}", e)),
        Ok(device) => match device {
            None => (),
            Some(_) => return Err("should not get other one".to_string()),
        },
    }
    Ok(())
}

/// Test `get()` by specifying a pair of network ID and device ID.
pub fn get_by_network_device(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let conn = state.sqlite.as_ref().unwrap().get_connection();
    let model = state.sqlite.as_ref().unwrap().device();

    let now = Utc::now().trunc_subsecs(3);
    let device_id_get = "device_id_get";
    let device_id_not_get = "device_id_not_get";
    let device_id_get_other = "device_id_get_other";
    let sql = match SqlBuilder::insert_into(TABLE_NAME)
        .fields(FIELDS)
        .values(&vec![
            quote(device_id_get),
            quote("unit_id_get"),
            quote(""),
            quote("network_id_get"),
            quote("network_code_get"),
            quote("network_addr_get"),
            now.timestamp_millis().to_string(),
            now.timestamp_millis().to_string(),
            quote("profile_get"),
            quote("name_get"),
            quote("{}"),
        ])
        .sql()
    {
        Err(e) => return Err(format!("sql() get error: {}", e.to_string())),
        Ok(sql) => sql,
    };
    if let Err(e) = runtime.block_on(async { sqlx::query(&sql).execute(conn).await }) {
        return Err(format!("insert_into() get error: {}", e.to_string()));
    }
    let sql = match SqlBuilder::insert_into(TABLE_NAME)
        .fields(FIELDS)
        .values(&vec![
            quote(device_id_not_get),
            quote("unit_id_get"),
            quote(""),
            quote("network_id_get"),
            quote("network_code_not_get"),
            quote("network_addr_not_get"),
            now.timestamp_millis().to_string(),
            now.timestamp_millis().to_string(),
            quote("profile_get"),
            quote("name_get"),
            quote("{}"),
        ])
        .sql()
    {
        Err(e) => return Err(format!("sql() not-get error: {}", e.to_string())),
        Ok(sql) => sql,
    };
    if let Err(e) = runtime.block_on(async { sqlx::query(&sql).execute(conn).await }) {
        return Err(format!("insert_into() not-get error: {}", e.to_string()));
    }
    let sql = match SqlBuilder::insert_into(TABLE_NAME)
        .fields(FIELDS)
        .values(&vec![
            quote(device_id_get_other),
            quote("unit_id_get_other"),
            quote(""),
            quote("network_id_get_other"),
            quote("network_code_get_other"),
            quote("network_addr_get_other"),
            now.timestamp_millis().to_string(),
            now.timestamp_millis().to_string(),
            quote("profile_get"),
            quote("name_get"),
            quote("{}"),
        ])
        .sql()
    {
        Err(e) => return Err(format!("sql() other error: {}", e.to_string())),
        Ok(sql) => sql,
    };
    if let Err(e) = runtime.block_on(async { sqlx::query(&sql).execute(conn).await }) {
        return Err(format!("insert_into() other error: {}", e.to_string()));
    }

    let cond = QueryCond {
        device_id: Some(device_id_get),
        network_id: Some("network_id_get"),
        ..Default::default()
    };
    let device = match runtime.block_on(async { model.get(&cond).await }) {
        Err(e) => return Err(format!("model.get() error: {}", e)),
        Ok(device) => match device {
            None => return Err("should get one".to_string()),
            Some(device) => device,
        },
    };
    if device.device_id.as_str() != device_id_get {
        return Err("get wrong device".to_string());
    }

    let cond = QueryCond {
        device_id: Some(device_id_get_other),
        network_id: Some("network_id_get"),
        ..Default::default()
    };
    match runtime.block_on(async { model.get(&cond).await }) {
        Err(e) => return Err(format!("model.get() other error: {}", e)),
        Ok(device) => match device {
            None => (),
            Some(_) => return Err("should not get other one".to_string()),
        },
    }
    Ok(())
}

/// Test `get()` by specifying a pair of unit code and network code/address.
pub fn get_by_unit_network(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let conn = state.sqlite.as_ref().unwrap().get_connection();
    let model = state.sqlite.as_ref().unwrap().device();

    let now = Utc::now().trunc_subsecs(3);
    let device_id_get_none = "device_id_get_none";
    let device_id_get_some = "device_id_get_some";
    let sql = match SqlBuilder::insert_into(TABLE_NAME)
        .fields(FIELDS)
        .values(&vec![
            quote(device_id_get_none),
            quote("unit_id_get_none"),
            quote(""),
            quote("network_id_get_none"),
            quote("network_code_get_none"),
            quote("network_addr_get_none"),
            now.timestamp_millis().to_string(),
            now.timestamp_millis().to_string(),
            quote("profile_get"),
            quote("name_get"),
            quote("{}"),
        ])
        .sql()
    {
        Err(e) => return Err(format!("sql() none error: {}", e.to_string())),
        Ok(sql) => sql,
    };
    if let Err(e) = runtime.block_on(async { sqlx::query(&sql).execute(conn).await }) {
        return Err(format!("insert_into() none error: {}", e.to_string()));
    }
    let sql = match SqlBuilder::insert_into(TABLE_NAME)
        .fields(FIELDS)
        .values(&vec![
            quote(device_id_get_some),
            quote("unit_id_get_some"),
            quote("unit_code_get_some"),
            quote("network_id_get_some"),
            quote("network_code_get_some"),
            quote("network_addr_get_some"),
            now.timestamp_millis().to_string(),
            now.timestamp_millis().to_string(),
            quote("profile_get"),
            quote("name_get"),
            quote("{}"),
        ])
        .sql()
    {
        Err(e) => return Err(format!("sql() some error: {}", e.to_string())),
        Ok(sql) => sql,
    };
    if let Err(e) = runtime.block_on(async { sqlx::query(&sql).execute(conn).await }) {
        return Err(format!("insert_into() some error: {}", e.to_string()));
    }

    let cond = QueryCond {
        device: Some(QueryOneCond {
            unit_code: None,
            network_code: "network_code_get_none",
            network_addr: "network_addr_get_none",
        }),
        ..Default::default()
    };
    let device = match runtime.block_on(async { model.get(&cond).await }) {
        Err(e) => return Err(format!("model.get() error: {}", e)),
        Ok(device) => match device {
            None => return Err("should get none".to_string()),
            Some(device) => device,
        },
    };
    if device.device_id.as_str() != device_id_get_none {
        return Err("get wrong none device".to_string());
    }

    let cond = QueryCond {
        device: Some(QueryOneCond {
            unit_code: Some("unit_code_get_some"),
            network_code: "network_code_get_some",
            network_addr: "network_addr_get_some",
        }),
        ..Default::default()
    };
    let device = match runtime.block_on(async { model.get(&cond).await }) {
        Err(e) => return Err(format!("model.get() error: {}", e)),
        Ok(device) => match device {
            None => return Err("should get some".to_string()),
            Some(device) => device,
        },
    };
    if device.device_id.as_str() != device_id_get_some {
        return Err("get wrong some device".to_string());
    }
    Ok(())
}

/// Test `add()`.
pub fn add(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().device();

    common_test::add(runtime, model)
}

/// Test `add()` with duplicate key.
pub fn add_dup(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().device();

    common_test::add_dup(runtime, model)
}

/// Test `add_bulk()`.
pub fn add_bulk(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().device();

    common_test::add_bulk(runtime, model)
}

/// Test `del()` by specifying a device ID.
pub fn del_by_device_id(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().device();

    common_test::del_by_device_id(runtime, model)
}

/// Test `del()` twice.
pub fn del_twice(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().device();

    common_test::del_twice(runtime, model)
}

/// Test `del()` by specifying a unit ID.
pub fn del_by_unit_id(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().device();

    common_test::del_by_unit_id(runtime, model)
}

/// Test `del()` by specifying a pair of unit ID and device ID.
pub fn del_by_unit_device(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().device();

    common_test::del_by_unit_device(runtime, model)
}

/// Test `del()` by specifying a network ID.
pub fn del_by_network_id(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().device();

    common_test::del_by_network_id(runtime, model)
}

/// Test `del()` by specifying network addresses.
pub fn del_by_network_addrs(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().device();

    common_test::del_by_network_addrs(runtime, model)
}

/// Test `update()`.
pub fn update(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().device();

    common_test::update(runtime, model)
}

/// Test `update()` with a non-exist condition.
pub fn update_not_exist(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().device();

    common_test::update_not_exist(runtime, model)
}

/// Test `update()` with invalid update content.
pub fn update_invalid(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().device();

    common_test::update_invalid(runtime, model)
}

/// Test `count()`.
pub fn count(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().device();

    common_test::count(runtime, model)
}

/// Test `list()`.
pub fn list(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().device();

    common_test::list(runtime, model)
}

/// Test `list()` with sorting.
pub fn list_sort(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().device();

    common_test::list_sort(runtime, model)
}

/// Test `list()` with offset/limit.
pub fn list_offset_limit(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().device();

    common_test::list_offset_limit(runtime, model)
}

/// Test `list()` with cursors.
pub fn list_cursor(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().device();

    common_test::list_cursor(runtime, model)
}
