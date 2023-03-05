//! Traits and structs for device routes.

use std::error::Error as StdError;

use async_trait::async_trait;
use chrono::{DateTime, Utc};

/// The item content.
#[derive(Debug, PartialEq)]
pub struct DlDataBuffer {
    pub data_id: String,
    pub unit_id: String,
    pub unit_code: String,
    pub application_id: String,
    pub application_code: String,
    pub network_id: String,
    pub network_addr: String,
    pub device_id: String,
    pub created_at: DateTime<Utc>,
    pub expired_at: DateTime<Utc>,
}

/// The sort keys for the list operation.
pub enum SortKey {
    CreatedAt,
    ExpiredAt,
    ApplicationCode,
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
    pub data_id: Option<&'a str>,
    pub unit_id: Option<&'a str>,
    pub application_id: Option<&'a str>,
    pub network_id: Option<&'a str>,
    pub network_addrs: Option<&'a Vec<&'a str>>,
    pub device_id: Option<&'a str>,
}

/// The query condition for the list operation.
#[derive(Default)]
pub struct ListQueryCond<'a> {
    /// To get downlink data buffers of the specified unit.
    pub unit_id: Option<&'a str>,
    /// To get downlink data buffers of the specified application.
    pub application_id: Option<&'a str>,
    /// To get device data of the specified network.
    pub network_id: Option<&'a str>,
    /// To get device data of the specified device.
    pub device_id: Option<&'a str>,
}

/// Model operations.
#[async_trait]
pub trait DlDataBufferModel: Sync {
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
    ) -> Result<(Vec<DlDataBuffer>, Option<Box<dyn Cursor>>), Box<dyn StdError>>;

    /// To get an item.
    async fn get(&self, data_id: &str) -> Result<Option<DlDataBuffer>, Box<dyn StdError>>;

    /// To add an item.
    async fn add(&self, data: &DlDataBuffer) -> Result<(), Box<dyn StdError>>;

    /// To delete one or more items.
    async fn del(&self, cond: &QueryCond) -> Result<(), Box<dyn StdError>>;
}

/// The operations for cursors.
///
/// All functions are private to let programs to pass them as arguments directly without any
/// operation.
#[async_trait]
pub trait Cursor: Send {
    async fn try_next(&mut self) -> Result<Option<DlDataBuffer>, Box<dyn StdError>>;

    fn offset(&self) -> u64;
}
