use std::{collections::HashMap, net::SocketAddr};

use axum::{
    http::{header, HeaderValue, StatusCode},
    Router,
};
use axum_test::TestServer;
use chrono::{DateTime, TimeZone, Utc};
use laboratory::expect;
use serde::Deserialize;
use serde_json::{Map, Value};
use tokio::{net::TcpListener, runtime::Runtime};

use sylvia_iot_auth::{
    libs::config as sylvia_iot_auth_config,
    models::{
        self as sylvia_iot_auth_models, access_token::AccessToken, client::Client, user::User,
    },
    routes as sylvia_iot_auth_routes,
};
use sylvia_iot_broker::{
    libs::config as sylvia_iot_broker_config,
    models::{self as sylvia_iot_broker_models, unit::Unit},
    routes as sylvia_iot_broker_routes,
};
use sylvia_iot_corelib::{constants::DbEngine, strings};
use sylvia_iot_data::{
    libs::config::{
        self, Config, Db as DbConfig, MongoDb as MongoDbConfig, Sqlite as SqliteConfig,
    },
    models::{MongoDbModel, MongoDbOptions, SqliteModel, SqliteOptions},
    routes,
};

use crate::TestState;

#[derive(Deserialize)]
pub struct ApiError {
    pub code: String,
    pub message: Option<String>,
}

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

pub fn new_state(db_engine: Option<&'static str>) -> TestState {
    let runtime = match Runtime::new() {
        Err(e) => panic!("create runtime error: {}", e),
        Ok(runtime) => runtime,
    };

    if db_engine.is_none() {
        return TestState {
            runtime: Some(runtime),
            ..Default::default()
        };
    }

    let mut sqlite_path = std::env::temp_dir();
    sqlite_path.push(crate::TEST_SQLITE_PATH);
    let conf = Config {
        auth: Some(config::DEF_AUTH.to_string()),
        broker: Some(crate::TEST_BROKER_BASE.to_string()),
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
        ..Default::default()
    };
    let state = match runtime.block_on(async { routes::new_state("/data", &conf).await }) {
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
        broker_db,
        auth_broker_svc: Some(auth_broker_svc),
        auth_uri,
        mongodb,
        sqlite,
        routes_state: Some(state),
        ..Default::default()
    }
}

pub fn test_invalid_token(
    runtime: &Runtime,
    state: &routes::State,
    uri: &str,
) -> Result<(), String> {
    let app = Router::new().merge(routes::new_service(&state));
    let server = match TestServer::new(app) {
        Err(e) => return Err(format!("new server error: {}", e)),
        Ok(server) => server,
    };

    let req = server.get(uri).add_header(
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
