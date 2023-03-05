//! Traits and structs for coremgr operation data.

use std::error::Error as StdError;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json::{Map, Value};

/// The item content.
#[derive(Debug, PartialEq)]
pub struct CoremgrOpData {
    pub data_id: String,
    pub req_time: DateTime<Utc>,
    pub res_time: DateTime<Utc>,
    pub latency_ms: i64,
    pub status: i32,
    pub source_ip: String,
    pub method: String,
    pub path: String,
    pub body: Option<Map<String, Value>>,
    pub user_id: String,
    pub client_id: String,
    pub err_code: Option<String>,
    pub err_message: Option<String>,
}

/// The sort keys for the list operation.
pub enum SortKey {
    ReqTime,
    ResTime,
    Latency,
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

/// The query condition to delete item(s).
#[derive(Default)]
pub struct QueryCond<'a> {
    pub user_id: Option<&'a str>,
    pub client_id: Option<&'a str>,
    pub req_gte: Option<DateTime<Utc>>,
    pub req_lte: Option<DateTime<Utc>>,
}

/// The query condition for the list operation.
#[derive(Default)]
pub struct ListQueryCond<'a> {
    /// To get the specified user.
    pub user_id: Option<&'a str>,
    /// To get the specified client.
    pub client_id: Option<&'a str>,
    /// To get data greater than and equal to the specified `req_time` time.
    pub req_gte: Option<DateTime<Utc>>,
    /// To get data less than and equal to the specified `req_time` time.
    pub req_lte: Option<DateTime<Utc>>,
    /// To get data greater than and equal to the specified `res_time` time.
    pub res_gte: Option<DateTime<Utc>>,
    /// To get data less than and equal to the specified `res_time` time.
    pub res_lte: Option<DateTime<Utc>>,
}

/// Model operations.
#[async_trait]
pub trait CoremgrOpDataModel: Sync {
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
    ) -> Result<(Vec<CoremgrOpData>, Option<Box<dyn Cursor>>), Box<dyn StdError>>;

    /// To add an item.
    async fn add(&self, data: &CoremgrOpData) -> Result<(), Box<dyn StdError>>;

    /// To delete one or more items.
    async fn del(&self, cond: &QueryCond) -> Result<(), Box<dyn StdError>>;
}

/// The operations for cursors.
///
/// All functions are private to let programs to pass them as arguments directly without any
/// operation.
#[async_trait]
pub trait Cursor: Send {
    async fn try_next(&mut self) -> Result<Option<CoremgrOpData>, Box<dyn StdError>>;

    fn offset(&self) -> u64;
}

/// The expiration time of the data in seconds.
pub const EXPIRES: i64 = 100 * 86400;
