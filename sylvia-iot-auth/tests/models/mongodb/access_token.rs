use std::collections::HashMap;

use chrono::{SubsecRound, Utc};
use laboratory::{expect, SpecContext};
use mongodb::bson::{DateTime, Document};
use serde::{Deserialize, Serialize};

use sylvia_iot_auth::models::Model;

use super::{super::common::access_token as common_test, TestState, STATE};

#[derive(Debug, Deserialize, Serialize)]
struct Schema {
    #[serde(rename = "accessToken")]
    access_token: String,
    #[serde(rename = "refreshToken", skip_serializing_if = "Option::is_none")]
    refresh_token: Option<String>,
    #[serde(rename = "expiresAt")]
    expires_at: DateTime,
    scope: Option<String>,
    #[serde(rename = "clientId")]
    client_id: String,
    #[serde(rename = "redirectUri")]
    redirect_uri: String,
    #[serde(rename = "userId")]
    user_id: String,
    #[serde(rename = "createdAt")]
    created_at: DateTime,
}

const COL_NAME: &'static str = "accessToken";

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
    let model = state.mongodb.as_ref().unwrap().access_token();

    let result = runtime.block_on(async { model.init().await });
    expect(result.is_ok()).to_equal(true)
}

/// Test `get()`.
pub fn get(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let conn = state.mongodb.as_ref().unwrap().get_connection();
    let model = state.mongodb.as_ref().unwrap().access_token();

    let now = Utc::now().trunc_subsecs(3);
    let item = Schema {
        access_token: "token_get_none".to_string(),
        refresh_token: None,
        expires_at: now.into(),
        scope: None,
        client_id: "client_id_get".to_string(),
        redirect_uri: "redirect_uri_get".to_string(),
        user_id: "user_id_get".to_string(),
        created_at: now.into(),
    };
    if let Err(e) =
        runtime.block_on(async { conn.collection::<Schema>(COL_NAME).insert_one(item).await })
    {
        return Err(format!("insert_one() none error: {}", e));
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
    expect(token.access_token).to_equal("token_get_none".to_string())?;
    expect(token.refresh_token).to_equal(None)?;
    expect(token.expires_at).to_equal(now)?;
    expect(token.scope).to_equal(None)?;
    expect(token.client_id).to_equal("client_id_get".to_string())?;
    expect(token.redirect_uri).to_equal("redirect_uri_get".to_string())?;
    expect(token.user_id).to_equal("user_id_get".to_string())?;

    let item = Schema {
        access_token: "token_get_some".to_string(),
        refresh_token: Some("token_get".to_string()),
        expires_at: now.into(),
        scope: Some("scope_get".to_string()),
        client_id: "client_id_get".to_string(),
        redirect_uri: "redirect_uri_get".to_string(),
        user_id: "user_id_get".to_string(),
        created_at: now.into(),
    };
    if let Err(e) =
        runtime.block_on(async { conn.collection::<Schema>(COL_NAME).insert_one(item).await })
    {
        return Err(format!("insert_one() some error: {}", e));
    }

    let token = match runtime.block_on(async { model.get("token_get_some").await }) {
        Err(e) => return Err(format!("model.get() some error: {}", e)),
        Ok(token) => match token {
            None => return Err("should get some one".to_string()),
            Some(token) => token,
        },
    };
    expect(token.access_token).to_equal("token_get_some".to_string())?;
    expect(token.refresh_token).to_equal(Some("token_get".to_string()))?;
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
    let model = state.mongodb.as_ref().unwrap().access_token();

    common_test::add(runtime, model)
}

/// Test `add()` with duplicate key.
pub fn add_dup(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().access_token();

    common_test::add_dup(runtime, model)
}

/// Test `del()` by specifying an access token.
pub fn del_by_access_token(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().access_token();

    common_test::del_by_access_token(runtime, model)
}

/// Test `del()` by specifying a refresh token.
pub fn del_by_refresh_token(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().access_token();

    common_test::del_by_refresh_token(runtime, model)
}

/// Test `del()` twice.
pub fn del_twice(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().access_token();

    common_test::del_twice(runtime, model)
}

/// Test `del()` by specifying a client ID.
pub fn del_by_client_id(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().access_token();

    common_test::del_by_client_id(runtime, model)
}

/// Test `del()` by specifying a user ID.
pub fn del_by_user_id(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().access_token();

    common_test::del_by_user_id(runtime, model)
}

/// Test `del()` by specifying a pair of user ID and client ID.
pub fn del_by_user_client(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().access_token();

    common_test::del_by_user_client(runtime, model)
}
