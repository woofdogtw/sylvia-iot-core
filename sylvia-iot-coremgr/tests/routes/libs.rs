use std::{collections::HashMap, net::SocketAddr, time::Duration};

use axum::{
    Router,
    http::{HeaderValue, Method, StatusCode, header},
};
use axum_test::{TestRequest, TestServer};
use chrono::{DateTime, TimeZone, Utc};
use laboratory::expect;
use serde::Deserialize;
use serde_json::{Map, Value};
use tokio::{net::TcpListener, runtime::Runtime, time};

use sylvia_iot_auth::{
    libs::config as sylvia_iot_auth_config,
    models::{
        self as sylvia_iot_auth_models, Model, access_token::AccessToken, client::Client,
        user::User,
    },
    routes as sylvia_iot_auth_routes,
};
use sylvia_iot_broker::{
    libs::config as sylvia_iot_broker_config,
    models::{
        self as sylvia_iot_broker_models, application::Application, device::Device,
        device_route::DeviceRoute, dldata_buffer::DlDataBuffer, network::Network,
        network_route::NetworkRoute, unit::Unit,
    },
    routes as sylvia_iot_broker_routes,
};
use sylvia_iot_corelib::{
    constants::{DbEngine, MqEngine},
    role::Role,
    server_config::Config as ServerConfig,
    strings,
};
use sylvia_iot_coremgr::{
    libs::{
        config::{self, Config, Rumqttd as RumqttdOpts},
        mq::{emqx::ManagementOpts as EmqxOpts, rabbitmq::ManagementOpts as RabbitMqOpts, rumqttd},
    },
    routes,
};

use crate::{TestState, WAIT_COUNT, WAIT_TICK, libs::mq::emqx};

#[derive(Deserialize)]
pub struct ApiError {
    pub code: String,
}

#[derive(Deserialize)]
struct GetCountRes {
    data: CountData,
}

#[derive(Deserialize)]
struct CountData {
    count: usize,
}

#[derive(Deserialize)]
struct GetListRes {
    data: Vec<Data>,
}

#[derive(Deserialize)]
struct Data {}

pub const TOKEN_MANAGER: &'static str = "TOKEN_MANAGER";
pub const TOKEN_OWNER: &'static str = "TOKEN_OWNER";
pub const TOKEN_MEMBER: &'static str = "TOKEN_MEMBER";

pub fn create_user(name: &str, time: DateTime<Utc>, roles: HashMap<String, bool>) -> User {
    User {
        user_id: name.to_string(),
        account: name.to_string(),
        created_at: time,
        modified_at: time,
        verified_at: Some(time),
        expired_at: None,
        disabled_at: None,
        roles,
        password: strings::password_hash(name, name),
        salt: name.to_string(),
        name: name.to_string(),
        info: Map::<String, Value>::new(),
    }
}

pub fn create_client(name: &str, user_id: &str, secret: Option<String>) -> Client {
    let now = Utc::now();
    Client {
        client_id: name.to_string(),
        created_at: now,
        modified_at: now,
        client_secret: secret,
        redirect_uris: vec![crate::TEST_REDIRECT_URI.to_string()],
        scopes: vec![],
        user_id: user_id.to_string(),
        name: name.to_string(),
        image_url: None,
    }
}

pub fn create_token(token: &str, user_id: &str, client_id: &str) -> AccessToken {
    let expires_at = Utc.timestamp_nanos((Utc::now().timestamp() + 3600) * 1_000_000_000);
    AccessToken {
        access_token: token.to_string(),
        refresh_token: None,
        expires_at,
        scope: None,
        client_id: client_id.to_string(),
        redirect_uri: "http://localhost".to_string(),
        user_id: user_id.to_string(),
    }
}

pub fn create_unit(name: &str, owner_id: &str) -> Unit {
    let now = Utc::now();
    Unit {
        unit_id: name.to_string(),
        code: name.to_string(),
        created_at: now,
        modified_at: now,
        owner_id: owner_id.to_string(),
        member_ids: vec![owner_id.to_string()],
        name: name.to_string(),
        info: Map::<String, Value>::new(),
    }
}

pub fn create_application(name: &str, host: &str, unit_id: &str) -> Application {
    let now = Utc::now();
    Application {
        application_id: name.to_string(),
        code: name.to_string(),
        unit_id: unit_id.to_string(),
        unit_code: unit_id.to_string(),
        created_at: now,
        modified_at: now,
        host_uri: host.to_string(),
        name: name.to_string(),
        info: Map::<String, Value>::new(),
    }
}

