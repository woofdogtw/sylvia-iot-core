use std::collections::HashMap;

use chrono::{SubsecRound, Utc};
use laboratory::{SpecContext, expect};
use serde_json::{Map, Value};
use sql_builder::{SqlBuilder, quote};

use sylvia_iot_auth::models::{Model, user::QueryCond};

use super::{super::common::user as common_test, STATE, TestState};

const TABLE_NAME: &'static str = "user";
const FIELDS: &'static [&'static str] = &[
    "user_id",
    "account",
    "created_at",
    "modified_at",
    "verified_at",
    "expired_at",
    "disabled_at",
    "roles",
    "password",
    "salt",
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
    let model = state.sqlite.as_ref().unwrap().user();

    let result = runtime.block_on(async { model.init().await });
    expect(result.is_ok()).to_equal(true)
}

/// Test `get()` by specifying a user ID.
pub fn get_by_user_id(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let conn = state.sqlite.as_ref().unwrap().get_connection();
    let model = state.sqlite.as_ref().unwrap().user();

    let now = Utc::now().trunc_subsecs(3);
    let sql = match SqlBuilder::insert_into(TABLE_NAME)
        .fields(FIELDS)
        .values(&vec![
            quote("user_id_get_none"),
            quote("account_get_none"),
            now.timestamp_millis().to_string(),
            now.timestamp_millis().to_string(),
            "NULL".to_string(),
            "NULL".to_string(),
            "NULL".to_string(),
            quote("{}"),
            quote("password_get"),
            quote("salt_get"),
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
        user_id: Some("user_id_not_exist"),
        ..Default::default()
    };
    match runtime.block_on(async { model.get(&cond).await }) {
        Err(e) => return Err(format!("model.get() error: {}", e)),
        Ok(user) => match user {
            None => (),
            Some(_) => return Err(format!("should not get not-exist one")),
        },
    };

    let cond = QueryCond {
        user_id: Some("user_id_get_none"),
        ..Default::default()
    };
    let user = match runtime.block_on(async { model.get(&cond).await }) {
        Err(e) => return Err(format!("model.get() none error: {}", e)),
        Ok(user) => match user {
            None => return Err("should get none one".to_string()),
            Some(user) => user,
        },
    };
    expect(user.user_id).to_equal("user_id_get_none".to_string())?;
    expect(user.account).to_equal("account_get_none".to_string())?;
    expect(user.created_at).to_equal(now)?;
    expect(user.modified_at).to_equal(now)?;
    expect(user.verified_at).to_equal(None)?;
    expect(user.expired_at).to_equal(None)?;
    expect(user.disabled_at).to_equal(None)?;
    expect(user.roles).to_equal(HashMap::<String, bool>::new())?;
    expect(user.password).to_equal("password_get".to_string())?;
    expect(user.salt).to_equal("salt_get".to_string())?;
    expect(user.name).to_equal("".to_string())?;
    expect(user.info).to_equal(Map::<String, Value>::new())?;

    let mut roles = HashMap::<String, bool>::new();
    roles.insert("role1".to_string(), true);
    roles.insert("role2".to_string(), false);
    let sql = match SqlBuilder::insert_into(TABLE_NAME)
        .fields(FIELDS)
        .values(&vec![
            quote("user_id_get_some"),
            quote("account_get_some"),
            now.timestamp_millis().to_string(),
            now.timestamp_millis().to_string(),
            now.timestamp_millis().to_string(),
            now.timestamp_millis().to_string(),
            now.timestamp_millis().to_string(),
            quote(serde_json::to_string(&roles).unwrap()),
            quote("password_get"),
            quote("salt_get"),
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
        user_id: Some("user_id_get_some"),
        ..Default::default()
    };
    let user = match runtime.block_on(async { model.get(&cond).await }) {
        Err(e) => return Err(format!("model.get() some error: {}", e)),
        Ok(user) => match user {
            None => return Err("should get some one".to_string()),
            Some(user) => user,
        },
    };
    expect(user.user_id).to_equal("user_id_get_some".to_string())?;
    expect(user.account).to_equal("account_get_some".to_string())?;
    expect(user.created_at).to_equal(now)?;
    expect(user.modified_at).to_equal(now)?;
    expect(user.verified_at).to_equal(Some(now))?;
    expect(user.expired_at).to_equal(Some(now))?;
    expect(user.disabled_at).to_equal(Some(now))?;
    expect(user.roles).to_equal(roles)?;
    expect(user.password).to_equal("password_get".to_string())?;
    expect(user.salt).to_equal("salt_get".to_string())?;
    expect(user.name).to_equal("name_get".to_string())?;

    match user.info.get("boolean") {
        Some(Value::Bool(v)) => match *v {
            false => (),
            true => return Err("wrong info.boolean value".to_string()),
        },
        _ => return Err("wrong info.boolean type".to_string()),
    }
    match user.info.get("string") {
        Some(Value::String(v)) => match v.as_str() {
            "string" => (),
            _ => return Err("wrong info.string value".to_string()),
        },
        _ => return Err("wrong info.string type".to_string()),
    }
    match user.info.get("number") {
        Some(Value::Number(v)) => match v.as_i64() {
            Some(1) => (),
            _ => return Err("wrong info.number value".to_string()),
        },
        _ => return Err("wrong info.number type".to_string()),
    }
    match user.info.get("object") {
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

/// Test `get()` by specifying an account.
pub fn get_by_account(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let conn = state.sqlite.as_ref().unwrap().get_connection();
    let model = state.sqlite.as_ref().unwrap().user();

    let now = Utc::now().trunc_subsecs(3);
    let sql = match SqlBuilder::insert_into(TABLE_NAME)
        .fields(FIELDS)
        .values(&vec![
            quote("user_id_get"),
            quote("account_get"),
            now.timestamp_millis().to_string(),
            now.timestamp_millis().to_string(),
            "NULL".to_string(),
            "NULL".to_string(),
            "NULL".to_string(),
            quote("{}"),
            quote("password_get"),
            quote("salt_get"),
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
        account: Some("account_not_exist"),
        ..Default::default()
    };
    match runtime.block_on(async { model.get(&cond).await }) {
        Err(e) => return Err(format!("model.get() error: {}", e)),
        Ok(user) => match user {
            None => (),
            Some(_) => return Err(format!("should not get not-exist one")),
        },
    };

    let cond = QueryCond {
        account: Some("account_get"),
        ..Default::default()
    };
    let user = match runtime.block_on(async { model.get(&cond).await }) {
        Err(e) => return Err(format!("model.get() error: {}", e)),
        Ok(user) => match user {
            None => return Err("should get one".to_string()),
            Some(user) => user,
        },
    };
    expect(user.user_id).to_equal("user_id_get".to_string())?;
    expect(user.account).to_equal("account_get".to_string())?;
    expect(user.created_at).to_equal(now)?;
    expect(user.modified_at).to_equal(now)?;
    expect(user.verified_at).to_equal(None)?;
    expect(user.expired_at).to_equal(None)?;
    expect(user.disabled_at).to_equal(None)?;
    expect(user.roles).to_equal(HashMap::<String, bool>::new())?;
    expect(user.password).to_equal("password_get".to_string())?;
    expect(user.salt).to_equal("salt_get".to_string())?;
    expect(user.name).to_equal("name_get".to_string())?;
    expect(user.info).to_equal(Map::<String, Value>::new())
}

/// Test `add()`.
pub fn add(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().user();

    common_test::add(runtime, model)
}

/// Test `add()` with duplicate key.
pub fn add_dup(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().user();

    common_test::add_dup(runtime, model)
}

/// Test `del()`.
pub fn del(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().user();

    common_test::del(runtime, model)
}

/// Test `del()` twice.
pub fn del_twice(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().user();

    common_test::del_twice(runtime, model)
}

/// Test `update()`.
pub fn update(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().user();

    common_test::update(runtime, model)
}

/// Test `update()` with a non-exist condition.
pub fn update_not_exist(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().user();

    common_test::update_not_exist(runtime, model)
}

/// Test `update()` with invalid update content.
pub fn update_invalid(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().user();

    common_test::update_invalid(runtime, model)
}

/// Test `count()`.
pub fn count(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().user();

    common_test::count(runtime, model)
}

/// Test `list()`.
pub fn list(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().user();

    common_test::list(runtime, model)
}

/// Test `list()` with sorting.
pub fn list_sort(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().user();

    common_test::list_sort(runtime, model)
}

/// Test `list()` with offset/limit.
pub fn list_offset_limit(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().user();

    common_test::list_offset_limit(runtime, model)
}

/// Test `list()` with cursors.
pub fn list_cursor(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().user();

    common_test::list_cursor(runtime, model)
}
