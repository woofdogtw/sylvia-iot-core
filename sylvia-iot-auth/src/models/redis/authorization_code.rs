use std::error::Error as StdError;

use chrono::{TimeZone, Utc};
use redis::{aio::MultiplexedConnection, AsyncCommands};
use serde::{Deserialize, Serialize};
use serde_json;

use super::{
    super::authorization_code::{AuthorizationCode, QueryCond, EXPIRES},
    conn::{self, Options},
};

/// Model instance.
pub struct Model {
    /// The associated database connection.
    _conn: MultiplexedConnection,
}

/// Redis schema. Use JSON string as the value.
#[derive(Deserialize, Serialize)]
struct Schema {
    #[serde(rename = "code")]
    code: String,
    /// i64 as time tick from Epoch in milliseconds.
    #[serde(rename = "expiresAt")]
    expires_at: i64,
    #[serde(rename = "redirectUri")]
    redirect_uri: String,
    #[serde(rename = "scope")]
    scope: Option<String>,
    #[serde(rename = "clientId")]
    client_id: String,
    #[serde(rename = "userId")]
    user_id: String,
}

const PREFIX: &'static str = "auth:authorizationCode:";

impl Model {
    /// To create the model instance with a database connection.
    pub async fn new(opts: &Options) -> Result<Self, Box<dyn StdError>> {
        Ok(Model {
            _conn: conn::connect(opts).await?,
        })
    }
}

pub async fn init(_conn: &MultiplexedConnection) -> Result<(), Box<dyn StdError>> {
    Ok(())
}

pub async fn get(
    conn: &mut MultiplexedConnection,
    code: &str,
) -> Result<Option<AuthorizationCode>, Box<dyn StdError>> {
    let result: Option<String> = conn.get(PREFIX.to_string() + code).await?;
    let code_str = match result {
        None => return Ok(None),
        Some(code) => code,
    };
    let code: Schema = serde_json::from_str(code_str.as_str())?;
    Ok(Some(AuthorizationCode {
        code: code.code,
        expires_at: Utc.timestamp_nanos(code.expires_at * 1000000),
        redirect_uri: code.redirect_uri.clone(),
        scope: code.scope,
        client_id: code.client_id,
        user_id: code.user_id,
    }))
}

pub async fn add(
    conn: &mut MultiplexedConnection,
    code: &AuthorizationCode,
) -> Result<(), Box<dyn StdError>> {
    let code = Schema {
        code: code.code.to_string(),
        expires_at: code.expires_at.timestamp_millis(),
        redirect_uri: code.redirect_uri.clone(),
        scope: code.scope.clone(),
        client_id: code.client_id.to_string(),
        user_id: code.user_id.to_string(),
    };
    let item_str = serde_json::to_string(&code)?;
    let _ = conn
        .set_ex(
            PREFIX.to_string() + code.code.as_str(),
            item_str,
            (EXPIRES + 60) as u64,
        )
        .await?;
    Ok(())
}

pub async fn del<'a>(
    conn: &mut MultiplexedConnection,
    cond: &QueryCond<'a>,
) -> Result<(), Box<dyn StdError>> {
    if cond.code.is_none() {
        return Ok(());
    }
    let _ = conn.del(PREFIX.to_string() + cond.code.unwrap()).await?;
    Ok(())
}