/// Empty unit_id means public network.
pub fn create_network(name: &str, host: &str, unit_id: &str) -> Network {
    let now = Utc::now();
    Network {
        network_id: name.to_string(),
        code: name.to_string(),
        unit_id: match unit_id.len() {
            0 => None,
            _ => Some(unit_id.to_string()),
        },
        unit_code: match unit_id.len() {
            0 => None,
            _ => Some(unit_id.to_string()),
        },
        created_at: now,
        modified_at: now,
        host_uri: host.to_string(),
        name: name.to_string(),
        info: Map::<String, Value>::new(),
    }
}

pub fn create_device(unit: &str, network: &str, addr: &str, is_public: bool) -> Device {
    let now = Utc::now();
    Device {
        device_id: addr.to_string(),
        unit_id: unit.to_string(),
        unit_code: match is_public {
            false => None,
            true => Some(unit.to_string()),
        },
        network_id: network.to_string(),
        network_code: network.to_string(),
        network_addr: addr.to_string(),
        created_at: now,
        modified_at: now,
        profile: "".to_string(),
        name: addr.to_string(),
        info: Map::<String, Value>::new(),
    }
}

pub fn create_device_route(
    id: &str,
    unit: &str,
    application: &str,
    network: &str,
    addr: &str,
) -> DeviceRoute {
    let now = Utc::now();
    DeviceRoute {
        route_id: id.to_string(),
        unit_id: unit.to_string(),
        unit_code: unit.to_string(),
        application_id: application.to_string(),
        application_code: application.to_string(),
        network_id: network.to_string(),
        network_code: network.to_string(),
        device_id: addr.to_string(),
        network_addr: addr.to_string(),
        profile: "".to_string(),
        created_at: now,
        modified_at: now,
    }
}

pub fn create_network_route(
    id: &str,
    unit: &str,
    application: &str,
    network: &str,
) -> NetworkRoute {
    let now = Utc::now();
    NetworkRoute {
        route_id: id.to_string(),
        unit_id: unit.to_string(),
        unit_code: unit.to_string(),
        application_id: application.to_string(),
        application_code: application.to_string(),
        network_id: network.to_string(),
        network_code: network.to_string(),
        created_at: now,
    }
}

pub fn create_dldata_buffer(
    id: &str,
    unit: &str,
    application: &str,
    network: &str,
    addr: &str,
) -> DlDataBuffer {
    let now = Utc::now();
    let ts_nanos = match now.timestamp_nanos_opt() {
        None => i64::MAX,
        Some(ts) => ts,
    };
    DlDataBuffer {
        data_id: id.to_string(),
        unit_id: unit.to_string(),
        unit_code: unit.to_string(),
        application_id: application.to_string(),
        application_code: application.to_string(),
        network_id: network.to_string(),
        network_addr: addr.to_string(),
        device_id: addr.to_string(),
        created_at: now,
        expired_at: Utc.timestamp_nanos(ts_nanos + 3_600_000_000_000),
    }
}

pub fn create_users_tokens(state: &TestState) -> () {
    let runtime = state.runtime.as_ref().unwrap();
    let auth_db = state.auth_db.as_ref().unwrap();

    let now = Utc::now();

    let mut roles = HashMap::<String, bool>::new();
    roles.insert(Role::ADMIN.to_string(), true); // for client APIs
    roles.insert(Role::MANAGER.to_string(), true);
    let user = create_user("manager", now, roles);
    if let Err(e) = runtime.block_on(async { auth_db.user().add(&user).await }) {
        panic!("create manager error: {}", e);
    }

    let user = create_user("owner", now, HashMap::<String, bool>::new());
    if let Err(e) = runtime.block_on(async { auth_db.user().add(&user).await }) {
        panic!("create owner error: {}", e);
    }

    let user = create_user("member", now, HashMap::<String, bool>::new());
    if let Err(e) = runtime.block_on(async { auth_db.user().add(&user).await }) {
        panic!("create member error: {}", e);
    }

    let client = create_client("client", "manager", None);
    if let Err(e) = runtime.block_on(async { auth_db.client().add(&client).await }) {
        panic!("create client error: {}", e);
    }

    let token = create_token(TOKEN_MANAGER, "manager", "client");
    if let Err(e) = runtime.block_on(async { auth_db.access_token().add(&token).await }) {
        panic!("create manager token error: {}", e);
    }

    let token = create_token(TOKEN_OWNER, "owner", "client");
    if let Err(e) = runtime.block_on(async { auth_db.access_token().add(&token).await }) {
        panic!("create owner token error: {}", e);
    }

    let token = create_token(TOKEN_MEMBER, "member", "client");
    if let Err(e) = runtime.block_on(async { auth_db.access_token().add(&token).await }) {
        panic!("create member token error: {}", e);
    }
}

