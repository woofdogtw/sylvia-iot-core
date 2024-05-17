//! Traits, enumerations and structs for users.

use std::{collections::HashMap, error::Error as StdError};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json::{Map, Value};

/// The item content.
#[derive(Clone, Debug, PartialEq)]
pub struct User {
    pub user_id: String,
    pub account: String,
    pub created_at: DateTime<Utc>,
    pub modified_at: DateTime<Utc>,
    pub verified_at: Option<DateTime<Utc>>,
    pub expired_at: Option<DateTime<Utc>>,
    pub disabled_at: Option<DateTime<Utc>>,
    pub roles: HashMap<String, bool>,
    pub password: String,
    pub salt: String,
    pub name: String,
    pub info: Map<String, Value>,
}

/// The sort keys for the list operation.
pub enum SortKey {
    Account,
    CreatedAt,
    ModifiedAt,
    VerifiedAt,
    ExpiredAt,
    DisabledAt,
    Name,
}

/// The sort condition for the list operation.
pub struct SortCond {
    pub key: SortKey,
    pub asc: bool,
}

/// The list operation options.
pub struct ListOptions<'a> {
    /// The query conditions.
    pub cond: &'a ListQueryCond<'a>,
    /// The data offset.
    pub offset: Option<u64>,
    /// The maximum number to query.
    pub limit: Option<u64>,
    /// The sort conditions.
    pub sort: Option<&'a [SortCond]>,
    /// The maximum number items one time the `list()` returns.
    ///
    /// Use cursors until reaching `limit` or all data.
    pub cursor_max: Option<u64>,
}

/// The query condition to get item(s).
#[derive(Default)]
pub struct QueryCond<'a> {
    pub user_id: Option<&'a str>,
    pub account: Option<&'a str>,
}

/// The query condition for the list operation.
#[derive(Default)]
pub struct ListQueryCond<'a> {
    /// To get the specified user.
    pub user_id: Option<&'a str>,
    /// To get the specified user by account.
    pub account: Option<&'a str>,
    /// To get users with the specified word.
    pub account_contains: Option<&'a str>,
    /// To get users that are only verified or only not verified.
    pub verified_at: Option<bool>,
    /// To get users that are only disabled or only not disabled.
    pub disabled_at: Option<bool>,
    /// To get users which name with the specified word.
    pub name_contains: Option<&'a str>,
}

/// The update fields by using [`Some`]s.
#[derive(Default)]
pub struct Updates<'a> {
    pub modified_at: Option<DateTime<Utc>>,
    pub verified_at: Option<DateTime<Utc>>,
    pub expired_at: Option<Option<DateTime<Utc>>>,
    pub disabled_at: Option<Option<DateTime<Utc>>>,
    pub roles: Option<&'a HashMap<String, bool>>,
    pub password: Option<String>,
    pub salt: Option<String>,
    pub name: Option<&'a str>,
    pub info: Option<&'a Map<String, Value>>,
}

/// Model operations.
#[async_trait]
pub trait UserModel: Sync {
    /// To create and initialize the table/collection.
    async fn init(&self) -> Result<(), Box<dyn StdError>>;

    /// To get item count for the query condition.
    ///
    /// **Note**: this may take a long time.
    async fn count(&self, cond: &ListQueryCond) -> Result<u64, Box<dyn StdError>>;

    /// To get item list. The maximum number of returned items will be controlled by the
    /// `cursor_max` of the list option.
    ///
    /// For the first time, `cursor` MUST use `None`. If one cursor is returned, it means that
    /// there are more items to get. Use the returned cursor to get more data items.
    ///
    /// **Note**: using cursors is recommended to prevent exhausting memory.
    async fn list(
        &self,
        opts: &ListOptions,
        cursor: Option<Box<dyn Cursor>>,
    ) -> Result<(Vec<User>, Option<Box<dyn Cursor>>), Box<dyn StdError>>;

    /// To get an item.
    async fn get(&self, cond: &QueryCond) -> Result<Option<User>, Box<dyn StdError>>;

    /// To add an item.
    async fn add(&self, user: &User) -> Result<(), Box<dyn StdError>>;

    /// To delete one or more items.
    async fn del(&self, user_id: &str) -> Result<(), Box<dyn StdError>>;

    /// To update one or more items.
    async fn update(&self, user_id: &str, updates: &Updates) -> Result<(), Box<dyn StdError>>;
}

/// The operations for cursors.
///
/// All functions are private to let programs to pass them as arguments directly without any
/// operation.
#[async_trait]
pub trait Cursor: Send {
    async fn try_next(&mut self) -> Result<Option<User>, Box<dyn StdError>>;

    fn offset(&self) -> u64;
}
