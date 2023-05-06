use std::collections::HashMap;

use chrono::{SubsecRound, Utc};
use laboratory::{expect, SpecContext};
use serde_json::{Map, Value};
use sql_builder::{quote, SqlBuilder};

use sylvia_iot_broker::models::{unit::QueryCond, Model};

use super::{super::common::unit as common_test, TestState, STATE};

const TABLE_NAME: &'static str = "unit";
const FIELDS: &'static [&'static str] = &[
    "unit_id",
    "code",
    "created_at",
    "modified_at",
    "owner_id",
    "member_ids",
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
    let model = state.sqlite.as_ref().unwrap().unit();

    let result = runtime.block_on(async { model.init().await });
    expect(result.is_ok()).to_equal(true)
}

/// Test `get()` by specifying a unit ID.
pub fn get_by_unit_id(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let conn = state.sqlite.as_ref().unwrap().get_connection();
    let model = state.sqlite.as_ref().unwrap().unit();

    let now = Utc::now().trunc_subsecs(3);
    let sql = match SqlBuilder::insert_into(TABLE_NAME)
        .fields(FIELDS)
        .values(&vec![
            quote("unit_id_get_none"),
            quote("code_get_none"),
            now.timestamp_millis().to_string(),
            now.timestamp_millis().to_string(),
            quote("owner_id_get_none".to_string()),
            quote(""),
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

    let cond = QueryCond {
        unit_id: Some("unit_id_not_exist"),
        ..Default::default()
    };
    match runtime.block_on(async { model.get(&cond).await }) {
        Err(e) => return Err(format!("model.get() error: {}", e)),
        Ok(unit) => match unit {
            None => (),
            Some(_) => return Err(format!("should not get not-exist one")),
        },
    };

    let cond = QueryCond {
        unit_id: Some("unit_id_get_none"),
        ..Default::default()
    };
    let unit = match runtime.block_on(async { model.get(&cond).await }) {
        Err(e) => return Err(format!("model.get() none error: {}", e)),
        Ok(unit) => match unit {
            None => return Err("should get none one".to_string()),
            Some(unit) => unit,
        },
    };
    expect(unit.unit_id).to_equal("unit_id_get_none".to_string())?;
    expect(unit.code).to_equal("code_get_none".to_string())?;
    expect(unit.created_at).to_equal(now)?;
    expect(unit.modified_at).to_equal(now)?;
    expect(unit.owner_id).to_equal("owner_id_get_none".to_string())?;
    expect(unit.member_ids.len()).to_equal(0)?;
    expect(unit.name).to_equal("name_get".to_string())?;
    expect(unit.info).to_equal(Map::<String, Value>::new())?;

    let sql = match SqlBuilder::insert_into(TABLE_NAME)
        .fields(FIELDS)
        .values(&vec![
            quote("unit_id_get_some"),
            quote("code_get_some"),
            now.timestamp_millis().to_string(),
            now.timestamp_millis().to_string(),
            quote("owner_id_get_some".to_string()),
            quote("member_id1 member_id2"),
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
        unit_id: Some("unit_id_get_some"),
        ..Default::default()
    };
    let unit = match runtime.block_on(async { model.get(&cond).await }) {
        Err(e) => return Err(format!("model.get() some error: {}", e)),
        Ok(unit) => match unit {
            None => return Err("should get some one".to_string()),
            Some(unit) => unit,
        },
    };
    expect(unit.unit_id).to_equal("unit_id_get_some".to_string())?;
    expect(unit.code).to_equal("code_get_some".to_string())?;
    expect(unit.created_at).to_equal(now)?;
    expect(unit.modified_at).to_equal(now)?;
    expect(unit.owner_id).to_equal("owner_id_get_some".to_string())?;
    expect(unit.member_ids.len()).to_equal(2)?;
    expect(unit.name).to_equal("name_get".to_string())?;

    match unit.info.get("boolean") {
        Some(Value::Bool(v)) => match *v {
            false => (),
            true => return Err("wrong info.boolean value".to_string()),
        },
        _ => return Err("wrong info.boolean type".to_string()),
    }
    match unit.info.get("string") {
        Some(Value::String(v)) => match v.as_str() {
            "string" => (),
            _ => return Err("wrong info.string value".to_string()),
        },
        _ => return Err("wrong info.string type".to_string()),
    }
    match unit.info.get("number") {
        Some(Value::Number(v)) => match v.as_i64() {
            Some(1) => (),
            _ => return Err("wrong info.number value".to_string()),
        },
        _ => return Err("wrong info.number type".to_string()),
    }
    match unit.info.get("object") {
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

/// Test `get()` by specifying a unit code.
pub fn get_by_code(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let conn = state.sqlite.as_ref().unwrap().get_connection();
    let model = state.sqlite.as_ref().unwrap().unit();

    let now = Utc::now().trunc_subsecs(3);
    let sql = match SqlBuilder::insert_into(TABLE_NAME)
        .fields(FIELDS)
        .values(&vec![
            quote("unit_id_get"),
            quote("code_get"),
            now.timestamp_millis().to_string(),
            now.timestamp_millis().to_string(),
            quote("owner_id_get"),
            quote(""),
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
        Ok(unit) => match unit {
            None => (),
            Some(_) => return Err(format!("should not get not-exist one")),
        },
    };

    let cond = QueryCond {
        code: Some("code_get"),
        ..Default::default()
    };
    let unit = match runtime.block_on(async { model.get(&cond).await }) {
        Err(e) => return Err(format!("model.get() error: {}", e)),
        Ok(unit) => match unit {
            None => return Err("should get one".to_string()),
            Some(unit) => unit,
        },
    };
    expect(unit.unit_id).to_equal("unit_id_get".to_string())?;
    expect(unit.code).to_equal("code_get".to_string())?;
    expect(unit.created_at).to_equal(now)?;
    expect(unit.modified_at).to_equal(now)?;
    expect(unit.owner_id).to_equal("owner_id_get".to_string())?;
    expect(unit.member_ids.len()).to_equal(0)?;
    expect(unit.name).to_equal("name_get".to_string())?;
    expect(unit.info).to_equal(Map::<String, Value>::new())
}

/// Test `get()` by specifying a pair of owner ID and unit ID.
pub fn get_by_owner_unit(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let conn = state.sqlite.as_ref().unwrap().get_connection();
    let model = state.sqlite.as_ref().unwrap().unit();

    let now = Utc::now().trunc_subsecs(3);
    let unit_id_get = "unit_id_get";
    let unit_id_not_get = "unit_id_not_get";
    let unit_id_get_other = "unit_id_get_other";
    let sql = match SqlBuilder::insert_into(TABLE_NAME)
        .fields(FIELDS)
        .values(&vec![
            quote(unit_id_get),
            quote("code_get"),
            now.timestamp_millis().to_string(),
            now.timestamp_millis().to_string(),
            quote("owner_id_get"),
            quote(""),
            quote("name_get"),
            quote("{}"),
        ])
        .sql()
    {
        Err(e) => return Err(format!("sql() error: {}", e.to_string())),
        Ok(sql) => sql,
    };
    if let Err(e) = runtime.block_on(async { sqlx::query(&sql).execute(conn).await }) {
        return Err(format!("insert_into() get error: {}", e.to_string()));
    }
    let sql = match SqlBuilder::insert_into(TABLE_NAME)
        .fields(FIELDS)
        .values(&vec![
            quote(unit_id_not_get),
            quote("code_not_get"),
            now.timestamp_millis().to_string(),
            now.timestamp_millis().to_string(),
            quote("owner_id_get"),
            quote(""),
            quote("name_get"),
            quote("{}"),
        ])
        .sql()
    {
        Err(e) => return Err(format!("sql() error: {}", e.to_string())),
        Ok(sql) => sql,
    };
    if let Err(e) = runtime.block_on(async { sqlx::query(&sql).execute(conn).await }) {
        return Err(format!("insert_into() not-get error: {}", e.to_string()));
    }
    let sql = match SqlBuilder::insert_into(TABLE_NAME)
        .fields(FIELDS)
        .values(&vec![
            quote(unit_id_get_other),
            quote("code_get_other"),
            now.timestamp_millis().to_string(),
            now.timestamp_millis().to_string(),
            quote("owner_id_get_other".to_string()),
            quote(""),
            quote("name_get"),
            quote("{}"),
        ])
        .sql()
    {
        Err(e) => return Err(format!("sql() error: {}", e.to_string())),
        Ok(sql) => sql,
    };
    if let Err(e) = runtime.block_on(async { sqlx::query(&sql).execute(conn).await }) {
        return Err(format!("insert_into() other error: {}", e.to_string()));
    }

    let cond = QueryCond {
        unit_id: Some(unit_id_get),
        owner_id: Some("owner_id_get"),
        ..Default::default()
    };
    let unit = match runtime.block_on(async { model.get(&cond).await }) {
        Err(e) => return Err(format!("model.get() error: {}", e)),
        Ok(unit) => match unit {
            None => return Err("should get one".to_string()),
            Some(unit) => unit,
        },
    };
    if unit.unit_id.as_str() != unit_id_get {
        return Err("get wrong unit".to_string());
    }

    let cond = QueryCond {
        unit_id: Some(unit_id_get_other),
        owner_id: Some("owner_id_get"),
        ..Default::default()
    };
    match runtime.block_on(async { model.get(&cond).await }) {
        Err(e) => return Err(format!("model.get() error: {}", e)),
        Ok(unit) => match unit {
            None => (),
            Some(_) => return Err("should not get other one".to_string()),
        },
    };
    Ok(())
}

/// Test `get()` by specifying a pair of member ID and unit ID.
pub fn get_by_member_unit(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let conn = state.sqlite.as_ref().unwrap().get_connection();
    let model = state.sqlite.as_ref().unwrap().unit();

    let now = Utc::now().trunc_subsecs(3);
    let unit_id_get = "unit_id_get";
    let unit_id_not_get = "unit_id_not_get";
    let unit_id_get_other = "unit_id_get_other";
    let sql = match SqlBuilder::insert_into(TABLE_NAME)
        .fields(FIELDS)
        .values(&vec![
            quote(unit_id_get),
            quote("code_get"),
            now.timestamp_millis().to_string(),
            now.timestamp_millis().to_string(),
            quote("owner_id_get"),
            quote("member_id_get"),
            quote("name_get"),
            quote("{}"),
        ])
        .sql()
    {
        Err(e) => return Err(format!("sql() error: {}", e.to_string())),
        Ok(sql) => sql,
    };
    if let Err(e) = runtime.block_on(async { sqlx::query(&sql).execute(conn).await }) {
        return Err(format!("insert_into() get error: {}", e.to_string()));
    }
    let sql = match SqlBuilder::insert_into(TABLE_NAME)
        .fields(FIELDS)
        .values(&vec![
            quote(unit_id_not_get),
            quote("code_not_get"),
            now.timestamp_millis().to_string(),
            now.timestamp_millis().to_string(),
            quote("owner_id_get"),
            quote("member_id_get"),
            quote("name_get"),
            quote("{}"),
        ])
        .sql()
    {
        Err(e) => return Err(format!("sql() error: {}", e.to_string())),
        Ok(sql) => sql,
    };
    if let Err(e) = runtime.block_on(async { sqlx::query(&sql).execute(conn).await }) {
        return Err(format!("insert_into() not-get error: {}", e.to_string()));
    }
    let sql = match SqlBuilder::insert_into(TABLE_NAME)
        .fields(FIELDS)
        .values(&vec![
            quote(unit_id_get_other),
            quote("code_get_other"),
            now.timestamp_millis().to_string(),
            now.timestamp_millis().to_string(),
            quote("owner_id_get_other".to_string()),
            quote("member_id__get_other"),
            quote("name_get"),
            quote("{}"),
        ])
        .sql()
    {
        Err(e) => return Err(format!("sql() error: {}", e.to_string())),
        Ok(sql) => sql,
    };
    if let Err(e) = runtime.block_on(async { sqlx::query(&sql).execute(conn).await }) {
        return Err(format!("insert_into() other error: {}", e.to_string()));
    }

    let cond = QueryCond {
        unit_id: Some(unit_id_get),
        member_id: Some("member_id_get"),
        ..Default::default()
    };
    let unit = match runtime.block_on(async { model.get(&cond).await }) {
        Err(e) => return Err(format!("model.get() error: {}", e)),
        Ok(unit) => match unit {
            None => return Err("should get one".to_string()),
            Some(unit) => unit,
        },
    };
    if unit.unit_id.as_str() != unit_id_get {
        return Err("get wrong unit".to_string());
    }

    let cond = QueryCond {
        unit_id: Some(unit_id_get_other),
        member_id: Some("member_id_get"),
        ..Default::default()
    };
    match runtime.block_on(async { model.get(&cond).await }) {
        Err(e) => return Err(format!("model.get() error: {}", e)),
        Ok(unit) => match unit {
            None => (),
            Some(_) => return Err("should not get other one".to_string()),
        },
    };
    Ok(())
}

/// Test `add()`.
pub fn add(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().unit();

    common_test::add(runtime, model)
}

/// Test `add()` with duplicate key.
pub fn add_dup(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().unit();

    common_test::add_dup(runtime, model)
}

/// Test `del()` by specifying a unit ID.
pub fn del_by_unit_id(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().unit();

    common_test::del_by_unit_id(runtime, model)
}

/// Test `del()` twice.
pub fn del_twice(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().unit();

    common_test::del_twice(runtime, model)
}

/// Test `del()` by specifying a owner ID.
pub fn del_by_owner_id(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().unit();

    common_test::del_by_owner_id(runtime, model)
}

/// Test `del()` by specifying a pair of owner ID and unit ID.
pub fn del_by_owner_unit(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().unit();

    common_test::del_by_owner_unit(runtime, model)
}

/// Test `update()`.
pub fn update(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().unit();

    common_test::update(runtime, model)
}

/// Test `update()` with a non-exist condition.
pub fn update_not_exist(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().unit();

    common_test::update_not_exist(runtime, model)
}

/// Test `update()` with invalid update content.
pub fn update_invalid(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().unit();

    common_test::update_invalid(runtime, model)
}

/// Test `count()`.
pub fn count(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().unit();

    common_test::count(runtime, model)
}

/// Test `list()`.
pub fn list(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().unit();

    common_test::list(runtime, model)
}

/// Test `list()` with sorting.
pub fn list_sort(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().unit();

    common_test::list_sort(runtime, model)
}

/// Test `list()` with offset/limit.
pub fn list_offset_limit(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().unit();

    common_test::list_offset_limit(runtime, model)
}

/// Test `list()` with cursors.
pub fn list_cursor(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().unit();

    common_test::list_cursor(runtime, model)
}
