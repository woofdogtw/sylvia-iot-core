use std::{collections::HashMap, error::Error as StdError, net::SocketAddr, time::Duration};

use axum::Router;
use chrono::{DateTime, Utc};
use laboratory::{Suite, describe};
use serde_json::{Map, Value};
use tokio::{net::TcpListener, runtime::Runtime, time};

use sylvia_iot_auth::{
    libs::config as sylvia_iot_broker_config,
    models::{self as sylvia_iot_broker_models},
    routes as sylvia_iot_broker_routes,
};
use sylvia_iot_auth::{
    libs::config as sylvia_iot_auth_config,
    models::{self as sylvia_iot_auth_models, Model, client::Client, user::User},
    routes as sylvia_iot_auth_routes,
};
use sylvia_iot_coremgr::{
    libs::config as sylvia_iot_coremgr_config, routes as sylvia_iot_coremgr_routes,
};

use crate::{TestState, WAIT_COUNT, WAIT_TICK};

const STATE: &'static str = "api";
const USER_ID: &'static str = "user";
const CLIENT_ID: &'static str = "client";
const CLIENT_SECRET: &'static str = "secret";

mod http;
mod user;

pub fn suite() -> Suite<TestState> {
    describe("api", |context| {
        context.describe_import(http::suite());
        context.describe_import(user::suite());
    })
}

fn before_all_fn(state: &mut HashMap<&'static str, TestState>) -> () {
    let runtime = match Runtime::new() {
        Err(e) => panic!("create runtime error: {}", e),
        Ok(runtime) => runtime,
    };

    let auth_state = match runtime.block_on(async {
        let mut path = std::env::temp_dir();
        path.push(sylvia_iot_auth_config::DEF_SQLITE_PATH);
        let conf = sylvia_iot_auth_config::Config {
            db: Some(sylvia_iot_auth_config::Db {
                engine: Some("sqlite".to_string()),
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
            db: Some(sylvia_iot_auth_config::Db {
                engine: Some("sqlite".to_string()),
                sqlite: Some(sylvia_iot_auth_config::Sqlite {
                    path: Some(path.to_str().unwrap().to_string()),
                }),
                ..Default::default()
            }),
            ..Default::default()
        };
        sylvia_iot_broker_routes::new_state("/broker", &conf).await
    }) {
        Err(e) => panic!("create auth state error: {}", e),
        Ok(state) => state,
    };
    let coremgr_state = match runtime.block_on(async {
        let conf = sylvia_iot_coremgr_config::Config {
            auth: Some(crate::TEST_AUTH_BASE.to_string()),
            broker: Some(crate::TEST_BROKER_BASE.to_string()),
            ..Default::default()
        };
        sylvia_iot_coremgr_routes::new_state("/coremgr", &conf).await
    }) {
        Err(e) => panic!("create auth state error: {}", e),
        Ok(state) => state,
    };

    let core_svc = runtime.spawn(async move {
        let app = Router::new()
            .merge(sylvia_iot_auth_routes::new_service(&auth_state))
            .merge(sylvia_iot_broker_routes::new_service(&broker_state))
            .merge(sylvia_iot_coremgr_routes::new_service(&coremgr_state));
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
        panic!("create auth server error: {}", e);
    }

    let auth_uri = Some(format!("{}/api/v1/auth/tokeninfo", crate::TEST_AUTH_BASE));

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

    let result: Result<(), Box<dyn StdError>> = runtime.block_on(async {
        let now = Utc::now();
        let auth_db = auth_db.as_ref().unwrap();
        let user = create_user(USER_ID, now, HashMap::<String, bool>::new());
        auth_db.user().add(&user).await?;
        let client = create_client(CLIENT_ID, USER_ID, Some(CLIENT_SECRET.to_string()));
        auth_db.client().add(&client).await?;
        Ok(())
    });
    if let Err(e) = result {
        panic!("create user/client error: {}", e);
    }

    state.insert(
        STATE,
        TestState {
            runtime: Some(runtime),
            auth_db,
            broker_db,
            core_svc: Some(core_svc),
            auth_uri,
            ..Default::default()
        },
    );
}

fn after_all_fn(state: &mut HashMap<&'static str, TestState>) -> () {
    let state = state.get_mut(STATE).unwrap();

    stop_core_svc(state);
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

fn stop_core_svc(state: &TestState) {
    if let Some(svc) = state.core_svc.as_ref() {
        svc.abort();
    }
    let mut path = std::env::temp_dir();
    path.push(sylvia_iot_auth_config::DEF_SQLITE_PATH);
    remove_sqlite(path.to_str().unwrap());
    let mut path = std::env::temp_dir();
    path.push(sylvia_iot_broker_config::DEF_SQLITE_PATH);
    remove_sqlite(path.to_str().unwrap());
}

fn create_user(name: &str, time: DateTime<Utc>, roles: HashMap<String, bool>) -> User {
    User {
        user_id: name.to_string(),
        account: name.to_string(),
        created_at: time,
        modified_at: time,
        verified_at: Some(time),
        expired_at: None,
        disabled_at: None,
        roles,
        password: "password".to_string(),
        salt: name.to_string(),
        name: name.to_string(),
        info: Map::<String, Value>::new(),
    }
}

fn create_client(name: &str, user_id: &str, secret: Option<String>) -> Client {
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
