use std::{collections::HashMap, net::SocketAddr, time::Duration};

use axum::{
    Router,
    http::{HeaderValue, Method, StatusCode, header},
};
use axum_test::TestServer;
use chrono::{DateTime, TimeZone, Utc};
use laboratory::expect;
use reqwest;
use serde::Deserialize;
use serde_json::{Map, Value};
use tokio::{net::TcpListener, runtime::Runtime, time};

use sylvia_iot_auth::{
    libs::config as sylvia_iot_auth_config,
    models::{
        self as sylvia_iot_auth_models, access_token::AccessToken, client::Client, user::User,
    },
    routes as sylvia_iot_auth_routes,
};
use sylvia_iot_broker::{
    libs::config::{
        self, Cache as CacheConfig, Config, Db as DbConfig, MongoDb as MongoDbConfig,
        Sqlite as SqliteConfig,
    },
    models::{
        MongoDbModel, MongoDbOptions, SqliteModel, SqliteOptions,
        application::{Application, QueryCond as ApplicationQueryCond},
        device::{Device, QueryCond as DeviceQueryCond},
        device_route::DeviceRoute,
        dldata_buffer::DlDataBuffer,
        network::{Network, QueryCond as NetworkQueryCond},
        network_route::NetworkRoute,
        unit::{QueryCond as UnitQueryCond, Unit},
    },
    routes,
};
use sylvia_iot_corelib::{constants::DbEngine, err, strings};

use crate::{TestState, WAIT_COUNT, WAIT_TICK};

#[derive(Deserialize)]
pub struct ApiError {
    pub code: String,
    pub message: Option<String>,
}

const DATA_EXPIRES: i64 = 3_600_000_000_000; // nanoseconds.

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
    let expires_at = Utc.timestamp_nanos((Utc::now().timestamp() + 3600) * 1000000000);
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

pub fn create_device(
    unit: &str,
    network: &str,
    addr: &str,
    is_public: bool,
    profile: &str,
) -> Device {
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
        profile: profile.to_string(),
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
    profile: &str,
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
        profile: profile.to_string(),
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
        expired_at: Utc.timestamp_nanos(ts_nanos + DATA_EXPIRES),
    }
}

pub fn add_unit_model(
    runtime: &Runtime,
    state: &routes::State,
    unit_id: &str,
    member_ids: Vec<&str>,
    user_id: &str,
) -> Result<(), String> {
    match runtime.block_on(async {
        let mut unit = create_unit(unit_id, user_id);
        for id in member_ids.iter() {
            unit.member_ids.push(id.to_string());
        }
        state.model.unit().add(&unit).await
    }) {
        Err(e) => Err(format!("add unit model info error: {}", e)),
        Ok(_) => Ok(()),
    }
}

pub fn add_application_model(
    runtime: &Runtime,
    state: &routes::State,
    unit_id: &str,
    application_id: &str,
    host: &str,
) -> Result<(), String> {
    match runtime.block_on(async {
        let application = create_application(application_id, host, unit_id);
        state.model.application().add(&application).await
    }) {
        Err(e) => Err(format!("add application model info error: {}", e)),
        Ok(_) => Ok(()),
    }
}

pub fn add_network_model(
    runtime: &Runtime,
    state: &routes::State,
    unit_id: &str,
    network_id: &str,
    host: &str,
) -> Result<(), String> {
    match runtime.block_on(async {
        let network = create_network(network_id, host, unit_id);
        state.model.network().add(&network).await
    }) {
        Err(e) => Err(format!("add network model info error: {}", e)),
        Ok(_) => Ok(()),
    }
}

pub fn add_device_model(
    runtime: &Runtime,
    state: &routes::State,
    unit_id: &str,
    network_id: &str,
    network_addr: &str,
    is_public: bool,
    profile: &str,
) -> Result<(), String> {
    match runtime.block_on(async {
        let device = create_device(unit_id, network_id, network_addr, is_public, profile);
        state.model.device().add(&device).await
    }) {
        Err(e) => Err(format!("add device model info error: {}", e)),
        Ok(_) => Ok(()),
    }
}

