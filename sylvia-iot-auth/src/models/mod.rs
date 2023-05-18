//! Traits and implementations for accessing databases and caches.
//!
//! Currently we only provide pure MongoDB/SQLite implementation. Mixing implementation is
//! possible. For example, put users/clients in MongoDB and put tokens/codes in Redis. Then use a
//! model struct and impl to mix both databases.

use std::{error::Error as StdError, sync::Arc};

use async_trait::async_trait;

pub mod access_token;
pub mod authorization_code;
pub mod client;
pub mod login_session;
pub mod redis;
pub mod refresh_token;
pub mod user;

mod model_mongodb;
mod model_sqlite;
mod mongodb;
mod sqlite;

pub use self::{
    mongodb::conn::{self as mongodb_conn, Options as MongoDbOptions},
    redis::conn::{self as redis_conn, Options as RedisOptions},
    sqlite::conn::{self as sqlite_conn, Options as SqliteOptions},
};
pub use model_mongodb::Model as MongoDbModel;
pub use model_sqlite::Model as SqliteModel;

/// Database connection options for model implementation.
pub enum ConnOptions {
    // Pure MongoDB model implementation.
    MongoDB(MongoDbOptions),
    //MongoRedis(MongoDbOptions, RedisOptions),
    /// Pure SQLite model implementation.
    Sqlite(SqliteOptions),
}

/// The top level trait to get all models (tables/collections).
#[async_trait]
pub trait Model: Send + Sync {
    /// Close database connection.
    async fn close(&self) -> Result<(), Box<dyn StdError>>;

    /// To get the user model.
    fn user(&self) -> &dyn user::UserModel;

    /// To get the client model.
    fn client(&self) -> &dyn client::ClientModel;

    /// To get the login session model.
    fn login_session(&self) -> &dyn login_session::LoginSessionModel;

    /// To get the authorization code model.
    fn authorization_code(&self) -> &dyn authorization_code::AuthorizationCodeModel;

    /// To get the access token model.
    fn access_token(&self) -> &dyn access_token::AccessTokenModel;

    /// To get the refresh token model.
    fn refresh_token(&self) -> &dyn refresh_token::RefreshTokenModel;
}

/// To create the database model with the specified database implementation.
pub async fn new(opts: &ConnOptions) -> Result<Arc<dyn Model>, Box<dyn StdError>> {
    let model: Arc<dyn Model> = match opts {
        ConnOptions::MongoDB(opts) => Arc::new(MongoDbModel::new(opts).await?),
        ConnOptions::Sqlite(opts) => Arc::new(SqliteModel::new(opts).await?),
    };
    model.user().init().await?;
    model.client().init().await?;
    model.login_session().init().await?;
    model.authorization_code().init().await?;
    model.access_token().init().await?;
    model.refresh_token().init().await?;
    Ok(model)
}
