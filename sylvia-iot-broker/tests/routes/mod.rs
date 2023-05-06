use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use actix_web::{
    http::StatusCode,
    test::{self, TestRequest},
    web, App,
};
use async_trait::async_trait;
use general_mq::{
    connection::Connection as MqConnection,
    queue::{Event, EventHandler, Message, Queue},
};
use laboratory::{describe, expect, SpecContext, Suite};
use reqwest;
use url::Url;

use sylvia_iot_auth::libs::config as sylvia_iot_auth_config;
use sylvia_iot_broker::{
    libs::{
        config::{self, Config},
        mq::{control, data, Connection},
    },
    models::{self, ConnOptions, SqliteOptions},
    routes,
};
use sylvia_iot_corelib::constants::{DbEngine, MqEngine};

use crate::TestState;

mod libs;
pub mod middleware;
pub mod v1;

use super::libs::libs::{conn_host_uri, remove_rabbitmq_queues};
use libs::new_state;

struct TestHandler;

#[async_trait]
impl EventHandler for TestHandler {
    async fn on_event(&self, _queue: Arc<dyn Queue>, _ev: Event) {}

    async fn on_message(&self, _queue: Arc<dyn Queue>, _msg: Box<dyn Message>) {}
}

pub const STATE: &'static str = "routes";

