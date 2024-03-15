use laboratory::{describe, LabResult};
use redis::aio::MultiplexedConnection as RedisConn;
use tokio::runtime::Runtime;

use sylvia_iot_auth::{
    models::{MongoDbModel, SqliteModel},
    routes::State,
};
use sylvia_iot_corelib::constants::DbEngine;

mod libs;
mod models;
mod routes;

#[derive(Default)]
pub struct TestState {
    pub runtime: Option<Runtime>, // use Option for Default. Always Some().
    pub mongodb: Option<MongoDbModel>,
    pub redis: Option<RedisConn>,
    pub sqlite: Option<SqliteModel>,
    pub routes_state: Option<State>,
}

pub const TEST_MONGODB_URL: &'static str = "mongodb://localhost:27017";
pub const TEST_MONGODB_DB: &'static str = "test";
pub const TEST_SQLITE_PATH: &'static str = "test.db";
pub const TEST_REDIRECT_URI: &'static str = "http://localhost:1080/auth/oauth2/redirect";

#[test]
pub fn integration_test() -> LabResult {
    describe("full test", |context| {
        context.describe_import(libs::suite());
        context.describe_import(models::mongodb::suite());
        context.describe_import(models::redis::suite());
        context.describe_import(models::sqlite::suite());
        context.describe_import(routes::suite());
        context.describe_import(routes::oauth2::suite(DbEngine::MONGODB));
        context.describe_import(routes::oauth2::suite(DbEngine::SQLITE));
        context.describe_import(routes::v1::suite(DbEngine::MONGODB));
        context.describe_import(routes::v1::suite(DbEngine::SQLITE));
    })
    .run()
}
