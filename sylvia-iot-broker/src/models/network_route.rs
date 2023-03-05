//! Traits and structs for network routes.

use std::error::Error as StdError;

use async_trait::async_trait;
use chrono::{DateTime, Utc};

/// The item content.
#[derive(Debug, PartialEq)]
pub struct NetworkRoute {
    pub route_id: String,
    pub unit_id: String,
    pub unit_code: String, // Application's unit code.
    pub application_id: String,
    pub application_code: String,
    pub network_id: String,
    pub network_code: String,
    pub created_at: DateTime<Utc>,
}

// The network route cache item for uplink data.
#[derive(Clone)]
pub struct NetworkRouteCacheUlData {
    pub app_mgr_keys: Vec<String>,
}

/// The sort keys for the list operation.
pub enum SortKey {
    CreatedAt,
    ApplicationCode,
    NetworkCode,
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
}

/// The query condition for the list operation.
#[derive(Default)]
pub struct ListQueryCond<'a> {
    /// To get the specified network route.
    pub route_id: Option<&'a str>,
    /// To get network routes of the specified unit.
    pub unit_id: Option<&'a str>,
    /// To get network routes of the specified unit code.
    pub unit_code: Option<&'a str>,
    /// To get network routes of the specified application.
    pub application_id: Option<&'a str>,
    /// To get network routes of the specified application code.
    pub application_code: Option<&'a str>,
    /// To get network routes of the specified network.
    pub network_id: Option<&'a str>,
    /// To get network routes of the specified network code.
    pub network_code: Option<&'a str>,
}

/// Model operations.
#[async_trait]
pub trait NetworkRouteModel: Sync {
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
    ) -> Result<(Vec<NetworkRoute>, Option<Box<dyn Cursor>>), Box<dyn StdError>>;

    /// To get an item.
    ///
    /// **Note**: this is only used for function test.
    async fn get(&self, route_id: &str) -> Result<Option<NetworkRoute>, Box<dyn StdError>>;

    /// To add an item.
    async fn add(&self, route: &NetworkRoute) -> Result<(), Box<dyn StdError>>;

    /// To delete one or more items.
    async fn del(&self, cond: &QueryCond) -> Result<(), Box<dyn StdError>>;
}

/// The operations for cursors.
///
/// All functions are private to let programs to pass them as arguments directly without any
/// operation.
#[async_trait]
pub trait Cursor: Send {
    async fn try_next(&mut self) -> Result<Option<NetworkRoute>, Box<dyn StdError>>;

    fn offset(&self) -> u64;
}

/// Cache operations.
#[async_trait]
pub trait NetworkRouteCache: Sync {
    /// To clear all network routes.
    async fn clear(&self) -> Result<(), Box<dyn StdError>>;

    /// To get network route for the uplink data.
    async fn get_uldata(
        &self,
        network_id: &str,
    ) -> Result<Option<NetworkRouteCacheUlData>, Box<dyn StdError>>;

    /// To set network route for the uplink data.
    async fn set_uldata(
        &self,
        network_id: &str,
        value: Option<&NetworkRouteCacheUlData>,
    ) -> Result<(), Box<dyn StdError>>;

    /// To delete network route for the uplink data.
    async fn del_uldata(&self, network_id: &str) -> Result<(), Box<dyn StdError>>;
}
