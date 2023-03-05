//! Traits and structs for application downlink data.

use std::error::Error as StdError;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json::{Map, Value};

/// The item content.
#[derive(Debug, PartialEq)]
pub struct ApplicationDlData {
    pub data_id: String,
    pub proc: DateTime<Utc>,
    pub resp: Option<DateTime<Utc>>,
    pub status: i32,
    pub unit_id: String,
    pub device_id: Option<String>,
    pub network_code: Option<String>,
    pub network_addr: Option<String>,
    pub data: String,
    pub extension: Option<Map<String, Value>>,
}

/// The sort keys for the list operation.
pub enum SortKey {
    Proc,
    Resp,
    NetworkCode,
    NetworkAddr,
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
    pub unit_id: Option<&'a str>,
    pub device_id: Option<&'a str>,
    pub network_code: Option<&'a str>,
    pub network_addr: Option<&'a str>,
    pub proc_gte: Option<DateTime<Utc>>,
    pub proc_lte: Option<DateTime<Utc>>,
}

/// The query condition for the list operation.
#[derive(Default)]
pub struct ListQueryCond<'a> {
    /// To get the specified unit.
    pub unit_id: Option<&'a str>,
    /// To get the specified device.
    pub device_id: Option<&'a str>,
    /// To get the specified device's network code.
    pub network_code: Option<&'a str>,
    /// To get the specified device's network address.
    pub network_addr: Option<&'a str>,
    /// To get data greater than and equal to the specified `proc` time.
    pub proc_gte: Option<DateTime<Utc>>,
    /// To get data less than and equal to the specified `proc` time.
    pub proc_lte: Option<DateTime<Utc>>,
    /// To get data greater than and equal to the specified `resp` time.
    pub resp_gte: Option<DateTime<Utc>>,
    /// To get data less than and equal to the specified `resp` time.
    pub resp_lte: Option<DateTime<Utc>>,
}

/// The query condition for the update operation.
pub struct UpdateQueryCond<'a> {
    /// The specified data.
    pub data_id: &'a str,
}

/// The update fields.
pub struct Updates {
    pub resp: DateTime<Utc>,
    /// The status will be changed for the following cases:
    /// - Zero and positive value changes non-zero status data.
    /// - Negative value changes smaller status value data.
    pub status: i32,
}

/// Model operations.
#[async_trait]
pub trait ApplicationDlDataModel: Sync {
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
    ) -> Result<(Vec<ApplicationDlData>, Option<Box<dyn Cursor>>), Box<dyn StdError>>;

    /// To add an item.
    async fn add(&self, data: &ApplicationDlData) -> Result<(), Box<dyn StdError>>;

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
    async fn try_next(&mut self) -> Result<Option<ApplicationDlData>, Box<dyn StdError>>;

    fn offset(&self) -> u64;
}

/// The expiration time of the data in seconds.
pub const EXPIRES: i64 = 100 * 86400;
