use chrono::{SubsecRound, Utc};
use laboratory::{expect, SpecContext};
use redis::{AsyncCommands, RedisResult};
use serde::{Deserialize, Serialize};
use serde_json;

use sylvia_iot_auth::models::{
    access_token::{AccessToken, QueryCond},
    redis::access_token,
};

use super::{TestState, STATE};

#[derive(Deserialize, Serialize)]
struct AccessTokenSchema {
    #[serde(rename = "accessToken")]
    access_token: String,
    #[serde(rename = "refreshToken")]
    refresh_token: Option<String>,
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

const PREFIX: &'static str = "auth:accessToken:";

/// Test get().
pub fn get(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let mut state = context.state.borrow_mut();
    let state = state.get_mut(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let pool = state.redis.as_mut().unwrap();

    let now = Utc::now().trunc_subsecs(3);
    let item = AccessTokenSchema {
        access_token: "token_get".to_string(),
        refresh_token: None,
        expires_at: now.timestamp_millis(),
        scope: Some("scope_get".to_string()),
        client_id: "client_id_get".to_string(),
        redirect_uri: "redirect_uri_get".to_string(),
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
            .set(PREFIX.to_string() + item.access_token.as_str(), item_str)
            .await;
        result
    }) {
        return Err(format!("execute insert error: {}", e.to_string()));
    }

    let token = match runtime.block_on(async { access_token::get(pool, "token_get").await }) {
        Err(e) => return Err(format!("get result error: {}", e.to_string())),
        Ok(token) => match token {
            None => return Err("empty token".to_string()),
            Some(token) => token,
        },
    };
    expect(token.access_token).to_equal("token_get".to_string())?;
    expect(token.refresh_token).to_equal(None)?;
    expect(token.expires_at.into()).to_equal(now)?;
    expect(token.scope).to_equal(Some("scope_get".to_string()))?;
    expect(token.client_id).to_equal("client_id_get".to_string())?;
    expect(token.redirect_uri).to_equal("redirect_uri_get".to_string())?;
    expect(token.user_id).to_equal("user_id_get".to_string())
}

/// Test add().
pub fn add(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let mut state = context.state.borrow_mut();
    let state = state.get_mut(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let pool = state.redis.as_mut().unwrap();

    let token = AccessToken {
        access_token: "token_add".to_string(),
        refresh_token: Some("token_add".to_string()),
        expires_at: Utc::now().trunc_subsecs(3),
        scope: Some("scope_add".to_string()),
        client_id: "client_id_add".to_string(),
        redirect_uri: "redirect_uri_add".to_string(),
        user_id: "user_id_add".to_string(),
    };
    if let Err(e) = runtime.block_on(async { access_token::add(pool, &token).await }) {
        return Err(format!("add error: {}", e.to_string()));
    }

    let get_token =
        match runtime.block_on(async { access_token::get(pool, &token.access_token).await }) {
            Err(e) => return Err(format!("get result error: {}", e.to_string())),
            Ok(token) => match token {
                None => return Err("empty token".to_string()),
                Some(token) => token,
            },
        };
    expect(get_token).to_equal(token)
}

/// Test del().
pub fn del(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let mut state = context.state.borrow_mut();
    let state = state.get_mut(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let pool = state.redis.as_mut().unwrap();

    let token = AccessToken {
        access_token: "token_del".to_string(),
        refresh_token: Some("token_del".to_string()),
        expires_at: Utc::now().trunc_subsecs(3),
        scope: Some("scope_del".to_string()),
        client_id: "client_id_del".to_string(),
        redirect_uri: "redirect_uri_del".to_string(),
        user_id: "user_id_del".to_string(),
    };
    let cond = QueryCond {
        access_token: Some(&token.access_token),
        ..Default::default()
    };
    if let Err(e) = runtime.block_on(async {
        access_token::add(pool, &token).await?;
        access_token::del(pool, &cond).await
    }) {
        return Err(format!("add/del error: {}", e.to_string()));
    }
    match runtime.block_on(async { access_token::get(pool, &token.access_token).await }) {
        Err(e) => return Err(e.to_string()),
        Ok(token) => match token {
            None => return Ok(()),
            Some(_) => return Err("delete token fail".to_string()),
        },
    };
}
