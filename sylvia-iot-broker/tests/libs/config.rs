use std::{collections::HashMap, env, ffi::OsStr};

use clap::Command;
use laboratory::{SpecContext, expect};

use sylvia_iot_broker::libs::config::{self, Config};
use sylvia_iot_corelib::constants::{CacheEngine, DbEngine};

use crate::TestState;

/// Test [`config::reg_args`].
pub fn reg_args(_context: &mut SpecContext<TestState>) -> Result<(), String> {
    config::reg_args(Command::new("test"));
    Ok(())
}

/// Test [`config::read_args`].
pub fn read_args(_context: &mut SpecContext<TestState>) -> Result<(), String> {
    // Default.
    let args = config::reg_args(Command::new("test")).get_matches_from(vec!["test"]);
    let conf = config::read_args(&args);
    expect(conf.auth.is_some()).to_equal(true)?;
    expect(conf.auth.as_ref().unwrap().as_str()).to_equal(config::DEF_AUTH)?;
    expect(conf.db.is_some()).to_equal(true)?;
    expect(conf.db.as_ref().unwrap().engine.as_ref().unwrap().as_str())
        .to_equal(config::DEF_ENGINE)?;
    expect(conf.db.as_ref().unwrap().mongodb.is_some()).to_equal(true)?;
    expect(conf.db.as_ref().unwrap().sqlite.is_some()).to_equal(true)?;
    let db_conf = conf.db.as_ref().unwrap().mongodb.as_ref().unwrap();
    expect(db_conf.url.as_ref().unwrap().as_str()).to_equal(config::DEF_MONGODB_URL)?;
    expect(db_conf.database.as_ref().unwrap().as_str()).to_equal(config::DEF_MONGODB_DB)?;
    expect(db_conf.pool_size).to_equal(None)?;
    let db_conf = conf.db.as_ref().unwrap().sqlite.as_ref().unwrap();
    expect(db_conf.path.as_ref().unwrap().as_str()).to_equal(config::DEF_SQLITE_PATH)?;
    expect(conf.cache.is_some()).to_equal(true)?;
    let cache_conf = conf.cache.as_ref().unwrap();
    expect(cache_conf.engine.as_ref().unwrap().as_str()).to_equal(config::DEF_CACHE_ENGINE)?;
    expect(cache_conf.memory.is_some()).to_equal(true)?;
    let cache_conf = cache_conf.memory.as_ref().unwrap();
    expect(cache_conf.device).to_equal(Some(config::DEF_MEMORY_DEVICE))?;
    expect(cache_conf.device_route).to_equal(Some(config::DEF_MEMORY_DEVICE_ROUTE))?;
    expect(cache_conf.network_route).to_equal(Some(config::DEF_MEMORY_NETWORK_ROUTE))?;
    expect(conf.mq.is_some()).to_equal(true)?;
    let mq_conf = conf.mq.as_ref().unwrap();
    expect(mq_conf.prefetch.is_some()).to_equal(true)?;
    expect(mq_conf.prefetch.unwrap()).to_equal(config::DEF_MQ_PREFETCH)?;
    expect(mq_conf.persistent.is_some()).to_equal(true)?;
    expect(mq_conf.persistent.unwrap()).to_equal(config::DEF_MQ_PERSISTENT)?;
    expect(mq_conf.shared_prefix.is_some()).to_equal(true)?;
    expect(mq_conf.shared_prefix.as_ref().unwrap().as_str())
        .to_equal(config::DEF_MQ_SHAREDPREFIX)?;
    expect(conf.mq_channels.is_some()).to_equal(true)?;
    let mq_channels_conf = conf.mq_channels.as_ref().unwrap();
    expect(mq_channels_conf.unit.is_some()).to_equal(true)?;
    let ctrl_conf = mq_channels_conf.unit.as_ref().unwrap();
    expect(ctrl_conf.url.is_some()).to_equal(true)?;
    expect(ctrl_conf.url.as_ref().unwrap().as_str()).to_equal(config::DEF_MQ_CHANNEL_URL)?;
    expect(ctrl_conf.prefetch.is_some()).to_equal(true)?;
    expect(ctrl_conf.prefetch.unwrap()).to_equal(config::DEF_MQ_PREFETCH)?;
    expect(mq_channels_conf.application.is_some()).to_equal(true)?;
    let ctrl_conf = mq_channels_conf.application.as_ref().unwrap();
    expect(ctrl_conf.url.is_some()).to_equal(true)?;
    expect(ctrl_conf.url.as_ref().unwrap().as_str()).to_equal(config::DEF_MQ_CHANNEL_URL)?;
    expect(ctrl_conf.prefetch.is_some()).to_equal(true)?;
    expect(ctrl_conf.prefetch.unwrap()).to_equal(config::DEF_MQ_PREFETCH)?;
    expect(mq_channels_conf.network.is_some()).to_equal(true)?;
    let ctrl_conf = mq_channels_conf.network.as_ref().unwrap();
    expect(ctrl_conf.url.is_some()).to_equal(true)?;
    expect(ctrl_conf.url.as_ref().unwrap().as_str()).to_equal(config::DEF_MQ_CHANNEL_URL)?;
    expect(ctrl_conf.prefetch.is_some()).to_equal(true)?;
    expect(ctrl_conf.prefetch.unwrap()).to_equal(config::DEF_MQ_PREFETCH)?;
    expect(mq_channels_conf.device.is_some()).to_equal(true)?;
    let ctrl_conf = mq_channels_conf.device.as_ref().unwrap();
    expect(ctrl_conf.url.is_some()).to_equal(true)?;
    expect(ctrl_conf.url.as_ref().unwrap().as_str()).to_equal(config::DEF_MQ_CHANNEL_URL)?;
    expect(ctrl_conf.prefetch.is_some()).to_equal(true)?;
    expect(ctrl_conf.prefetch.unwrap()).to_equal(config::DEF_MQ_PREFETCH)?;
    expect(mq_channels_conf.device_route.is_some()).to_equal(true)?;
    let ctrl_conf = mq_channels_conf.device_route.as_ref().unwrap();
    expect(ctrl_conf.url.is_some()).to_equal(true)?;
    expect(ctrl_conf.url.as_ref().unwrap().as_str()).to_equal(config::DEF_MQ_CHANNEL_URL)?;
    expect(ctrl_conf.prefetch.is_some()).to_equal(true)?;
    expect(ctrl_conf.prefetch.unwrap()).to_equal(config::DEF_MQ_PREFETCH)?;
    expect(mq_channels_conf.network_route.is_some()).to_equal(true)?;
    let ctrl_conf = mq_channels_conf.network_route.as_ref().unwrap();
    expect(ctrl_conf.url.is_some()).to_equal(true)?;
    expect(ctrl_conf.url.as_ref().unwrap().as_str()).to_equal(config::DEF_MQ_CHANNEL_URL)?;
    expect(ctrl_conf.prefetch.is_some()).to_equal(true)?;
    expect(ctrl_conf.prefetch.unwrap()).to_equal(config::DEF_MQ_PREFETCH)?;
    expect(mq_channels_conf.data.is_some()).to_equal(true)?;
    let data_conf = mq_channels_conf.data.as_ref().unwrap();
    expect(data_conf.url.is_none()).to_equal(true)?;
    expect(data_conf.persistent.is_some()).to_equal(true)?;
    expect(data_conf.persistent.unwrap()).to_equal(config::DEF_MQ_PERSISTENT)?;
    expect(conf.api_scopes.as_ref()).to_equal(Some(&HashMap::new()))?;

    // Modified default by command-line arguments.
    let args = vec![
        "test",
        "--broker.auth",
        "sylvia1",
        "--broker.db.engine",
        "mongodb",
        "--broker.db.mongodb.url",
        "url11",
        "--broker.db.mongodb.database",
        "db1",
        "--broker.db.mongodb.poolsize",
        "11",
        "--broker.db.sqlite.path",
        "path1",
        "--broker.cache.engine",
        "memory",
        "--broker.cache.memory.device",
        "111",
        "--broker.cache.memory.device-route",
        "112",
        "--broker.cache.memory.network-route",
        "113",
        "--broker.mq.prefetch",
        "12",
        "--broker.mq.persistent",
        "true",
        "--broker.mq.sharedprefix",
        "prefix1",
        "--broker.mq-channels.unit.url",
        "url13",
        "--broker.mq-channels.unit.prefetch",
        "13",
        "--broker.mq-channels.application.url",
        "url14",
        "--broker.mq-channels.application.prefetch",
        "14",
        "--broker.mq-channels.network.url",
        "url15",
        "--broker.mq-channels.network.prefetch",
        "15",
        "--broker.mq-channels.device.url",
        "url16",
        "--broker.mq-channels.device.prefetch",
        "16",
        "--broker.mq-channels.device-route.url",
        "url17",
        "--broker.mq-channels.device-route.prefetch",
        "17",
        "--broker.mq-channels.network-route.url",
        "url18",
        "--broker.mq-channels.network-route.prefetch",
        "18",
        "--broker.mq-channels.data.url",
        "url19",
        "--broker.mq-channels.data.persistent",
        "false",
        "--broker.api-scopes",
        "{\"key11\":[\"value11\"]}",
    ];
    let args = config::reg_args(Command::new("test")).get_matches_from(args);
    let conf = config::read_args(&args);
    expect(conf.auth.is_some()).to_equal(true)?;
    expect(conf.auth.as_ref().unwrap().as_str()).to_equal("sylvia1")?;
    expect(conf.db.is_some()).to_equal(true)?;
    expect(conf.db.as_ref().unwrap().engine.as_ref().unwrap().as_str())
        .to_equal(DbEngine::MONGODB)?;
    expect(conf.db.as_ref().unwrap().mongodb.is_some()).to_equal(true)?;
    expect(conf.db.as_ref().unwrap().sqlite.is_some()).to_equal(true)?;
    let db_conf = conf.db.as_ref().unwrap().mongodb.as_ref().unwrap();
    expect(db_conf.url.as_ref().unwrap().as_str()).to_equal("url11")?;
    expect(db_conf.database.as_ref().unwrap().as_str()).to_equal("db1")?;
    expect(db_conf.pool_size).to_equal(Some(11))?;
    let db_conf = conf.db.as_ref().unwrap().sqlite.as_ref().unwrap();
    expect(db_conf.path.as_ref().unwrap().as_str()).to_equal("path1")?;
    expect(conf.cache.is_some()).to_equal(true)?;
    let cache_conf = conf.cache.as_ref().unwrap();
    expect(cache_conf.engine.as_ref().unwrap().as_str()).to_equal(CacheEngine::MEMORY)?;
    expect(cache_conf.memory.is_some()).to_equal(true)?;
    let cache_conf = cache_conf.memory.as_ref().unwrap();
    expect(cache_conf.device).to_equal(Some(111))?;
    expect(cache_conf.device_route).to_equal(Some(112))?;
    expect(cache_conf.network_route).to_equal(Some(113))?;
    expect(conf.mq.is_some()).to_equal(true)?;
    let mq_conf = conf.mq.as_ref().unwrap();
    expect(mq_conf.prefetch.is_some()).to_equal(true)?;
    expect(mq_conf.prefetch.unwrap()).to_equal(12)?;
    expect(mq_conf.persistent.is_some()).to_equal(true)?;
    expect(mq_conf.persistent.unwrap()).to_equal(true)?;
    expect(mq_conf.shared_prefix.is_some()).to_equal(true)?;
    expect(mq_conf.shared_prefix.as_ref().unwrap().as_str()).to_equal("prefix1")?;
    expect(conf.mq_channels.is_some()).to_equal(true)?;
    let mq_channels_conf = conf.mq_channels.as_ref().unwrap();
    expect(mq_channels_conf.unit.is_some()).to_equal(true)?;
    let ctrl_conf = mq_channels_conf.unit.as_ref().unwrap();
    expect(ctrl_conf.url.is_some()).to_equal(true)?;
    expect(ctrl_conf.url.as_ref().unwrap().as_str()).to_equal("url13")?;
    expect(ctrl_conf.prefetch.is_some()).to_equal(true)?;
    expect(ctrl_conf.prefetch.unwrap()).to_equal(13)?;
    expect(mq_channels_conf.application.is_some()).to_equal(true)?;
    let ctrl_conf = mq_channels_conf.application.as_ref().unwrap();
    expect(ctrl_conf.url.is_some()).to_equal(true)?;
    expect(ctrl_conf.url.as_ref().unwrap().as_str()).to_equal("url14")?;
    expect(ctrl_conf.prefetch.is_some()).to_equal(true)?;
    expect(ctrl_conf.prefetch.unwrap()).to_equal(14)?;
    expect(mq_channels_conf.network.is_some()).to_equal(true)?;
    let ctrl_conf = mq_channels_conf.network.as_ref().unwrap();
    expect(ctrl_conf.url.is_some()).to_equal(true)?;
    expect(ctrl_conf.url.as_ref().unwrap().as_str()).to_equal("url15")?;
    expect(ctrl_conf.prefetch.is_some()).to_equal(true)?;
    expect(ctrl_conf.prefetch.unwrap()).to_equal(15)?;
    expect(mq_channels_conf.device.is_some()).to_equal(true)?;
    let ctrl_conf = mq_channels_conf.device.as_ref().unwrap();
    expect(ctrl_conf.url.is_some()).to_equal(true)?;
    expect(ctrl_conf.url.as_ref().unwrap().as_str()).to_equal("url16")?;
    expect(ctrl_conf.prefetch.is_some()).to_equal(true)?;
    expect(ctrl_conf.prefetch.unwrap()).to_equal(16)?;
    expect(mq_channels_conf.device_route.is_some()).to_equal(true)?;
    let ctrl_conf = mq_channels_conf.device_route.as_ref().unwrap();
    expect(ctrl_conf.url.is_some()).to_equal(true)?;
    expect(ctrl_conf.url.as_ref().unwrap().as_str()).to_equal("url17")?;
    expect(ctrl_conf.prefetch.is_some()).to_equal(true)?;
    expect(ctrl_conf.prefetch.unwrap()).to_equal(17)?;
    expect(mq_channels_conf.network_route.is_some()).to_equal(true)?;
    let ctrl_conf = mq_channels_conf.network_route.as_ref().unwrap();
    expect(ctrl_conf.url.is_some()).to_equal(true)?;
    expect(ctrl_conf.url.as_ref().unwrap().as_str()).to_equal("url18")?;
    expect(ctrl_conf.prefetch.is_some()).to_equal(true)?;
    expect(ctrl_conf.prefetch.unwrap()).to_equal(18)?;
    expect(mq_channels_conf.data.is_some()).to_equal(true)?;
    let data_conf = mq_channels_conf.data.as_ref().unwrap();
    expect(data_conf.url.is_some()).to_equal(true)?;
    expect(data_conf.url.as_ref().unwrap().as_str()).to_equal("url19")?;
    expect(data_conf.persistent.is_some()).to_equal(true)?;
    expect(data_conf.persistent.unwrap()).to_equal(false)?;
    let mut map: HashMap<String, Vec<String>> = HashMap::new();
    map.insert("key11".to_string(), vec!["value11".to_string()]);
    expect(conf.api_scopes.as_ref()).to_equal(Some(&map))?;

    let args = vec![
        "test",
        "--broker.db.engine",
        "sqlite",
        "--broker.cache.engine",
        "none",
        "--broker.mq.sharedprefix",
        "",
        "--broker.mq-channels.data.persistent",
        "true",
        "--broker.api-scopes",
        "",
    ];
    let args = config::reg_args(Command::new("test")).get_matches_from(args);
    let conf = config::read_args(&args);
    expect(conf.db.is_some()).to_equal(true)?;
    expect(conf.db.as_ref().unwrap().engine.as_ref().unwrap().as_str())
        .to_equal(DbEngine::SQLITE)?;
    expect(conf.cache.is_some()).to_equal(true)?;
    let cache_conf = conf.cache.as_ref().unwrap();
    expect(cache_conf.engine.as_ref().unwrap().as_str()).to_equal(CacheEngine::NONE)?;
    expect(conf.mq.is_some()).to_equal(true)?;
    let mq_conf = conf.mq.as_ref().unwrap();
    expect(mq_conf.shared_prefix.as_ref().unwrap().as_str()).to_equal("")?;
    let mq_channels_conf = conf.mq_channels.as_ref().unwrap();
    expect(mq_channels_conf.data.is_some()).to_equal(true)?;
    let data_conf = mq_channels_conf.data.as_ref().unwrap();
    expect(data_conf.url.is_none()).to_equal(true)?;
    expect(data_conf.persistent.is_some()).to_equal(true)?;
    expect(data_conf.persistent.unwrap()).to_equal(true)?;
    expect(conf.api_scopes.as_ref()).to_equal(Some(&HashMap::new()))?;

    // Test wrong command-line arguments.
    let args = vec!["test", "--broker.api-scopes", "{"];
    let args = config::reg_args(Command::new("test")).get_matches_from(args);
    let conf = config::read_args(&args);
    expect(conf.api_scopes.as_ref()).to_equal(Some(&HashMap::new()))?;

    // Clear command-line arguments.
    let args = config::reg_args(Command::new("test")).get_matches_from(vec!["test"]);

    // Modified default by environment variables.
    set_env_var("BROKER_AUTH", "sylvia2");
    set_env_var("BROKER_DB_ENGINE", "mongodb");
    set_env_var("BROKER_DB_MONGODB_URL", "url21");
    set_env_var("BROKER_DB_MONGODB_DATABASE", "db2");
    set_env_var("BROKER_DB_MONGODB_POOLSIZE", "21");
    set_env_var("BROKER_DB_SQLITE_PATH", "path2");
    set_env_var("BROKER_CACHE_ENGINE", "memory");
    set_env_var("BROKER_CACHE_MEMORY_DEVICE", "121");
    set_env_var("BROKER_CACHE_MEMORY_DEVICE_ROUTE", "122");
    set_env_var("BROKER_CACHE_MEMORY_NETWORK_ROUTE", "123");
    set_env_var("BROKER_MQ_PREFETCH", "22");
    set_env_var("BROKER_MQ_PERSISTENT", "true");
    set_env_var("BROKER_MQ_SHAREDPREFIX", "prefix2");
    set_env_var("BROKER_MQCHANNELS_UNIT_URL", "url23");
    set_env_var("BROKER_MQCHANNELS_UNIT_PREFETCH", "23");
    set_env_var("BROKER_MQCHANNELS_APPLICATION_URL", "url24");
    set_env_var("BROKER_MQCHANNELS_APPLICATION_PREFETCH", "24");
    set_env_var("BROKER_MQCHANNELS_NETWORK_URL", "url25");
    set_env_var("BROKER_MQCHANNELS_NETWORK_PREFETCH", "25");
    set_env_var("BROKER_MQCHANNELS_DEVICE_URL", "url26");
    set_env_var("BROKER_MQCHANNELS_DEVICE_PREFETCH", "26");
    set_env_var("BROKER_MQCHANNELS_DEVICE_ROUTE_URL", "url27");
    set_env_var("BROKER_MQCHANNELS_DEVICE_ROUTE_PREFETCH", "27");
    set_env_var("BROKER_MQCHANNELS_NETWORK_ROUTE_URL", "url28");
    set_env_var("BROKER_MQCHANNELS_NETWORK_ROUTE_PREFETCH", "28");
    set_env_var("BROKER_MQCHANNELS_DATA_URL", "url29");
    set_env_var("BROKER_MQCHANNELS_DATA_PERSISTENT", "false");
    set_env_var("BROKER_API_SCOPES", "{\"key21\":[\"value21\"]}");
    let conf = config::read_args(&args);
    expect(conf.auth.is_some()).to_equal(true)?;
    expect(conf.auth.as_ref().unwrap().as_str()).to_equal("sylvia2")?;
    expect(conf.db.is_some()).to_equal(true)?;
    expect(conf.db.as_ref().unwrap().engine.as_ref().unwrap().as_str())
        .to_equal(DbEngine::MONGODB)?;
    expect(conf.db.as_ref().unwrap().mongodb.is_some()).to_equal(true)?;
    expect(conf.db.as_ref().unwrap().sqlite.is_some()).to_equal(true)?;
    let db_conf = conf.db.as_ref().unwrap().mongodb.as_ref().unwrap();
    expect(db_conf.url.as_ref().unwrap().as_str()).to_equal("url21")?;
    expect(db_conf.database.as_ref().unwrap().as_str()).to_equal("db2")?;
    expect(db_conf.pool_size).to_equal(Some(21))?;
    let db_conf = conf.db.as_ref().unwrap().sqlite.as_ref().unwrap();
    expect(db_conf.path.as_ref().unwrap().as_str()).to_equal("path2")?;
    expect(conf.cache.is_some()).to_equal(true)?;
    let cache_conf = conf.cache.as_ref().unwrap();
    expect(cache_conf.engine.as_ref().unwrap().as_str()).to_equal(CacheEngine::MEMORY)?;
    expect(cache_conf.memory.is_some()).to_equal(true)?;
    let cache_conf = cache_conf.memory.as_ref().unwrap();
    expect(cache_conf.device).to_equal(Some(121))?;
    expect(cache_conf.device_route).to_equal(Some(122))?;
    expect(cache_conf.network_route).to_equal(Some(123))?;
    expect(conf.mq.is_some()).to_equal(true)?;
    let mq_conf = conf.mq.as_ref().unwrap();
    expect(mq_conf.prefetch.is_some()).to_equal(true)?;
    expect(mq_conf.prefetch.unwrap()).to_equal(22)?;
    expect(mq_conf.persistent.is_some()).to_equal(true)?;
    expect(mq_conf.persistent.unwrap()).to_equal(true)?;
    expect(mq_conf.shared_prefix.is_some()).to_equal(true)?;
    expect(mq_conf.shared_prefix.as_ref().unwrap().as_str()).to_equal("prefix2")?;
    expect(conf.mq_channels.is_some()).to_equal(true)?;
    let mq_channels_conf = conf.mq_channels.as_ref().unwrap();
    expect(mq_channels_conf.unit.is_some()).to_equal(true)?;
    let ctrl_conf = mq_channels_conf.unit.as_ref().unwrap();
    expect(ctrl_conf.url.is_some()).to_equal(true)?;
    expect(ctrl_conf.url.as_ref().unwrap().as_str()).to_equal("url23")?;
    expect(ctrl_conf.prefetch.is_some()).to_equal(true)?;
    expect(ctrl_conf.prefetch.unwrap()).to_equal(23)?;
    expect(mq_channels_conf.application.is_some()).to_equal(true)?;
    let ctrl_conf = mq_channels_conf.application.as_ref().unwrap();
    expect(ctrl_conf.url.is_some()).to_equal(true)?;
    expect(ctrl_conf.url.as_ref().unwrap().as_str()).to_equal("url24")?;
    expect(ctrl_conf.prefetch.is_some()).to_equal(true)?;
    expect(ctrl_conf.prefetch.unwrap()).to_equal(24)?;
    expect(mq_channels_conf.network.is_some()).to_equal(true)?;
    let ctrl_conf = mq_channels_conf.network.as_ref().unwrap();
    expect(ctrl_conf.url.is_some()).to_equal(true)?;
    expect(ctrl_conf.url.as_ref().unwrap().as_str()).to_equal("url25")?;
    expect(ctrl_conf.prefetch.is_some()).to_equal(true)?;
    expect(ctrl_conf.prefetch.unwrap()).to_equal(25)?;
    expect(mq_channels_conf.device.is_some()).to_equal(true)?;
    let ctrl_conf = mq_channels_conf.device.as_ref().unwrap();
    expect(ctrl_conf.url.is_some()).to_equal(true)?;
    expect(ctrl_conf.url.as_ref().unwrap().as_str()).to_equal("url26")?;
    expect(ctrl_conf.prefetch.is_some()).to_equal(true)?;
    expect(ctrl_conf.prefetch.unwrap()).to_equal(26)?;
    expect(mq_channels_conf.device_route.is_some()).to_equal(true)?;
    let ctrl_conf = mq_channels_conf.device_route.as_ref().unwrap();
    expect(ctrl_conf.url.is_some()).to_equal(true)?;
    expect(ctrl_conf.url.as_ref().unwrap().as_str()).to_equal("url27")?;
    expect(ctrl_conf.prefetch.is_some()).to_equal(true)?;
    expect(ctrl_conf.prefetch.unwrap()).to_equal(27)?;
    expect(mq_channels_conf.network_route.is_some()).to_equal(true)?;
    let ctrl_conf = mq_channels_conf.network_route.as_ref().unwrap();
    expect(ctrl_conf.url.is_some()).to_equal(true)?;
    expect(ctrl_conf.url.as_ref().unwrap().as_str()).to_equal("url28")?;
    expect(ctrl_conf.prefetch.is_some()).to_equal(true)?;
    expect(ctrl_conf.prefetch.unwrap()).to_equal(28)?;
    expect(mq_channels_conf.data.is_some()).to_equal(true)?;
    let data_conf = mq_channels_conf.data.as_ref().unwrap();
    expect(data_conf.url.is_some()).to_equal(true)?;
    expect(data_conf.url.as_ref().unwrap().as_str()).to_equal("url29")?;
    expect(data_conf.persistent.is_some()).to_equal(true)?;
    expect(data_conf.persistent.unwrap()).to_equal(false)?;
    let mut map: HashMap<String, Vec<String>> = HashMap::new();
    map.insert("key21".to_string(), vec!["value21".to_string()]);
    expect(conf.api_scopes.as_ref()).to_equal(Some(&map))?;

    set_env_var("BROKER_DB_ENGINE", "sqlite");
    set_env_var("BROKER_CACHE_ENGINE", "none");
    set_env_var("BROKER_API_SCOPES", "");
    let conf = config::read_args(&args);
    expect(conf.db.is_some()).to_equal(true)?;
    expect(conf.db.as_ref().unwrap().engine.as_ref().unwrap().as_str())
        .to_equal(DbEngine::SQLITE)?;
    expect(conf.cache.is_some()).to_equal(true)?;
    let cache_conf = conf.cache.as_ref().unwrap();
    expect(cache_conf.engine.as_ref().unwrap().as_str()).to_equal(CacheEngine::NONE)?;
    expect(conf.api_scopes.as_ref()).to_equal(Some(&HashMap::new()))?;

    // Test wrong environment variables.
    set_env_var("BROKER_DB_ENGINE", "test2");
    set_env_var("BROKER_CACHE_ENGINE", "test3");
    set_env_var("BROKER_DB_MONGODB_POOLSIZE", "12_000");
    set_env_var("BROKER_CACHE_MEMORY_DEVICE", "12_000");
    set_env_var("BROKER_CACHE_MEMORY_DEVICE_ROUTE", "12_000");
    set_env_var("BROKER_CACHE_MEMORY_NETWORK_ROUTE", "12_000");
    set_env_var("BROKER_MQ_PREFETCH", "12_000");
    set_env_var("BROKER_MQ_PERSISTENT", "1");
    set_env_var("BROKER_MQCHANNELS_UNIT_PREFETCH", "12_000");
    set_env_var("BROKER_MQCHANNELS_APPLICATION_PREFETCH", "12_000");
    set_env_var("BROKER_MQCHANNELS_NETWORK_PREFETCH", "12_000");
    set_env_var("BROKER_MQCHANNELS_DEVICE_PREFETCH", "12_000");
    set_env_var("BROKER_MQCHANNELS_DEVICE_ROUTE_PREFETCH", "12_000");
    set_env_var("BROKER_MQCHANNELS_NETWORK_ROUTE_PREFETCH", "12_000");
    set_env_var("BROKER_MQCHANNELS_DATA_PERSISTENT", "0");
    set_env_var("BROKER_API_SCOPES", "}");
    let args = config::reg_args(Command::new("test")).get_matches_from(vec!["test"]);
    let conf = config::read_args(&args);
    let db_conf = conf.db.as_ref().unwrap().mongodb.as_ref().unwrap();
    expect(conf.db.as_ref().unwrap().engine.as_ref().unwrap().as_str())
        .to_equal(config::DEF_ENGINE)?;
    expect(db_conf.pool_size).to_equal(None)?;
    expect(conf.cache.is_some()).to_equal(true)?;
    let cache_conf = conf.cache.as_ref().unwrap();
    expect(cache_conf.engine.as_ref().unwrap().as_str()).to_equal(CacheEngine::NONE)?;
    expect(cache_conf.memory.is_some()).to_equal(true)?;
    let cache_conf = cache_conf.memory.as_ref().unwrap();
    expect(cache_conf.device).to_equal(Some(config::DEF_MEMORY_DEVICE))?;
    expect(cache_conf.device_route).to_equal(Some(config::DEF_MEMORY_DEVICE_ROUTE))?;
    expect(cache_conf.network_route).to_equal(Some(config::DEF_MEMORY_NETWORK_ROUTE))?;
    let mq_channels_conf = conf.mq_channels.as_ref().unwrap();
    expect(mq_channels_conf.data.is_some()).to_equal(true)?;
    let data_conf = mq_channels_conf.data.as_ref().unwrap();
    expect(data_conf.persistent.is_some()).to_equal(true)?;
    expect(data_conf.persistent.unwrap()).to_equal(config::DEF_MQ_PERSISTENT)?;
    expect(conf.api_scopes.as_ref()).to_equal(Some(&HashMap::new()))?;

    // Test command-line arguments overwrite environment variables.
    let args = vec![
        "test",
        "--broker.auth",
        "sylvia3",
        "--broker.db.engine",
        "mongodb",
        "--broker.db.mongodb.url",
        "url31",
        "--broker.db.mongodb.database",
        "db3",
        "--broker.db.mongodb.poolsize",
        "31",
        "--broker.db.sqlite.path",
        "path3",
        "--broker.cache.engine",
        "memory",
        "--broker.cache.memory.device",
        "131",
        "--broker.cache.memory.device-route",
        "132",
        "--broker.cache.memory.network-route",
        "133",
        "--broker.mq.prefetch",
        "32",
        "--broker.mq.persistent",
        "true",
        "--broker.mq.sharedprefix",
        "prefix3",
        "--broker.mq-channels.unit.url",
        "url33",
        "--broker.mq-channels.unit.prefetch",
        "33",
        "--broker.mq-channels.application.url",
        "url34",
        "--broker.mq-channels.application.prefetch",
        "34",
        "--broker.mq-channels.network.url",
        "url35",
        "--broker.mq-channels.network.prefetch",
        "35",
        "--broker.mq-channels.device.url",
        "url36",
        "--broker.mq-channels.device.prefetch",
        "36",
        "--broker.mq-channels.device-route.url",
        "url37",
        "--broker.mq-channels.device-route.prefetch",
        "37",
        "--broker.mq-channels.network-route.url",
        "url38",
        "--broker.mq-channels.network-route.prefetch",
        "38",
        "--broker.mq-channels.data.url",
        "url39",
        "--broker.mq-channels.data.persistent",
        "false",
        "--broker.api-scopes",
        "{\"key31\":[\"value31\"]}",
    ];
    set_env_var("BROKER_AUTH", "sylvia4");
    set_env_var("BROKER_DB_ENGINE", "mongodb");
    set_env_var("BROKER_DB_MONGODB_URL", "url41");
    set_env_var("BROKER_DB_MONGODB_DATABASE", "db4");
    set_env_var("BROKER_DB_MONGODB_POOLSIZE", "41");
    set_env_var("BROKER_DB_SQLITE_PATH", "path4");
    set_env_var("BROKER_CACHE_ENGINE", "memory");
    set_env_var("BROKER_CACHE_MEMORY_DEVICE", "141");
    set_env_var("BROKER_CACHE_MEMORY_DEVICE_ROUTE", "142");
    set_env_var("BROKER_CACHE_MEMORY_NETWORK_ROUTE", "143");
    set_env_var("BROKER_MQ_PREFETCH", "42");
    set_env_var("BROKER_MQ_PERSISTENT", "false");
    set_env_var("BROKER_MQ_SHAREDPREFIX", "prefix4");
    set_env_var("BROKER_MQCHANNELS_UNIT_URL", "url43");
    set_env_var("BROKER_MQCHANNELS_UNIT_PREFETCH", "43");
    set_env_var("BROKER_MQCHANNELS_APPLICATION_URL", "url44");
    set_env_var("BROKER_MQCHANNELS_APPLICATION_PREFETCH", "44");
    set_env_var("BROKER_MQCHANNELS_NETWORK_URL", "url45");
    set_env_var("BROKER_MQCHANNELS_NETWORK_PREFETCH", "45");
    set_env_var("BROKER_MQCHANNELS_DEVICE_URL", "url46");
    set_env_var("BROKER_MQCHANNELS_DEVICE_PREFETCH", "46");
    set_env_var("BROKER_MQCHANNELS_DEVICE_ROUTE_URL", "url47");
    set_env_var("BROKER_MQCHANNELS_DEVICE_ROUTE_PREFETCH", "47");
    set_env_var("BROKER_MQCHANNELS_NETWORK_ROUTE_URL", "url48");
    set_env_var("BROKER_MQCHANNELS_NETWORK_ROUTE_PREFETCH", "48");
    set_env_var("BROKER_MQCHANNELS_DATA_URL", "url49");
    set_env_var("BROKER_MQCHANNELS_DATA_PERSISTENT", "true");
    set_env_var("BROKER_API_SCOPES", "{\"key41\":[\"value41\"]}");
    let args = config::reg_args(Command::new("test")).get_matches_from(args);
    let conf = config::read_args(&args);
    expect(conf.auth.is_some()).to_equal(true)?;
    expect(conf.auth.as_ref().unwrap().as_str()).to_equal("sylvia3")?;
    expect(conf.db.is_some()).to_equal(true)?;
    expect(conf.db.as_ref().unwrap().engine.as_ref().unwrap().as_str())
        .to_equal(DbEngine::MONGODB)?;
    expect(conf.db.as_ref().unwrap().mongodb.is_some()).to_equal(true)?;
    expect(conf.db.as_ref().unwrap().sqlite.is_some()).to_equal(true)?;
    let db_conf = conf.db.as_ref().unwrap().mongodb.as_ref().unwrap();
    expect(db_conf.url.as_ref().unwrap().as_str()).to_equal("url31")?;
    expect(db_conf.database.as_ref().unwrap().as_str()).to_equal("db3")?;
    expect(db_conf.pool_size).to_equal(Some(31))?;
    let db_conf = conf.db.as_ref().unwrap().sqlite.as_ref().unwrap();
    expect(db_conf.path.as_ref().unwrap().as_str()).to_equal("path3")?;
    expect(conf.cache.is_some()).to_equal(true)?;
    let cache_conf = conf.cache.as_ref().unwrap();
    expect(cache_conf.engine.as_ref().unwrap().as_str()).to_equal(CacheEngine::MEMORY)?;
    expect(cache_conf.memory.is_some()).to_equal(true)?;
    let cache_conf = cache_conf.memory.as_ref().unwrap();
    expect(cache_conf.device).to_equal(Some(131))?;
    expect(cache_conf.device_route).to_equal(Some(132))?;
    expect(cache_conf.network_route).to_equal(Some(133))?;
    expect(conf.mq.is_some()).to_equal(true)?;
    let mq_conf = conf.mq.as_ref().unwrap();
    expect(mq_conf.prefetch.is_some()).to_equal(true)?;
    expect(mq_conf.prefetch.unwrap()).to_equal(32)?;
    expect(mq_conf.persistent.is_some()).to_equal(true)?;
    expect(mq_conf.persistent.unwrap()).to_equal(true)?;
    expect(mq_conf.shared_prefix.is_some()).to_equal(true)?;
    expect(mq_conf.shared_prefix.as_ref().unwrap().as_str()).to_equal("prefix3")?;
    expect(conf.mq_channels.is_some()).to_equal(true)?;
    let mq_channels_conf = conf.mq_channels.as_ref().unwrap();
    expect(mq_channels_conf.unit.is_some()).to_equal(true)?;
    let ctrl_conf = mq_channels_conf.unit.as_ref().unwrap();
    expect(ctrl_conf.url.is_some()).to_equal(true)?;
    expect(ctrl_conf.url.as_ref().unwrap().as_str()).to_equal("url33")?;
    expect(ctrl_conf.prefetch.is_some()).to_equal(true)?;
    expect(ctrl_conf.prefetch.unwrap()).to_equal(33)?;
    expect(mq_channels_conf.application.is_some()).to_equal(true)?;
    let ctrl_conf = mq_channels_conf.application.as_ref().unwrap();
    expect(ctrl_conf.url.is_some()).to_equal(true)?;
    expect(ctrl_conf.url.as_ref().unwrap().as_str()).to_equal("url34")?;
    expect(ctrl_conf.prefetch.is_some()).to_equal(true)?;
    expect(ctrl_conf.prefetch.unwrap()).to_equal(34)?;
    expect(mq_channels_conf.network.is_some()).to_equal(true)?;
    let ctrl_conf = mq_channels_conf.network.as_ref().unwrap();
    expect(ctrl_conf.url.is_some()).to_equal(true)?;
    expect(ctrl_conf.url.as_ref().unwrap().as_str()).to_equal("url35")?;
    expect(ctrl_conf.prefetch.is_some()).to_equal(true)?;
    expect(ctrl_conf.prefetch.unwrap()).to_equal(35)?;
    expect(mq_channels_conf.device.is_some()).to_equal(true)?;
    let ctrl_conf = mq_channels_conf.device.as_ref().unwrap();
    expect(ctrl_conf.url.is_some()).to_equal(true)?;
    expect(ctrl_conf.url.as_ref().unwrap().as_str()).to_equal("url36")?;
    expect(ctrl_conf.prefetch.is_some()).to_equal(true)?;
    expect(ctrl_conf.prefetch.unwrap()).to_equal(36)?;
    expect(mq_channels_conf.device_route.is_some()).to_equal(true)?;
    let ctrl_conf = mq_channels_conf.device_route.as_ref().unwrap();
    expect(ctrl_conf.url.is_some()).to_equal(true)?;
    expect(ctrl_conf.url.as_ref().unwrap().as_str()).to_equal("url37")?;
    expect(ctrl_conf.prefetch.is_some()).to_equal(true)?;
    expect(ctrl_conf.prefetch.unwrap()).to_equal(37)?;
    expect(mq_channels_conf.network_route.is_some()).to_equal(true)?;
    let ctrl_conf = mq_channels_conf.network_route.as_ref().unwrap();
    expect(ctrl_conf.url.is_some()).to_equal(true)?;
    expect(ctrl_conf.url.as_ref().unwrap().as_str()).to_equal("url38")?;
    expect(ctrl_conf.prefetch.is_some()).to_equal(true)?;
    expect(ctrl_conf.prefetch.unwrap()).to_equal(38)?;
    expect(mq_channels_conf.data.is_some()).to_equal(true)?;
    let data_conf = mq_channels_conf.data.as_ref().unwrap();
    expect(data_conf.url.is_some()).to_equal(true)?;
    expect(data_conf.url.as_ref().unwrap().as_str()).to_equal("url39")?;
    expect(data_conf.persistent.is_some()).to_equal(true)?;
    expect(data_conf.persistent.unwrap()).to_equal(false)?;
    let mut map: HashMap<String, Vec<String>> = HashMap::new();
    map.insert("key31".to_string(), vec!["value31".to_string()]);
    expect(conf.api_scopes.as_ref()).to_equal(Some(&map))
}

