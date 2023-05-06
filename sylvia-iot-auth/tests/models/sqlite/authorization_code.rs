use std::collections::HashMap;

use chrono::{SubsecRound, Utc};
use laboratory::{expect, SpecContext};
use sql_builder::{quote, SqlBuilder};

use sylvia_iot_auth::models::Model;

use super::{super::common::authorization_code as common_test, TestState, STATE};

const TABLE_NAME: &'static str = "authorization_code";
const FIELDS: &'static [&'static str] = &[
    "code",
    "expires_at",
    "redirect_uri",
    "scope",
    "client_id",
    "user_id",
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
    let model = state.sqlite.as_ref().unwrap().authorization_code();

    let result = runtime.block_on(async { model.init().await });
    expect(result.is_ok()).to_equal(true)
}

/// Test `get()`.
pub fn get(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let conn = state.sqlite.as_ref().unwrap().get_connection();
    let model = state.sqlite.as_ref().unwrap().authorization_code();

    let now = Utc::now().trunc_subsecs(3);
    let sql = match SqlBuilder::insert_into(TABLE_NAME)
        .fields(FIELDS)
        .values(&vec![
            quote("code_get_none"),
            now.timestamp_millis().to_string(),
            quote("redirect_uri_get"),
            "NULL".to_string(),
            quote("client_id_get"),
            quote("user_id_get"),
        ])
        .sql()
    {
        Err(e) => return Err(format!("sql() none error: {}", e.to_string())),
        Ok(sql) => sql,
    };
    if let Err(e) = runtime.block_on(async { sqlx::query(&sql).execute(conn).await }) {
        return Err(format!("insert_into() none error: {}", e.to_string()));
    }

    match runtime.block_on(async { model.get("code_get_not_exist").await }) {
        Err(e) => return Err(format!("model.get() not-exist error: {}", e)),
        Ok(code) => match code {
            None => (),
            Some(_) => return Err(format!("should not get not-exist one")),
        },
    };

    let code = match runtime.block_on(async { model.get("code_get_none").await }) {
        Err(e) => return Err(format!("model.get() none error: {}", e)),
        Ok(code) => match code {
            None => return Err("should get none one".to_string()),
            Some(code) => code,
        },
    };
    expect(code.code).to_equal("code_get_none".to_string())?;
    expect(code.expires_at).to_equal(now)?;
    expect(code.redirect_uri).to_equal("redirect_uri_get".to_string())?;
    expect(code.scope).to_equal(None)?;
    expect(code.client_id).to_equal("client_id_get".to_string())?;
    expect(code.user_id).to_equal("user_id_get".to_string())?;

    let sql = match SqlBuilder::insert_into(TABLE_NAME)
        .fields(FIELDS)
        .values(&vec![
            quote("code_get_some"),
            now.timestamp_millis().to_string(),
            quote("redirect_uri_get"),
            quote("scope_get"),
            quote("client_id_get"),
            quote("user_id_get"),
        ])
        .sql()
    {
        Err(e) => return Err(format!("sql() some error: {}", e.to_string())),
        Ok(sql) => sql,
    };
    if let Err(e) = runtime.block_on(async { sqlx::query(&sql).execute(conn).await }) {
        return Err(format!("insert_into() some error: {}", e.to_string()));
    }

    let code = match runtime.block_on(async { model.get("code_get_some").await }) {
        Err(e) => return Err(format!("model.get() some error: {}", e)),
        Ok(code) => match code {
            None => return Err("should get some one".to_string()),
            Some(code) => code,
        },
    };
    expect(code.code).to_equal("code_get_some".to_string())?;
    expect(code.expires_at).to_equal(now)?;
    expect(code.redirect_uri).to_equal("redirect_uri_get".to_string())?;
    expect(code.scope).to_equal(Some("scope_get".to_string()))?;
    expect(code.client_id).to_equal("client_id_get".to_string())?;
    expect(code.user_id).to_equal("user_id_get".to_string())
}

/// Test `add()`.
pub fn add(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().authorization_code();

    common_test::add(runtime, model)
}

/// Test `add()` with duplicate key.
pub fn add_dup(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().authorization_code();

    common_test::add_dup(runtime, model)
}

/// Test `del()` by specifying an authorization code.
pub fn del_by_code(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().authorization_code();

    common_test::del_by_code(runtime, model)
}

/// Test `del()` twice.
pub fn del_twice(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().authorization_code();

    common_test::del_twice(runtime, model)
}

/// Test `del()` by specifying a client ID.
pub fn del_by_client_id(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().authorization_code();

    common_test::del_by_client_id(runtime, model)
}

/// Test `del()` by specifying a user ID.
pub fn del_by_user_id(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().authorization_code();

    common_test::del_by_user_id(runtime, model)
}

/// Test `del()` by specifying a pair of user ID and client ID.
pub fn del_by_user_client(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().authorization_code();

    common_test::del_by_user_client(runtime, model)
}
