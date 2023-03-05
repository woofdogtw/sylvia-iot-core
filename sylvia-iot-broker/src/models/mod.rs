//! Traits and implementations for accessing databases and caches.
//!
//! Currently we only provide pure MongoDB/SQLite implementation. Mixing implementation is
//! possible. For example, put units/devices in MongoDB and put routes in Redis. Then use a
//! model struct and impl to mix both databases.

use std::{error::Error as StdError, sync::Arc};

use async_trait::async_trait;

pub mod application;
pub mod device;
pub mod device_route;
pub mod dldata_buffer;
pub mod network;
pub mod network_route;
pub mod unit;

mod cache_memory;
mod memory;
mod model_mongodb;
mod model_sqlite;
mod mongodb;
mod sqlite;

pub use self::{
    cache_memory::{Cache as MemoryCache, Options as MemoryOptions},
    memory::{
        device::Options as DeviceOptions, device_route::Options as DeviceRouteOptions,
        network_route::Options as NetworkRouteOptions,
    },
    mongodb::conn::{self as mongodb_conn, Options as MongoDbOptions},
    sqlite::conn::{self as sqlite_conn, Options as SqliteOptions},
};
pub use model_mongodb::Model as MongoDbModel;
pub use model_sqlite::Model as SqliteModel;

/// Database connection options for model implementation.
pub enum ConnOptions {
    // Pure MongoDB model implementation.
    MongoDB(MongoDbOptions),
    /// Pure SQLite model implementation.
    Sqlite(SqliteOptions),
}

/// Database connection options for cache implementation.
pub enum CacheConnOptions {
    Memory {
        device: DeviceOptions,
        device_route: DeviceRouteOptions,
        network_route: NetworkRouteOptions,
    },
}

/// The top level trait to get all models (tables/collections).
#[async_trait]
pub trait Model: Send + Sync {
    /// Close database connection.
    async fn close(&self) -> Result<(), Box<dyn StdError>>;

    /// To get the unit model.
    fn unit(&self) -> &dyn unit::UnitModel;

    /// To get the application model.
    fn application(&self) -> &dyn application::ApplicationModel;

    /// To get the network model.
    fn network(&self) -> &dyn network::NetworkModel;

    /// To get the device model.
    fn device(&self) -> &dyn device::DeviceModel;

    /// To get the device route model.
    fn device_route(&self) -> &dyn device_route::DeviceRouteModel;

    /// To get the network route model.
    fn network_route(&self) -> &dyn network_route::NetworkRouteModel;

    /// To get the downlink data buffer model.
    fn dldata_buffer(&self) -> &dyn dldata_buffer::DlDataBufferModel;
}

/// The top level trait to get all caches.
#[async_trait]
pub trait Cache: Send + Sync {
    /// Close database connection.
    async fn close(&self) -> Result<(), Box<dyn StdError>>;

    /// To get the device cache.
    fn device(&self) -> &dyn device::DeviceCache;

    /// To get the device route cache.
    fn device_route(&self) -> &dyn device_route::DeviceRouteCache;

    /// To get the network route cache.
    fn network_route(&self) -> &dyn network_route::NetworkRouteCache;
}

/// To create the database model with the specified database implementation.
pub async fn new(opts: &ConnOptions) -> Result<Arc<dyn Model>, Box<dyn StdError>> {
    let model: Arc<dyn Model> = match opts {
        ConnOptions::MongoDB(opts) => Arc::new(MongoDbModel::new(opts).await?),
        ConnOptions::Sqlite(opts) => Arc::new(SqliteModel::new(opts).await?),
    };
    model.unit().init().await?;
    model.application().init().await?;
    model.network().init().await?;
    model.device().init().await?;
    model.device_route().init().await?;
    model.network_route().init().await?;
    model.dldata_buffer().init().await?;
    Ok(model)
}

/// To create the database cache with the specified database implementation.
pub async fn new_cache(
    opts: &CacheConnOptions,
    model: &Arc<dyn Model>,
) -> Result<Arc<dyn Cache>, Box<dyn StdError>> {
    let cache: Arc<dyn Cache> = match opts {
        CacheConnOptions::Memory {
            device,
            device_route,
            network_route,
        } => {
            let opts = MemoryOptions {
                device: &device,
                device_route: &device_route,
                network_route: &network_route,
            };
            Arc::new(MemoryCache::new(&opts, model))
        }
    };
    Ok(cache)
}
