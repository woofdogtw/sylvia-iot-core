//! Program configurations.

use std::{collections::HashMap, env};

use clap::{builder::RangedU64ValueParser, Arg, ArgMatches, Command};
use serde::Deserialize;

use sylvia_iot_corelib::constants::{CacheEngine, DbEngine};

/// Configuration file object.
#[derive(Default, Deserialize)]
pub struct Config {
    /// **sylvia-iot-auth** API base path with host. For example: `http://localhost:1080/auth`.
    pub auth: Option<String>,
    pub db: Option<Db>,
    pub cache: Option<Cache>,
    pub mq: Option<Mq>,
    #[serde(rename = "mqChannels")]
    pub mq_channels: Option<MqChannels>,
    #[serde(rename = "apiScopes")]
    pub api_scopes: Option<HashMap<String, Vec<String>>>,
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

/// Cache configuration object.
#[derive(Default, Deserialize)]
pub struct Cache {
    /// Select the cache implementation.
    /// - `none`: disable cache.
    /// - `memory`: pure memory.
    pub engine: Option<String>,
    pub memory: Option<MemoryCache>,
}

/// Memory cache configuration object.
#[derive(Default, Deserialize)]
pub struct MemoryCache {
    /// Maximum number of device cache count.
    pub device: Option<usize>,
    /// Maximum number of device route cache count.
    #[serde(rename = "deviceRoute")]
    pub device_route: Option<usize>,
    /// Maximum number of network route cache count.
    #[serde(rename = "networkRoute")]
    pub network_route: Option<usize>,
}

/// Message queue configuration object.
#[derive(Default, Deserialize)]
pub struct Mq {
    /// AMQP QoS prefetch from **1** to **65535**. None or zero use default value **100**.
    pub prefetch: Option<u16>,
    /// MQTT shared subscription topic prefix.
    #[serde(rename = "sharedPrefix")]
    pub shared_prefix: Option<String>,
}

/// Message channels configuration object.
#[derive(Default, Deserialize)]
pub struct MqChannels {
    pub unit: Option<BrokerCtrl>,
    pub application: Option<BrokerCtrl>,
    pub network: Option<BrokerCtrl>,
    pub device: Option<BrokerCtrl>,
    #[serde(rename = "deviceRoute")]
    pub device_route: Option<BrokerCtrl>,
    #[serde(rename = "networkRoute")]
    pub network_route: Option<BrokerCtrl>,
    pub data: Option<BrokerData>,
}

/// Channel `broker.ctrl` configuration object.
#[derive(Default, Deserialize)]
pub struct BrokerCtrl {
    /// Queue connection URL of the control channel.
    pub url: Option<String>,
    /// AMQP QoS prefetch from **1** to **65535**. None or zero use default value **100**.
    pub prefetch: Option<u16>,
}

/// Channel `broker.data` configuration object.
#[derive(Default, Deserialize)]
pub struct BrokerData {
    /// Queue connection URL of the data channel.
    pub url: Option<String>,
}

pub const DEF_AUTH: &'static str = "http://localhost:1080/auth";
pub const DEF_ENGINE: &'static str = DbEngine::SQLITE;
pub const DEF_MONGODB_URL: &'static str = "mongodb://localhost:27017";
pub const DEF_MONGODB_DB: &'static str = "broker";
pub const DEF_SQLITE_PATH: &'static str = "broker.db";
pub const DEF_CACHE_ENGINE: &'static str = CacheEngine::NONE;
pub const DEF_MEMORY_DEVICE: usize = 1_000_000;
pub const DEF_MEMORY_DEVICE_STR: &'static str = "1000000";
pub const DEF_MEMORY_DEVICE_ROUTE: usize = 1_000_000;
pub const DEF_MEMORY_DEVICE_ROUTE_STR: &'static str = "1000000";
pub const DEF_MEMORY_NETWORK_ROUTE: usize = 1_000_000;
pub const DEF_MEMORY_NETWORK_ROUTE_STR: &'static str = "1000000";
pub const DEF_MQ_PREFETCH: u16 = 100;
pub const DEF_MQ_PREFETCH_STR: &'static str = "100";
pub const DEF_MQ_SHAREDPREFIX: &'static str = "$share/sylvia-iot-broker/";
pub const DEF_MQ_CHANNEL_URL: &'static str = "amqp://localhost";

/// To register Clap arguments.
pub fn reg_args(cmd: Command) -> Command {
    cmd.arg(
        Arg::new("broker.auth")
            .long("broker.auth")
            .help("sylvia-iot-auth host (ex: http://localhost:1080/auth)")
            .num_args(1)
            .default_value(DEF_AUTH),
    )
    .arg(
        Arg::new("broker.db.engine")
            .long("broker.db.engine")
            .help("database engine")
            .num_args(1)
            .value_parser([DbEngine::MONGODB, DbEngine::SQLITE])
            .default_value(DEF_ENGINE),
    )
    .arg(
        Arg::new("broker.db.mongodb.url")
            .long("broker.db.mongodb.url")
            .help("MongoDB URL (scheme://[username][:password][@][host][:port]")
            .num_args(1)
            .default_value(DEF_MONGODB_URL),
    )
    .arg(
        Arg::new("broker.db.mongodb.database")
            .long("broker.db.mongodb.database")
            .help("database nane")
            .num_args(1)
            .default_value(DEF_MONGODB_DB),
    )
    .arg(
        Arg::new("broker.db.mongodb.poolsize")
            .long("broker.db.mongodb.poolsize")
            .help("connection pool size")
            .num_args(1)
            .value_parser(RangedU64ValueParser::<u64>::new().range(1..=u32::MAX as u64))
            .default_value("1"),
    )
    .arg(
        Arg::new("broker.db.sqlite.path")
            .long("broker.db.sqlite.path")
            .help("SQLite path")
            .num_args(1)
            .default_value(DEF_SQLITE_PATH),
    )
    .arg(
        Arg::new("broker.cache.engine")
            .long("broker.cache.engine")
            .help("cache engine")
            .num_args(1)
            .value_parser([CacheEngine::MEMORY, CacheEngine::NONE])
            .default_value(DEF_CACHE_ENGINE),
    )
    .arg(
        Arg::new("broker.cache.memory.device")
            .long("broker.cache.memory.device")
            .help("Device cache size")
            .num_args(1)
            .value_parser(RangedU64ValueParser::<u64>::new().range(1..=usize::MAX as u64))
            .default_value(DEF_MEMORY_DEVICE_STR),
    )
    .arg(
        Arg::new("broker.cache.memory.device-route")
            .long("broker.cache.memory.device-route")
            .help("Device route cache size")
            .num_args(1)
            .value_parser(RangedU64ValueParser::<u64>::new().range(1..=usize::MAX as u64))
            .default_value(DEF_MEMORY_DEVICE_ROUTE_STR),
    )
    .arg(
        Arg::new("broker.cache.memory.network-route")
            .long("broker.cache.memory.network-route")
            .help("Network route cache size")
            .num_args(1)
            .value_parser(RangedU64ValueParser::<u64>::new().range(1..=usize::MAX as u64))
            .default_value(DEF_MEMORY_NETWORK_ROUTE_STR),
    )
    .arg(
        Arg::new("broker.mq.prefetch")
            .long("broker.mq.prefetch")
            .help("AMQP prefetch")
            .num_args(1)
            .value_parser(RangedU64ValueParser::<u64>::new().range(1..=u16::MAX as u64))
            .default_value(DEF_MQ_PREFETCH_STR),
    )
    .arg(
        Arg::new("broker.mq.sharedprefix")
            .long("broker.mq.sharedprefix")
            .help("MQTT shared subscription prefix")
            .num_args(1)
            .default_value(DEF_MQ_SHAREDPREFIX),
    )
    .arg(
        Arg::new("broker.mq-channels.unit.url")
            .long("broker.mq-channels.unit.url")
            .help("URL of `broker.ctrl.unit` channel")
            .num_args(1)
            .default_value(DEF_MQ_CHANNEL_URL),
    )
    .arg(
        Arg::new("broker.mq-channels.unit.prefetch")
            .long("broker.mq-channels.unit.prefetch")
            .help("AMQP prefetch for `broker.ctrl.unit` channel")
            .num_args(1)
            .value_parser(RangedU64ValueParser::<u64>::new().range(1..=u16::MAX as u64))
            .default_value(DEF_MQ_PREFETCH_STR),
    )
    .arg(
        Arg::new("broker.mq-channels.application.url")
            .long("broker.mq-channels.application.url")
            .help("URL of `broker.ctrl.application` channel")
            .num_args(1)
            .default_value(DEF_MQ_CHANNEL_URL),
    )
    .arg(
        Arg::new("broker.mq-channels.application.prefetch")
            .long("broker.mq-channels.application.prefetch")
            .help("AMQP prefetch for `broker.ctrl.application` channel")
            .num_args(1)
            .value_parser(RangedU64ValueParser::<u64>::new().range(1..=u16::MAX as u64))
            .default_value(DEF_MQ_PREFETCH_STR),
    )
    .arg(
        Arg::new("broker.mq-channels.network.url")
            .long("broker.mq-channels.network.url")
            .help("URL of `broker.ctrl.network` channel")
            .num_args(1)
            .default_value(DEF_MQ_CHANNEL_URL),
    )
    .arg(
        Arg::new("broker.mq-channels.network.prefetch")
            .long("broker.mq-channels.network.prefetch")
            .help("AMQP prefetch for `broker.ctrl.network` channel")
            .num_args(1)
            .value_parser(RangedU64ValueParser::<u64>::new().range(1..=u16::MAX as u64))
            .default_value(DEF_MQ_PREFETCH_STR),
    )
    .arg(
        Arg::new("broker.mq-channels.device.url")
            .long("broker.mq-channels.device.url")
            .help("URL of `broker.ctrl.device` channel")
            .num_args(1)
            .default_value(DEF_MQ_CHANNEL_URL),
    )
    .arg(
        Arg::new("broker.mq-channels.device.prefetch")
            .long("broker.mq-channels.device.prefetch")
            .help("AMQP prefetch for `broker.ctrl.device` channel")
            .num_args(1)
            .value_parser(RangedU64ValueParser::<u64>::new().range(1..=u16::MAX as u64))
            .default_value(DEF_MQ_PREFETCH_STR),
    )
    .arg(
        Arg::new("broker.mq-channels.device-route.url")
            .long("broker.mq-channels.device-route.url")
            .help("URL of `broker.ctrl.device-route` channel")
            .num_args(1)
            .default_value(DEF_MQ_CHANNEL_URL),
    )
    .arg(
        Arg::new("broker.mq-channels.device-route.prefetch")
            .long("broker.mq-channels.device-route.prefetch")
            .help("AMQP prefetch for `broker.ctrl.device-route` channel")
            .num_args(1)
            .value_parser(RangedU64ValueParser::<u64>::new().range(1..=u16::MAX as u64))
            .default_value(DEF_MQ_PREFETCH_STR),
    )
    .arg(
        Arg::new("broker.mq-channels.network-route.url")
            .long("broker.mq-channels.network-route.url")
            .help("URL of `broker.ctrl.network-route` channel")
            .num_args(1)
            .default_value(DEF_MQ_CHANNEL_URL),
    )
    .arg(
        Arg::new("broker.mq-channels.network-route.prefetch")
            .long("broker.mq-channels.network-route.prefetch")
            .help("AMQP prefetch for `broker.ctrl.network-route` channel")
            .num_args(1)
            .value_parser(RangedU64ValueParser::<u64>::new().range(1..=u16::MAX as u64))
            .default_value(DEF_MQ_PREFETCH_STR),
    )
    .arg(
        Arg::new("broker.mq-channels.data.url")
            .long("broker.mq-channels.data.url")
            .help("URL of `broker.data` channel")
            .num_args(1),
    )
}

/// To read input arguments from command-line arguments and environment variables.
///
/// This function will call [`apply_default()`] to fill missing values so you do not need call it
/// again.
pub fn read_args(args: &ArgMatches) -> Config {
    apply_default(&Config {
        auth: match args.get_one::<String>("broker.auth") {
            None => match env::var("BROKER_AUTH") {
                Err(_) => None,
                Ok(v) => Some(v),
            },
            Some(v) => Some(v.clone()),
        },
        db: Some(Db {
            engine: match args.get_one::<String>("broker.db.engine") {
                None => match env::var("BROKER_DB_ENGINE") {
                    Err(_) => None,
                    Ok(v) => Some(v),
                },
                Some(v) => Some(v.clone()),
            },
            mongodb: Some(MongoDb {
                url: match args.get_one::<String>("broker.db.mongodb.url") {
                    None => match env::var("BROKER_DB_MONGODB_URL") {
                        Err(_) => None,
                        Ok(v) => Some(v),
                    },
                    Some(v) => Some(v.clone()),
                },
                database: match args.get_one::<String>("broker.db.mongodb.database") {
                    None => match env::var("BROKER_DB_MONGODB_DATABASE") {
                        Err(_) => None,
                        Ok(v) => Some(v),
                    },
                    Some(v) => Some(v.clone()),
                },
                pool_size: match args.get_one::<u64>("broker.db.mongodb.poolsize") {
                    None => match env::var("BROKER_DB_MONGODB_POOLSIZE") {
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
                path: match args.get_one::<String>("broker.db.sqlite.path") {
                    None => match env::var("BROKER_DB_SQLITE_PATH") {
                        Err(_) => None,
                        Ok(v) => Some(v),
                    },
                    Some(v) => Some(v.clone()),
                },
            }),
        }),
        cache: Some(Cache {
            engine: match args.get_one::<String>("broker.cache.engine") {
                None => match env::var("BROKER_CACHE_ENGINE") {
                    Err(_) => None,
                    Ok(v) => Some(v),
                },
                Some(v) => Some(v.clone()),
            },
            memory: Some(MemoryCache {
                device: match args.get_one::<u64>("broker.cache.memory.device") {
                    None => match env::var("BROKER_MEMORY_DEVICE") {
                        Err(_) => None,
                        Ok(v) => match v.parse::<usize>() {
                            Err(_) => None,
                            Ok(v) => Some(v),
                        },
                    },
                    Some(v) => Some(*v as usize),
                },
                device_route: match args.get_one::<u64>("broker.cache.memory.device-route") {
                    None => match env::var("BROKER_MEMORY_DEVICE_ROUTE") {
                        Err(_) => None,
                        Ok(v) => match v.parse::<usize>() {
                            Err(_) => None,
                            Ok(v) => Some(v),
                        },
                    },
                    Some(v) => Some(*v as usize),
                },
                network_route: match args.get_one::<u64>("broker.cache.memory.network-route") {
                    None => match env::var("BROKER_MEMORY_NETWORK_ROUTE") {
                        Err(_) => None,
                        Ok(v) => match v.parse::<usize>() {
                            Err(_) => None,
                            Ok(v) => Some(v),
                        },
                    },
                    Some(v) => Some(*v as usize),
                },
            }),
        }),
        mq: Some(Mq {
            prefetch: match args.get_one::<u64>("broker.mq.prefetch") {
                None => match env::var("BROKER_MQ_PREFETCH") {
                    Err(_) => None,
                    Ok(v) => match v.parse::<u16>() {
                        Err(_) => None,
                        Ok(v) => Some(v),
                    },
                },
                Some(v) => Some(*v as u16),
            },
            shared_prefix: match args.get_one::<String>("broker.mq.sharedprefix") {
                None => match env::var("BROKER_MQ_SHAREDPREFIX") {
                    Err(_) => None,
                    Ok(v) => Some(v),
                },
                Some(v) => Some(v.clone()),
            },
        }),
        mq_channels: Some(MqChannels {
            unit: Some(BrokerCtrl {
                url: match args.get_one::<String>("broker.mq-channels.unit.url") {
                    None => match env::var("BROKER_MQCHANNELS_UNIT_URL") {
                        Err(_) => None,
                        Ok(v) => Some(v),
                    },
                    Some(v) => Some(v.clone()),
                },
                prefetch: match args.get_one::<u64>("broker.mq-channels.unit.prefetch") {
                    None => match env::var("BROKER_MQCHANNELS_UNIT_PREFETCH") {
                        Err(_) => None,
                        Ok(v) => match v.parse::<u16>() {
                            Err(_) => None,
                            Ok(v) => Some(v),
                        },
                    },
                    Some(v) => Some(*v as u16),
                },
            }),
            application: Some(BrokerCtrl {
                url: match args.get_one::<String>("broker.mq-channels.application.url") {
                    None => match env::var("BROKER_MQCHANNELS_APPLICATION_URL") {
                        Err(_) => None,
                        Ok(v) => Some(v),
                    },
                    Some(v) => Some(v.clone()),
                },
                prefetch: match args.get_one::<u64>("broker.mq-channels.application.prefetch") {
                    None => match env::var("BROKER_MQCHANNELS_APPLICATION_PREFETCH") {
                        Err(_) => None,
                        Ok(v) => match v.parse::<u16>() {
                            Err(_) => None,
                            Ok(v) => Some(v),
                        },
                    },
                    Some(v) => Some(*v as u16),
                },
            }),
            network: Some(BrokerCtrl {
                url: match args.get_one::<String>("broker.mq-channels.network.url") {
                    None => match env::var("BROKER_MQCHANNELS_NETWORK_URL") {
                        Err(_) => None,
                        Ok(v) => Some(v),
                    },
                    Some(v) => Some(v.clone()),
                },
                prefetch: match args.get_one::<u64>("broker.mq-channels.network.prefetch") {
                    None => match env::var("BROKER_MQCHANNELS_NETWORK_PREFETCH") {
                        Err(_) => None,
                        Ok(v) => match v.parse::<u16>() {
                            Err(_) => None,
                            Ok(v) => Some(v),
                        },
                    },
                    Some(v) => Some(*v as u16),
                },
            }),
            device: Some(BrokerCtrl {
                url: match args.get_one::<String>("broker.mq-channels.device.url") {
                    None => match env::var("BROKER_MQCHANNELS_DEVICE_URL") {
                        Err(_) => None,
                        Ok(v) => Some(v),
                    },
                    Some(v) => Some(v.clone()),
                },
                prefetch: match args.get_one::<u64>("broker.mq-channels.device.prefetch") {
                    None => match env::var("BROKER_MQCHANNELS_DEVICE_PREFETCH") {
                        Err(_) => None,
                        Ok(v) => match v.parse::<u16>() {
                            Err(_) => None,
                            Ok(v) => Some(v),
                        },
                    },
                    Some(v) => Some(*v as u16),
                },
            }),
            device_route: Some(BrokerCtrl {
                url: match args.get_one::<String>("broker.mq-channels.device-route.url") {
                    None => match env::var("BROKER_MQCHANNELS_DEVICE_ROUTE_URL") {
                        Err(_) => None,
                        Ok(v) => Some(v),
                    },
                    Some(v) => Some(v.clone()),
                },
                prefetch: match args.get_one::<u64>("broker.mq-channels.device-route.prefetch") {
                    None => match env::var("BROKER_MQCHANNELS_DEVICE_ROUTE_PREFETCH") {
                        Err(_) => None,
                        Ok(v) => match v.parse::<u16>() {
                            Err(_) => None,
                            Ok(v) => Some(v),
                        },
                    },
                    Some(v) => Some(*v as u16),
                },
            }),
            network_route: Some(BrokerCtrl {
                url: match args.get_one::<String>("broker.mq-channels.network-route.url") {
                    None => match env::var("BROKER_MQCHANNELS_NETWORK_ROUTE_URL") {
                        Err(_) => None,
                        Ok(v) => Some(v),
                    },
                    Some(v) => Some(v.clone()),
                },
                prefetch: match args.get_one::<u64>("broker.mq-channels.network-route.prefetch") {
                    None => match env::var("BROKER_MQCHANNELS_NETWORK_ROUTE_PREFETCH") {
                        Err(_) => None,
                        Ok(v) => match v.parse::<u16>() {
                            Err(_) => None,
                            Ok(v) => Some(v),
                        },
                    },
                    Some(v) => Some(*v as u16),
                },
            }),
            data: match args.get_one::<String>("broker.mq-channels.data.url") {
                None => match env::var("BROKER_MQCHANNELS_DATA_URL") {
                    Err(_) => None,
                    Ok(v) => Some(BrokerData { url: Some(v) }),
                },
                Some(v) => Some(BrokerData {
                    url: Some(v.clone()),
                }),
            },
        }),
        api_scopes: match args.get_one::<String>("broker.api-scopes") {
            None => match env::var("BROKER_API_SCOPES") {
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
    })
}

/// Fill missing configuration with default values.
pub fn apply_default(config: &Config) -> Config {
    Config {
        auth: match config.auth.as_ref() {
            None => Some(DEF_AUTH.to_string()),
            Some(auth) => Some(auth.clone()),
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
        cache: match config.cache.as_ref() {
            None => Some(Cache {
                engine: Some(DEF_CACHE_ENGINE.to_string()),
                memory: Some(MemoryCache {
                    device: Some(DEF_MEMORY_DEVICE),
                    device_route: Some(DEF_MEMORY_DEVICE_ROUTE),
                    network_route: Some(DEF_MEMORY_NETWORK_ROUTE),
                }),
            }),
            Some(cache) => Some(Cache {
                engine: match cache.engine.as_ref() {
                    None => Some(DEF_CACHE_ENGINE.to_string()),
                    Some(engine) => match engine.as_str() {
                        CacheEngine::MEMORY => Some(CacheEngine::MEMORY.to_string()),
                        _ => Some(DEF_CACHE_ENGINE.to_string()),
                    },
                },
                memory: match cache.memory.as_ref() {
                    None => Some(MemoryCache {
                        device: Some(DEF_MEMORY_DEVICE),
                        device_route: Some(DEF_MEMORY_DEVICE_ROUTE),
                        network_route: Some(DEF_MEMORY_NETWORK_ROUTE),
                    }),
                    Some(memory) => Some(MemoryCache {
                        device: match memory.device {
                            None | Some(0) => Some(DEF_MEMORY_DEVICE),
                            Some(v) => Some(v),
                        },
                        device_route: match memory.device_route {
                            None | Some(0) => Some(DEF_MEMORY_DEVICE_ROUTE),
                            Some(v) => Some(v),
                        },
                        network_route: match memory.network_route {
                            None | Some(0) => Some(DEF_MEMORY_NETWORK_ROUTE),
                            Some(v) => Some(v),
                        },
                    }),
                },
            }),
        },
        mq: match config.mq.as_ref() {
            None => Some(Mq {
                prefetch: Some(DEF_MQ_PREFETCH),
                shared_prefix: Some(DEF_MQ_SHAREDPREFIX.to_string()),
            }),
            Some(mq) => Some(Mq {
                prefetch: match mq.prefetch.as_ref() {
                    None | Some(0) => Some(DEF_MQ_PREFETCH),
                    Some(prefetch) => Some(*prefetch),
                },
                shared_prefix: match mq.shared_prefix.as_ref() {
                    None => Some(DEF_MQ_SHAREDPREFIX.to_string()),
                    Some(shared_prefix) => Some(shared_prefix.to_string()),
                },
            }),
        },
        mq_channels: match config.mq_channels.as_ref() {
            None => Some(MqChannels {
                unit: Some(BrokerCtrl {
                    url: Some(DEF_MQ_CHANNEL_URL.to_string()),
                    prefetch: Some(DEF_MQ_PREFETCH),
                }),
                application: Some(BrokerCtrl {
                    url: Some(DEF_MQ_CHANNEL_URL.to_string()),
                    prefetch: Some(DEF_MQ_PREFETCH),
                }),
                network: Some(BrokerCtrl {
                    url: Some(DEF_MQ_CHANNEL_URL.to_string()),
                    prefetch: Some(DEF_MQ_PREFETCH),
                }),
                device: Some(BrokerCtrl {
                    url: Some(DEF_MQ_CHANNEL_URL.to_string()),
                    prefetch: Some(DEF_MQ_PREFETCH),
                }),
                device_route: Some(BrokerCtrl {
                    url: Some(DEF_MQ_CHANNEL_URL.to_string()),
                    prefetch: Some(DEF_MQ_PREFETCH),
                }),
                network_route: Some(BrokerCtrl {
                    url: Some(DEF_MQ_CHANNEL_URL.to_string()),
                    prefetch: Some(DEF_MQ_PREFETCH),
                }),
                data: None,
            }),
            Some(mq_channels) => Some(MqChannels {
                unit: match mq_channels.unit.as_ref() {
                    None => Some(BrokerCtrl {
                        url: Some(DEF_MQ_CHANNEL_URL.to_string()),
                        prefetch: Some(DEF_MQ_PREFETCH),
                    }),
                    Some(channel) => Some(BrokerCtrl {
                        url: match channel.url.as_ref() {
                            None => Some(DEF_MQ_CHANNEL_URL.to_string()),
                            Some(url) => Some(url.to_string()),
                        },
                        prefetch: match channel.prefetch {
                            None => Some(DEF_MQ_PREFETCH),
                            Some(prefetch) => Some(prefetch),
                        },
                    }),
                },
                application: match mq_channels.application.as_ref() {
                    None => Some(BrokerCtrl {
                        url: Some(DEF_MQ_CHANNEL_URL.to_string()),
                        prefetch: Some(DEF_MQ_PREFETCH),
                    }),
                    Some(channel) => Some(BrokerCtrl {
                        url: match channel.url.as_ref() {
                            None => Some(DEF_MQ_CHANNEL_URL.to_string()),
                            Some(url) => Some(url.to_string()),
                        },
                        prefetch: match channel.prefetch {
                            None => Some(DEF_MQ_PREFETCH),
                            Some(prefetch) => Some(prefetch),
                        },
                    }),
                },
                network: match mq_channels.network.as_ref() {
                    None => Some(BrokerCtrl {
                        url: Some(DEF_MQ_CHANNEL_URL.to_string()),
                        prefetch: Some(DEF_MQ_PREFETCH),
                    }),
                    Some(channel) => Some(BrokerCtrl {
                        url: match channel.url.as_ref() {
                            None => Some(DEF_MQ_CHANNEL_URL.to_string()),
                            Some(url) => Some(url.to_string()),
                        },
                        prefetch: match channel.prefetch {
                            None => Some(DEF_MQ_PREFETCH),
                            Some(prefetch) => Some(prefetch),
                        },
                    }),
                },
                device: match mq_channels.device.as_ref() {
                    None => Some(BrokerCtrl {
                        url: Some(DEF_MQ_CHANNEL_URL.to_string()),
                        prefetch: Some(DEF_MQ_PREFETCH),
                    }),
                    Some(channel) => Some(BrokerCtrl {
                        url: match channel.url.as_ref() {
                            None => Some(DEF_MQ_CHANNEL_URL.to_string()),
                            Some(url) => Some(url.to_string()),
                        },
                        prefetch: match channel.prefetch {
                            None => Some(DEF_MQ_PREFETCH),
                            Some(prefetch) => Some(prefetch),
                        },
                    }),
                },
                device_route: match mq_channels.device_route.as_ref() {
                    None => Some(BrokerCtrl {
                        url: Some(DEF_MQ_CHANNEL_URL.to_string()),
                        prefetch: Some(DEF_MQ_PREFETCH),
                    }),
                    Some(channel) => Some(BrokerCtrl {
                        url: match channel.url.as_ref() {
                            None => Some(DEF_MQ_CHANNEL_URL.to_string()),
                            Some(url) => Some(url.to_string()),
                        },
                        prefetch: match channel.prefetch {
                            None => Some(DEF_MQ_PREFETCH),
                            Some(prefetch) => Some(prefetch),
                        },
                    }),
                },
                network_route: match mq_channels.network_route.as_ref() {
                    None => Some(BrokerCtrl {
                        url: Some(DEF_MQ_CHANNEL_URL.to_string()),
                        prefetch: Some(DEF_MQ_PREFETCH),
                    }),
                    Some(channel) => Some(BrokerCtrl {
                        url: match channel.url.as_ref() {
                            None => Some(DEF_MQ_CHANNEL_URL.to_string()),
                            Some(url) => Some(url.to_string()),
                        },
                        prefetch: match channel.prefetch {
                            None => Some(DEF_MQ_PREFETCH),
                            Some(prefetch) => Some(prefetch),
                        },
                    }),
                },
                data: match mq_channels.data.as_ref() {
                    None => None,
                    Some(channel) => match channel.url.as_ref() {
                        None => None,
                        Some(url) => Some(BrokerData {
                            url: Some(url.to_string()),
                        }),
                    },
                },
            }),
        },
        api_scopes: match config.api_scopes.as_ref() {
            None => Some(HashMap::new()),
            Some(scopes) => Some(scopes.clone()),
        },
    }
}
