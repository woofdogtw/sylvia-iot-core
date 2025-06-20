//! Program configurations.

use std::env;

use clap::{Arg, ArgMatches, Command, builder::RangedU64ValueParser};
use serde::Deserialize;

use sylvia_iot_corelib::constants::DbEngine;

/// Configuration file object.
#[derive(Default, Deserialize)]
pub struct Config {
    /// **sylvia-iot-auth** API base path with host. For example: `http://localhost:1080/auth`.
    pub auth: Option<String>,
    /// **sylvia-iot-broker** API base path with host. For example: `http://localhost:2080/broker`.
    pub broker: Option<String>,
    pub db: Option<Db>,
    #[serde(rename = "mqChannels")]
    pub mq_channels: Option<MqChannels>,
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

/// Message channels configuration object.
#[derive(Default, Deserialize)]
pub struct MqChannels {
    pub broker: Option<DataData>,
    pub coremgr: Option<DataData>,
}

/// Channel `broker.data` `coremgr.data` configuration object.
#[derive(Default, Deserialize)]
pub struct DataData {
    /// Queue connection URL of the data channel.
    pub url: Option<String>,
    /// AMQP QoS prefetch from **1** to **65535**. None or zero use default value **100**.
    pub prefetch: Option<u16>,
    /// MQTT shared subscription topic prefix.
    #[serde(rename = "sharedPrefix")]
    pub shared_prefix: Option<String>,
}

pub const DEF_AUTH: &'static str = "http://localhost:1080/auth";
pub const DEF_BROKER: &'static str = "http://localhost:2080/broker";
pub const DEF_ENGINE: &'static str = DbEngine::SQLITE;
pub const DEF_MONGODB_URL: &'static str = "mongodb://localhost:27017";
pub const DEF_MONGODB_DB: &'static str = "data";
pub const DEF_SQLITE_PATH: &'static str = "data.db";
pub const DEF_MQ_PREFETCH: u16 = 100;
pub const DEF_MQ_SHAREDPREFIX: &'static str = "$share/sylvia-iot-data/";
pub const DEF_MQ_CHANNEL_URL: &'static str = "amqp://localhost";

/// To register Clap arguments.
pub fn reg_args(cmd: Command) -> Command {
    cmd.arg(
        Arg::new("data.auth")
            .long("data.auth")
            .help("sylvia-iot-auth host (ex: http://localhost:1080/auth)")
            .num_args(1),
    )
    .arg(
        Arg::new("data.broker")
            .long("data.broker")
            .help("sylvia-iot-broker host (ex: http://localhost:2080/broker)")
            .num_args(1),
    )
    .arg(
        Arg::new("data.db.engine")
            .long("data.db.engine")
            .help("database engine")
            .num_args(1)
            .value_parser([DbEngine::MONGODB, DbEngine::SQLITE]),
    )
    .arg(
        Arg::new("data.db.mongodb.url")
            .long("data.db.mongodb.url")
            .help("MongoDB URL (scheme://[username][:password][@][host][:port]")
            .num_args(1),
    )
    .arg(
        Arg::new("data.db.mongodb.database")
            .long("data.db.mongodb.database")
            .help("database nane")
            .num_args(1),
    )
    .arg(
        Arg::new("data.db.mongodb.poolsize")
            .long("data.db.mongodb.poolsize")
            .help("connection pool size")
            .num_args(1)
            .value_parser(RangedU64ValueParser::<u64>::new().range(1..=u32::MAX as u64)),
    )
    .arg(
        Arg::new("data.db.sqlite.path")
            .long("data.db.sqlite.path")
            .help("SQLite path")
            .num_args(1),
    )
    .arg(
        Arg::new("data.mq-channels.broker.url")
            .long("data.mq-channels.broker.url")
            .help("URL of `broker.data` channel")
            .num_args(1),
    )
    .arg(
        Arg::new("data.mq-channels.broker.prefetch")
            .long("data.mq-channels.broker.prefetch")
            .help("AMQP prefetch for `broker.data` channel")
            .num_args(1)
            .value_parser(RangedU64ValueParser::<u64>::new().range(1..=u16::MAX as u64)),
    )
    .arg(
        Arg::new("data.mq-channels.broker.sharedprefix")
            .long("data.mq-channels.broker.sharedprefix")
            .help("MQTT shared subscription prefix of `broker.data` channel")
            .num_args(1),
    )
    .arg(
        Arg::new("data.mq-channels.coremgr.url")
            .long("data.mq-channels.coremgr.url")
            .help("URL of `coremgr.data` channel")
            .num_args(1),
    )
    .arg(
        Arg::new("data.mq-channels.coremgr.prefetch")
            .long("data.mq-channels.coremgr.prefetch")
            .help("AMQP prefetch for `coremgr.data` channel")
            .num_args(1)
            .value_parser(RangedU64ValueParser::<u64>::new().range(1..=u16::MAX as u64)),
    )
    .arg(
        Arg::new("data.mq-channels.coremgr.sharedprefix")
            .long("data.mq-channels.coremgr.sharedprefix")
            .help("MQTT shared subscription prefix of `coremgr.data` channel")
            .num_args(1),
    )
}

/// To read input arguments from command-line arguments and environment variables.
///
/// This function will call [`apply_default()`] to fill missing values so you do not need call it
/// again.
pub fn read_args(args: &ArgMatches) -> Config {
    apply_default(&Config {
        auth: match args.get_one::<String>("data.auth") {
            None => match env::var("DATA_AUTH") {
                Err(_) => None,
                Ok(v) => Some(v),
            },
            Some(v) => Some(v.clone()),
        },
        broker: match args.get_one::<String>("data.broker") {
            None => match env::var("DATA_BROKER") {
                Err(_) => None,
                Ok(v) => Some(v),
            },
            Some(v) => Some(v.clone()),
        },
        db: Some(Db {
            engine: match args.get_one::<String>("data.db.engine") {
                None => match env::var("DATA_DB_ENGINE") {
                    Err(_) => None,
                    Ok(v) => Some(v),
                },
                Some(v) => Some(v.clone()),
            },
            mongodb: Some(MongoDb {
                url: match args.get_one::<String>("data.db.mongodb.url") {
                    None => match env::var("DATA_DB_MONGODB_URL") {
                        Err(_) => None,
                        Ok(v) => Some(v),
                    },
                    Some(v) => Some(v.clone()),
                },
                database: match args.get_one::<String>("data.db.mongodb.database") {
                    None => match env::var("DATA_DB_MONGODB_DATABASE") {
                        Err(_) => None,
                        Ok(v) => Some(v),
                    },
                    Some(v) => Some(v.clone()),
                },
                pool_size: match args.get_one::<u64>("data.db.mongodb.poolsize") {
                    None => match env::var("DATA_DB_MONGODB_POOLSIZE") {
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
                path: match args.get_one::<String>("data.db.sqlite.path") {
                    None => match env::var("DATA_DB_SQLITE_PATH") {
                        Err(_) => None,
                        Ok(v) => Some(v),
                    },
                    Some(v) => Some(v.clone()),
                },
            }),
        }),
        mq_channels: Some(MqChannels {
            broker: Some(DataData {
                url: match args.get_one::<String>("data.mq-channels.broker.url") {
                    None => match env::var("DATA_MQCHANNELS_BROKER_URL") {
                        Err(_) => None,
                        Ok(v) => Some(v),
                    },
                    Some(v) => Some(v.clone()),
                },
                prefetch: match args.get_one::<u64>("data.mq-channels.broker.prefetch") {
                    None => match env::var("DATA_MQCHANNELS_BROKER_PREFETCH") {
                        Err(_) => None,
                        Ok(v) => match v.parse::<u16>() {
                            Err(_) => None,
                            Ok(v) => Some(v),
                        },
                    },
                    Some(v) => Some(*v as u16),
                },
                shared_prefix: match args.get_one::<String>("data.mq-channels.broker.sharedprefix")
                {
                    None => match env::var("DATA_MQCHANNELS_BROKER_SHAREDPREFIX") {
                        Err(_) => None,
                        Ok(v) => Some(v),
                    },
                    Some(v) => Some(v.clone()),
                },
            }),
            coremgr: Some(DataData {
                url: match args.get_one::<String>("data.mq-channels.coremgr.url") {
                    None => match env::var("DATA_MQCHANNELS_COREMGR_URL") {
                        Err(_) => None,
                        Ok(v) => Some(v),
                    },
                    Some(v) => Some(v.clone()),
                },
                prefetch: match args.get_one::<u64>("data.mq-channels.coremgr.prefetch") {
                    None => match env::var("DATA_MQCHANNELS_COREMGR_PREFETCH") {
                        Err(_) => None,
                        Ok(v) => match v.parse::<u16>() {
                            Err(_) => None,
                            Ok(v) => Some(v),
                        },
                    },
                    Some(v) => Some(*v as u16),
                },
                shared_prefix: match args.get_one::<String>("data.mq-channels.coremgr.sharedprefix")
                {
                    None => match env::var("DATA_MQCHANNELS_COREMGR_SHAREDPREFIX") {
                        Err(_) => None,
                        Ok(v) => Some(v),
                    },
                    Some(v) => Some(v.clone()),
                },
            }),
        }),
    })
}

/// Fill missing configuration with default values.
pub fn apply_default(config: &Config) -> Config {
    Config {
        auth: match config.auth.as_ref() {
            None => Some(DEF_AUTH.to_string()),
            Some(auth) => Some(auth.clone()),
        },
        broker: match config.broker.as_ref() {
            None => Some(DEF_BROKER.to_string()),
            Some(broker) => Some(broker.clone()),
        },
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
        mq_channels: match config.mq_channels.as_ref() {
            None => Some(MqChannels {
                broker: Some(DataData {
                    url: Some(DEF_MQ_CHANNEL_URL.to_string()),
                    prefetch: Some(DEF_MQ_PREFETCH),
                    shared_prefix: Some(DEF_MQ_SHAREDPREFIX.to_string()),
                }),
                coremgr: Some(DataData {
                    url: Some(DEF_MQ_CHANNEL_URL.to_string()),
                    prefetch: Some(DEF_MQ_PREFETCH),
                    shared_prefix: Some(DEF_MQ_SHAREDPREFIX.to_string()),
                }),
            }),
            Some(mq_channels) => Some(MqChannels {
                broker: match mq_channels.broker.as_ref() {
                    None => Some(DataData {
                        url: Some(DEF_MQ_CHANNEL_URL.to_string()),
                        prefetch: Some(DEF_MQ_PREFETCH),
                        shared_prefix: Some(DEF_MQ_SHAREDPREFIX.to_string()),
                    }),
                    Some(channel) => Some(DataData {
                        url: match channel.url.as_ref() {
                            None => Some(DEF_MQ_CHANNEL_URL.to_string()),
                            Some(url) => Some(url.to_string()),
                        },
                        prefetch: match channel.prefetch {
                            None => Some(DEF_MQ_PREFETCH),
                            Some(prefetch) => Some(prefetch),
                        },
                        shared_prefix: match channel.shared_prefix.as_ref() {
                            None => Some(DEF_MQ_SHAREDPREFIX.to_string()),
                            Some(shared_prefix) => Some(shared_prefix.to_string()),
                        },
                    }),
                },
                coremgr: match mq_channels.coremgr.as_ref() {
                    None => Some(DataData {
                        url: Some(DEF_MQ_CHANNEL_URL.to_string()),
                        prefetch: Some(DEF_MQ_PREFETCH),
                        shared_prefix: Some(DEF_MQ_SHAREDPREFIX.to_string()),
                    }),
                    Some(channel) => Some(DataData {
                        url: match channel.url.as_ref() {
                            None => Some(DEF_MQ_CHANNEL_URL.to_string()),
                            Some(url) => Some(url.to_string()),
                        },
                        prefetch: match channel.prefetch {
                            None => Some(DEF_MQ_PREFETCH),
                            Some(prefetch) => Some(prefetch),
                        },
                        shared_prefix: match channel.shared_prefix.as_ref() {
                            None => Some(DEF_MQ_SHAREDPREFIX.to_string()),
                            Some(shared_prefix) => Some(shared_prefix.to_string()),
                        },
                    }),
                },
            }),
        },
    }
}
