use std::collections::HashMap;

use axum::{Router, http::StatusCode, routing};
use axum_test::TestServer;
use laboratory::{SpecContext, Suite, describe, expect};
use reqwest;

use general_mq::Queue;
use sylvia_iot_auth::libs::config as sylvia_iot_auth_config;
use sylvia_iot_broker::libs::config as sylvia_iot_broker_config;
use sylvia_iot_corelib::constants::DbEngine;
use sylvia_iot_data::{
    libs::{
        config::{self, Config},
        mq::Connection,
    },
    models::{self, ConnOptions, SqliteOptions},
    routes,
};

mod libs;
pub mod middleware;
pub mod v1;

use crate::TestState;
use libs::new_state;

pub const STATE: &'static str = "routes";

pub fn suite() -> Suite<TestState> {
    describe("routes", |context| {
        context.it("new_state", fn_new_state);
        context.it("new_service", fn_new_service);
        context.it("GET /version", api_get_version);

        context
            .before_all(|state| {
                state.insert(STATE, new_state(None));
            })
            .after_all(|state| {
                remove_sqlite(config::DEF_SQLITE_PATH);
                let mut path = std::env::temp_dir();
                path.push(config::DEF_SQLITE_PATH);
                remove_sqlite(path.to_str().unwrap());
                let mut path = std::env::temp_dir();
                path.push(crate::TEST_SQLITE_PATH);
                remove_sqlite(path.to_str().unwrap());

                let state = state.get_mut(STATE).unwrap();
                let runtime = state.runtime.as_ref().unwrap();
                if let Some(state) = state.routes_state.as_mut() {
                    runtime.block_on(async {
                        clear_state(state).await;
                    });
                }
            });
    })
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

fn stop_auth_broker_svc(state: &TestState) {
    if let Some(svc) = state.auth_broker_svc.as_ref() {
        svc.abort();
    }
    let mut path = std::env::temp_dir();
    path.push(sylvia_iot_auth_config::DEF_SQLITE_PATH);
    remove_sqlite(path.to_str().unwrap());
    let mut path = std::env::temp_dir();
    path.push(sylvia_iot_broker_config::DEF_SQLITE_PATH);
    remove_sqlite(path.to_str().unwrap());
}

fn fn_new_state(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();

    let conf = Config {
        ..Default::default()
    };
    let mut state = match runtime.block_on(async { routes::new_state("scope", &conf).await }) {
        Err(e) => return Err(format!("default config error: {}", e)),
        Ok(state) => state,
    };
    runtime.block_on(async { clear_state(&mut state).await });
    expect(state.scope_path).to_equal("scope")?;

    let conf = Config {
        db: Some(config::Db {
            engine: Some(DbEngine::MONGODB.to_string()),
            ..Default::default()
        }),
        ..Default::default()
    };
    let mut state = match runtime.block_on(async { routes::new_state("scope", &conf).await }) {
        Err(e) => return Err(format!("mongodb config error: {}", e)),
        Ok(state) => state,
    };
    runtime.block_on(async { clear_state(&mut state).await });
    expect(state.scope_path).to_equal("scope")?;

    let conf = Config {
        db: Some(config::Db {
            engine: Some(DbEngine::SQLITE.to_string()),
            ..Default::default()
        }),
        ..Default::default()
    };
    let mut state = match runtime.block_on(async { routes::new_state("scope", &conf).await }) {
        Err(e) => return Err(format!("sqlite config error: {}", e)),
        Ok(state) => state,
    };
    runtime.block_on(async { clear_state(&mut state).await });
    expect(state.scope_path).to_equal("scope")?;

    let conf = Config {
        db: Some(config::Db {
            engine: Some("test".to_string()),
            ..Default::default()
        }),
        ..Default::default()
    };
    let mut state = match runtime.block_on(async { routes::new_state("scope", &conf).await }) {
        Err(e) => return Err(format!("test config error: {}", e)),
        Ok(state) => state,
    };
    runtime.block_on(async { clear_state(&mut state).await });
    expect(state.scope_path).to_equal("scope")
}

fn fn_new_service(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();

    let model = match runtime.block_on(async {
        let mut path = std::env::temp_dir();
        path.push(crate::TEST_SQLITE_PATH);
        let opts = ConnOptions::Sqlite(SqliteOptions {
            path: path.to_str().unwrap().to_string(),
        });
        models::new(&opts).await
    }) {
        Err(e) => return Err(format!("new model error: {}", e)),
        Ok(model) => model,
    };

    let _ = routes::new_service(&routes::State {
        auth_base: config::DEF_AUTH.to_string(),
        broker_base: config::DEF_BROKER.to_string(),
        scope_path: "/test",
        client: reqwest::Client::new(),
        model,
        mq_conns: HashMap::<String, Connection>::new(),
        data_receivers: HashMap::<String, Queue>::new(),
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

async fn clear_state(state: &mut routes::State) {
    if let Err(e) = state.model.close().await {
        println!("close model error: {}", e);
    }
}
