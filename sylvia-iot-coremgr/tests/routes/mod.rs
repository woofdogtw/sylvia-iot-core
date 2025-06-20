use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use axum::{Router, http::StatusCode, routing};
use axum_test::TestServer;
use laboratory::{SpecContext, Suite, describe, expect};
use reqwest;

use general_mq::queue::GmqQueue;
use sylvia_iot_auth::libs::config as sylvia_iot_auth_config;
use sylvia_iot_broker::libs::config as sylvia_iot_broker_config;
use sylvia_iot_coremgr::{
    libs::{
        config::{self, Config},
        mq::{emqx::ManagementOpts as EmqxOpts, rabbitmq::ManagementOpts as RabbitMqOpts},
    },
    routes::{self, AmqpState, MqttState},
};

use crate::{TestState, libs::mq::emqx};

mod libs;
pub mod middleware;
pub mod v1;

use libs::new_state;

pub const STATE: &'static str = "routes";

pub fn suite() -> Suite<TestState> {
    describe("routes", |context| {
        context.it("new_state", fn_new_state);
        context.it("new_service", fn_new_service);
        context.it("GET /version", api_get_version);

        context
            .before_all(|state| {
                state.insert(STATE, new_state(None, None));
            })
            .after_all(|state| {
                let state = state.get_mut(STATE).unwrap();
                let runtime = state.runtime.as_ref().unwrap();
                runtime.block_on(async {
                    if let Err(e) = emqx::after_del_api_key().await {
                        println!("delete EMQX API key error: {}", e);
                    }
                });
            });
    })
}

fn fn_new_state(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();

    let conf = Config {
        ..Default::default()
    };
    let state = match runtime.block_on(async { routes::new_state("scope", &conf).await }) {
        Err(e) => return Err(format!("default config error: {}", e)),
        Ok(state) => state,
    };
    expect(state.scope_path).to_equal("scope")
}

fn fn_new_service(_context: &mut SpecContext<TestState>) -> Result<(), String> {
    let _ = routes::new_service(&routes::State {
        auth_base: config::DEF_AUTH.to_string(),
        broker_base: config::DEF_BROKER.to_string(),
        scope_path: "/test",
        client: reqwest::Client::new(),
        amqp: AmqpState::RabbitMq(RabbitMqOpts {
            username: "guest".to_string(),
            password: "guest".to_string(),
            ttl: Some(86400),
            length: Some(1000),
        }),
        mqtt: MqttState::Emqx(EmqxOpts {
            api_key: "key".to_string(),
            api_secret: "secret".to_string(),
        }),
        mq_conns: Arc::new(Mutex::new(HashMap::new())),
        data_sender: None,
    });
    Ok(())
}

fn api_get_version(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();

    const SERV_NAME: &'static str = env!("CARGO_PKG_NAME");
    const SERV_VER: &'static str = env!("CARGO_PKG_VERSION");

    let app = Router::new().route("/version", routing::get(routes::get_version));
    let server = match TestServer::new(app) {
        Err(e) => return Err(format!("new server error: {}", e)),
        Ok(server) => server,
    };

    // Default.
    let req = server.get("/version");
    let resp = runtime.block_on(async { req.await });
    expect(resp.status_code()).to_equal(StatusCode::OK)?;
    let body = resp.text();
    let expect_body = format!(
        "{{\"data\":{{\"name\":\"{}\",\"version\":\"{}\"}}}}",
        SERV_NAME, SERV_VER
    );
    expect(body.as_ref()).to_equal(expect_body.as_str().as_bytes())?;

    // Invalid query.
    let req = server.get("/version").add_query_param("q", "test");
    let resp = runtime.block_on(async { req.await });
    expect(resp.status_code()).to_equal(StatusCode::OK)?;
    let body = resp.text();
    let expect_body = format!(
        "{{\"data\":{{\"name\":\"{}\",\"version\":\"{}\"}}}}",
        SERV_NAME, SERV_VER
    );
    expect(body.as_ref()).to_equal(expect_body.as_str().as_bytes())?;

    // Query service name.
    let req = server.get("/version").add_query_param("q", "name");
    let resp = runtime.block_on(async { req.await });
    expect(resp.status_code()).to_equal(StatusCode::OK)?;
    let body = resp.text();
    expect(body.as_ref()).to_equal(SERV_NAME.as_bytes())?;

    // Query service version.
    let req = server.get("/version").add_query_param("q", "version");
    let resp = runtime.block_on(async { req.await });
    expect(resp.status_code()).to_equal(StatusCode::OK)?;
    let body = resp.text();
    expect(body.as_ref()).to_equal(SERV_VER.as_bytes())
}

fn remove_sqlite(path: &str) {
    if let Err(e) = std::fs::remove_file(path) {
        println!("remove file {} error: {}", path, e);
    }
    let file = format!("{}-shm", path);
    if let Err(e) = std::fs::remove_file(file.as_str()) {
        println!("remove file {} error: {}", file.as_str(), e);
    }
    let file = format!("{}-wal", path);
    if let Err(e) = std::fs::remove_file(file.as_str()) {
        println!("remove file {} error: {}", file.as_str(), e);
    }
}

fn stop_auth_broker_svc(state: &mut TestState) {
    if let Some(svc) = state.auth_broker_svc.take() {
        svc.abort();
    }
    let mut path = std::env::temp_dir();
    path.push(sylvia_iot_auth_config::DEF_SQLITE_PATH);
    remove_sqlite(path.to_str().unwrap());
    let mut path = std::env::temp_dir();
    path.push(sylvia_iot_broker_config::DEF_SQLITE_PATH);
    remove_sqlite(path.to_str().unwrap());
}

async fn clear_state(state: &mut routes::State) {
    if let Some(mut q) = state.data_sender.take() {
        if let Err(e) = q.close().await {
            println!("close data channel {} error: {}", q.name(), e);
        }
    }
}