pub fn suite() -> Suite<TestState> {
    describe("routes", |context| {
        context.it("new_state", fn_new_state);
        context.it("new_service", fn_new_service);
        context.it("new_service with API scopes", fn_api_scopes);
        context.it("GET /version", api_get_version);

        context.before_all(|state| {
            state.insert(STATE, new_state(None, None, None));
        });
        context.after_all(|state| {
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
            remove_rabbitmq_queues(state);
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

fn stop_auth_svc(state: &TestState) {
    let runtime = state.runtime.as_ref().unwrap();
    if let Some(svc) = state.auth_svc.as_ref() {
        runtime.block_on(async { svc.stop(false).await });
    }
    let mut path = std::env::temp_dir();
    path.push(sylvia_iot_auth_config::DEF_SQLITE_PATH);
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
        mq_channels: Some(config::MqChannels {
            data: Some(config::BrokerData {
                url: Some(config::DEF_MQ_CHANNEL_URL.to_string()),
            }),
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
        mq_channels: Some(config::MqChannels {
            data: Some(config::BrokerData {
                url: Some(crate::TEST_MQTT_HOST_URI.to_string()),
            }),
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
    let cache = match runtime.block_on(async {
        let opts = models::CacheConnOptions::Memory {
            device: models::DeviceOptions {
                uldata_size: crate::TEST_CACHE_SIZE,
            },
            device_route: models::DeviceRouteOptions {
                uldata_size: crate::TEST_CACHE_SIZE,
                dldata_size: crate::TEST_CACHE_SIZE,
                dldata_pub_size: crate::TEST_CACHE_SIZE,
            },
            network_route: models::NetworkRouteOptions {
                uldata_size: crate::TEST_CACHE_SIZE,
            },
        };
        match models::new_cache(&opts, &model).await {
            Err(e) => Err(e),
            Ok(cache) => Ok(Some(cache)),
        }
    }) {
        Err(e) => return Err(format!("new cache error: {}", e)),
        Ok(cache) => cache,
    };

    let mq_conns = Arc::new(Mutex::new(HashMap::new()));
    let url = Url::parse(conn_host_uri(MqEngine::RABBITMQ)?.as_str()).unwrap();
    let unit_ctrl = control::new(
        mq_conns.clone(),
        &url,
        None,
        "unit",
        false,
        Arc::new(TestHandler {}),
    )?;
    let app_ctrl = control::new(
        mq_conns.clone(),
        &url,
        None,
        "application",
        false,
        Arc::new(TestHandler {}),
    )?;
    let net_ctrl = control::new(
        mq_conns.clone(),
        &url,
        None,
        "network",
        false,
        Arc::new(TestHandler {}),
    )?;
    let dev_ctrl = control::new(
        mq_conns.clone(),
        &url,
        None,
        "device",
        false,
        Arc::new(TestHandler {}),
    )?;
    let devr_ctrl = control::new(
        mq_conns.clone(),
        &url,
        None,
        "device-route",
        false,
        Arc::new(TestHandler {}),
    )?;
    let netr_ctrl = control::new(
        mq_conns.clone(),
        &url,
        None,
        "network-route",
        false,
        Arc::new(TestHandler {}),
    )?;
    let data_sender = data::new(&mq_conns, &url, Arc::new(TestHandler {}))?;
    let mut state = routes::State {
        auth_base: config::DEF_AUTH.to_string(),
        api_scopes: HashMap::new(),
        scope_path: "test",
        model: model.clone(),
        cache,
        amqp_prefetch: config::DEF_MQ_PREFETCH,
        mqtt_shared_prefix: config::DEF_MQ_SHAREDPREFIX.to_string(),
        client: reqwest::Client::new(),
        mq_conns,
        application_mgrs: Arc::new(Mutex::new(HashMap::new())),
        network_mgrs: Arc::new(Mutex::new(HashMap::new())),
        ctrl_receivers: Arc::new(Mutex::new(HashMap::new())),
        ctrl_senders: routes::CtrlSenders {
            unit: Arc::new(Mutex::new(unit_ctrl)),
            application: Arc::new(Mutex::new(app_ctrl)),
            network: Arc::new(Mutex::new(net_ctrl)),
            device: Arc::new(Mutex::new(dev_ctrl)),
            device_route: Arc::new(Mutex::new(devr_ctrl)),
            network_route: Arc::new(Mutex::new(netr_ctrl)),
        },
        data_sender: Some(data_sender),
    };
    let _ = routes::new_service(&state);
    if let Err(e) = runtime.block_on(async { model.close().await }) {
        return Err(format!("close model error: {}", e));
    }
    runtime.block_on(async { clear_state(&mut state).await });
    Ok(())
}

fn fn_api_scopes(context: &mut SpecContext<TestState>) -> Result<(), String> {
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
    let cache = match runtime.block_on(async {
        let opts = models::CacheConnOptions::Memory {
            device: models::DeviceOptions {
                uldata_size: crate::TEST_CACHE_SIZE,
            },
            device_route: models::DeviceRouteOptions {
                uldata_size: crate::TEST_CACHE_SIZE,
                dldata_size: crate::TEST_CACHE_SIZE,
                dldata_pub_size: crate::TEST_CACHE_SIZE,
            },
            network_route: models::NetworkRouteOptions {
                uldata_size: crate::TEST_CACHE_SIZE,
            },
        };
        match models::new_cache(&opts, &model).await {
            Err(e) => Err(e),
            Ok(cache) => Ok(Some(cache)),
        }
    }) {
        Err(e) => return Err(format!("new cache error: {}", e)),
        Ok(cache) => cache,
    };

    let mut api_scopes: HashMap<String, Vec<String>> = HashMap::new();
    api_scopes.insert("unit.post".to_string(), vec![]);
    api_scopes.insert("unit.get".to_string(), vec![]);
    api_scopes.insert("unit.patch".to_string(), vec![]);
    api_scopes.insert("unit.delete".to_string(), vec![]);
    api_scopes.insert("unit.delete.user".to_string(), vec![]);
    api_scopes.insert("application.post".to_string(), vec![]);
    api_scopes.insert("application.get".to_string(), vec![]);
    api_scopes.insert("application.patch".to_string(), vec![]);
    api_scopes.insert("application.delete".to_string(), vec![]);
    api_scopes.insert("network.post".to_string(), vec![]);
    api_scopes.insert("network.get".to_string(), vec![]);
    api_scopes.insert("network.patch".to_string(), vec![]);
    api_scopes.insert("network.delete".to_string(), vec![]);
    api_scopes.insert("device.post".to_string(), vec![]);
    api_scopes.insert("device.get".to_string(), vec![]);
    api_scopes.insert("device.patch".to_string(), vec![]);
    api_scopes.insert("device.delete".to_string(), vec![]);
    api_scopes.insert("device-route.post".to_string(), vec![]);
    api_scopes.insert("device-route.get".to_string(), vec![]);
    api_scopes.insert("device-route.patch".to_string(), vec![]);
    api_scopes.insert("device-route.delete".to_string(), vec![]);
    api_scopes.insert("network-route.post".to_string(), vec![]);
    api_scopes.insert("network-route.get".to_string(), vec![]);
    api_scopes.insert("network-route.patch".to_string(), vec![]);
    api_scopes.insert("network-route.delete".to_string(), vec![]);
    api_scopes.insert("dldata-buffer.post".to_string(), vec![]);
    api_scopes.insert("dldata-buffer.get".to_string(), vec![]);
    api_scopes.insert("dldata-buffer.patch".to_string(), vec![]);
    api_scopes.insert("dldata-buffer.delete".to_string(), vec![]);

    let mq_conns = Arc::new(Mutex::new(HashMap::new()));
    let url = Url::parse(conn_host_uri(MqEngine::RABBITMQ)?.as_str()).unwrap();
    let unit_ctrl = control::new(
        mq_conns.clone(),
        &url,
        None,
        "unit",
        false,
        Arc::new(TestHandler {}),
    )?;
    let app_ctrl = control::new(
        mq_conns.clone(),
        &url,
        None,
        "application",
        false,
        Arc::new(TestHandler {}),
    )?;
    let net_ctrl = control::new(
        mq_conns.clone(),
        &url,
        None,
        "network",
        false,
        Arc::new(TestHandler {}),
    )?;
    let dev_ctrl = control::new(
        mq_conns.clone(),
        &url,
        None,
        "device",
        false,
        Arc::new(TestHandler {}),
    )?;
    let devr_ctrl = control::new(
        mq_conns.clone(),
        &url,
        None,
        "device-route",
        false,
        Arc::new(TestHandler {}),
    )?;
    let netr_ctrl = control::new(
        mq_conns.clone(),
        &url,
        None,
        "network-route",
        false,
        Arc::new(TestHandler {}),
    )?;
    let mut state = routes::State {
        auth_base: config::DEF_AUTH.to_string(),
        api_scopes,
        scope_path: "test",
        model: model.clone(),
        cache,
        amqp_prefetch: config::DEF_MQ_PREFETCH,
        mqtt_shared_prefix: config::DEF_MQ_SHAREDPREFIX.to_string(),
        client: reqwest::Client::new(),
        mq_conns,
        application_mgrs: Arc::new(Mutex::new(HashMap::new())),
        network_mgrs: Arc::new(Mutex::new(HashMap::new())),
        ctrl_receivers: Arc::new(Mutex::new(HashMap::new())),
        ctrl_senders: routes::CtrlSenders {
            unit: Arc::new(Mutex::new(unit_ctrl)),
            application: Arc::new(Mutex::new(app_ctrl)),
            network: Arc::new(Mutex::new(net_ctrl)),
            device: Arc::new(Mutex::new(dev_ctrl)),
            device_route: Arc::new(Mutex::new(devr_ctrl)),
            network_route: Arc::new(Mutex::new(netr_ctrl)),
        },
        data_sender: None,
    };
    let _ = routes::new_service(&state);
    if let Err(e) = runtime.block_on(async { model.close().await }) {
        return Err(format!("close model error: {}", e));
    }
    runtime.block_on(async { clear_state(&mut state).await });
    Ok(())
}

fn api_get_version(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();

    const SERV_NAME: &'static str = env!("CARGO_PKG_NAME");
    const SERV_VER: &'static str = env!("CARGO_PKG_VERSION");

    let mut app = runtime.block_on(async {
        test::init_service(App::new().route("/version", web::get().to(routes::get_version))).await
    });

    // Default.
    let req = TestRequest::get().uri("/version").to_request();
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    expect(resp.status()).to_equal(StatusCode::OK)?;
    let body = runtime.block_on(async { test::read_body(resp).await });
    let expect_body = format!(
        "{{\"data\":{{\"name\":\"{}\",\"version\":\"{}\"}}}}",
        SERV_NAME, SERV_VER
    );
    expect(body.as_ref()).to_equal(expect_body.as_str().as_bytes())?;

    // Invalid query.
    let req = TestRequest::get().uri("/version?q=test").to_request();
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    expect(resp.status()).to_equal(StatusCode::OK)?;
    let body = runtime.block_on(async { test::read_body(resp).await });
    let expect_body = format!(
        "{{\"data\":{{\"name\":\"{}\",\"version\":\"{}\"}}}}",
        SERV_NAME, SERV_VER
    );
    expect(body.as_ref()).to_equal(expect_body.as_str().as_bytes())?;

    // Query service name.
    let req = TestRequest::get().uri("/version?q=name").to_request();
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    expect(resp.status()).to_equal(StatusCode::OK)?;
    let body = runtime.block_on(async { test::read_body(resp).await });
    expect(body.as_ref()).to_equal(SERV_NAME.as_bytes())?;

    // Query service version.
    let req = TestRequest::get().uri("/version?q=version").to_request();
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    expect(resp.status()).to_equal(StatusCode::OK)?;
    let body = runtime.block_on(async { test::read_body(resp).await });
    expect(body.as_ref()).to_equal(SERV_VER.as_bytes())
}

async fn clear_state(state: &mut routes::State) {
    if let Err(e) = state.model.close().await {
        println!("close model error: {}", e);
    }
    let mgrs = { state.application_mgrs.lock().unwrap().clone() };
    for (_, mgr) in mgrs {
        if let Err(e) = mgr.close().await {
            println!("close ApplicationMgr error: {}", e);
        }
    }
    {
        state.application_mgrs.lock().unwrap().clear();
    }
    let mgrs = { state.network_mgrs.lock().unwrap().clone() };
    for (_, mgr) in mgrs {
        if let Err(e) = mgr.close().await {
            println!("close NetworkMgr error: {}", e);
        }
    }
    {
        state.network_mgrs.lock().unwrap().clear();
    }
    let receivers = { state.ctrl_receivers.lock().unwrap().clone() };
    for (_, mut recv) in receivers.into_iter() {
        if let Err(e) = recv.close().await {
            println!("close receiver {} error: {}", recv.name(), e);
        }
    }
    {
        state.ctrl_receivers.lock().unwrap().clear();
    }
    let mut q = { state.ctrl_senders.application.lock().unwrap().clone() };
    if let Err(e) = q.close().await {
        println!("close application control {} error: {}", q.name(), e);
    }
    let mut q = { state.ctrl_senders.network.lock().unwrap().clone() };
    if let Err(e) = q.close().await {
        println!("close network control {} error: {}", q.name(), e);
    }
    if let Some(mut q) = state.data_sender.take() {
        if let Err(e) = q.close().await {
            println!("close data channel {} error: {}", q.name(), e);
        }
    }
    let conns = { state.mq_conns.lock().unwrap().clone() };
    for (_, conn) in conns {
        match conn {
            Connection::Amqp(mut c, _) => {
                if let Err(e) = c.close().await {
                    println!("close connection error {}", e);
                }
            }
            Connection::Mqtt(mut c, _) => {
                if let Err(e) = c.close().await {
                    println!("close connection error {}", e);
                }
            }
        }
    }
    {
        state.mq_conns.lock().unwrap().clear();
    }
}
