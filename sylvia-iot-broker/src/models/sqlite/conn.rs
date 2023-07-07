use std::error::Error as StdError;
use std::str::FromStr;

use log::LevelFilter;
use sqlx::{sqlite::SqliteConnectOptions, ConnectOptions, SqlitePool};

/// SQLite connection options.
pub struct Options {
    /// SQLite database file path. Use absolute/relative path.
    pub path: String,
}

/// Connect to SQLite.
pub async fn connect(options: &Options) -> Result<SqlitePool, Box<dyn StdError>> {
    let opts = SqliteConnectOptions::from_str(&options.path)?
        .create_if_missing(true)
        .log_statements(LevelFilter::Off);
    let result = SqlitePool::connect_with(opts).await?;
    Ok(result)
}
