use std::collections::HashMap;

use chrono::{SubsecRound, Utc};
use laboratory::{SpecContext, expect};
use sql_builder::{SqlBuilder, quote};

use sylvia_iot_auth::models::Model;

use super::{super::common::refresh_token as common_test, STATE, TestState};

const TABLE_NAME: &'static str = "refresh_token";
const FIELDS: &'static [&'static str] = &[
    "refresh_token",
    "expires_at",
    "scope",
    "client_id",
    "redirect_uri",
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
    let model = state.sqlite.as_ref().unwrap().refresh_token();

    let result = runtime.block_on(async { model.init().await });
    expect(result.is_ok()).to_equal(true)
}

/// Test `get()`.
pub fn get(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let conn = state.sqlite.as_ref().unwrap().get_connection();
    let model = state.sqlite.as_ref().unwrap().refresh_token();

    let now = Utc::now().trunc_subsecs(3);
    let sql = match SqlBuilder::insert_into(TABLE_NAME)
        .fields(FIELDS)
        .values(&vec![
            quote("token_get_none"),
            now.timestamp_millis().to_string(),
            "NULL".to_string(),
            quote("client_id_get"),
            quote("redirect_uri_get"),
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

    match runtime.block_on(async { model.get("token_get_not_exist").await }) {
        Err(e) => return Err(format!("model.get() not-exist error: {}", e)),
        Ok(token) => match token {
            None => (),
            Some(_) => return Err(format!("should not get not-exist one")),
        },
    };

    let token = match runtime.block_on(async { model.get("token_get_none").await }) {
        Err(e) => return Err(format!("model.get() none error: {}", e)),
        Ok(token) => match token {
            None => return Err("should get none one".to_string()),
            Some(token) => token,
        },
    };
    expect(token.refresh_token).to_equal("token_get_none".to_string())?;
    expect(token.expires_at).to_equal(now)?;
    expect(token.scope).to_equal(None)?;
    expect(token.client_id).to_equal("client_id_get".to_string())?;
    expect(token.redirect_uri).to_equal("redirect_uri_get".to_string())?;
    expect(token.user_id).to_equal("user_id_get".to_string())?;

    let sql = match SqlBuilder::insert_into(TABLE_NAME)
        .fields(FIELDS)
        .values(&vec![
            quote("token_get_some"),
            now.timestamp_millis().to_string(),
            quote("scope_get"),
            quote("client_id_get"),
            quote("redirect_uri_get"),
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

    let token = match runtime.block_on(async { model.get("token_get_some").await }) {
        Err(e) => return Err(format!("model.get() some error: {}", e)),
        Ok(token) => match token {
            None => return Err("should get some one".to_string()),
            Some(token) => token,
        },
    };
    expect(token.refresh_token).to_equal("token_get_some".to_string())?;
    expect(token.expires_at).to_equal(now)?;
    expect(token.scope).to_equal(Some("scope_get".to_string()))?;
    expect(token.client_id).to_equal("client_id_get".to_string())?;
    expect(token.redirect_uri).to_equal("redirect_uri_get".to_string())?;
    expect(token.user_id).to_equal("user_id_get".to_string())
}

/// Test `add()`.
pub fn add(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().refresh_token();

    common_test::add(runtime, model)
}

/// Test `add()` with duplicate key.
pub fn add_dup(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().refresh_token();

    common_test::add_dup(runtime, model)
}

/// Test `del()` by specifying a refresh token.
pub fn del_by_token(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().refresh_token();

    common_test::del_by_token(runtime, model)
}

/// Test `del()` twice.
pub fn del_twice(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().refresh_token();

    common_test::del_twice(runtime, model)
}

/// Test `del()` by specifying a client ID.
pub fn del_by_client_id(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().refresh_token();

    common_test::del_by_client_id(runtime, model)
}

/// Test `del()` by specifying a user ID.
pub fn del_by_user_id(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().refresh_token();

    common_test::del_by_user_id(runtime, model)
}

/// Test `del()` by specifying a pair of user ID and client ID.
pub fn del_by_user_client(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().refresh_token();

    common_test::del_by_user_client(runtime, model)
}
