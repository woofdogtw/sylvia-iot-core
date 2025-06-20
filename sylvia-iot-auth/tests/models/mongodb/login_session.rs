use std::collections::HashMap;

use chrono::{SubsecRound, Utc};
use laboratory::{SpecContext, expect};
use mongodb::bson::{DateTime, Document};
use serde::{Deserialize, Serialize};

use sylvia_iot_auth::models::Model;

use super::{super::common::login_session as common_test, STATE, TestState};

#[derive(Debug, Deserialize, Serialize)]
struct Schema {
    #[serde(rename = "sessionId")]
    session_id: String,
    #[serde(rename = "expiresAt")]
    expires_at: DateTime,
    #[serde(rename = "userId")]
    user_id: String,
    #[serde(rename = "createdAt")]
    created_at: DateTime,
}

const COL_NAME: &'static str = "loginSession";

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
    let model = state.mongodb.as_ref().unwrap().login_session();

    let result = runtime.block_on(async { model.init().await });
    expect(result.is_ok()).to_equal(true)
}

/// Test `get()`.
pub fn get(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let conn = state.mongodb.as_ref().unwrap().get_connection();
    let model = state.mongodb.as_ref().unwrap().login_session();

    let now = Utc::now().trunc_subsecs(3);
    let item = Schema {
        session_id: "session_id_get_none".to_string(),
        expires_at: now.into(),
        user_id: "user_id_get".to_string(),
        created_at: now.into(),
    };
    if let Err(e) =
        runtime.block_on(async { conn.collection::<Schema>(COL_NAME).insert_one(item).await })
    {
        return Err(format!("insert_one() none error: {}", e));
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

    let item = Schema {
        session_id: "session_id_get_some".to_string(),
        expires_at: now.into(),
        user_id: "user_id_get".to_string(),
        created_at: now.into(),
    };
    if let Err(e) =
        runtime.block_on(async { conn.collection::<Schema>(COL_NAME).insert_one(item).await })
    {
        return Err(format!("insert_one() some error: {}", e));
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
    let model = state.mongodb.as_ref().unwrap().login_session();

    common_test::add(runtime, model)
}

/// Test `add()` with duplicate key.
pub fn add_dup(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().login_session();

    common_test::add_dup(runtime, model)
}

/// Test `del()` by specifying a session ID.
pub fn del_by_session(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().login_session();

    common_test::del_by_session(runtime, model)
}

/// Test `del()` twice.
pub fn del_twice(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().login_session();

    common_test::del_twice(runtime, model)
}

/// Test `del()` by specifying a user ID.
pub fn del_by_user_id(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().login_session();

    common_test::del_by_user_id(runtime, model)
}
