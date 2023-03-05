//! Traits and structs for units.

use std::error::Error as StdError;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json::{Map, Value};

/// The item content.
#[derive(Debug, PartialEq)]
pub struct Unit {
    pub unit_id: String,
    pub code: String,
    pub created_at: DateTime<Utc>,
    pub modified_at: DateTime<Utc>,
    pub owner_id: String,
    pub member_ids: Vec<String>,
    pub name: String,
    pub info: Map<String, Value>,
}

/// The sort keys for the list operation.
pub enum SortKey {
    CreatedAt,
    ModifiedAt,
    Code,
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
    pub unit_id: Option<&'a str>,
    pub code: Option<&'a str>,
    pub owner_id: Option<&'a str>,
    pub member_id: Option<&'a str>,
}

/// The query condition for the list operation.
#[derive(Default)]
pub struct ListQueryCond<'a> {
    /// To get units of the specified owner.
    pub owner_id: Option<&'a str>,
    /// To get units of the specified member.
    pub member_id: Option<&'a str>,
    /// To get the specified unit.
    pub unit_id: Option<&'a str>,
    /// To get unit that their **code** contains the specified (partial) word.
    pub code_contains: Option<&'a str>,
    /// To get unit that their **name** contains the specified (partial) word.
    pub name_contains: Option<&'a str>,
}

/// The query condition for the update operation.
pub struct UpdateQueryCond<'a> {
    /// The specified unit.
    pub unit_id: &'a str,
}

/// The update fields by using [`Some`]s.
#[derive(Default)]
pub struct Updates<'a> {
    pub modified_at: Option<DateTime<Utc>>,
    pub owner_id: Option<&'a str>,
    pub member_ids: Option<&'a Vec<String>>,
    pub name: Option<&'a str>,
    pub info: Option<&'a Map<String, Value>>,
}

/// Model operations.
#[async_trait]
pub trait UnitModel: Sync {
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
    ) -> Result<(Vec<Unit>, Option<Box<dyn Cursor>>), Box<dyn StdError>>;

    /// To get an item.
    async fn get(&self, cond: &QueryCond) -> Result<Option<Unit>, Box<dyn StdError>>;

    /// To add an item.
    async fn add(&self, unit: &Unit) -> Result<(), Box<dyn StdError>>;

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
    async fn try_next(&mut self) -> Result<Option<Unit>, Box<dyn StdError>>;

    fn offset(&self) -> u64;
}
