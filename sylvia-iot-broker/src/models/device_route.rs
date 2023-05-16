//! Traits and structs for device routes.

use std::error::Error as StdError;

use async_trait::async_trait;
use chrono::{DateTime, Utc};

/// The item content.
#[derive(Debug, PartialEq)]
pub struct DeviceRoute {
    pub route_id: String,
    pub unit_id: String,
    pub unit_code: String, // Application's unit code.
    pub application_id: String,
    pub application_code: String,
    pub device_id: String,
    pub network_id: String,
    pub network_code: String,
    pub network_addr: String,
    pub profile: String,
    pub created_at: DateTime<Utc>,
    pub modified_at: DateTime<Utc>,
}

// The device route cache item for uplink data.
#[derive(Clone)]
pub struct DeviceRouteCacheUlData {
    pub app_mgr_keys: Vec<String>,
}

// The device route cache item for downlink data.
// All None or all Some.
#[derive(Clone)]
pub struct DeviceRouteCacheDlData {
    pub net_mgr_key: String,
    pub network_id: String,
    pub network_addr: String,
    pub device_id: String,
    pub profile: String,
}

/// The sort keys for the list operation.
pub enum SortKey {
    CreatedAt,
    ModifiedAt,
    ApplicationCode,
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

/// The query condition to get item(s).
#[derive(Default)]
pub struct QueryCond<'a> {
    pub route_id: Option<&'a str>,
    pub unit_id: Option<&'a str>,
    pub application_id: Option<&'a str>,
    pub network_id: Option<&'a str>,
    pub device_id: Option<&'a str>,
    pub network_addrs: Option<&'a Vec<&'a str>>,
}

/// The query condition for the list operation.
#[derive(Default)]
pub struct ListQueryCond<'a> {
    /// To get the specified device route.
    pub route_id: Option<&'a str>,
    /// To get device routes of the specified unit.
    pub unit_id: Option<&'a str>,
    /// To get device routes of the specified unit code.
    pub unit_code: Option<&'a str>,
    /// To get device routes of the specified application.
    pub application_id: Option<&'a str>,
    /// To get device routes of the specified application code.
    pub application_code: Option<&'a str>,
    /// To get device routes of the specified network.
    pub network_id: Option<&'a str>,
    /// To get device routes of the specified network code.
    pub network_code: Option<&'a str>,
    /// To get device routes of the specified network address.
    pub network_addr: Option<&'a str>,
    /// To get devices of the specified network addresses.
    pub network_addrs: Option<&'a Vec<&'a str>>,
    /// To get device routes of the specified device.
    pub device_id: Option<&'a str>,
}

/// The query condition for the get cache operation.
pub struct GetCacheQueryCond<'a> {
    /// To get device routes of the specified network unit code.
    pub unit_code: &'a str,
    /// To get device routes of the specified network code.
    pub network_code: &'a str,
    /// To get device routes of the specified network address.
    pub network_addr: &'a str,
}

/// The query condition for the delete cache operation.
pub struct DelCacheQueryCond<'a> {
    /// To delete device routes of the specified network unit code. Empty for public network.
    pub unit_code: &'a str,
    /// To delete device routes of the specified network code.
    pub network_code: Option<&'a str>,
    /// To delete device routes of the specified network address.
    pub network_addr: Option<&'a str>,
}

/// The query condition for the get (public network) downlink data cache operation.
pub struct GetCachePubQueryCond<'a> {
    /// To get device routes of the specified device's unit ID.
    pub unit_id: &'a str,
    /// To get device routes of the specified device ID.
    pub device_id: &'a str,
}

/// The query condition for the delete (public network) downlink data cache operation.
pub struct DelCachePubQueryCond<'a> {
    /// To delete device routes of the specified device unit ID.
    pub unit_id: &'a str,
    /// To delete device routes of the specified device ID.
    pub device_id: Option<&'a str>,
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
    pub profile: Option<&'a str>,
}

/// Model operations.
#[async_trait]
pub trait DeviceRouteModel: Sync {
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
    ) -> Result<(Vec<DeviceRoute>, Option<Box<dyn Cursor>>), Box<dyn StdError>>;

    /// To get an item.
    ///
    /// **Note**: this is only used for function test.
    async fn get(&self, route_id: &str) -> Result<Option<DeviceRoute>, Box<dyn StdError>>;

    /// To add an item.
    async fn add(&self, route: &DeviceRoute) -> Result<(), Box<dyn StdError>>;

    /// To add items in bulk. Duplicate items will be skipped without errors.
    async fn add_bulk(&self, devices: &Vec<DeviceRoute>) -> Result<(), Box<dyn StdError>>;

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
    async fn try_next(&mut self) -> Result<Option<DeviceRoute>, Box<dyn StdError>>;

    fn offset(&self) -> u64;
}

/// Cache operations.
#[async_trait]
pub trait DeviceRouteCache: Sync {
    /// To clear all device routes.
    async fn clear(&self) -> Result<(), Box<dyn StdError>>;

    /// To get device route for the uplink data.
    async fn get_uldata(
        &self,
        device_id: &str,
    ) -> Result<Option<DeviceRouteCacheUlData>, Box<dyn StdError>>;

    /// To set device route for the uplink data.
    async fn set_uldata(
        &self,
        device_id: &str,
        value: Option<&DeviceRouteCacheUlData>,
    ) -> Result<(), Box<dyn StdError>>;

    /// To delete device route for the uplink data.
    async fn del_uldata(&self, device_id: &str) -> Result<(), Box<dyn StdError>>;

    /// To get device route for the downlink data.
    async fn get_dldata(
        &self,
        cond: &GetCacheQueryCond,
    ) -> Result<Option<DeviceRouteCacheDlData>, Box<dyn StdError>>;

    /// To set device route for the downlink data.
    async fn set_dldata(
        &self,
        cond: &GetCacheQueryCond,
        value: Option<&DeviceRouteCacheDlData>,
    ) -> Result<(), Box<dyn StdError>>;

    /// To delete device route for the downlink data.
    async fn del_dldata(&self, cond: &DelCacheQueryCond) -> Result<(), Box<dyn StdError>>;

    /// To get device route for the (public network) downlink data.
    async fn get_dldata_pub(
        &self,
        cond: &GetCachePubQueryCond,
    ) -> Result<Option<DeviceRouteCacheDlData>, Box<dyn StdError>>;

    /// To set device route for the (public network) downlink data.
    async fn set_dldata_pub(
        &self,
        cond: &GetCachePubQueryCond,
        value: Option<&DeviceRouteCacheDlData>,
    ) -> Result<(), Box<dyn StdError>>;

    /// To delete device route for the (public network) downlink data.
    async fn del_dldata_pub(&self, cond: &DelCachePubQueryCond) -> Result<(), Box<dyn StdError>>;
}