pub fn add_device_bulk_model(
    runtime: &Runtime,
    state: &routes::State,
    unit_id: &str,
    network_id: &str,
    network_addrs: &Vec<String>,
    is_public: bool,
    profile: &str,
) -> Result<(), String> {
    match runtime.block_on(async {
        let mut devices = vec![];
        for addr in network_addrs.iter() {
            devices.push(create_device(
                unit_id,
                network_id,
                addr.as_str(),
                is_public,
                profile,
            ));
        }
        state.model.device().add_bulk(&devices).await
    }) {
        Err(e) => Err(format!("add device model in bulk info error: {}", e)),
        Ok(_) => Ok(()),
    }
}

pub fn rm_device_bulk_model(
    runtime: &Runtime,
    state: &routes::State,
    unit_id: &str,
    network_id: &str,
    network_addrs: &Vec<String>,
) -> Result<(), String> {
    match runtime.block_on(async {
        let addrs: Vec<&str> = network_addrs.iter().map(|x| x.as_str()).collect();
        let cond = DeviceQueryCond {
            unit_id: Some(unit_id),
            network_id: Some(network_id),
            network_addrs: Some(&addrs),
            ..Default::default()
        };
        state.model.device().del(&cond).await
    }) {
        Err(e) => Err(format!("delete device model in bulk info error: {}", e)),
        Ok(_) => Ok(()),
    }
}

pub fn add_device_route_model(
    runtime: &Runtime,
    state: &routes::State,
    id: &str,
    unit_id: &str,
    application_id: &str,
    network_id: &str,
    network_addr: &str,
    profile: &str,
) -> Result<(), String> {
    match runtime.block_on(async {
        let route = create_device_route(
            id,
            unit_id,
            application_id,
            network_id,
            network_addr,
            profile,
        );
        state.model.device_route().add(&route).await
    }) {
        Err(e) => Err(format!("add device route model info error: {}", e)),
        Ok(_) => Ok(()),
    }
}

pub fn add_network_route_model(
    runtime: &Runtime,
    state: &routes::State,
    id: &str,
    unit_id: &str,
    application_id: &str,
    network_id: &str,
) -> Result<(), String> {
    match runtime.block_on(async {
        let route = create_network_route(id, unit_id, application_id, network_id);
        state.model.network_route().add(&route).await
    }) {
        Err(e) => Err(format!("add network route model info error: {}", e)),
        Ok(_) => Ok(()),
    }
}

pub fn add_dldata_buffer_model(
    runtime: &Runtime,
    state: &routes::State,
    data_id: &str,
    unit_id: &str,
    application_id: &str,
    network_id: &str,
    network_addr: &str,
) -> Result<(), String> {
    match runtime.block_on(async {
        let data = create_dldata_buffer(data_id, unit_id, application_id, network_id, network_addr);
        state.model.dldata_buffer().add(&data).await
    }) {
        Err(e) => Err(format!("add dldata buffer model info error: {}", e)),
        Ok(_) => Ok(()),
    }
}

