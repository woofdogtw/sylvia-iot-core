use std::thread::JoinHandle as ThreadHandle;

use actix_web::dev::ServerHandle;
use laboratory::{describe, LabResult};
use reqwest::Client;
use tokio::{runtime::Runtime, task};

use general_mq::{AmqpConnection, Queue};
use sylvia_iot_auth::models::SqliteModel as AuthDbModel;
use sylvia_iot_broker::models::SqliteModel as BrokerDbModel;
use sylvia_iot_corelib::constants::MqEngine;
use sylvia_iot_coremgr::{
    libs::{
        config::Rumqttd as RumqttdOpts,
        mq::{
            emqx::ManagementOpts as EmqxOpts, rabbitmq::ManagementOpts as RabbitMqOpts, Connection,
        },
    },
    routes::State,
};

mod libs;
mod routes;

#[derive(Default)]
pub struct TestState {
    pub runtime: Option<Runtime>, // use Option for Default. Always Some().
    pub auth_db: Option<AuthDbModel>, // sylvia-iot-auth relative databases.
    pub broker_db: Option<BrokerDbModel>, // sylvia-iot-broker relative databases.
    pub auth_broker_svc: Option<ServerHandle>, // sylvia-iot-auth and sylvia-iot-broker service.
    pub auth_uri: Option<String>, // the /tokeninfo URI.
    pub routes_state: Option<State>,
    pub client: Option<Client>, // HTTP client.
    pub mq_opts: Option<(RabbitMqOpts, EmqxOpts, RumqttdOpts)>,
    pub amqp_conn: Option<Vec<AmqpConnection>>,
    pub rumqttd_handles: Option<(ThreadHandle<()>, ThreadHandle<()>)>,
    pub mq_conn: Option<Connection>,
    pub data_queue: Option<Queue>, // receive queue to test data channel.
}

pub const WAIT_COUNT: isize = 100;
pub const WAIT_TICK: u64 = 100;
pub const TEST_REDIRECT_URI: &'static str = "http://localhost:1080/auth/oauth2/redirect";
pub const TEST_BROKER_BASE: &'static str = "http://localhost:1080/broker"; // share with sylvia-iot-auth
pub const TEST_RABBITMQ_USER: &'static str = "guest";
pub const TEST_RABBITMQ_PASS: &'static str = "guest";
pub const TEST_EMQX_USER: &'static str = "admin";
pub const TEST_EMQX_PASS: &'static str = "public";
pub const TEST_EMQX_KEY_NAME: &'static str = "admin";
pub const TEST_RUMQTTD_MQTT_PORT: u16 = 1884;
pub const TEST_RUMQTTD_MQTTS_PORT: u16 = 8884;
pub const TEST_RUMQTTD_CONSOLE_PORT: u16 = 18084;
pub const TEST_MQ_HOST: &'static str = "localhost";
pub const TEST_MQ_USER: &'static str = "test-user";
pub const TEST_MQ_USER_SUPER: &'static str = "test-user-super";
pub const TEST_MQ_USER_WRONG: &'static str = "test-user-wrong";
pub const TEST_MQ_PASS: &'static str = "test-pass";
pub const TEST_AMQP_HOST_URI: &'static str = "amqp://localhost";
pub const TEST_EMQX_HOST_URI: &'static str = "mqtt://admin:public@localhost";
pub const TEST_RUMQTTD_HOST_URI: &'static str = "mqtt://localhost:1884";

#[tokio::test]
async fn integration_test() -> LabResult {
    let handle = task::spawn_blocking(|| {
        describe("full test", |context| {
            context.describe_import(libs::suite());
            context.describe_import(libs::mq::suite());
            context.describe_import(routes::suite());
            context.describe_import(routes::middleware::suite(
                Some(MqEngine::EMQX),
                TEST_AMQP_HOST_URI,
            ));
            context.describe_import(routes::middleware::suite(
                Some(MqEngine::EMQX),
                TEST_EMQX_HOST_URI,
            ));
            context.describe_import(routes::middleware::suite(
                Some(MqEngine::RUMQTTD),
                TEST_RUMQTTD_HOST_URI,
            ));
            context.describe_import(routes::v1::suite(MqEngine::EMQX));
            context.describe_import(routes::v1::suite(MqEngine::RUMQTTD));
        })
        .run()
    });

    match handle.await {
        Err(e) => Err(format!("join error: {}", e)),
        Ok(result) => result,
    }
}
