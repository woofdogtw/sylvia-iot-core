use std::error::Error as StdError;

use chrono::{TimeZone, Utc};
use redis::{AsyncCommands, aio::MultiplexedConnection};
use serde::{Deserialize, Serialize};
use serde_json;

use super::{
    super::refresh_token::{EXPIRES, QueryCond, RefreshToken},
    conn::{self, Options},
};

/// Model instance.
pub struct Model {
    /// The associated database connection.
    _conn: MultiplexedConnection,
}

/// Redis schema. Use JSON string as the value.
#[derive(Deserialize, Serialize)]
struct RefreshTokenSchema {
    #[serde(rename = "refreshToken")]
    refresh_token: String,
    /// i64 as time tick from Epoch in milliseconds.
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

const PREFIX: &'static str = "auth:refreshToken:";

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
    refresh_token: &str,
) -> Result<Option<RefreshToken>, Box<dyn StdError>> {
    let result: Option<String> = conn.get(PREFIX.to_string() + refresh_token).await?;
    let token_str = match result {
        None => return Ok(None),
        Some(token) => token,
    };
    let token: RefreshTokenSchema = serde_json::from_str(token_str.as_str())?;
    Ok(Some(RefreshToken {
        refresh_token: token.refresh_token,
        expires_at: Utc.timestamp_nanos(token.expires_at * 1000000),
        scope: token.scope,
        client_id: token.client_id,
        redirect_uri: token.redirect_uri,
        user_id: token.user_id,
    }))
}

pub async fn add(
    conn: &mut MultiplexedConnection,
    token: &RefreshToken,
) -> Result<(), Box<dyn StdError>> {
    let token = RefreshTokenSchema {
        refresh_token: token.refresh_token.to_string(),
        expires_at: token.expires_at.timestamp_millis(),
        scope: match token.scope.as_deref() {
            None => None,
            Some(scope) => Some(scope.to_string()),
        },
        client_id: token.client_id.to_string(),
        redirect_uri: token.redirect_uri.to_string(),
        user_id: token.user_id.to_string(),
    };
    let item_str = serde_json::to_string(&token)?;
    let _: () = conn
        .set_ex(
            PREFIX.to_string() + token.refresh_token.as_str(),
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
    if cond.refresh_token.is_none() {
        return Ok(());
    }
    let _: () = conn
        .del(PREFIX.to_string() + cond.refresh_token.unwrap())
        .await?;
    Ok(())
}
