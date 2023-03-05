//! Traits and structs for access tokens.

use std::error::Error as StdError;

use async_trait::async_trait;
use chrono::{DateTime, Utc};

/// The item content.
#[derive(Debug, PartialEq)]
pub struct AccessToken {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: DateTime<Utc>,
    pub scope: Option<String>,
    pub client_id: String,
    pub redirect_uri: String,
    pub user_id: String,
}

/// The query condition to get item(s).
#[derive(Default)]
pub struct QueryCond<'a> {
    pub access_token: Option<&'a str>,
    pub refresh_token: Option<&'a str>,
    pub client_id: Option<&'a str>,
    pub user_id: Option<&'a str>,
}

/// Model operations.
#[async_trait]
pub trait AccessTokenModel: Sync {
    /// To create and initialize the table/collection.
    async fn init(&self) -> Result<(), Box<dyn StdError>>;

    /// To get an item.
    async fn get(&self, access_token: &str) -> Result<Option<AccessToken>, Box<dyn StdError>>;

    /// To add an item.
    async fn add(&self, token: &AccessToken) -> Result<(), Box<dyn StdError>>;

    /// To delete one or more items.
    async fn del(&self, cond: &QueryCond) -> Result<(), Box<dyn StdError>>;
}

/// The expiration time of the access token in seconds.
pub const EXPIRES: i64 = 1 * 60 * 60;
