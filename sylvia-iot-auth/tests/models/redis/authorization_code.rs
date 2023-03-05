use chrono::{SubsecRound, Utc};
use laboratory::{expect, SpecContext};
use redis::{AsyncCommands, RedisResult};
use serde::{Deserialize, Serialize};
use serde_json;

use sylvia_iot_auth::models::{
    authorization_code::{AuthorizationCode, QueryCond},
    redis::authorization_code,
};

use super::{TestState, STATE};

#[derive(Deserialize, Serialize)]
struct AuthorizationCodeSchema {
    #[serde(rename = "code")]
    code: String,
    #[serde(rename = "expiresAt")]
    expires_at: i64,
    #[serde(rename = "scope")]
    scope: Option<String>,
    #[serde(rename = "clientId")]
    client_id: String,
    #[serde(rename = "redirectUri")]
    redirect_uri: String,
    #[serde(rename = "userId")]
    user_id: String,
}

const PREFIX: &'static str = "auth:authorizationCode:";

/// Test get() with None scope.
pub fn get_none(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let mut state = context.state.borrow_mut();
    let state = state.get_mut(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let pool = state.redis.as_mut().unwrap();

    let now = Utc::now().trunc_subsecs(3);
    let item = AuthorizationCodeSchema {
        code: "code_get_none".to_string(),
        expires_at: now.timestamp_millis(),
        redirect_uri: "redirect_uri_get".to_string(),
        scope: None,
        client_id: "client_id_get".to_string(),
        user_id: "user_id_get".to_string(),
    };
    let item_str = match serde_json::to_string(&item) {
        Err(e) => {
            return Err(e.to_string());
        }
        Ok(str) => str,
    };
    if let Err(e) = runtime.block_on(async {
        let result: RedisResult<()> = pool
            .set(PREFIX.to_string() + item.code.as_str(), item_str)
            .await;
        result
    }) {
        return Err(format!("execute set error: {}", e.to_string()));
    }

    let code =
        match runtime.block_on(async { authorization_code::get(pool, "code_get_none").await }) {
            Err(e) => return Err(format!("get result error: {}", e.to_string())),
            Ok(code) => match code {
                None => return Err("empty authorization code".to_string()),
                Some(code) => code,
            },
        };
    expect(code.code).to_equal("code_get_none".to_string())?;
    expect(code.expires_at.into()).to_equal(now)?;
    expect(code.redirect_uri).to_equal("redirect_uri_get".to_string())?;
    expect(code.scope).to_equal(None)?;
    expect(code.client_id).to_equal("client_id_get".to_string())?;
    expect(code.user_id).to_equal("user_id_get".to_string())
}

/// Test get() with Some scope.
pub fn get_some(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let mut state = context.state.borrow_mut();
    let state = state.get_mut(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let pool = state.redis.as_mut().unwrap();

    let now = Utc::now().trunc_subsecs(3);
    let item = AuthorizationCodeSchema {
        code: "code_get_some".to_string(),
        expires_at: now.timestamp_millis(),
        redirect_uri: "redirect_uri_get".to_string(),
        scope: None,
        client_id: "client_id_get".to_string(),
        user_id: "user_id_get".to_string(),
    };
    let item_str = match serde_json::to_string(&item) {
        Err(e) => {
            return Err(e.to_string());
        }
        Ok(str) => str,
    };
    if let Err(e) = runtime.block_on(async {
        let result: RedisResult<()> = pool
            .set(PREFIX.to_string() + item.code.as_str(), item_str)
            .await;
        result
    }) {
        return Err(format!("execute set error: {}", e.to_string()));
    }

    let code =
        match runtime.block_on(async { authorization_code::get(pool, "code_get_some").await }) {
            Err(e) => return Err(format!("get result error: {}", e.to_string())),
            Ok(code) => match code {
                None => return Err("empty authorization code".to_string()),
                Some(code) => code,
            },
        };
    expect(code.code).to_equal("code_get_some".to_string())?;
    expect(code.expires_at.into()).to_equal(now)?;
    expect(code.redirect_uri).to_equal("redirect_uri_get".to_string())?;
    expect(code.scope).to_equal(None)?;
    expect(code.client_id).to_equal("client_id_get".to_string())?;
    expect(code.user_id).to_equal("user_id_get".to_string())
}

/// Test add() with None scope.
pub fn add_none(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let mut state = context.state.borrow_mut();
    let state = state.get_mut(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let pool = state.redis.as_mut().unwrap();

    let code = AuthorizationCode {
        code: "code_add_none".to_string(),
        expires_at: Utc::now().trunc_subsecs(3),
        redirect_uri: "redirect_uri_add".to_string(),
        scope: None,
        client_id: "client_id_add".to_string(),
        user_id: "user_id_add".to_string(),
    };
    if let Err(e) = runtime.block_on(async { authorization_code::add(pool, &code).await }) {
        return Err(format!("add error: {}", e.to_string()));
    }

    let get_code = match runtime.block_on(async { authorization_code::get(pool, &code.code).await })
    {
        Err(e) => return Err(format!("get result error: {}", e.to_string())),
        Ok(code) => match code {
            None => return Err("empty authorization code".to_string()),
            Some(code) => code,
        },
    };
    expect(get_code).to_equal(code)
}

/// Test add() with Some scope.
pub fn add_some(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let mut state = context.state.borrow_mut();
    let state = state.get_mut(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let pool = state.redis.as_mut().unwrap();

    let code = AuthorizationCode {
        code: "code_add_some".to_string(),
        expires_at: Utc::now().trunc_subsecs(3),
        redirect_uri: "redirect_uri_add".to_string(),
        scope: Some("scope_add".to_string()),
        client_id: "client_id_add".to_string(),
        user_id: "user_id_add".to_string(),
    };
    if let Err(e) = runtime.block_on(async { authorization_code::add(pool, &code).await }) {
        return Err(format!("add error: {}", e.to_string()));
    }

    let get_code = match runtime.block_on(async { authorization_code::get(pool, &code.code).await })
    {
        Err(e) => return Err(format!("get result error: {}", e.to_string())),
        Ok(code) => match code {
            None => return Err("empty authorization code".to_string()),
            Some(code) => code,
        },
    };
    expect(get_code).to_equal(code)
}

/// Test del().
pub fn del(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let mut state = context.state.borrow_mut();
    let state = state.get_mut(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let pool = state.redis.as_mut().unwrap();

    let code = AuthorizationCode {
        code: "code_del".to_string(),
        expires_at: Utc::now().trunc_subsecs(3),
        scope: Some("scope_del".to_string()),
        client_id: "client_id_del".to_string(),
        redirect_uri: "redirect_uri_del".to_string(),
        user_id: "user_id_del".to_string(),
    };
    let cond = QueryCond {
        code: Some(&code.code),
        ..Default::default()
    };
    if let Err(e) = runtime.block_on(async {
        authorization_code::add(pool, &code).await?;
        authorization_code::del(pool, &cond).await
    }) {
        return Err(format!("add/del error: {}", e.to_string()));
    }
    match runtime.block_on(async { authorization_code::get(pool, &code.code).await }) {
        Err(e) => return Err(e.to_string()),
        Ok(code) => match code {
            None => return Ok(()),
            Some(_) => return Err("delete code fail".to_string()),
        },
    };
}
