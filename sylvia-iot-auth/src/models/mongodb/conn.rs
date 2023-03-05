use std::{error::Error as StdError, sync::Arc};

use log::{error, info};
use mongodb::{
    event::cmap::{
        CmapEventHandler, ConnectionClosedEvent, ConnectionCreatedEvent, ConnectionReadyEvent,
        PoolClearedEvent, PoolClosedEvent, PoolCreatedEvent, PoolReadyEvent,
    },
    options::ClientOptions,
    Client, Database,
};

/// MongoDB connection options.
pub struct Options {
    /// MongoDB URL. Use `mongodb://username:password@host:port` format.
    pub url: String,
    /// The database.
    pub db: String,
    /// Connection pool size.
    pub pool_size: Option<u32>,
}

struct ConnectionHandler;

/// Connect to MongoDB.
pub async fn connect(options: &Options) -> Result<Database, Box<dyn StdError>> {
    let mut opts = ClientOptions::parse(&options.url).await?;
    if let Some(pool_size) = options.pool_size {
        opts.max_pool_size = Some(pool_size);
    }
    opts.cmap_event_handler = Some(Arc::new(ConnectionHandler));
    let client = Client::with_options(opts)?;
    client.list_database_names(None, None).await?;
    Ok(client.database(&options.db))
}

impl CmapEventHandler for ConnectionHandler {
    fn handle_pool_created_event(&self, _event: PoolCreatedEvent) {
        info!("[handle_pool_created_event]");
    }
    fn handle_pool_ready_event(&self, _event: PoolReadyEvent) {
        info!("[handle_pool_ready_event]");
    }
    fn handle_pool_cleared_event(&self, _event: PoolClearedEvent) {
        error!("[handle_pool_cleared_event]");
    }
    fn handle_pool_closed_event(&self, _event: PoolClosedEvent) {
        error!("[handle_pool_closed_event]");
    }
    fn handle_connection_created_event(&self, _event: ConnectionCreatedEvent) {
        info!("[handle_connection_created_event]");
    }
    fn handle_connection_ready_event(&self, _event: ConnectionReadyEvent) {
        info!("[handle_connection_ready_event]");
    }
    fn handle_connection_closed_event(&self, _event: ConnectionClosedEvent) {
        error!("[handle_connection_closed_event]");
    }
}
