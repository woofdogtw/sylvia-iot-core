use std::{collections::HashMap, sync::Arc};

use actix_web::dev::ServerHandle;
use general_mq::{connection::GmqConnection, queue::GmqQueue, Queue};
use laboratory::{describe, LabResult};
use tokio::{runtime::Runtime, task};

use sylvia_iot_auth::models::SqliteModel as AuthDbModel;
use sylvia_iot_broker::{
    libs::mq::{application::ApplicationMgr, network::NetworkMgr, Connection},
    models::{Cache, Model, MongoDbModel, SqliteModel},
    routes::State,
};
use sylvia_iot_corelib::constants::{CacheEngine, DbEngine, MqEngine};

mod libs;
mod models;
mod routes;

#[derive(Default)]
pub struct TestState {
    pub runtime: Option<Runtime>, // use Option for Default. Always Some().
    pub auth_db: Option<AuthDbModel>, // sylvia-iot-auth relative databases.
    pub auth_svc: Option<ServerHandle>, // sylvia-iot-auth service.
    pub auth_uri: Option<String>, // the /tokeninfo URI.
    pub mongodb: Option<MongoDbModel>,
    pub sqlite: Option<SqliteModel>,
    pub cache: Option<Arc<dyn Cache>>,
    pub cache_model: Option<Arc<dyn Model>>,
    pub mq_engine: Option<String>,
    pub mq_conn: Option<Connection>,
    pub app_mgrs: Option<Vec<ApplicationMgr>>,
    pub net_mgrs: Option<Vec<NetworkMgr>>,
    pub ctrl_queues: Option<Vec<Queue>>,
    pub data_queue: Option<Queue>, // receive queue to test data channel.
    pub data_ch_host: Option<String>, // receive queue host.
    pub routes_state: Option<State>,
    pub routing_conns: Option<Vec<Box<dyn GmqConnection>>>, // for routing/data cases.
    pub routing_queues: Option<Vec<Box<dyn GmqQueue>>>,     // for routing/data cases.
    pub routing_values: Option<HashMap<String, String>>,    // for routing/data cases.
    pub routing_device_id: Option<String>,                  // for routing/data cases.
    pub amqp_prefetch: Option<u16>,
    pub mqtt_shared_prefix: Option<String>,
}

pub const WAIT_COUNT: isize = 100;
pub const WAIT_TICK: u64 = 100;
pub const TEST_MONGODB_URL: &'static str = "mongodb://localhost:27017";
pub const TEST_MONGODB_DB: &'static str = "test";
pub const TEST_SQLITE_PATH: &'static str = "test.db";
pub const TEST_CACHE_SIZE: usize = 10000;
pub const TEST_REDIRECT_URI: &'static str = "http://localhost:1080/auth/oauth2/redirect";
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
            context.describe_import(models::memory::suite());
            context.describe_import(routes::suite());
            context.describe_import(routes::middleware::suite(DbEngine::MONGODB));
            context.describe_import(routes::middleware::suite(DbEngine::SQLITE));
            context.describe_import(routes::v1::suite(DbEngine::MONGODB, CacheEngine::NONE));
            context.describe_import(routes::v1::suite(DbEngine::SQLITE, CacheEngine::NONE));
            context.describe_import(routes::v1::suite(DbEngine::SQLITE, CacheEngine::MEMORY));
            context.describe_import(routes::v1::suite_data(
                DbEngine::MONGODB,
                CacheEngine::NONE,
                TEST_AMQP_HOST_URI,
            ));
            context.describe_import(routes::v1::suite_data(
                DbEngine::SQLITE,
                CacheEngine::NONE,
                TEST_MQTT_HOST_URI,
            ));
            context.describe_import(routes::v1::suite_data(
                DbEngine::SQLITE,
                CacheEngine::MEMORY,
                TEST_AMQP_HOST_URI,
            ));
        })
        .run()
    });

    match handle.await {
        Err(e) => Err(format!("join error: {}", e)),
        Ok(result) => result,
    }
}