pub fn new_state(
    mqtt_engine: Option<&'static str>,
    data_channel_host: Option<&'static str>,
) -> TestState {
    let runtime = match Runtime::new() {
        Err(e) => panic!("create runtime error: {}", e),
        Ok(runtime) => runtime,
    };

    if mqtt_engine.is_none() {
        return TestState {
            runtime: Some(runtime),
            ..Default::default()
        };
    }

    let (api_key, api_secret) = match runtime.block_on(async {
        let _ = emqx::after_del_api_key().await;
        emqx::before_add_api_key().await
    }) {
        Err(e) => panic!("create API key error: {}", e),
        Ok(result) => (result.0, result.1),
    };

    let conf = Config {
        auth: Some(config::DEF_AUTH.to_string()),
        broker: Some(crate::TEST_BROKER_BASE.to_string()),
        mq: Some(config::Mq {
            engine: Some(config::Engine {
                amqp: Some(MqEngine::RABBITMQ.to_string()),
                mqtt: Some(mqtt_engine.unwrap().to_string()),
            }),
            rabbitmq: Some(config::RabbitMq {
                username: Some(crate::TEST_RABBITMQ_USER.to_string()),
                password: Some(crate::TEST_RABBITMQ_PASS.to_string()),
                ..Default::default()
            }),
            emqx: Some(config::Emqx {
                api_key: Some(api_key.clone()),
                api_secret: Some(api_secret.clone()),
                ..Default::default()
            }),
            rumqttd: Some(config::Rumqttd {
                mqtt_port: Some(crate::TEST_RUMQTTD_MQTT_PORT),
                mqtts_port: Some(crate::TEST_RUMQTTD_MQTTS_PORT),
                console_port: Some(crate::TEST_RUMQTTD_CONSOLE_PORT),
            }),
        }),
        mq_channels: match data_channel_host {
            None => None,
            Some(host) => Some(config::MqChannels {
                data: Some(config::CoremgrData {
                    url: Some(host.to_string()),
                    ..Default::default()
                }),
                ..Default::default()
            }),
        },
    };
    let state = match runtime.block_on(async { routes::new_state("/coremgr", &conf).await }) {
        Err(e) => panic!("create route state error: {}", e),
        Ok(state) => state,
    };

    let auth_state = match runtime.block_on(async {
        let mut path = std::env::temp_dir();
        path.push(sylvia_iot_auth_config::DEF_SQLITE_PATH);
        let conf = sylvia_iot_auth_config::Config {
            db: Some(sylvia_iot_auth_config::Db {
                engine: Some(DbEngine::SQLITE.to_string()),
                sqlite: Some(sylvia_iot_auth_config::Sqlite {
                    path: Some(path.to_str().unwrap().to_string()),
                }),
                ..Default::default()
            }),
            ..Default::default()
        };
        sylvia_iot_auth_routes::new_state("/auth", &conf).await
    }) {
        Err(e) => panic!("create auth state error: {}", e),
        Ok(state) => state,
    };
    let broker_state = match runtime.block_on(async {
        let mut path = std::env::temp_dir();
        path.push(sylvia_iot_broker_config::DEF_SQLITE_PATH);
        let conf = sylvia_iot_broker_config::Config {
            db: Some(sylvia_iot_broker_config::Db {
                engine: Some(DbEngine::SQLITE.to_string()),
                sqlite: Some(sylvia_iot_broker_config::Sqlite {
                    path: Some(path.to_str().unwrap().to_string()),
                }),
                ..Default::default()
            }),
            ..Default::default()
        };
        sylvia_iot_broker_routes::new_state("/broker", &conf).await
    }) {
        Err(e) => panic!("create broker state error: {}", e),
        Ok(state) => state,
    };

    let auth_broker_svc = runtime.spawn(async move {
        let app = Router::new()
            .merge(sylvia_iot_auth_routes::new_service(&auth_state))
            .merge(sylvia_iot_broker_routes::new_service(&broker_state));
        let listener = match TcpListener::bind("0.0.0.0:1080").await {
            Err(e) => panic!("bind auth/broker server error: {}", e),
            Ok(listener) => listener,
        };
        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .await
        .unwrap()
    });

    if let Err(e) = runtime.block_on(async {
        for _ in 0..WAIT_COUNT {
            if reqwest::get("http://localhost:1080").await.is_ok() {
                return Ok(());
            }
            time::sleep(Duration::from_millis(WAIT_TICK)).await;
        }
        Err("timeout")
    }) {
        panic!("create auth/broker server error: {}", e);
    }

    let auth_uri = Some(format!("{}/api/v1/auth/tokeninfo", config::DEF_AUTH));

    let auth_db = match runtime.block_on(async {
        let mut path = std::env::temp_dir();
        path.push(sylvia_iot_auth_config::DEF_SQLITE_PATH);
        sylvia_iot_auth_models::SqliteModel::new(&sylvia_iot_auth_models::SqliteOptions {
            path: path.to_str().unwrap().to_string(),
        })
        .await
    }) {
        Err(e) => panic!("create auth DB model error: {}", e),
        Ok(model) => Some(model),
    };
    let broker_db = match runtime.block_on(async {
        let mut path = std::env::temp_dir();
        path.push(sylvia_iot_broker_config::DEF_SQLITE_PATH);
        sylvia_iot_broker_models::SqliteModel::new(&sylvia_iot_broker_models::SqliteOptions {
            path: path.to_str().unwrap().to_string(),
        })
        .await
    }) {
        Err(e) => panic!("create broker DB model error: {}", e),
        Ok(model) => Some(model),
    };

    let rabbitmq_opts = RabbitMqOpts {
        username: crate::TEST_RABBITMQ_USER.to_string(),
        password: crate::TEST_RABBITMQ_PASS.to_string(),
        ttl: None,
        length: None,
    };
    let emqx_opts = EmqxOpts {
        api_key,
        api_secret,
    };
    let rumqttd_opts = RumqttdOpts {
        mqtt_port: Some(crate::TEST_RUMQTTD_MQTT_PORT),
        mqtts_port: Some(crate::TEST_RUMQTTD_MQTTS_PORT),
        console_port: Some(crate::TEST_RUMQTTD_CONSOLE_PORT),
    };
    let rumqttd_handles = match mqtt_engine {
        Some(MqEngine::RUMQTTD) => Some(rumqttd::start_rumqttd(
            &ServerConfig::default(),
            &rumqttd_opts,
        )),
        _ => None,
    };

    TestState {
        runtime: Some(runtime),
        auth_db,
        broker_db,
        auth_broker_svc: Some(auth_broker_svc),
        auth_uri,
        routes_state: Some(state),
        client: Some(reqwest::Client::new()),
        mq_opts: Some((rabbitmq_opts, emqx_opts, rumqttd_opts)),
        rumqttd_handles,
        ..Default::default()
    }
}

