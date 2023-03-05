//! Traits and structs for refresh tokens.

use std::error::Error as StdError;

use async_trait::async_trait;
use chrono::{DateTime, Utc};

/// The item content.
#[derive(Debug, PartialEq)]
pub struct RefreshToken {
    pub refresh_token: String,
    pub expires_at: DateTime<Utc>,
    pub scope: Option<String>,
    pub client_id: String,
    pub redirect_uri: String,
    pub user_id: String,
}

/// The query condition to get item(s).
#[derive(Default)]
pub struct QueryCond<'a> {
    pub refresh_token: Option<&'a str>,
    pub client_id: Option<&'a str>,
    pub user_id: Option<&'a str>,
}

/// Model operations.
#[async_trait]
pub trait RefreshTokenModel: Sync {
    /// To create and initialize the table/collection.
    async fn init(&self) -> Result<(), Box<dyn StdError>>;

    /// To get an item.
    async fn get(&self, refresh_token: &str) -> Result<Option<RefreshToken>, Box<dyn StdError>>;

    /// To add an item.
    async fn add(&self, token: &RefreshToken) -> Result<(), Box<dyn StdError>>;

    /// To delete one or more items.
    async fn del(&self, cond: &QueryCond) -> Result<(), Box<dyn StdError>>;
}

/// The expiration time of the refresh token in seconds.
pub const EXPIRES: i64 = 14 * 86400;
