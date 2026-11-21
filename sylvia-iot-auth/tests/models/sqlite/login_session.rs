use std::collections::HashMap;

use chrono::{SubsecRound, TimeDelta, Utc};
use laboratory::{SpecContext, expect};
use sql_builder::{SqlBuilder, quote};
use sqlx::AssertSqlSafe;

use sylvia_iot_auth::models::{Model, login_session::LoginSession};

use super::{super::common::login_session as common_test, STATE, TestState};

const TABLE_NAME: &'static str = "login_session";
const FIELDS: &'static [&'static str] = &[
    "session_id",
    "expires_at",
    "user_id",
    "client_id",
    "redirect_uri",
];

pub fn after_each_fn(state: &mut HashMap<&'static str, TestState>) -> () {
    let state = state.get_mut(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let conn = state.sqlite.as_ref().unwrap().get_connection();
    let sql = SqlBuilder::delete_from(TABLE_NAME).sql().unwrap();
    let _ = runtime.block_on(async { sqlx::query(AssertSqlSafe(sql)).execute(conn).await });
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
            quote("client_id_get"),
            quote("http://localhost/redirect"),
        ])
        .sql()
    {
        Err(e) => return Err(format!("sql() none error: {}", e.to_string())),
        Ok(sql) => sql,
    };
    if let Err(e) = runtime.block_on(async { sqlx::query(AssertSqlSafe(sql)).execute(conn).await })
    {
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
    expect(session.client_id).to_equal("client_id_get".to_string())?;
    expect(session.redirect_uri).to_equal("http://localhost/redirect".to_string())?;

    let sql = match SqlBuilder::insert_into(TABLE_NAME)
        .fields(FIELDS)
        .values(&vec![
            quote("session_id_get_some"),
            now.timestamp_millis().to_string(),
            quote("user_id_get"),
            quote("client_id_get"),
            quote("http://localhost/redirect"),
        ])
        .sql()
    {
        Err(e) => return Err(format!("sql() some error: {}", e.to_string())),
        Ok(sql) => sql,
    };
    if let Err(e) = runtime.block_on(async { sqlx::query(AssertSqlSafe(sql)).execute(conn).await })
    {
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
    expect(session.user_id).to_equal("user_id_get".to_string())?;
    expect(session.client_id).to_equal("client_id_get".to_string())?;
    expect(session.redirect_uri).to_equal("http://localhost/redirect".to_string())
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

/// Test `add()` that removes expired sessions.
pub fn add_remove_expired(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().login_session();

    let mut session = LoginSession {
        session_id: "session_id_expired".to_string(),
        expires_at: Utc::now().trunc_subsecs(3) - TimeDelta::try_hours(1).unwrap(),
        user_id: "user_id".to_string(),
        client_id: "client_id".to_string(),
        redirect_uri: "http://localhost/redirect".to_string(),
    };
    if let Err(e) = runtime.block_on(async {
        model.add(&session).await?;
        session.session_id = "session_id_valid".to_string();
        session.expires_at = Utc::now().trunc_subsecs(3) + TimeDelta::try_hours(1).unwrap();
        model.add(&session).await
    }) {
        return Err(format!("model.add() error: {}", e));
    }
    match runtime.block_on(async { model.get("session_id_expired").await }) {
        Err(e) => return Err(format!("model.get() expired error: {}", e)),
        Ok(get_session) => expect(get_session.is_none()).to_equal(true)?,
    }
    match runtime.block_on(async { model.get("session_id_valid").await }) {
        Err(e) => Err(format!("model.get() valid error: {}", e)),
        Ok(get_session) => expect(get_session.is_some()).to_equal(true),
    }
}
