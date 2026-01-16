use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use laboratory::{LabResult, describe};
use reqwest::Client;
use tokio::{
    runtime::Runtime,
    task::{self, JoinHandle},
};

use general_mq::Queue;
use sylvia_iot_auth::models::SqliteModel as AuthDbModel;
use sylvia_iot_sdk::mq::{Connection, application::ApplicationMgr, network::NetworkMgr};

mod api;
mod middlewares;
mod mq;

#[derive(Default)]
pub struct TestState {
    pub runtime: Option<Runtime>, // use Option for Default. Always Some().
    pub auth_db: Option<AuthDbModel>, // sylvia-iot-auth relative databases.
    pub broker_db: Option<AuthDbModel>, // sylvia-iot-broker relative databases.
    pub core_svc: Option<JoinHandle<()>>, // sylvia-iot service.
    pub auth_uri: Option<String>, // the /tokeninfo URI.
    pub mq_engine: Option<String>,
    pub mgr_conns: Option<Arc<Mutex<HashMap<String, Connection>>>>,
    pub app_mgrs: Option<Vec<ApplicationMgr>>,
    pub net_mgrs: Option<Vec<NetworkMgr>>,
    pub app_net_conn: Option<Connection>, // application/network side connection.
    pub app_net_queues: Option<Vec<Queue>>, // application/network side queues.
    pub mqtt_shared_prefix: Option<String>,
    pub client: Option<Client>, // HTTP client.
}

pub const WAIT_COUNT: isize = 100;
pub const WAIT_TICK: u64 = 100;
pub const TEST_AUTH_BASE: &'static str = "http://localhost:1080/auth";
pub const TEST_REDIRECT_URI: &'static str = "http://localhost:1080/auth/oauth2/redirect";
pub const TEST_BROKER_BASE: &'static str = "http://localhost:1080/broker";
pub const TEST_COREMGR_BASE: &'static str = "http://localhost:1080/coremgr";
pub const TEST_AMQP_HOST_URI: &'static str = "amqp://localhost";
pub const TEST_MQTT_HOST_URI: &'static str = "mqtt://localhost";

#[tokio::test]
async fn integration_test() -> LabResult {
    let handle = task::spawn_blocking(|| {
        describe("full test", |context| {
            context.describe_import(api::suite());
            context.describe_import(middlewares::suite());
            context.describe_import(mq::suite(mq::MqEngine::RABBITMQ));
            context.describe_import(mq::suite(mq::MqEngine::EMQX));
        })
        .run()
    });

    match handle.await {
        Err(e) => Err(format!("join error: {}", e)),
        Ok(result) => result,
    }
}
