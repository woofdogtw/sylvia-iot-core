use std::error::Error as StdError;

use redis::{Client, aio::MultiplexedConnection};

/// Redis connection options.
pub struct Options {
    /// Redis URL. Use `redis://:password@host:port` format.
    pub url: String,
}

/// Connect to Redis.
pub async fn connect(options: &Options) -> Result<MultiplexedConnection, Box<dyn StdError>> {
    let conn = Client::open(options.url.as_str())?
        .get_multiplexed_async_connection()
        .await?;
    Ok(conn)
}
