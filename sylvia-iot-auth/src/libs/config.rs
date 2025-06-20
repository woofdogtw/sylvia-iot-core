//! Program configurations.

use std::{collections::HashMap, env};

use clap::{Arg, ArgMatches, Command, builder::RangedU64ValueParser};
use serde::Deserialize;
use serde_json;

use sylvia_iot_corelib::constants::DbEngine;

/// Configuration file object.
#[derive(Default, Deserialize)]
pub struct Config {
    pub db: Option<Db>,
    #[serde(rename = "apiScopes")]
    pub api_scopes: Option<HashMap<String, Vec<String>>>,
    pub templates: Option<HashMap<String, String>>,
}

/// Database configuration object.
#[derive(Default, Deserialize)]
pub struct Db {
    /// Select the model implementation.
    /// - `mongodb`: pure MongoDB.
    /// - `sqlite`: pure SQLite.
    pub engine: Option<String>,
    pub mongodb: Option<MongoDb>,
    pub sqlite: Option<Sqlite>,
}

/// MongoDB configuration object.
#[derive(Default, Deserialize)]
pub struct MongoDb {
    /// Use `mongodb://username:password@host:port` format.
    pub url: Option<String>,
    pub database: Option<String>,
    #[serde(rename = "poolSize")]
    pub pool_size: Option<u32>,
}

/// SQLite configuration object.
#[derive(Default, Deserialize)]
pub struct Sqlite {
    /// Use absolute/relative path.
    pub path: Option<String>,
}

pub const DEF_ENGINE: &'static str = DbEngine::SQLITE;
pub const DEF_MONGODB_URL: &'static str = "mongodb://localhost:27017";
pub const DEF_MONGODB_DB: &'static str = "auth";
pub const DEF_SQLITE_PATH: &'static str = "auth.db";

/// To register Clap arguments.
pub fn reg_args(cmd: Command) -> Command {
    cmd.arg(
        Arg::new("auth.db.engine")
            .long("auth.db.engine")
            .help("database engine")
            .num_args(1)
            .value_parser([DbEngine::MONGODB, DbEngine::SQLITE]),
    )
    .arg(
        Arg::new("auth.db.mongodb.url")
            .long("auth.db.mongodb.url")
            .help("MongoDB URL (scheme://[username][:password][@][host][:port]")
            .num_args(1),
    )
    .arg(
        Arg::new("auth.db.mongodb.database")
            .long("auth.db.mongodb.database")
            .help("database nane")
            .num_args(1),
    )
    .arg(
        Arg::new("auth.db.mongodb.poolsize")
            .long("auth.db.mongodb.poolsize")
            .help("connection pool size")
            .num_args(1)
            .value_parser(RangedU64ValueParser::<u64>::new().range(1..=u32::MAX as u64)),
    )
    .arg(
        Arg::new("auth.db.sqlite.path")
            .long("auth.db.sqlite.path")
            .help("SQLite path")
            .num_args(1),
    )
    .arg(
        Arg::new("auth.api-scopes")
            .long("auth.api-scopes")
            .help("API scopes")
            .num_args(0..),
    )
    .arg(
        Arg::new("auth.templates")
            .long("auth.templates")
            .help("Template paths for pages")
            .num_args(0..),
    )
}

