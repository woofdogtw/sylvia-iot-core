//! Traits and structs for devices.

use std::error::Error as StdError;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json::{Map, Value};

/// The item content.
#[derive(Debug, PartialEq)]
pub struct Device {
    pub device_id: String,
    pub unit_id: String,           // Associated device unit.
    pub unit_code: Option<String>, // Associated network's unit.
    pub network_id: String,
    pub network_code: String,
    pub network_addr: String,
    pub created_at: DateTime<Utc>,
    pub modified_at: DateTime<Utc>,
    pub name: String,
    pub info: Map<String, Value>,
}

// The device cache item.
#[derive(Clone)]
pub struct DeviceCacheItem {
    pub device_id: String,
}

/// The sort keys for the list operation.
pub enum SortKey {
    CreatedAt,
    ModifiedAt,
    NetworkCode,
    NetworkAddr,
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
    pub device_id: Option<&'a str>,
    pub network_id: Option<&'a str>,
    pub network_addrs: Option<&'a Vec<&'a str>>,
    pub device: Option<QueryOneCond<'a>>,
}

/// The query condition for the exact one device.
#[derive(Clone)]
pub struct QueryOneCond<'a> {
    pub unit_code: Option<&'a str>,
    pub network_code: &'a str,
    pub network_addr: &'a str,
}

/// The query condition for the get cache operation.
pub enum GetCacheQueryCond<'a> {
    CodeAddr(QueryOneCond<'a>),
}

/// The query condition for the delete cache operation.
pub struct DelCacheQueryCond<'a> {
    /// To delete devices of the specified network unit code. Empty for public network.
    pub unit_code: &'a str,
    /// To delete devices of the specified network code.
    pub network_code: Option<&'a str>,
    /// To delete a device of the specified network address.
    pub network_addr: Option<&'a str>,
}

/// The query condition for the list operation.
#[derive(Default)]
pub struct ListQueryCond<'a> {
    /// To get devices of the specified unit.
    pub unit_id: Option<&'a str>,
    /// To get the specified device.
    pub device_id: Option<&'a str>,
    /// To get devices of the specified network.
    pub network_id: Option<&'a str>,
    /// To get devices of the specified network code.
    pub network_code: Option<&'a str>,
    /// To get devices of the specified network address.
    /// This has priorier than `network_addrs`.
    pub network_addr: Option<&'a str>,
    /// To get devices of the specified network addresses.
    pub network_addrs: Option<&'a Vec<&'a str>>,
    /// To get unit that their **name** contains the specified (partial) word.
    pub name_contains: Option<&'a str>,
}

/// The query condition for the update operation.
pub struct UpdateQueryCond<'a> {
    /// The specified device.
    pub device_id: &'a str,
}

/// The update fields by using [`Some`]s.
#[derive(Default)]
pub struct Updates<'a> {
    pub modified_at: Option<DateTime<Utc>>,
    pub name: Option<&'a str>,
    pub info: Option<&'a Map<String, Value>>,
}

/// Model operations.
#[async_trait]
pub trait DeviceModel: Sync {
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
    ) -> Result<(Vec<Device>, Option<Box<dyn Cursor>>), Box<dyn StdError>>;

    /// To get an item.
    async fn get(&self, cond: &QueryCond) -> Result<Option<Device>, Box<dyn StdError>>;

    /// To add an item.
    async fn add(&self, device: &Device) -> Result<(), Box<dyn StdError>>;

    /// To add items in bulk. Duplicate items will be skipped without errors.
    async fn add_bulk(&self, devices: &Vec<Device>) -> Result<(), Box<dyn StdError>>;

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
    async fn try_next(&mut self) -> Result<Option<Device>, Box<dyn StdError>>;

    fn offset(&self) -> u64;
}

/// Cache operations.
#[async_trait]
pub trait DeviceCache: Sync {
    /// To clear all devices.
    async fn clear(&self) -> Result<(), Box<dyn StdError>>;

    /// To get device.
    async fn get(
        &self,
        cond: &GetCacheQueryCond,
    ) -> Result<Option<DeviceCacheItem>, Box<dyn StdError>>;

    /// To set device.
    async fn set(
        &self,
        cond: &GetCacheQueryCond,
        value: Option<&DeviceCacheItem>,
    ) -> Result<(), Box<dyn StdError>>;

    /// To delete device.
    async fn del(&self, cond: &DelCacheQueryCond) -> Result<(), Box<dyn StdError>>;
}
