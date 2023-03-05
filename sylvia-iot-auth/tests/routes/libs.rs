use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde_json::{Map, Value};
use tokio::runtime::Runtime;

use sylvia_iot_auth::{
    libs::config::{Config, Db as DbConfig, MongoDb as MongoDbConfig, Sqlite as SqliteConfig},
    models::{
        client::Client, user::User, MongoDbModel, MongoDbOptions, SqliteModel, SqliteOptions,
    },
    routes,
};
use sylvia_iot_corelib::{constants::DbEngine, strings};

use crate::TestState;

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
    let state = match runtime.block_on(async { routes::new_state("/auth", &conf).await }) {
        Err(e) => panic!("create route state error: {}", e),
        Ok(state) => state,
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
        mongodb,
        sqlite,
        routes_state: Some(state),
        ..Default::default()
    }
}