pub fn add_delete_rsc(runtime: &Runtime, state: &routes::State) -> Result<(), String> {
    add_unit_model(runtime, state, "manager", vec![], "manager")?;
    add_unit_model(runtime, state, "owner", vec![], "owner")?;
    add_application_model(runtime, state, "manager", "manager", "amqp://host")?;
    add_application_model(runtime, state, "owner", "owner", "amqp://host")?;
    add_network_model(runtime, state, "", "public", "amqp://host")?;
    add_network_model(runtime, state, "manager", "manager", "amqp://host")?;
    add_network_model(runtime, state, "owner", "owner", "amqp://host")?;
    add_device_model(
        runtime,
        state,
        "manager",
        "public",
        "manager-public",
        true,
        "",
    )?;
    add_device_model(runtime, state, "manager", "manager", "manager", false, "")?;
    add_device_model(runtime, state, "owner", "public", "owner-public", true, "")?;
    add_device_model(runtime, state, "owner", "owner", "owner1", false, "")?;
    add_device_model(runtime, state, "owner", "owner", "owner2", true, "")?;
    add_network_route_model(
        runtime,
        state,
        "public-manager",
        "manager",
        "manager",
        "public",
    )?;
    add_network_route_model(
        runtime,
        state,
        "manager-manager",
        "manager",
        "manager",
        "manager",
    )?;
    add_network_route_model(runtime, state, "owner-owner", "owner", "owner", "owner")?;
    add_device_route_model(
        runtime,
        state,
        "manager-public-manager",
        "manager",
        "manager",
        "public",
        "manager-public",
        "",
    )?;
    add_device_route_model(
        runtime,
        state,
        "manager-manager",
        "manager",
        "manager",
        "manager",
        "manager",
        "",
    )?;
    add_device_route_model(
        runtime,
        state,
        "owner-public-owner",
        "owner",
        "owner",
        "public",
        "owner-public",
        "",
    )?;
    add_device_route_model(
        runtime,
        state,
        "owner1-owner",
        "owner",
        "owner",
        "owner",
        "owner1",
        "",
    )?;
    add_device_route_model(
        runtime,
        state,
        "owner2-owner",
        "owner",
        "owner",
        "owner",
        "owner2",
        "",
    )?;
    add_dldata_buffer_model(
        runtime,
        state,
        "manager-public-manager",
        "manager",
        "manager",
        "public",
        "manager-public",
    )?;
    add_dldata_buffer_model(
        runtime,
        state,
        "manager-manager",
        "manager",
        "manager",
        "manager",
        "manager",
    )?;
    add_dldata_buffer_model(
        runtime,
        state,
        "owner-public-owner",
        "owner",
        "owner",
        "public",
        "owner-public",
    )?;
    add_dldata_buffer_model(
        runtime,
        state,
        "owner1-owner",
        "owner",
        "owner",
        "owner",
        "owner1",
    )?;
    add_dldata_buffer_model(
        runtime,
        state,
        "owner2-owner",
        "owner",
        "owner",
        "owner",
        "owner2",
    )?;

    Ok(())
}

pub fn get_unit_model(
    runtime: &Runtime,
    state: &routes::State,
    unit_id: &str,
    should_exist: bool,
) -> Result<Option<Unit>, String> {
    match runtime.block_on(async {
        let cond = UnitQueryCond {
            unit_id: Some(unit_id),
            ..Default::default()
        };
        state.model.unit().get(&cond).await
    }) {
        Err(e) => return Err(format!("get unit model info error: {}", e)),
        Ok(unit) => {
            if should_exist && unit.is_none() {
                return Err(format!("should get unit {}", unit_id));
            } else if !should_exist && unit.is_some() {
                return Err(format!("should not get unit {}", unit_id));
            }
            Ok(unit)
        }
    }
}

pub fn get_application_model(
    runtime: &Runtime,
    state: &routes::State,
    application_id: &str,
    should_exist: bool,
) -> Result<Option<Application>, String> {
    match runtime.block_on(async {
        let cond = ApplicationQueryCond {
            application_id: Some(application_id),
            ..Default::default()
        };
        state.model.application().get(&cond).await
    }) {
        Err(e) => return Err(format!("get application model info error: {}", e)),
        Ok(application) => {
            if should_exist && application.is_none() {
                return Err(format!("should get application {}", application_id));
            } else if !should_exist && application.is_some() {
                return Err(format!("should not get application {}", application_id));
            }
            Ok(application)
        }
    }
}

pub fn get_network_model(
    runtime: &Runtime,
    state: &routes::State,
    network_id: &str,
    should_exist: bool,
) -> Result<Option<Network>, String> {
    match runtime.block_on(async {
        let cond = NetworkQueryCond {
            network_id: Some(network_id),
            ..Default::default()
        };
        state.model.network().get(&cond).await
    }) {
        Err(e) => return Err(format!("get network model info error: {}", e)),
        Ok(network) => {
            if should_exist && network.is_none() {
                return Err(format!("should get network {}", network_id));
            } else if !should_exist && network.is_some() {
                return Err(format!("should not get network {}", network_id));
            }
            Ok(network)
        }
    }
}

