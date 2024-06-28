use std::{error::Error as StdError, sync::Arc};

use log::{error, info};
use mongodb::{
    event::{cmap::CmapEvent, EventHandler},
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

/// Connect to MongoDB.
pub async fn connect(options: &Options) -> Result<Database, Box<dyn StdError>> {
    let mut opts = ClientOptions::parse(&options.url).await?;
    if let Some(pool_size) = options.pool_size {
        opts.max_pool_size = Some(pool_size);
    }
    opts.cmap_event_handler = Some(EventHandler::Callback(Arc::new(event_handler)));
    let client = Client::with_options(opts)?;
    client.list_database_names().await?;
    Ok(client.database(&options.db))
}

fn event_handler(ev: CmapEvent) {
    match ev {
        CmapEvent::PoolCreated(_ev) => {
            info!("[CmapEvent::PoolCreated]");
        }
        CmapEvent::PoolReady(_ev) => {
            info!("[CmapEvent::PoolReady]");
        }
        CmapEvent::PoolCleared(_ev) => {
            error!("[CmapEvent::PoolCleared]");
        }
        CmapEvent::PoolClosed(_ev) => {
            error!("[CmapEvent::PoolClosed]");
        }
        CmapEvent::ConnectionCreated(_ev) => {
            info!("[CmapEvent::ConnectionCreated]");
        }
        CmapEvent::ConnectionReady(_ev) => {
            info!("[CmapEvent::ConnectionReady]");
        }
        CmapEvent::ConnectionClosed(_ev) => {
            error!("[CmapEvent::ConnectionClosed]");
        }
        CmapEvent::ConnectionCheckoutStarted(_ev) => {
            info!("[CmapEvent::ConnectionCheckoutStarted]");
        }
        CmapEvent::ConnectionCheckoutFailed(_ev) => {
            error!("[CmapEvent::ConnectionCheckoutFailed]");
        }
        CmapEvent::ConnectionCheckedOut(_ev) => {
            info!("[CmapEvent::ConnectionCheckedOut]");
        }
        CmapEvent::ConnectionCheckedIn(_ev) => {
            info!("[CmapEvent::ConnectionCheckedIn]");
        }
        _ => {
            error!("[CmapEvent::Unknown]");
        }
    }
}
