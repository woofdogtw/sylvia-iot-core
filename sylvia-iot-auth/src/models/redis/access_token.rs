use std::error::Error as StdError;

use chrono::{TimeZone, Utc};
use redis::{aio::Connection, AsyncCommands};
use serde::{Deserialize, Serialize};
use serde_json;

use super::{
    super::access_token::{AccessToken, QueryCond, EXPIRES},
    conn::{self, Options},
};

/// Model instance.
pub struct Model {
    /// The associated database connection.
    _conn: Connection,
}

/// Redis schema. Use JSON string as the value.
#[derive(Deserialize, Serialize)]
struct Schema {
    #[serde(rename = "accessToken")]
    access_token: String,
    #[serde(rename = "refreshToken")]
    refresh_token: Option<String>,
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

const PREFIX: &'static str = "auth:accessToken:";

impl Model {
    /// To create the model instance with a database connection.
    pub async fn new(opts: &Options) -> Result<Self, Box<dyn StdError>> {
        Ok(Model {
            _conn: conn::connect(opts).await?,
        })
    }
}

pub async fn init(_conn: &Connection) -> Result<(), Box<dyn StdError>> {
    Ok(())
}

pub async fn get(
    conn: &mut Connection,
    access_token: &str,
) -> Result<Option<AccessToken>, Box<dyn StdError>> {
    let result: Option<String> = conn.get(PREFIX.to_string() + access_token).await?;
    let token_str = match result {
        None => return Ok(None),
        Some(token) => token,
    };
    let token: Schema = serde_json::from_str(token_str.as_str())?;
    Ok(Some(AccessToken {
        access_token: token.access_token,
        refresh_token: token.refresh_token,
        expires_at: Utc.timestamp_nanos(token.expires_at * 1000000),
        scope: token.scope,
        client_id: token.client_id,
        redirect_uri: token.redirect_uri,
        user_id: token.user_id,
    }))
}

pub async fn add(conn: &mut Connection, token: &AccessToken) -> Result<(), Box<dyn StdError>> {
    let token = Schema {
        access_token: token.access_token.to_string(),
        refresh_token: match token.refresh_token.as_deref() {
            None => None,
            Some(token) => Some(token.to_string()),
        },
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
    let _ = conn
        .set_ex(
            PREFIX.to_string() + token.access_token.as_str(),
            item_str,
            (EXPIRES + 60) as usize,
        )
        .await?;
    Ok(())
}

pub async fn del<'a>(conn: &mut Connection, cond: &QueryCond<'a>) -> Result<(), Box<dyn StdError>> {
    if cond.access_token.is_none() {
        return Ok(());
    }
    let _ = conn
        .del(PREFIX.to_string() + cond.access_token.unwrap())
        .await?;
    Ok(())
}
