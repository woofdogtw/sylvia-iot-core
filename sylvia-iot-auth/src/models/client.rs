//! Traits, enumerations and structs for clients.

use std::error::Error as StdError;

use async_trait::async_trait;
use chrono::{DateTime, Utc};

/// The item content.
#[derive(Debug, PartialEq)]
pub struct Client {
    pub client_id: String,
    pub created_at: DateTime<Utc>,
    pub modified_at: DateTime<Utc>,
    pub client_secret: Option<String>,
    pub redirect_uris: Vec<String>,
    pub scopes: Vec<String>,
    pub user_id: String,
    pub name: String,
    pub image_url: Option<String>,
}

/// The sort keys for the list operation.
pub enum SortKey {
    CreatedAt,
    ModifiedAt,
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
    pub client_id: Option<&'a str>,
}

/// The query condition for the list operation.
#[derive(Default)]
pub struct ListQueryCond<'a> {
    /// To get clients of the specified user.
    pub user_id: Option<&'a str>,
    /// To get the specified client.
    pub client_id: Option<&'a str>,
    /// To get client that their **name** contains the specified (partial) word.
    pub name_contains: Option<&'a str>,
}

/// The query condition for the update operation.
#[derive(Default)]
pub struct UpdateQueryCond<'a> {
    /// The specified user.
    pub user_id: &'a str,
    /// The specified client.
    pub client_id: &'a str,
}

/// The update fields by using [`Some`]s.
#[derive(Default)]
pub struct Updates<'a> {
    pub modified_at: Option<DateTime<Utc>>,
    pub client_secret: Option<Option<String>>,
    pub redirect_uris: Option<&'a Vec<String>>,
    pub scopes: Option<&'a Vec<String>>,
    pub name: Option<&'a str>,
    pub image_url: Option<Option<&'a str>>,
}

/// Model operations.
#[async_trait]
pub trait ClientModel: Sync {
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
    ) -> Result<(Vec<Client>, Option<Box<dyn Cursor>>), Box<dyn StdError>>;

    /// To get an item.
    async fn get(&self, cond: &QueryCond) -> Result<Option<Client>, Box<dyn StdError>>;

    /// To add an item.
    async fn add(&self, client: &Client) -> Result<(), Box<dyn StdError>>;

    /// To delete one or more items.
    async fn del(&self, cond: &QueryCond) -> Result<(), Box<dyn StdError>>;

    /// To update one or more items.
    async fn update(
        &self,
        cond: &UpdateQueryCond,
        updates: &Updates,
    ) -> Result<(), Box<dyn StdError>>;
}

/// The operations for cursors.
///
/// All functions are private to let programs to pass them as arguments directly without any
/// operation.
#[async_trait]
pub trait Cursor: Send {
    async fn try_next(&mut self) -> Result<Option<Client>, Box<dyn StdError>>;

    fn offset(&self) -> u64;
}