/// A utility function for [`test_invalid_param`].
pub fn new_test_server(state: &routes::State) -> Result<TestServer, String> {
    let app = Router::new().merge(routes::new_service(&state));
    match TestServer::new(app) {
        Err(e) => Err(format!("new server error: {}", e)),
        Ok(server) => Ok(server),
    }
}

pub fn test_invalid_token(
    runtime: &Runtime,
    state: &routes::State,
    method: Method,
    uri: &str,
) -> Result<(), String> {
    let server = new_test_server(state)?;

    let req = server.method(method, uri).add_header(
        header::AUTHORIZATION,
        HeaderValue::from_str("Bearer token").unwrap(),
    );
    let resp = runtime.block_on(async { req.await });
    expect(resp.status_code()).to_equal(StatusCode::UNAUTHORIZED)
}

pub fn test_invalid_param(
    runtime: &Runtime,
    req: TestRequest,
    expect_code: &str,
) -> Result<(), String> {
    let resp = runtime.block_on(async { req.await });
    match expect_code {
        "err_not_found" => expect(resp.status_code()).to_equal(StatusCode::NOT_FOUND)?,
        _ => expect(resp.status_code()).to_equal(StatusCode::BAD_REQUEST)?,
    }
    match serde_json::from_str::<ApiError>(resp.text().as_str()) {
        Err(e) => Err(format!("response error format error: {}", e)),
        Ok(err) => match err.code.as_str() == expect_code {
            false => Err(format!(
                "error code {} not equal to {}",
                err.code, expect_code
            )),
            true => Ok(()),
        },
    }
}

