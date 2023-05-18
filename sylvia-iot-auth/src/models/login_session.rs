//! Traits and structs for authorization codes.

use std::error::Error as StdError;

use async_trait::async_trait;
use chrono::{DateTime, Utc};

/// The item content.
#[derive(Debug, PartialEq)]
pub struct LoginSession {
    pub session_id: String,
    pub expires_at: DateTime<Utc>,
    pub user_id: String,
}

/// The query condition to get item(s).
#[derive(Default)]
pub struct QueryCond<'a> {
    pub session_id: Option<&'a str>,
    pub user_id: Option<&'a str>,
}

/// Model operations.
#[async_trait]
pub trait LoginSessionModel: Sync {
    /// To create and initialize the table/collection.
    async fn init(&self) -> Result<(), Box<dyn StdError>>;

    /// To get an item.
    async fn get(&self, session_id: &str) -> Result<Option<LoginSession>, Box<dyn StdError>>;

    /// To add an item.
    async fn add(&self, code: &LoginSession) -> Result<(), Box<dyn StdError>>;

    /// To delete one or more items.
    async fn del(&self, cond: &QueryCond) -> Result<(), Box<dyn StdError>>;
}

/// The expiration time of the authorization code in seconds.
pub const EXPIRES: i64 = 60;
