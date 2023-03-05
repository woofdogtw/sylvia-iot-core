//! Traits and implementations for accessing databases and caches.
//!
//! Currently we only provide pure MongoDB/SQLite implementation. Mixing implementation is
//! possible.

use std::{error::Error as StdError, sync::Arc};

use async_trait::async_trait;

pub mod application_dldata;
pub mod application_uldata;
pub mod coremgr_opdata;
pub mod network_dldata;
pub mod network_uldata;

mod model_mongodb;
mod model_sqlite;
mod mongodb;
mod sqlite;

pub use self::{
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

/// The top level trait to get all models (tables/collections).
#[async_trait]
pub trait Model: Send + Sync {
    /// Close database connection.
    async fn close(&self) -> Result<(), Box<dyn StdError>>;

    /// To get the application downlink data model.
    fn application_dldata(&self) -> &dyn application_dldata::ApplicationDlDataModel;

    /// To get the application uplink data model.
    fn application_uldata(&self) -> &dyn application_uldata::ApplicationUlDataModel;

    /// To get the coremgr operation data model.
    fn coremgr_opdata(&self) -> &dyn coremgr_opdata::CoremgrOpDataModel;

    /// To get the network downlink data model.
    fn network_dldata(&self) -> &dyn network_dldata::NetworkDlDataModel;

    /// To get the network uplink data model.
    fn network_uldata(&self) -> &dyn network_uldata::NetworkUlDataModel;
}

/// To create the database model with the specified database implementation.
pub async fn new(opts: &ConnOptions) -> Result<Arc<dyn Model>, Box<dyn StdError>> {
    let model: Arc<dyn Model> = match opts {
        ConnOptions::MongoDB(opts) => Arc::new(MongoDbModel::new(opts).await?),
        ConnOptions::Sqlite(opts) => Arc::new(SqliteModel::new(opts).await?),
    };
    model.application_dldata().init().await?;
    model.application_uldata().init().await?;
    model.coremgr_opdata().init().await?;
    model.network_dldata().init().await?;
    model.network_uldata().init().await?;
    Ok(model)
}
