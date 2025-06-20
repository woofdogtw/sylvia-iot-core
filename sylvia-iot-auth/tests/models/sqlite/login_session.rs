use std::collections::HashMap;

use chrono::{SubsecRound, Utc};
use laboratory::{SpecContext, expect};
use sql_builder::{SqlBuilder, quote};

use sylvia_iot_auth::models::Model;

use super::{super::common::login_session as common_test, STATE, TestState};

const TABLE_NAME: &'static str = "login_session";
const FIELDS: &'static [&'static str] = &["session_id", "expires_at", "user_id"];

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
    let model = state.sqlite.as_ref().unwrap().login_session();

    let result = runtime.block_on(async { model.init().await });
    expect(result.is_ok()).to_equal(true)
}

/// Test `get()`.
pub fn get(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let conn = state.sqlite.as_ref().unwrap().get_connection();
    let model = state.sqlite.as_ref().unwrap().login_session();

    let now = Utc::now().trunc_subsecs(3);
    let sql = match SqlBuilder::insert_into(TABLE_NAME)
        .fields(FIELDS)
        .values(&vec![
            quote("session_id_get_none"),
            now.timestamp_millis().to_string(),
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

    match runtime.block_on(async { model.get("session_id_get_not_exist").await }) {
        Err(e) => return Err(format!("model.get() not-exist error: {}", e)),
        Ok(session) => match session {
            None => (),
            Some(_) => return Err(format!("should not get not-exist one")),
        },
    };

    let session = match runtime.block_on(async { model.get("session_id_get_none").await }) {
        Err(e) => return Err(format!("model.get() none error: {}", e)),
        Ok(session) => match session {
            None => return Err("should get none one".to_string()),
            Some(session) => session,
        },
    };
    expect(session.session_id).to_equal("session_id_get_none".to_string())?;
    expect(session.expires_at).to_equal(now)?;
    expect(session.user_id).to_equal("user_id_get".to_string())?;

    let sql = match SqlBuilder::insert_into(TABLE_NAME)
        .fields(FIELDS)
        .values(&vec![
            quote("session_id_get_some"),
            now.timestamp_millis().to_string(),
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

    let session = match runtime.block_on(async { model.get("session_id_get_some").await }) {
        Err(e) => return Err(format!("model.get() some error: {}", e)),
        Ok(session) => match session {
            None => return Err("should get some one".to_string()),
            Some(session) => session,
        },
    };
    expect(session.session_id).to_equal("session_id_get_some".to_string())?;
    expect(session.expires_at).to_equal(now)?;
    expect(session.user_id).to_equal("user_id_get".to_string())
}

/// Test `add()`.
pub fn add(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().login_session();

    common_test::add(runtime, model)
}

/// Test `add()` with duplicate key.
pub fn add_dup(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().login_session();

    common_test::add_dup(runtime, model)
}

/// Test `del()` by specifying a session ID.
pub fn del_by_session(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().login_session();

    common_test::del_by_session(runtime, model)
}

/// Test `del()` twice.
pub fn del_twice(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().login_session();

    common_test::del_twice(runtime, model)
}

/// Test `del()` by specifying a user ID.
pub fn del_by_user_id(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().login_session();

    common_test::del_by_user_id(runtime, model)
}