/// Test [`config::apply_default`].
pub fn apply_default(_context: &mut SpecContext<TestState>) -> Result<(), String> {
    let conf = Config {
        ..Default::default()
    };
    let conf = config::apply_default(&conf);
    expect(conf.auth.is_some()).to_equal(true)?;
    expect(conf.auth.as_ref().unwrap().as_str()).to_equal(config::DEF_AUTH)?;
    expect(conf.db.is_some()).to_equal(true)?;
    expect(conf.db.as_ref().unwrap().engine.as_ref().unwrap().as_str())
        .to_equal(config::DEF_ENGINE)?;
    expect(conf.db.as_ref().unwrap().mongodb.is_some()).to_equal(true)?;
    expect(conf.db.as_ref().unwrap().sqlite.is_some()).to_equal(true)?;
    let db_conf = conf.db.as_ref().unwrap().mongodb.as_ref().unwrap();
    expect(db_conf.url.as_ref().unwrap().as_str()).to_equal(config::DEF_MONGODB_URL)?;
    expect(db_conf.database.as_ref().unwrap().as_str()).to_equal(config::DEF_MONGODB_DB)?;
    expect(db_conf.pool_size).to_equal(None)?;
    let db_conf = conf.db.as_ref().unwrap().sqlite.as_ref().unwrap();
    expect(db_conf.path.as_ref().unwrap().as_str()).to_equal(config::DEF_SQLITE_PATH)?;
    expect(conf.cache.is_some()).to_equal(true)?;
    let cache_conf = conf.cache.as_ref().unwrap();
    expect(cache_conf.engine.as_ref().unwrap().as_str()).to_equal(config::DEF_CACHE_ENGINE)?;
    expect(cache_conf.memory.is_some()).to_equal(true)?;
    let cache_conf = cache_conf.memory.as_ref().unwrap();
    expect(cache_conf.device).to_equal(Some(config::DEF_MEMORY_DEVICE))?;
    expect(cache_conf.device_route).to_equal(Some(config::DEF_MEMORY_DEVICE_ROUTE))?;
    expect(cache_conf.network_route).to_equal(Some(config::DEF_MEMORY_NETWORK_ROUTE))?;
    let mq_conf = conf.mq.as_ref().unwrap();
    expect(mq_conf.prefetch.is_some()).to_equal(true)?;
    expect(mq_conf.prefetch.unwrap()).to_equal(config::DEF_MQ_PREFETCH)?;
    expect(mq_conf.persistent.is_some()).to_equal(true)?;
    expect(mq_conf.persistent.unwrap()).to_equal(config::DEF_MQ_PERSISTENT)?;
    expect(mq_conf.shared_prefix.is_some()).to_equal(true)?;
    expect(mq_conf.shared_prefix.as_ref().unwrap().as_str())
        .to_equal(config::DEF_MQ_SHAREDPREFIX)?;
    let mq_channels_conf = conf.mq_channels.as_ref().unwrap();
    expect(mq_channels_conf.unit.is_some()).to_equal(true)?;
    let ctrl_conf = mq_channels_conf.unit.as_ref().unwrap();
    expect(ctrl_conf.url.is_some()).to_equal(true)?;
    expect(ctrl_conf.url.as_ref().unwrap().as_str()).to_equal(config::DEF_MQ_CHANNEL_URL)?;
    expect(ctrl_conf.prefetch.is_some()).to_equal(true)?;
    expect(ctrl_conf.prefetch.unwrap()).to_equal(config::DEF_MQ_PREFETCH)?;
    expect(mq_channels_conf.application.is_some()).to_equal(true)?;
    let ctrl_conf = mq_channels_conf.application.as_ref().unwrap();
    expect(ctrl_conf.url.is_some()).to_equal(true)?;
    expect(ctrl_conf.url.as_ref().unwrap().as_str()).to_equal(config::DEF_MQ_CHANNEL_URL)?;
    expect(ctrl_conf.prefetch.is_some()).to_equal(true)?;
    expect(ctrl_conf.prefetch.unwrap()).to_equal(config::DEF_MQ_PREFETCH)?;
    expect(mq_channels_conf.network.is_some()).to_equal(true)?;
    let ctrl_conf = mq_channels_conf.network.as_ref().unwrap();
    expect(ctrl_conf.url.is_some()).to_equal(true)?;
    expect(ctrl_conf.url.as_ref().unwrap().as_str()).to_equal(config::DEF_MQ_CHANNEL_URL)?;
    expect(ctrl_conf.prefetch.is_some()).to_equal(true)?;
    expect(ctrl_conf.prefetch.unwrap()).to_equal(config::DEF_MQ_PREFETCH)?;
    expect(mq_channels_conf.device.is_some()).to_equal(true)?;
    let ctrl_conf = mq_channels_conf.device.as_ref().unwrap();
    expect(ctrl_conf.url.is_some()).to_equal(true)?;
    expect(ctrl_conf.url.as_ref().unwrap().as_str()).to_equal(config::DEF_MQ_CHANNEL_URL)?;
    expect(ctrl_conf.prefetch.is_some()).to_equal(true)?;
    expect(ctrl_conf.prefetch.unwrap()).to_equal(config::DEF_MQ_PREFETCH)?;
    expect(mq_channels_conf.device_route.is_some()).to_equal(true)?;
    let ctrl_conf = mq_channels_conf.device_route.as_ref().unwrap();
    expect(ctrl_conf.url.is_some()).to_equal(true)?;
    expect(ctrl_conf.url.as_ref().unwrap().as_str()).to_equal(config::DEF_MQ_CHANNEL_URL)?;
    expect(ctrl_conf.prefetch.is_some()).to_equal(true)?;
    expect(ctrl_conf.prefetch.unwrap()).to_equal(config::DEF_MQ_PREFETCH)?;
    expect(mq_channels_conf.network_route.is_some()).to_equal(true)?;
    let ctrl_conf = mq_channels_conf.network_route.as_ref().unwrap();
    expect(ctrl_conf.url.is_some()).to_equal(true)?;
    expect(ctrl_conf.url.as_ref().unwrap().as_str()).to_equal(config::DEF_MQ_CHANNEL_URL)?;
    expect(ctrl_conf.prefetch.is_some()).to_equal(true)?;
    expect(ctrl_conf.prefetch.unwrap()).to_equal(config::DEF_MQ_PREFETCH)?;
    expect(mq_channels_conf.data.is_none()).to_equal(true)?;

    let conf = Config {
        db: Some(config::Db {
            ..Default::default()
        }),
        cache: Some(config::Cache {
            ..Default::default()
        }),
        mq: Some(config::Mq {
            ..Default::default()
        }),
        mq_channels: Some(config::MqChannels {
            ..Default::default()
        }),
        ..Default::default()
    };
    let conf = config::apply_default(&conf);
    expect(conf.auth.is_some()).to_equal(true)?;
    expect(conf.auth.as_ref().unwrap().as_str()).to_equal(config::DEF_AUTH)?;
    expect(conf.db.is_some()).to_equal(true)?;
    expect(conf.db.as_ref().unwrap().engine.as_ref().unwrap().as_str())
        .to_equal(config::DEF_ENGINE)?;
    expect(conf.db.as_ref().unwrap().mongodb.is_some()).to_equal(true)?;
    expect(conf.db.as_ref().unwrap().sqlite.is_some()).to_equal(true)?;
    let db_conf = conf.db.as_ref().unwrap().mongodb.as_ref().unwrap();
    expect(db_conf.url.as_ref().unwrap().as_str()).to_equal(config::DEF_MONGODB_URL)?;
    expect(db_conf.database.as_ref().unwrap().as_str()).to_equal(config::DEF_MONGODB_DB)?;
    expect(db_conf.pool_size).to_equal(None)?;
    let db_conf = conf.db.as_ref().unwrap().sqlite.as_ref().unwrap();
    expect(db_conf.path.as_ref().unwrap().as_str()).to_equal(config::DEF_SQLITE_PATH)?;
    expect(conf.cache.is_some()).to_equal(true)?;
    let cache_conf = conf.cache.as_ref().unwrap();
    expect(cache_conf.engine.as_ref().unwrap().as_str()).to_equal(config::DEF_CACHE_ENGINE)?;
    expect(cache_conf.memory.is_some()).to_equal(true)?;
    let cache_conf = cache_conf.memory.as_ref().unwrap();
    expect(cache_conf.device).to_equal(Some(config::DEF_MEMORY_DEVICE))?;
    expect(cache_conf.device_route).to_equal(Some(config::DEF_MEMORY_DEVICE_ROUTE))?;
    expect(cache_conf.network_route).to_equal(Some(config::DEF_MEMORY_NETWORK_ROUTE))?;
    let mq_conf = conf.mq.as_ref().unwrap();
    expect(mq_conf.prefetch.is_some()).to_equal(true)?;
    expect(mq_conf.prefetch.unwrap()).to_equal(config::DEF_MQ_PREFETCH)?;
    expect(mq_conf.persistent.is_some()).to_equal(true)?;
    expect(mq_conf.persistent.unwrap()).to_equal(config::DEF_MQ_PERSISTENT)?;
    expect(mq_conf.shared_prefix.is_some()).to_equal(true)?;
    expect(mq_conf.shared_prefix.as_ref().unwrap().as_str())
        .to_equal(config::DEF_MQ_SHAREDPREFIX)?;
    let mq_channels_conf = conf.mq_channels.as_ref().unwrap();
    expect(mq_channels_conf.unit.is_some()).to_equal(true)?;
    let ctrl_conf = mq_channels_conf.unit.as_ref().unwrap();
    expect(ctrl_conf.url.is_some()).to_equal(true)?;
    expect(ctrl_conf.url.as_ref().unwrap().as_str()).to_equal(config::DEF_MQ_CHANNEL_URL)?;
    expect(ctrl_conf.prefetch.is_some()).to_equal(true)?;
    expect(ctrl_conf.prefetch.unwrap()).to_equal(config::DEF_MQ_PREFETCH)?;
    expect(mq_channels_conf.application.is_some()).to_equal(true)?;
    let ctrl_conf = mq_channels_conf.application.as_ref().unwrap();
    expect(ctrl_conf.url.is_some()).to_equal(true)?;
    expect(ctrl_conf.url.as_ref().unwrap().as_str()).to_equal(config::DEF_MQ_CHANNEL_URL)?;
    expect(ctrl_conf.prefetch.is_some()).to_equal(true)?;
    expect(ctrl_conf.prefetch.unwrap()).to_equal(config::DEF_MQ_PREFETCH)?;
    expect(mq_channels_conf.network.is_some()).to_equal(true)?;
    let ctrl_conf = mq_channels_conf.network.as_ref().unwrap();
    expect(ctrl_conf.url.is_some()).to_equal(true)?;
    expect(ctrl_conf.url.as_ref().unwrap().as_str()).to_equal(config::DEF_MQ_CHANNEL_URL)?;
    expect(ctrl_conf.prefetch.is_some()).to_equal(true)?;
    expect(ctrl_conf.prefetch.unwrap()).to_equal(config::DEF_MQ_PREFETCH)?;
    expect(mq_channels_conf.device.is_some()).to_equal(true)?;
    let ctrl_conf = mq_channels_conf.device.as_ref().unwrap();
    expect(ctrl_conf.url.is_some()).to_equal(true)?;
    expect(ctrl_conf.url.as_ref().unwrap().as_str()).to_equal(config::DEF_MQ_CHANNEL_URL)?;
    expect(ctrl_conf.prefetch.is_some()).to_equal(true)?;
    expect(ctrl_conf.prefetch.unwrap()).to_equal(config::DEF_MQ_PREFETCH)?;
    expect(mq_channels_conf.device_route.is_some()).to_equal(true)?;
    let ctrl_conf = mq_channels_conf.device_route.as_ref().unwrap();
    expect(ctrl_conf.url.is_some()).to_equal(true)?;
    expect(ctrl_conf.url.as_ref().unwrap().as_str()).to_equal(config::DEF_MQ_CHANNEL_URL)?;
    expect(ctrl_conf.prefetch.is_some()).to_equal(true)?;
    expect(ctrl_conf.prefetch.unwrap()).to_equal(config::DEF_MQ_PREFETCH)?;
    expect(mq_channels_conf.network_route.is_some()).to_equal(true)?;
    let ctrl_conf = mq_channels_conf.network_route.as_ref().unwrap();
    expect(ctrl_conf.url.is_some()).to_equal(true)?;
    expect(ctrl_conf.url.as_ref().unwrap().as_str()).to_equal(config::DEF_MQ_CHANNEL_URL)?;
    expect(ctrl_conf.prefetch.is_some()).to_equal(true)?;
    expect(ctrl_conf.prefetch.unwrap()).to_equal(config::DEF_MQ_PREFETCH)?;
    expect(mq_channels_conf.data.is_none()).to_equal(true)?;

    let conf = Config {
        auth: Some("sylvia2".to_string()),
        db: Some(config::Db {
            engine: Some(DbEngine::MONGODB.to_string()),
            ..Default::default()
        }),
        cache: Some(config::Cache {
            engine: Some("test3".to_string()),
            ..Default::default()
        }),
        mq: Some(config::Mq {
            prefetch: Some(0),
            ..Default::default()
        }),
        mq_channels: Some(config::MqChannels {
            unit: Some(config::BrokerCtrl {
                ..Default::default()
            }),
            application: Some(config::BrokerCtrl {
                ..Default::default()
            }),
            network: Some(config::BrokerCtrl {
                ..Default::default()
            }),
            device: Some(config::BrokerCtrl {
                ..Default::default()
            }),
            device_route: Some(config::BrokerCtrl {
                ..Default::default()
            }),
            network_route: Some(config::BrokerCtrl {
                ..Default::default()
            }),
            data: Some(config::BrokerData {
                ..Default::default()
            }),
        }),
        ..Default::default()
    };
    let conf = config::apply_default(&conf);
    expect(conf.auth.is_some()).to_equal(true)?;
    expect(conf.auth.as_ref().unwrap().as_str()).to_equal("sylvia2")?;
    expect(conf.db.is_some()).to_equal(true)?;
    expect(conf.db.as_ref().unwrap().engine.as_ref().unwrap().as_str())
        .to_equal(DbEngine::MONGODB)?;
    expect(conf.cache.is_some()).to_equal(true)?;
    let cache_conf = conf.cache.as_ref().unwrap();
    expect(cache_conf.engine.as_ref().unwrap().as_str()).to_equal(config::DEF_CACHE_ENGINE)?;
    let mq_conf = conf.mq.as_ref().unwrap();
    expect(mq_conf.prefetch.is_some()).to_equal(true)?;
    expect(mq_conf.prefetch.unwrap()).to_equal(config::DEF_MQ_PREFETCH)?;
    expect(mq_conf.persistent.is_some()).to_equal(true)?;
    expect(mq_conf.persistent.unwrap()).to_equal(config::DEF_MQ_PERSISTENT)?;
    expect(mq_conf.shared_prefix.is_some()).to_equal(true)?;
    expect(mq_conf.shared_prefix.as_ref().unwrap().as_str())
        .to_equal(config::DEF_MQ_SHAREDPREFIX)?;
    let mq_channels_conf = conf.mq_channels.as_ref().unwrap();
    expect(mq_channels_conf.unit.is_some()).to_equal(true)?;
    let ctrl_conf = mq_channels_conf.unit.as_ref().unwrap();
    expect(ctrl_conf.url.is_some()).to_equal(true)?;
    expect(ctrl_conf.url.as_ref().unwrap().as_str()).to_equal(config::DEF_MQ_CHANNEL_URL)?;
    expect(ctrl_conf.prefetch.is_some()).to_equal(true)?;
    expect(ctrl_conf.prefetch.unwrap()).to_equal(config::DEF_MQ_PREFETCH)?;
    expect(mq_channels_conf.application.is_some()).to_equal(true)?;
    let ctrl_conf = mq_channels_conf.application.as_ref().unwrap();
    expect(ctrl_conf.url.is_some()).to_equal(true)?;
    expect(ctrl_conf.url.as_ref().unwrap().as_str()).to_equal(config::DEF_MQ_CHANNEL_URL)?;
    expect(ctrl_conf.prefetch.is_some()).to_equal(true)?;
    expect(ctrl_conf.prefetch.unwrap()).to_equal(config::DEF_MQ_PREFETCH)?;
    let ctrl_conf = mq_channels_conf.network.as_ref().unwrap();
    expect(ctrl_conf.url.is_some()).to_equal(true)?;
    expect(ctrl_conf.url.as_ref().unwrap().as_str()).to_equal(config::DEF_MQ_CHANNEL_URL)?;
    expect(ctrl_conf.prefetch.is_some()).to_equal(true)?;
    expect(ctrl_conf.prefetch.unwrap()).to_equal(config::DEF_MQ_PREFETCH)?;
    expect(mq_channels_conf.device.is_some()).to_equal(true)?;
    let ctrl_conf = mq_channels_conf.device.as_ref().unwrap();
    expect(ctrl_conf.url.is_some()).to_equal(true)?;
    expect(ctrl_conf.url.as_ref().unwrap().as_str()).to_equal(config::DEF_MQ_CHANNEL_URL)?;
    expect(ctrl_conf.prefetch.is_some()).to_equal(true)?;
    expect(ctrl_conf.prefetch.unwrap()).to_equal(config::DEF_MQ_PREFETCH)?;
    expect(mq_channels_conf.device_route.is_some()).to_equal(true)?;
    let ctrl_conf = mq_channels_conf.device_route.as_ref().unwrap();
    expect(ctrl_conf.url.is_some()).to_equal(true)?;
    expect(ctrl_conf.url.as_ref().unwrap().as_str()).to_equal(config::DEF_MQ_CHANNEL_URL)?;
    expect(ctrl_conf.prefetch.is_some()).to_equal(true)?;
    expect(ctrl_conf.prefetch.unwrap()).to_equal(config::DEF_MQ_PREFETCH)?;
    expect(mq_channels_conf.network_route.is_some()).to_equal(true)?;
    let ctrl_conf = mq_channels_conf.network_route.as_ref().unwrap();
    expect(ctrl_conf.url.is_some()).to_equal(true)?;
    expect(ctrl_conf.url.as_ref().unwrap().as_str()).to_equal(config::DEF_MQ_CHANNEL_URL)?;
    expect(ctrl_conf.prefetch.is_some()).to_equal(true)?;
    expect(ctrl_conf.prefetch.unwrap()).to_equal(config::DEF_MQ_PREFETCH)?;
    expect(mq_channels_conf.data.is_some()).to_equal(true)?;
    let data_conf = mq_channels_conf.data.as_ref().unwrap();
    expect(data_conf.url.is_none()).to_equal(true)?;
    expect(data_conf.persistent.is_some()).to_equal(true)?;
    expect(data_conf.persistent.unwrap()).to_equal(config::DEF_MQ_PERSISTENT)?;
    expect(conf.api_scopes.as_ref()).to_equal(Some(&HashMap::new()))?;

    let mut api_scopes: HashMap<String, Vec<String>> = HashMap::new();
    api_scopes.insert("api".to_string(), vec!["scope1".to_string()]);
    let conf = Config {
        auth: Some("sylvia3".to_string()),
        db: Some(config::Db {
            engine: Some("test".to_string()),
            mongodb: Some(config::MongoDb {
                url: Some("url".to_string()),
                database: Some("db".to_string()),
                pool_size: Some(11),
            }),
            sqlite: Some(config::Sqlite {
                path: Some("path".to_string()),
            }),
        }),
        cache: Some(config::Cache {
            engine: Some(CacheEngine::MEMORY.to_string()),
            memory: Some(config::MemoryCache {
                device: Some(100),
                device_route: Some(101),
                network_route: Some(102),
            }),
        }),
        mq: Some(config::Mq {
            prefetch: Some(10),
            persistent: Some(true),
            shared_prefix: Some("$shared/group/".to_string()),
        }),
        mq_channels: Some(config::MqChannels {
            unit: Some(config::BrokerCtrl {
                url: Some("url3".to_string()),
                prefetch: Some(13),
            }),
            application: Some(config::BrokerCtrl {
                url: Some("url4".to_string()),
                prefetch: Some(14),
            }),
            network: Some(config::BrokerCtrl {
                url: Some("url5".to_string()),
                prefetch: Some(15),
            }),
            device: Some(config::BrokerCtrl {
                url: Some("url6".to_string()),
                prefetch: Some(16),
            }),
            device_route: Some(config::BrokerCtrl {
                url: Some("url7".to_string()),
                prefetch: Some(17),
            }),
            network_route: Some(config::BrokerCtrl {
                url: Some("url8".to_string()),
                prefetch: Some(18),
            }),
            data: Some(config::BrokerData {
                url: Some("url9".to_string()),
                persistent: Some(false),
            }),
        }),
        api_scopes: Some(api_scopes.clone()),
    };
    let conf = config::apply_default(&conf);
    expect(conf.auth.is_some()).to_equal(true)?;
    expect(conf.auth.as_ref().unwrap().as_str()).to_equal("sylvia3")?;
    expect(conf.db.is_some()).to_equal(true)?;
    expect(conf.db.as_ref().unwrap().engine.as_ref().unwrap().as_str())
        .to_equal(config::DEF_ENGINE)?;
    expect(conf.db.as_ref().unwrap().mongodb.is_some()).to_equal(true)?;
    expect(conf.db.as_ref().unwrap().sqlite.is_some()).to_equal(true)?;
    let db_conf = conf.db.as_ref().unwrap().mongodb.as_ref().unwrap();
    expect(db_conf.url.as_ref().unwrap().as_str()).to_equal("url")?;
    expect(db_conf.database.as_ref().unwrap().as_str()).to_equal("db")?;
    expect(db_conf.pool_size).to_equal(Some(11))?;
    let db_conf = conf.db.as_ref().unwrap().sqlite.as_ref().unwrap();
    expect(db_conf.path.as_ref().unwrap().as_str()).to_equal("path")?;
    expect(conf.cache.is_some()).to_equal(true)?;
    let cache_conf = conf.cache.as_ref().unwrap();
    expect(cache_conf.engine.as_ref().unwrap().as_str()).to_equal(CacheEngine::MEMORY)?;
    expect(cache_conf.memory.is_some()).to_equal(true)?;
    let cache_conf = cache_conf.memory.as_ref().unwrap();
    expect(cache_conf.device).to_equal(Some(100))?;
    expect(cache_conf.device_route).to_equal(Some(101))?;
    expect(cache_conf.network_route).to_equal(Some(102))?;
    let mq_conf = conf.mq.as_ref().unwrap();
    expect(mq_conf.prefetch.is_some()).to_equal(true)?;
    expect(mq_conf.prefetch.unwrap()).to_equal(10)?;
    expect(mq_conf.persistent.is_some()).to_equal(true)?;
    expect(mq_conf.persistent.unwrap()).to_equal(true)?;
    expect(mq_conf.shared_prefix.is_some()).to_equal(true)?;
    expect(mq_conf.shared_prefix.as_ref().unwrap().as_str()).to_equal("$shared/group/")?;
    let mq_channels_conf = conf.mq_channels.as_ref().unwrap();
    expect(mq_channels_conf.unit.is_some()).to_equal(true)?;
    let ctrl_conf = mq_channels_conf.unit.as_ref().unwrap();
    expect(ctrl_conf.url.is_some()).to_equal(true)?;
    expect(ctrl_conf.url.as_ref().unwrap().as_str()).to_equal("url3")?;
    expect(ctrl_conf.prefetch.is_some()).to_equal(true)?;
    expect(ctrl_conf.prefetch.unwrap()).to_equal(13)?;
    expect(mq_channels_conf.application.is_some()).to_equal(true)?;
    let ctrl_conf = mq_channels_conf.application.as_ref().unwrap();
    expect(ctrl_conf.url.is_some()).to_equal(true)?;
    expect(ctrl_conf.url.as_ref().unwrap().as_str()).to_equal("url4")?;
    expect(ctrl_conf.prefetch.is_some()).to_equal(true)?;
    expect(ctrl_conf.prefetch.unwrap()).to_equal(14)?;
    expect(mq_channels_conf.network.is_some()).to_equal(true)?;
    let ctrl_conf = mq_channels_conf.network.as_ref().unwrap();
    expect(ctrl_conf.url.is_some()).to_equal(true)?;
    expect(ctrl_conf.url.as_ref().unwrap().as_str()).to_equal("url5")?;
    expect(ctrl_conf.prefetch.is_some()).to_equal(true)?;
    expect(ctrl_conf.prefetch.unwrap()).to_equal(15)?;
    expect(mq_channels_conf.device.is_some()).to_equal(true)?;
    let ctrl_conf = mq_channels_conf.device.as_ref().unwrap();
    expect(ctrl_conf.url.is_some()).to_equal(true)?;
    expect(ctrl_conf.url.as_ref().unwrap().as_str()).to_equal("url6")?;
    expect(ctrl_conf.prefetch.is_some()).to_equal(true)?;
    expect(ctrl_conf.prefetch.unwrap()).to_equal(16)?;
    expect(mq_channels_conf.device_route.is_some()).to_equal(true)?;
    let ctrl_conf = mq_channels_conf.device_route.as_ref().unwrap();
    expect(ctrl_conf.url.is_some()).to_equal(true)?;
    expect(ctrl_conf.url.as_ref().unwrap().as_str()).to_equal("url7")?;
    expect(ctrl_conf.prefetch.is_some()).to_equal(true)?;
    expect(ctrl_conf.prefetch.unwrap()).to_equal(17)?;
    expect(mq_channels_conf.network_route.is_some()).to_equal(true)?;
    let ctrl_conf = mq_channels_conf.network_route.as_ref().unwrap();
    expect(ctrl_conf.url.is_some()).to_equal(true)?;
    expect(ctrl_conf.url.as_ref().unwrap().as_str()).to_equal("url8")?;
    expect(ctrl_conf.prefetch.is_some()).to_equal(true)?;
    expect(ctrl_conf.prefetch.unwrap()).to_equal(18)?;
    expect(mq_channels_conf.data.is_some()).to_equal(true)?;
    let data_conf = mq_channels_conf.data.as_ref().unwrap();
    expect(data_conf.url.is_some()).to_equal(true)?;
    expect(data_conf.url.as_ref().unwrap().as_str()).to_equal("url9")?;
    expect(data_conf.persistent.is_some()).to_equal(true)?;
    expect(data_conf.persistent.unwrap()).to_equal(false)?;
    expect(conf.api_scopes.as_ref()).to_equal(Some(&api_scopes))
}

fn set_env_var(key: &str, val: &str) {
    unsafe {
        env::set_var(&OsStr::new(key), val);
    }
}