pub fn get_device_model(
    runtime: &Runtime,
    state: &routes::State,
    device_id: &str,
    should_exist: bool,
) -> Result<Option<Device>, String> {
    match runtime.block_on(async {
        let cond = DeviceQueryCond {
            device_id: Some(device_id),
            ..Default::default()
        };
        state.model.device().get(&cond).await
    }) {
        Err(e) => return Err(format!("get device model info error: {}", e)),
        Ok(device) => {
            if should_exist && device.is_none() {
                return Err(format!("should get device {}", device_id));
            } else if !should_exist && device.is_some() {
                return Err(format!("should not get device {}", device_id));
            }
            Ok(device)
        }
    }
}

pub fn get_device_route_model(
    runtime: &Runtime,
    state: &routes::State,
    route_id: &str,
    should_exist: bool,
) -> Result<Option<DeviceRoute>, String> {
    match runtime.block_on(async { state.model.device_route().get(route_id).await }) {
        Err(e) => return Err(format!("get device route model info error: {}", e)),
        Ok(route) => {
            if should_exist && route.is_none() {
                return Err(format!("should get device route {}", route_id));
            } else if !should_exist && route.is_some() {
                return Err(format!("should not get device route {}", route_id));
            }
            Ok(route)
        }
    }
}

pub fn get_network_route_model(
    runtime: &Runtime,
    state: &routes::State,
    route_id: &str,
    should_exist: bool,
) -> Result<Option<NetworkRoute>, String> {
    match runtime.block_on(async { state.model.network_route().get(route_id).await }) {
        Err(e) => return Err(format!("get network route model info error: {}", e)),
        Ok(route) => {
            if should_exist && route.is_none() {
                return Err(format!("should get network route {}", route_id));
            } else if !should_exist && route.is_some() {
                return Err(format!("should not get network route {}", route_id));
            }
            Ok(route)
        }
    }
}

pub fn get_dldata_buffer_model(
    runtime: &Runtime,
    state: &routes::State,
    data_id: &str,
    should_exist: bool,
) -> Result<Option<DlDataBuffer>, String> {
    match runtime.block_on(async { state.model.dldata_buffer().get(data_id).await }) {
        Err(e) => return Err(format!("get dldata buffer model info error: {}", e)),
        Ok(data) => {
            if should_exist && data.is_none() {
                return Err(format!("should get dldata buffer {}", data_id));
            } else if !should_exist && data.is_some() {
                return Err(format!("should not get dldata buffer {}", data_id));
            }
            Ok(data)
        }
    }
}