/// To read input arguments from command-line arguments and environment variables.
///
/// This function will call [`apply_default()`] to fill missing values so you do not need call it
/// again.
pub fn read_args(args: &ArgMatches) -> Config {
    apply_default(&Config {
        db: Some(Db {
            engine: match args.get_one::<String>("auth.db.engine") {
                None => match env::var("AUTH_DB_ENGINE") {
                    Err(_) => None,
                    Ok(v) => Some(v),
                },
                Some(v) => Some(v.clone()),
            },
            mongodb: Some(MongoDb {
                url: match args.get_one::<String>("auth.db.mongodb.url") {
                    None => match env::var("AUTH_DB_MONGODB_URL") {
                        Err(_) => None,
                        Ok(v) => Some(v),
                    },
                    Some(v) => Some(v.clone()),
                },
                database: match args.get_one::<String>("auth.db.mongodb.database") {
                    None => match env::var("AUTH_DB_MONGODB_DATABASE") {
                        Err(_) => None,
                        Ok(v) => Some(v),
                    },
                    Some(v) => Some(v.clone()),
                },
                pool_size: match args.get_one::<u64>("auth.db.mongodb.poolsize") {
                    None => match env::var("AUTH_DB_MONGODB_POOLSIZE") {
                        Err(_) => None,
                        Ok(v) => match v.parse::<u32>() {
                            Err(_) => None,
                            Ok(v) => Some(v),
                        },
                    },
                    Some(v) => Some(*v as u32),
                },
            }),
            sqlite: Some(Sqlite {
                path: match args.get_one::<String>("auth.db.sqlite.path") {
                    None => match env::var("AUTH_DB_SQLITE_PATH") {
                        Err(_) => None,
                        Ok(v) => Some(v),
                    },
                    Some(v) => Some(v.clone()),
                },
            }),
        }),
        api_scopes: match args.get_one::<String>("auth.api-scopes") {
            None => match env::var("AUTH_API_SCOPES") {
                Err(_) => None,
                Ok(v) => match v.len() {
                    0 => None,
                    _ => match serde_json::from_str::<HashMap<String, Vec<String>>>(v.as_str()) {
                        Err(_) => None,
                        Ok(v) => Some(v),
                    },
                },
            },
            Some(v) => match v.len() {
                0 => None,
                _ => match serde_json::from_str::<HashMap<String, Vec<String>>>(v.as_str()) {
                    Err(_) => None,
                    Ok(v) => Some(v),
                },
            },
        },
        templates: match args.get_one::<String>("auth.templates") {
            None => match env::var("AUTH_TEMPLATES") {
                Err(_) => None,
                Ok(v) => match v.len() {
                    0 => None,
                    _ => match serde_json::from_str::<HashMap<String, String>>(v.as_str()) {
                        Err(_) => None,
                        Ok(v) => Some(v),
                    },
                },
            },
            Some(v) => match serde_json::from_str::<HashMap<String, String>>(v.as_str()) {
                Err(_) => None,
                Ok(v) => Some(v),
            },
        },
    })
}

/// Fill missing configuration with default values.
pub fn apply_default(config: &Config) -> Config {
    Config {
        db: match config.db.as_ref() {
            None => Some(Db {
                engine: Some(DEF_ENGINE.to_string()),
                mongodb: Some(MongoDb {
                    url: Some(DEF_MONGODB_URL.to_string()),
                    database: Some(DEF_MONGODB_DB.to_string()),
                    pool_size: None,
                }),
                sqlite: Some(Sqlite {
                    path: Some(DEF_SQLITE_PATH.to_string()),
                }),
            }),
            Some(db) => Some(Db {
                engine: match db.engine.as_ref() {
                    None => Some(DEF_ENGINE.to_string()),
                    Some(engine) => match engine.as_str() {
                        DbEngine::MONGODB => Some(DbEngine::MONGODB.to_string()),
                        DbEngine::SQLITE => Some(DbEngine::SQLITE.to_string()),
                        _ => Some(DEF_ENGINE.to_string()),
                    },
                },
                mongodb: match db.mongodb.as_ref() {
                    None => Some(MongoDb {
                        url: Some(DEF_MONGODB_URL.to_string()),
                        database: Some(DEF_MONGODB_DB.to_string()),
                        pool_size: None,
                    }),
                    Some(mongodb) => Some(MongoDb {
                        url: match mongodb.url.as_ref() {
                            None => Some(DEF_MONGODB_URL.to_string()),
                            Some(url) => Some(url.to_string()),
                        },
                        database: match mongodb.database.as_ref() {
                            None => Some(DEF_MONGODB_DB.to_string()),
                            Some(database) => Some(database.to_string()),
                        },
                        pool_size: mongodb.pool_size,
                    }),
                },
                sqlite: match db.sqlite.as_ref() {
                    None => Some(Sqlite {
                        path: Some(DEF_SQLITE_PATH.to_string()),
                    }),
                    Some(sqlite) => Some(Sqlite {
                        path: match sqlite.path.as_ref() {
                            None => Some(DEF_SQLITE_PATH.to_string()),
                            Some(path) => Some(path.to_string()),
                        },
                    }),
                },
            }),
        },
        api_scopes: match config.api_scopes.as_ref() {
            None => Some(HashMap::new()),
            Some(scopes) => Some(scopes.clone()),
        },
        templates: match config.templates.as_ref() {
            None => Some(HashMap::new()),
            Some(templates) => Some(templates.clone()),
        },
    }
}