pub fn test_count(
    runtime: &Runtime,
    state: &routes::State,
    uri: &str,
    query: &[(&str, &str)],
    token: &str,
    count: usize,
) -> Result<(), String> {
    let server = new_test_server(state)?;

    let req = server
        .method(Method::GET, uri)
        .add_query_params(query)
        .add_header(
            header::AUTHORIZATION,
            HeaderValue::from_str(format!("Bearer {}", token).as_str()).unwrap(),
        );
    let resp = runtime.block_on(async { req.await });
    expect(resp.status_code()).to_equal(StatusCode::OK)?;
    let body = resp.text();
    match uri.contains("/list") {
        false => match serde_json::from_str::<GetCountRes>(body.as_str()) {
            Err(e) => Err(format!("count format error: {}", e)),
            Ok(res) => expect(count).equals(res.data.count),
        },
        true => match serde_json::from_str::<GetListRes>(body.as_str()) {
            Err(e) => Err(format!("list format error: {}", e)),
            Ok(res) => expect(count).equals(res.data.len()),
        },
    }
}

pub fn test_list(
    runtime: &Runtime,
    state: &routes::State,
    uri: &str,
    token: &str,
    fields: &str,
) -> Result<(), String> {
    let server = new_test_server(state)?;

    let req = server.get(uri).add_header(
        header::AUTHORIZATION,
        HeaderValue::from_str(format!("Bearer {}", token).as_str()).unwrap(),
    );
    let resp = runtime.block_on(async { req.await });
    expect(resp.status_code()).to_equal(StatusCode::OK)?;
    let list = match serde_json::from_str::<GetListRes>(resp.text().as_str()) {
        Err(e) => return Err(format!("list format error: {}", e)),
        Ok(list) => list.data,
    };
    expect(list.len() == 100).to_equal(true)?;

    let req = server.get(uri).add_query_param("limit", 0).add_header(
        header::AUTHORIZATION,
        HeaderValue::from_str(format!("Bearer {}", token).as_str()).unwrap(),
    );
    let resp = runtime.block_on(async { req.await });
    expect(resp.status_code()).to_equal(StatusCode::OK)?;
    let list = match serde_json::from_str::<GetListRes>(resp.text().as_str()) {
        Err(e) => return Err(format!("list format error: {}", e)),
        Ok(list) => list.data,
    };
    expect(list.len() > 100).to_equal(true)?;

    let req = server
        .get(uri)
        .add_query_param("limit", 0)
        .add_query_param("format", "array")
        .add_header(
            header::AUTHORIZATION,
            HeaderValue::from_str(format!("Bearer {}", token).as_str()).unwrap(),
        );
    let resp = runtime.block_on(async { req.await });
    expect(resp.status_code()).to_equal(StatusCode::OK)?;
    let list = match serde_json::from_str::<Vec<Data>>(resp.text().as_str()) {
        Err(e) => return Err(format!("list format error: {}", e)),
        Ok(list) => list,
    };
    expect(list.len() > 100).to_equal(true)?;

    let req = server
        .get(uri)
        .add_query_param("limit", 0)
        .add_query_param("format", "csv")
        .add_header(
            header::AUTHORIZATION,
            HeaderValue::from_str(format!("Bearer {}", token).as_str()).unwrap(),
        );
    let resp = runtime.block_on(async { req.await });
    expect(resp.status_code()).to_equal(StatusCode::OK)?;
    let mut count = 0;
    for line in resp.text().lines() {
        if count == 0 {
            let mut fields_line: Vec<u8> = vec![0xEF, 0xBB, 0xBF];
            fields_line.extend_from_slice(fields.as_bytes());
            expect(fields_line.as_slice()).equals(line.as_bytes())?;
        }
        count += 1;
    }
    expect(list.len() + 1).to_equal(count)?;

    let req = server
        .get(uri)
        .add_query_param("format", "test")
        .add_header(
            header::AUTHORIZATION,
            HeaderValue::from_str(format!("Bearer {}", token).as_str()).unwrap(),
        );
    let resp = runtime.block_on(async { req.await });
    expect(resp.status_code()).to_equal(StatusCode::BAD_REQUEST)?;
    match serde_json::from_str::<ApiError>(resp.text().as_str()) {
        Err(e) => Err(format!("response error format error: {}", e)),
        Ok(err) => match err.code.as_str() == "err_param" {
            false => Err(format!("error code {} not equal to err_param", err.code)),
            true => Ok(()),
        },
    }
}
