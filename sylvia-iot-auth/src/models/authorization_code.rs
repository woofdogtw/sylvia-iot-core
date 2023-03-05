//! Traits and structs for authorization codes.

use std::error::Error as StdError;

use async_trait::async_trait;
use chrono::{DateTime, Utc};

/// The item content.
#[derive(Debug, PartialEq)]
pub struct AuthorizationCode {
    pub code: String,
    pub expires_at: DateTime<Utc>,
    pub redirect_uri: String,
    pub scope: Option<String>,
    pub client_id: String,
    pub user_id: String,
}

/// The query condition to get item(s).
#[derive(Default)]
pub struct QueryCond<'a> {
    pub code: Option<&'a str>,
    pub client_id: Option<&'a str>,
    pub user_id: Option<&'a str>,
}

/// Model operations.
#[async_trait]
pub trait AuthorizationCodeModel: Sync {
    /// To create and initialize the table/collection.
    async fn init(&self) -> Result<(), Box<dyn StdError>>;

    /// To get an item.
    async fn get(
        &self,
        authorization_code: &str,
    ) -> Result<Option<AuthorizationCode>, Box<dyn StdError>>;

    /// To add an item.
    async fn add(&self, code: &AuthorizationCode) -> Result<(), Box<dyn StdError>>;

    /// To delete one or more items.
    async fn del(&self, cond: &QueryCond) -> Result<(), Box<dyn StdError>>;
}

/// The expiration time of the authorization code in seconds.
pub const EXPIRES: i64 = 30;