pub fn new_state(
    db_engine: Option<&'static str>,
    cache_engine: Option<&'static str>,
    data_channel_host: Option<&'static str>,
) -> TestState {
    let runtime = match Runtime::new() {
        Err(e) => panic!("create runtime error: {}", e),
        Ok(runtime) => runtime,
    };
    let client = reqwest::Client::new();

    if db_engine.is_none() {
        return TestState {
            runtime: Some(runtime),
            client: Some(client.clone()),
            ..Default::default()
        };
    }

    let mut sqlite_path = std::env::temp_dir();
    sqlite_path.push(crate::TEST_SQLITE_PATH);
    let conf = Config {
        auth: Some(config::DEF_AUTH.to_string()),
        db: Some(DbConfig {
            engine: Some(db_engine.unwrap().to_string()),
            mongodb: Some(MongoDbConfig {
                url: Some(crate::TEST_MONGODB_URL.to_string()),
                database: Some(crate::TEST_MONGODB_DB.to_string()),
                pool_size: None,
            }),
            sqlite: Some(SqliteConfig {
                path: Some(sqlite_path.to_str().unwrap().to_string()),
            }),
        }),
        cache: match cache_engine {
            None => None,
            Some(engine) => Some(CacheConfig {
                engine: Some(engine.to_string()),
                ..Default::default()
            }),
        },
        mq_channels: match data_channel_host {
            None => None,
            Some(host) => Some(config::MqChannels {
                data: Some(config::BrokerData {
                    url: Some(host.to_string()),
                    ..Default::default()
                }),
                ..Default::default()
            }),
        },
        ..Default::default()
    };
    let state = match runtime.block_on(async { routes::new_state("/broker", &conf).await }) {
        Err(e) => panic!("create route state error: {}", e),
        Ok(state) => state,
    };

    let svc_state = match runtime.block_on(async {
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

    let auth_svc = runtime.spawn(async move {
        let app = Router::new().merge(sylvia_iot_auth_routes::new_service(&svc_state));
        let listener = match TcpListener::bind("0.0.0.0:1080").await {
            Err(e) => panic!("bind auth server error: {}", e),
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
        panic!("create auth server error: {}", e);
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

    let mongodb = match db_engine {
        Some(DbEngine::MONGODB) => match runtime.block_on(async {
            MongoDbModel::new(&MongoDbOptions {
                url: crate::TEST_MONGODB_URL.to_string(),
                db: crate::TEST_MONGODB_DB.to_string(),
                pool_size: None,
            })
            .await
        }) {
            Err(e) => panic!("create mongodb model error: {}", e),
            Ok(model) => Some(model),
        },
        _ => None,
    };

    let sqlite = match db_engine {
        Some(DbEngine::MONGODB) => None,
        _ => match runtime.block_on(async {
            let mut path = std::env::temp_dir();
            path.push(crate::TEST_SQLITE_PATH);
            SqliteModel::new(&SqliteOptions {
                path: path.to_str().unwrap().to_string(),
            })
            .await
        }) {
            Err(e) => panic!("create sqlite model error: {}", e),
            Ok(model) => Some(model),
        },
    };

    TestState {
        runtime: Some(runtime),
        auth_db,
        auth_svc: Some(auth_svc),
        auth_uri,
        mongodb,
        sqlite,
        data_ch_host: match data_channel_host {
            None => None,
            Some(host) => Some(host.to_string()),
        },
        routes_state: Some(state),
        client: Some(client),
        test_values: Some(HashMap::new()),
        ..Default::default()
    }
}

pub fn test_invalid_perm(
    runtime: &Runtime,
    state: &routes::State,
    token: &str,
    method: Method,
    uri: &str,
) -> Result<(), String> {
    let app = Router::new().merge(routes::new_service(&state));
    let server = match TestServer::new(app) {
        Err(e) => return Err(format!("new server error: {}", e)),
        Ok(server) => server,
    };

    let req = server.method(method, uri).add_header(
        header::AUTHORIZATION,
        HeaderValue::from_str(format!("Bearer {}", token).as_str()).unwrap(),
    );
    let resp = runtime.block_on(async { req.await });
    expect(resp.status_code()).to_equal(StatusCode::FORBIDDEN)?;
    let body: ApiError = resp.json();
    if body.code.as_str() != err::E_PERM {
        return Err(format!("unexpected 403 error: {}", body.code.as_str()));
    }
    Ok(())
}

pub fn test_invalid_token(
    runtime: &Runtime,
    state: &routes::State,
    method: Method,
    uri: &str,
) -> Result<(), String> {
    let app = Router::new().merge(routes::new_service(&state));
    let server = match TestServer::new(app) {
        Err(e) => return Err(format!("new server error: {}", e)),
        Ok(server) => server,
    };

    let req = server.method(method, uri).add_header(
        header::AUTHORIZATION,
        HeaderValue::from_str("Bearer token").unwrap(),
    );
    let resp = runtime.block_on(async { req.await });
    expect(resp.status_code()).to_equal(StatusCode::UNAUTHORIZED)
}

pub fn test_get_400(
    runtime: &Runtime,
    state: &routes::State,
    token: &str,
    uri: &str,
    param: &Map<String, Value>,
    expect_code: &str,
) -> Result<(), String> {
    let app = Router::new().merge(routes::new_service(&state));
    let server = match TestServer::new(app) {
        Err(e) => return Err(format!("new server error: {}", e)),
        Ok(server) => server,
    };

    let req = server.get(uri).add_query_params(param).add_header(
        header::AUTHORIZATION,
        HeaderValue::from_str(format!("Bearer {}", token).as_str()).unwrap(),
    );
    let resp = runtime.block_on(async { req.await });
    expect(resp.status_code()).to_equal(StatusCode::BAD_REQUEST)?;
    let body: ApiError = resp.json();
    if body.code.as_str() != expect_code {
        return Err(format!(
            "unexpected 400 error: {}, not {}",
            body.code.as_str(),
            expect_code
        ));
    }
    Ok(())
}
