use std::collections::HashMap;

use chrono::{SubsecRound, Utc};
use laboratory::{SpecContext, expect};
use serde_json::{Map, Value};
use sql_builder::{SqlBuilder, quote};

use sylvia_iot_broker::models::{Model, application::QueryCond};

use super::{super::common::application as common_test, STATE, TestState};

const TABLE_NAME: &'static str = "application";
const FIELDS: &'static [&'static str] = &[
    "application_id",
    "code",
    "unit_id",
    "unit_code",
    "created_at",
    "modified_at",
    "host_uri",
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
    let model = state.sqlite.as_ref().unwrap().application();

    let result = runtime.block_on(async { model.init().await });
    expect(result.is_ok()).to_equal(true)
}

/// Test `get()` by specifying a application ID.
pub fn get_by_application_id(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let conn = state.sqlite.as_ref().unwrap().get_connection();
    let model = state.sqlite.as_ref().unwrap().application();

    let now = Utc::now().trunc_subsecs(3);
    let sql = match SqlBuilder::insert_into(TABLE_NAME)
        .fields(FIELDS)
        .values(&vec![
            quote("application_id_get_none"),
            quote("code_get_none"),
            quote("unit_id_get"),
            quote("unit_code_get"),
            now.timestamp_millis().to_string(),
            now.timestamp_millis().to_string(),
            quote("host_uri_get"),
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
        application_id: Some("application_id_not_exist"),
        ..Default::default()
    };
    match runtime.block_on(async { model.get(&cond).await }) {
        Err(e) => return Err(format!("model.get() error: {}", e)),
        Ok(application) => match application {
            None => (),
            Some(_) => return Err(format!("should not get not-exist one")),
        },
    };

    let cond = QueryCond {
        application_id: Some("application_id_get_none"),
        ..Default::default()
    };
    let application = match runtime.block_on(async { model.get(&cond).await }) {
        Err(e) => return Err(format!("model.get() none error: {}", e)),
        Ok(application) => match application {
            None => return Err("should get none one".to_string()),
            Some(application) => application,
        },
    };
    expect(application.application_id).to_equal("application_id_get_none".to_string())?;
    expect(application.code).to_equal("code_get_none".to_string())?;
    expect(application.unit_id).to_equal("unit_id_get".to_string())?;
    expect(application.unit_code).to_equal("unit_code_get".to_string())?;
    expect(application.created_at).to_equal(now)?;
    expect(application.modified_at).to_equal(now)?;
    expect(application.host_uri).to_equal("host_uri_get".to_string())?;
    expect(application.name).to_equal("".to_string())?;
    expect(application.info).to_equal(Map::<String, Value>::new())?;

    let sql = match SqlBuilder::insert_into(TABLE_NAME)
        .fields(FIELDS)
        .values(&vec![
            quote("application_id_get_some"),
            quote("code_get_some"),
            quote("unit_id_get"),
            quote("unit_code_get"),
            now.timestamp_millis().to_string(),
            now.timestamp_millis().to_string(),
            quote("host_uri_get"),
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
        application_id: Some("application_id_get_some"),
        ..Default::default()
    };
    let application = match runtime.block_on(async { model.get(&cond).await }) {
        Err(e) => return Err(format!("model.get() some error: {}", e)),
        Ok(application) => match application {
            None => return Err("should get some one".to_string()),
            Some(application) => application,
        },
    };
    expect(application.application_id).to_equal("application_id_get_some".to_string())?;
    expect(application.code).to_equal("code_get_some".to_string())?;
    expect(application.unit_id).to_equal("unit_id_get".to_string())?;
    expect(application.unit_code).to_equal("unit_code_get".to_string())?;
    expect(application.created_at).to_equal(now)?;
    expect(application.modified_at).to_equal(now)?;
    expect(application.host_uri).to_equal("host_uri_get".to_string())?;
    expect(application.name).to_equal("name_get".to_string())?;

    match application.info.get("boolean") {
        Some(Value::Bool(v)) => match *v {
            false => (),
            true => return Err("wrong info.boolean value".to_string()),
        },
        _ => return Err("wrong info.boolean type".to_string()),
    }
    match application.info.get("string") {
        Some(Value::String(v)) => match v.as_str() {
            "string" => (),
            _ => return Err("wrong info.string value".to_string()),
        },
        _ => return Err("wrong info.string type".to_string()),
    }
    match application.info.get("number") {
        Some(Value::Number(v)) => match v.as_i64() {
            Some(1) => (),
            _ => return Err("wrong info.number value".to_string()),
        },
        _ => return Err("wrong info.number type".to_string()),
    }
    match application.info.get("object") {
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

/// Test `get()` by specifying a application code.
pub fn get_by_code(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let conn = state.sqlite.as_ref().unwrap().get_connection();
    let model = state.sqlite.as_ref().unwrap().application();

    let now = Utc::now().trunc_subsecs(3);
    let sql = match SqlBuilder::insert_into(TABLE_NAME)
        .fields(FIELDS)
        .values(&vec![
            quote("application_id_get"),
            quote("code_get"),
            quote("unit_id_get"),
            quote("unit_code_get"),
            now.timestamp_millis().to_string(),
            now.timestamp_millis().to_string(),
            quote("host_uri_get"),
            quote("name_get"),
            quote("{}"),
        ])
        .sql()
    {
        Err(e) => return Err(format!("sql() error: {}", e.to_string())),
        Ok(sql) => sql,
    };
    if let Err(e) = runtime.block_on(async { sqlx::query(&sql).execute(conn).await }) {
        return Err(format!("insert_into() error: {}", e.to_string()));
    }

    let cond = QueryCond {
        code: Some("code_not_exist"),
        ..Default::default()
    };
    match runtime.block_on(async { model.get(&cond).await }) {
        Err(e) => return Err(format!("model.get() error: {}", e)),
        Ok(application) => match application {
            None => (),
            Some(_) => return Err(format!("should not get not-exist one")),
        },
    };

    let cond = QueryCond {
        code: Some("code_get"),
        ..Default::default()
    };
    let application = match runtime.block_on(async { model.get(&cond).await }) {
        Err(e) => return Err(format!("model.get() error: {}", e)),
        Ok(application) => match application {
            None => return Err("should get one".to_string()),
            Some(application) => application,
        },
    };
    expect(application.application_id).to_equal("application_id_get".to_string())?;
    expect(application.code).to_equal("code_get".to_string())?;
    expect(application.unit_id).to_equal("unit_id_get".to_string())?;
    expect(application.unit_code).to_equal("unit_code_get".to_string())?;
    expect(application.created_at).to_equal(now)?;
    expect(application.modified_at).to_equal(now)?;
    expect(application.host_uri).to_equal("host_uri_get".to_string())?;
    expect(application.name).to_equal("name_get".to_string())?;
    expect(application.info).to_equal(Map::<String, Value>::new())
}

/// Test `get()` by specifying a pair of unit ID and application ID.
pub fn get_by_unit_application(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let conn = state.sqlite.as_ref().unwrap().get_connection();
    let model = state.sqlite.as_ref().unwrap().application();

    let now = Utc::now().trunc_subsecs(3);
    let application_id_get = "application_id_get";
    let application_id_not_get = "application_id_not_get";
    let application_id_get_other = "application_id_get_other";
    let sql = match SqlBuilder::insert_into(TABLE_NAME)
        .fields(FIELDS)
        .values(&vec![
            quote(application_id_get),
            quote("code_get"),
            quote("unit_id_get"),
            quote("unit_code_get"),
            now.timestamp_millis().to_string(),
            now.timestamp_millis().to_string(),
            quote("host_uri_get"),
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
            quote(application_id_not_get),
            quote("code_not_get"),
            quote("unit_id_get"),
            quote("unit_code_get"),
            now.timestamp_millis().to_string(),
            now.timestamp_millis().to_string(),
            quote("host_uri_get"),
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
            quote(application_id_get_other),
            quote("code_get"),
            quote("unit_id_get_other"),
            quote("unit_code_get_other"),
            now.timestamp_millis().to_string(),
            now.timestamp_millis().to_string(),
            quote("host_uri_get"),
            quote("name_get"),
            quote("{}"),
        ])
        .sql()
    {
        Err(e) => return Err(format!("sql() get other error: {}", e.to_string())),
        Ok(sql) => sql,
    };
    if let Err(e) = runtime.block_on(async { sqlx::query(&sql).execute(conn).await }) {
        return Err(format!("insert_into() get other error: {}", e.to_string()));
    }

    let cond = QueryCond {
        application_id: Some(application_id_get),
        unit_id: Some("unit_id_get"),
        ..Default::default()
    };
    let application = match runtime.block_on(async { model.get(&cond).await }) {
        Err(e) => return Err(format!("model.get() error: {}", e)),
        Ok(application) => match application {
            None => return Err("should get one".to_string()),
            Some(application) => application,
        },
    };
    if application.application_id.as_str() != application_id_get {
        return Err("get wrong application".to_string());
    }

    let cond = QueryCond {
        application_id: Some(application_id_get_other),
        unit_id: Some("unit_id_get"),
        ..Default::default()
    };
    match runtime.block_on(async { model.get(&cond).await }) {
        Err(e) => return Err(format!("model.get() other error: {}", e)),
        Ok(application) => match application {
            None => (),
            Some(_) => return Err("should not get other one".to_string()),
        },
    }
    Ok(())
}

/// Test `add()`.
pub fn add(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().application();

    common_test::add(runtime, model)
}

/// Test `add()` with duplicate key.
pub fn add_dup(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().application();

    common_test::add_dup(runtime, model)
}

/// Test `del()` by specifying an application ID.
pub fn del_by_application_id(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().application();

    common_test::del_by_application_id(runtime, model)
}

/// Test `del()` twice.
pub fn del_twice(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().application();

    common_test::del_twice(runtime, model)
}

/// Test `del()` by specifying a unit ID.
pub fn del_by_unit_id(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().application();

    common_test::del_by_unit_id(runtime, model)
}

/// Test `del()` by specifying a pair of unit ID and application ID.
pub fn del_by_unit_application(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().application();

    common_test::del_by_unit_application(runtime, model)
}

/// Test `update()`.
pub fn update(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().application();

    common_test::update(runtime, model)
}

/// Test `update()` with a non-exist condition.
pub fn update_not_exist(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().application();

    common_test::update_not_exist(runtime, model)
}

/// Test `update()` with invalid update content.
pub fn update_invalid(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().application();

    common_test::update_invalid(runtime, model)
}

/// Test `count()`.
pub fn count(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().application();

    common_test::count(runtime, model)
}

/// Test `list()`.
pub fn list(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().application();

    common_test::list(runtime, model)
}

/// Test `list()` with sorting.
pub fn list_sort(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().application();

    common_test::list_sort(runtime, model)
}

/// Test `list()` with offset/limit.
pub fn list_offset_limit(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().application();

    common_test::list_offset_limit(runtime, model)
}

/// Test `list()` with cursors.
pub fn list_cursor(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().application();

    common_test::list_cursor(runtime, model)
}
