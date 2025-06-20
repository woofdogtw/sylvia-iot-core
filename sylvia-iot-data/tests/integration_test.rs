use std::collections::HashMap;

use laboratory::{LabResult, describe};
use tokio::{
    runtime::Runtime,
    task::{self, JoinHandle},
};

use general_mq::{Queue, connection::GmqConnection};
use sylvia_iot_auth::models::SqliteModel as AuthDbModel;
use sylvia_iot_broker::models::SqliteModel as BrokerDbModel;
use sylvia_iot_corelib::constants::{DbEngine, MqEngine};
use sylvia_iot_data::{
    libs::mq::Connection,
    models::{MongoDbModel, SqliteModel},
    routes::State,
};

mod libs;
mod models;
mod routes;

#[derive(Default)]
pub struct TestState {
    pub runtime: Option<Runtime>, // use Option for Default. Always Some().
    pub auth_db: Option<AuthDbModel>, // sylvia-iot-auth relative databases.
    pub broker_db: Option<BrokerDbModel>, // sylvia-iot-broker relative databases.
    pub auth_broker_svc: Option<JoinHandle<()>>, // sylvia-iot-auth and sylvia-iot-broker service.
    pub auth_uri: Option<String>, // the /tokeninfo URI.
    pub mongodb: Option<MongoDbModel>,
    pub sqlite: Option<SqliteModel>,
    pub mq_engine: Option<String>,
    pub recv_conns: Option<HashMap<String, Connection>>, // the connection of the recv queue.
    pub recv_queue: Option<Queue>, // recv queue for new() to test data channel.
    pub mq_conn: Option<Box<dyn GmqConnection>>, // connection for send queue.
    pub data_queue: Option<Queue>, // queue for sending data.
    pub routes_state: Option<State>,
}

pub const WAIT_COUNT: isize = 100;
pub const WAIT_TICK: u64 = 100;
pub const TEST_MONGODB_URL: &'static str = "mongodb://localhost:27017";
pub const TEST_MONGODB_DB: &'static str = "test";
pub const TEST_SQLITE_PATH: &'static str = "test.db";
pub const TEST_REDIRECT_URI: &'static str = "http://localhost:1080/auth/oauth2/redirect";
pub const TEST_BROKER_BASE: &'static str = "http://localhost:1080/broker"; // share with sylvia-iot-auth
pub const TEST_AMQP_HOST_URI: &'static str = "amqp://localhost";
pub const TEST_MQTT_HOST_URI: &'static str = "mqtt://localhost";

#[tokio::test]
async fn integration_test() -> LabResult {
    let handle = task::spawn_blocking(|| {
        describe("full test", |context| {
            context.describe_import(libs::suite());
            context.describe_import(libs::mq::suite(MqEngine::RABBITMQ));
            context.describe_import(libs::mq::suite(MqEngine::EMQX));
            context.describe_import(models::mongodb::suite());
            context.describe_import(models::sqlite::suite());
            context.describe_import(routes::suite());
            context.describe_import(routes::middleware::suite(DbEngine::MONGODB));
            context.describe_import(routes::middleware::suite(DbEngine::SQLITE));
            context.describe_import(routes::v1::suite(DbEngine::MONGODB));
            context.describe_import(routes::v1::suite(DbEngine::SQLITE));
        })
        .run()
    });

    match handle.await {
        Err(e) => Err(format!("join error: {}", e)),
        Ok(result) => result,
    }
}
