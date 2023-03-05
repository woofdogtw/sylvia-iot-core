use std::{collections::HashMap, env, ffi::OsStr};

use clap::Command;
use laboratory::{expect, SpecContext};

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
    let args = Command::new("test").get_matches();
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
    expect(*mq_conf.prefetch.as_ref().unwrap()).to_equal(config::DEF_MQ_PREFETCH)?;
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
    expect(*ctrl_conf.prefetch.as_ref().unwrap()).to_equal(config::DEF_MQ_PREFETCH)?;
    expect(mq_channels_conf.application.is_some()).to_equal(true)?;
    let ctrl_conf = mq_channels_conf.application.as_ref().unwrap();
    expect(ctrl_conf.url.is_some()).to_equal(true)?;
    expect(ctrl_conf.url.as_ref().unwrap().as_str()).to_equal(config::DEF_MQ_CHANNEL_URL)?;
    expect(ctrl_conf.prefetch.is_some()).to_equal(true)?;
    expect(*ctrl_conf.prefetch.as_ref().unwrap()).to_equal(config::DEF_MQ_PREFETCH)?;
    expect(mq_channels_conf.network.is_some()).to_equal(true)?;
    let ctrl_conf = mq_channels_conf.network.as_ref().unwrap();
    expect(ctrl_conf.url.is_some()).to_equal(true)?;
    expect(ctrl_conf.url.as_ref().unwrap().as_str()).to_equal(config::DEF_MQ_CHANNEL_URL)?;
    expect(ctrl_conf.prefetch.is_some()).to_equal(true)?;
    expect(*ctrl_conf.prefetch.as_ref().unwrap()).to_equal(config::DEF_MQ_PREFETCH)?;
    expect(mq_channels_conf.device.is_some()).to_equal(true)?;
    let ctrl_conf = mq_channels_conf.device.as_ref().unwrap();
    expect(ctrl_conf.url.is_some()).to_equal(true)?;
    expect(ctrl_conf.url.as_ref().unwrap().as_str()).to_equal(config::DEF_MQ_CHANNEL_URL)?;
    expect(ctrl_conf.prefetch.is_some()).to_equal(true)?;
    expect(*ctrl_conf.prefetch.as_ref().unwrap()).to_equal(config::DEF_MQ_PREFETCH)?;
    expect(mq_channels_conf.device_route.is_some()).to_equal(true)?;
    let ctrl_conf = mq_channels_conf.device_route.as_ref().unwrap();
    expect(ctrl_conf.url.is_some()).to_equal(true)?;
    expect(ctrl_conf.url.as_ref().unwrap().as_str()).to_equal(config::DEF_MQ_CHANNEL_URL)?;
    expect(ctrl_conf.prefetch.is_some()).to_equal(true)?;
    expect(*ctrl_conf.prefetch.as_ref().unwrap()).to_equal(config::DEF_MQ_PREFETCH)?;
    expect(mq_channels_conf.network_route.is_some()).to_equal(true)?;
    let ctrl_conf = mq_channels_conf.network_route.as_ref().unwrap();
    expect(ctrl_conf.url.is_some()).to_equal(true)?;
    expect(ctrl_conf.url.as_ref().unwrap().as_str()).to_equal(config::DEF_MQ_CHANNEL_URL)?;
    expect(ctrl_conf.prefetch.is_some()).to_equal(true)?;
    expect(*ctrl_conf.prefetch.as_ref().unwrap()).to_equal(config::DEF_MQ_PREFETCH)?;
    expect(mq_channels_conf.data.is_none()).to_equal(true)?;

    env::set_var(&OsStr::new("BROKER_AUTH"), "sylvia2");
    env::set_var(&OsStr::new("BROKER_DB_ENGINE"), "test2");
    env::set_var(&OsStr::new("BROKER_DB_MONGODB_URL"), "url2");
    env::set_var(&OsStr::new("BROKER_DB_MONGODB_DATABASE"), "db2");
    env::set_var(&OsStr::new("BROKER_DB_MONGODB_POOLSIZE"), "12");
    env::set_var(&OsStr::new("BROKER_DB_SQLITE_PATH"), "path2");
    env::set_var(&OsStr::new("BROKER_CACHE_ENGINE"), "test3");
    env::set_var(&OsStr::new("BROKER_MEMORY_DEVICE"), "100");
    env::set_var(&OsStr::new("BROKER_MEMORY_DEVICE_ROUTE"), "101");
    env::set_var(&OsStr::new("BROKER_MEMORY_NETWORK_ROUTE"), "102");
    env::set_var(&OsStr::new("BROKER_MQ_PREFETCH"), "10");
    env::set_var(&OsStr::new("BROKER_MQ_SHAREDPREFIX"), "$shared/group/");
    env::set_var(&OsStr::new("BROKER_MQCHANNELS_UNIT_URL"), "url3");
    env::set_var(&OsStr::new("BROKER_MQCHANNELS_UNIT_PREFETCH"), "13");
    env::set_var(&OsStr::new("BROKER_MQCHANNELS_APPLICATION_URL"), "url4");
    env::set_var(&OsStr::new("BROKER_MQCHANNELS_APPLICATION_PREFETCH"), "14");
    env::set_var(&OsStr::new("BROKER_MQCHANNELS_NETWORK_URL"), "url5");
    env::set_var(&OsStr::new("BROKER_MQCHANNELS_NETWORK_PREFETCH"), "15");
    env::set_var(&OsStr::new("BROKER_MQCHANNELS_DEVICE_URL"), "url6");
    env::set_var(&OsStr::new("BROKER_MQCHANNELS_DEVICE_PREFETCH"), "16");
    env::set_var(&OsStr::new("BROKER_MQCHANNELS_DEVICE_ROUTE_URL"), "url7");
    env::set_var(&OsStr::new("BROKER_MQCHANNELS_DEVICE_ROUTE_PREFETCH"), "17");
    env::set_var(&OsStr::new("BROKER_MQCHANNELS_NETWORK_ROUTE_URL"), "url8");
    env::set_var(
        &OsStr::new("BROKER_MQCHANNELS_NETWORK_ROUTE_PREFETCH"),
        "18",
    );
    env::set_var(&OsStr::new("BROKER_MQCHANNELS_DATA_URL"), "url9");
    let args = Command::new("test").get_matches();
    let conf = config::read_args(&args);
    expect(conf.auth.is_some()).to_equal(true)?;
    expect(conf.auth.as_ref().unwrap().as_str()).to_equal("sylvia2")?;
    expect(conf.db.is_some()).to_equal(true)?;
    expect(conf.db.as_ref().unwrap().engine.as_ref().unwrap().as_str())
        .to_equal(config::DEF_ENGINE)?;
    expect(conf.db.as_ref().unwrap().mongodb.is_some()).to_equal(true)?;
    expect(conf.db.as_ref().unwrap().sqlite.is_some()).to_equal(true)?;
    let db_conf = conf.db.as_ref().unwrap().mongodb.as_ref().unwrap();
    expect(db_conf.url.as_ref().unwrap().as_str()).to_equal("url2")?;
    expect(db_conf.database.as_ref().unwrap().as_str()).to_equal("db2")?;
    expect(db_conf.pool_size).to_equal(Some(12))?;
    let db_conf = conf.db.as_ref().unwrap().sqlite.as_ref().unwrap();
    expect(db_conf.path.as_ref().unwrap().as_str()).to_equal("path2")?;
    expect(conf.cache.is_some()).to_equal(true)?;
    let cache_conf = conf.cache.as_ref().unwrap();
    expect(cache_conf.engine.as_ref().unwrap().as_str()).to_equal(config::DEF_CACHE_ENGINE)?;
    expect(cache_conf.memory.is_some()).to_equal(true)?;
    let cache_conf = cache_conf.memory.as_ref().unwrap();
    expect(cache_conf.device).to_equal(Some(100))?;
    expect(cache_conf.device_route).to_equal(Some(101))?;
    expect(cache_conf.network_route).to_equal(Some(102))?;
    expect(conf.mq.is_some()).to_equal(true)?;
    let mq_conf = conf.mq.as_ref().unwrap();
    expect(mq_conf.prefetch.is_some()).to_equal(true)?;
    expect(*mq_conf.prefetch.as_ref().unwrap()).to_equal(10)?;
    expect(mq_conf.shared_prefix.is_some()).to_equal(true)?;
    expect(mq_conf.shared_prefix.as_ref().unwrap().as_str()).to_equal("$shared/group/")?;
    expect(conf.mq_channels.is_some()).to_equal(true)?;
    let mq_channels_conf = conf.mq_channels.as_ref().unwrap();
    expect(mq_channels_conf.application.is_some()).to_equal(true)?;
    let ctrl_conf = mq_channels_conf.application.as_ref().unwrap();
    expect(ctrl_conf.url.is_some()).to_equal(true)?;
    expect(ctrl_conf.url.as_ref().unwrap().as_str()).to_equal("url4")?;
    expect(ctrl_conf.prefetch.is_some()).to_equal(true)?;
    expect(*ctrl_conf.prefetch.as_ref().unwrap()).to_equal(14)?;
    expect(mq_channels_conf.network.is_some()).to_equal(true)?;
    let ctrl_conf = mq_channels_conf.network.as_ref().unwrap();
    expect(ctrl_conf.url.is_some()).to_equal(true)?;
    expect(ctrl_conf.url.as_ref().unwrap().as_str()).to_equal("url5")?;
    expect(ctrl_conf.prefetch.is_some()).to_equal(true)?;
    expect(*ctrl_conf.prefetch.as_ref().unwrap()).to_equal(15)?;
    expect(mq_channels_conf.device.is_some()).to_equal(true)?;
    let ctrl_conf = mq_channels_conf.device.as_ref().unwrap();
    expect(ctrl_conf.url.is_some()).to_equal(true)?;
    expect(ctrl_conf.url.as_ref().unwrap().as_str()).to_equal("url6")?;
    expect(ctrl_conf.prefetch.is_some()).to_equal(true)?;
    expect(*ctrl_conf.prefetch.as_ref().unwrap()).to_equal(16)?;
    expect(mq_channels_conf.device_route.is_some()).to_equal(true)?;
    let ctrl_conf = mq_channels_conf.device_route.as_ref().unwrap();
    expect(ctrl_conf.url.is_some()).to_equal(true)?;
    expect(ctrl_conf.url.as_ref().unwrap().as_str()).to_equal("url7")?;
    expect(ctrl_conf.prefetch.is_some()).to_equal(true)?;
    expect(*ctrl_conf.prefetch.as_ref().unwrap()).to_equal(17)?;
    expect(mq_channels_conf.network_route.is_some()).to_equal(true)?;
    let ctrl_conf = mq_channels_conf.network_route.as_ref().unwrap();
    expect(ctrl_conf.url.is_some()).to_equal(true)?;
    expect(ctrl_conf.url.as_ref().unwrap().as_str()).to_equal("url8")?;
    expect(ctrl_conf.prefetch.is_some()).to_equal(true)?;
    expect(*ctrl_conf.prefetch.as_ref().unwrap()).to_equal(18)?;
    expect(mq_channels_conf.data.is_some()).to_equal(true)?;
    let data_conf = mq_channels_conf.data.as_ref().unwrap();
    expect(data_conf.url.is_some()).to_equal(true)?;
    expect(data_conf.url.as_ref().unwrap().as_str()).to_equal("url9")?;

    env::set_var(&OsStr::new("BROKER_DB_MONGODB_POOLSIZE"), "12_000");
    let args = Command::new("test").get_matches();
    let conf = config::read_args(&args);
    let db_conf = conf.db.as_ref().unwrap().mongodb.as_ref().unwrap();
    expect(db_conf.pool_size).to_equal(None)
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
    expect(*mq_conf.prefetch.as_ref().unwrap()).to_equal(config::DEF_MQ_PREFETCH)?;
    expect(mq_conf.shared_prefix.is_some()).to_equal(true)?;
    expect(mq_conf.shared_prefix.as_ref().unwrap().as_str())
        .to_equal(config::DEF_MQ_SHAREDPREFIX)?;
    let mq_channels_conf = conf.mq_channels.as_ref().unwrap();
    expect(mq_channels_conf.unit.is_some()).to_equal(true)?;
    let ctrl_conf = mq_channels_conf.unit.as_ref().unwrap();
    expect(ctrl_conf.url.is_some()).to_equal(true)?;
    expect(ctrl_conf.url.as_ref().unwrap().as_str()).to_equal(config::DEF_MQ_CHANNEL_URL)?;
    expect(ctrl_conf.prefetch.is_some()).to_equal(true)?;
    expect(*ctrl_conf.prefetch.as_ref().unwrap()).to_equal(config::DEF_MQ_PREFETCH)?;
    expect(mq_channels_conf.application.is_some()).to_equal(true)?;
    let ctrl_conf = mq_channels_conf.application.as_ref().unwrap();
    expect(ctrl_conf.url.is_some()).to_equal(true)?;
    expect(ctrl_conf.url.as_ref().unwrap().as_str()).to_equal(config::DEF_MQ_CHANNEL_URL)?;
    expect(ctrl_conf.prefetch.is_some()).to_equal(true)?;
    expect(*ctrl_conf.prefetch.as_ref().unwrap()).to_equal(config::DEF_MQ_PREFETCH)?;
    expect(mq_channels_conf.network.is_some()).to_equal(true)?;
    let ctrl_conf = mq_channels_conf.network.as_ref().unwrap();
    expect(ctrl_conf.url.is_some()).to_equal(true)?;
    expect(ctrl_conf.url.as_ref().unwrap().as_str()).to_equal(config::DEF_MQ_CHANNEL_URL)?;
    expect(ctrl_conf.prefetch.is_some()).to_equal(true)?;
    expect(*ctrl_conf.prefetch.as_ref().unwrap()).to_equal(config::DEF_MQ_PREFETCH)?;
    expect(mq_channels_conf.device.is_some()).to_equal(true)?;
    let ctrl_conf = mq_channels_conf.device.as_ref().unwrap();
    expect(ctrl_conf.url.is_some()).to_equal(true)?;
    expect(ctrl_conf.url.as_ref().unwrap().as_str()).to_equal(config::DEF_MQ_CHANNEL_URL)?;
    expect(ctrl_conf.prefetch.is_some()).to_equal(true)?;
    expect(*ctrl_conf.prefetch.as_ref().unwrap()).to_equal(config::DEF_MQ_PREFETCH)?;
    expect(mq_channels_conf.device_route.is_some()).to_equal(true)?;
    let ctrl_conf = mq_channels_conf.device_route.as_ref().unwrap();
    expect(ctrl_conf.url.is_some()).to_equal(true)?;
    expect(ctrl_conf.url.as_ref().unwrap().as_str()).to_equal(config::DEF_MQ_CHANNEL_URL)?;
    expect(ctrl_conf.prefetch.is_some()).to_equal(true)?;
    expect(*ctrl_conf.prefetch.as_ref().unwrap()).to_equal(config::DEF_MQ_PREFETCH)?;
    expect(mq_channels_conf.network_route.is_some()).to_equal(true)?;
    let ctrl_conf = mq_channels_conf.network_route.as_ref().unwrap();
    expect(ctrl_conf.url.is_some()).to_equal(true)?;
    expect(ctrl_conf.url.as_ref().unwrap().as_str()).to_equal(config::DEF_MQ_CHANNEL_URL)?;
    expect(ctrl_conf.prefetch.is_some()).to_equal(true)?;
    expect(*ctrl_conf.prefetch.as_ref().unwrap()).to_equal(config::DEF_MQ_PREFETCH)?;
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
    expect(*mq_conf.prefetch.as_ref().unwrap()).to_equal(config::DEF_MQ_PREFETCH)?;
    expect(mq_conf.shared_prefix.is_some()).to_equal(true)?;
    expect(mq_conf.shared_prefix.as_ref().unwrap().as_str())
        .to_equal(config::DEF_MQ_SHAREDPREFIX)?;
    let mq_channels_conf = conf.mq_channels.as_ref().unwrap();
    expect(mq_channels_conf.unit.is_some()).to_equal(true)?;
    let ctrl_conf = mq_channels_conf.unit.as_ref().unwrap();
    expect(ctrl_conf.url.is_some()).to_equal(true)?;
    expect(ctrl_conf.url.as_ref().unwrap().as_str()).to_equal(config::DEF_MQ_CHANNEL_URL)?;
    expect(ctrl_conf.prefetch.is_some()).to_equal(true)?;
    expect(*ctrl_conf.prefetch.as_ref().unwrap()).to_equal(config::DEF_MQ_PREFETCH)?;
    expect(mq_channels_conf.application.is_some()).to_equal(true)?;
    let ctrl_conf = mq_channels_conf.application.as_ref().unwrap();
    expect(ctrl_conf.url.is_some()).to_equal(true)?;
    expect(ctrl_conf.url.as_ref().unwrap().as_str()).to_equal(config::DEF_MQ_CHANNEL_URL)?;
    expect(ctrl_conf.prefetch.is_some()).to_equal(true)?;
    expect(*ctrl_conf.prefetch.as_ref().unwrap()).to_equal(config::DEF_MQ_PREFETCH)?;
    expect(mq_channels_conf.network.is_some()).to_equal(true)?;
    let ctrl_conf = mq_channels_conf.network.as_ref().unwrap();
    expect(ctrl_conf.url.is_some()).to_equal(true)?;
    expect(ctrl_conf.url.as_ref().unwrap().as_str()).to_equal(config::DEF_MQ_CHANNEL_URL)?;
    expect(ctrl_conf.prefetch.is_some()).to_equal(true)?;
    expect(*ctrl_conf.prefetch.as_ref().unwrap()).to_equal(config::DEF_MQ_PREFETCH)?;
    expect(mq_channels_conf.device.is_some()).to_equal(true)?;
    let ctrl_conf = mq_channels_conf.device.as_ref().unwrap();
    expect(ctrl_conf.url.is_some()).to_equal(true)?;
    expect(ctrl_conf.url.as_ref().unwrap().as_str()).to_equal(config::DEF_MQ_CHANNEL_URL)?;
    expect(ctrl_conf.prefetch.is_some()).to_equal(true)?;
    expect(*ctrl_conf.prefetch.as_ref().unwrap()).to_equal(config::DEF_MQ_PREFETCH)?;
    expect(mq_channels_conf.device_route.is_some()).to_equal(true)?;
    let ctrl_conf = mq_channels_conf.device_route.as_ref().unwrap();
    expect(ctrl_conf.url.is_some()).to_equal(true)?;
    expect(ctrl_conf.url.as_ref().unwrap().as_str()).to_equal(config::DEF_MQ_CHANNEL_URL)?;
    expect(ctrl_conf.prefetch.is_some()).to_equal(true)?;
    expect(*ctrl_conf.prefetch.as_ref().unwrap()).to_equal(config::DEF_MQ_PREFETCH)?;
    expect(mq_channels_conf.network_route.is_some()).to_equal(true)?;
    let ctrl_conf = mq_channels_conf.network_route.as_ref().unwrap();
    expect(ctrl_conf.url.is_some()).to_equal(true)?;
    expect(ctrl_conf.url.as_ref().unwrap().as_str()).to_equal(config::DEF_MQ_CHANNEL_URL)?;
    expect(ctrl_conf.prefetch.is_some()).to_equal(true)?;
    expect(*ctrl_conf.prefetch.as_ref().unwrap()).to_equal(config::DEF_MQ_PREFETCH)?;
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
    expect(*mq_conf.prefetch.as_ref().unwrap()).to_equal(config::DEF_MQ_PREFETCH)?;
    expect(mq_conf.shared_prefix.is_some()).to_equal(true)?;
    expect(mq_conf.shared_prefix.as_ref().unwrap().as_str())
        .to_equal(config::DEF_MQ_SHAREDPREFIX)?;
    let mq_channels_conf = conf.mq_channels.as_ref().unwrap();
    expect(mq_channels_conf.unit.is_some()).to_equal(true)?;
    let ctrl_conf = mq_channels_conf.unit.as_ref().unwrap();
    expect(ctrl_conf.url.is_some()).to_equal(true)?;
    expect(ctrl_conf.url.as_ref().unwrap().as_str()).to_equal(config::DEF_MQ_CHANNEL_URL)?;
    expect(ctrl_conf.prefetch.is_some()).to_equal(true)?;
    expect(*ctrl_conf.prefetch.as_ref().unwrap()).to_equal(config::DEF_MQ_PREFETCH)?;
    expect(mq_channels_conf.application.is_some()).to_equal(true)?;
    let ctrl_conf = mq_channels_conf.application.as_ref().unwrap();
    expect(ctrl_conf.url.is_some()).to_equal(true)?;
    expect(ctrl_conf.url.as_ref().unwrap().as_str()).to_equal(config::DEF_MQ_CHANNEL_URL)?;
    expect(ctrl_conf.prefetch.is_some()).to_equal(true)?;
    expect(*ctrl_conf.prefetch.as_ref().unwrap()).to_equal(config::DEF_MQ_PREFETCH)?;
    let ctrl_conf = mq_channels_conf.network.as_ref().unwrap();
    expect(ctrl_conf.url.is_some()).to_equal(true)?;
    expect(ctrl_conf.url.as_ref().unwrap().as_str()).to_equal(config::DEF_MQ_CHANNEL_URL)?;
    expect(ctrl_conf.prefetch.is_some()).to_equal(true)?;
    expect(*ctrl_conf.prefetch.as_ref().unwrap()).to_equal(config::DEF_MQ_PREFETCH)?;
    expect(mq_channels_conf.device.is_some()).to_equal(true)?;
    let ctrl_conf = mq_channels_conf.device.as_ref().unwrap();
    expect(ctrl_conf.url.is_some()).to_equal(true)?;
    expect(ctrl_conf.url.as_ref().unwrap().as_str()).to_equal(config::DEF_MQ_CHANNEL_URL)?;
    expect(ctrl_conf.prefetch.is_some()).to_equal(true)?;
    expect(*ctrl_conf.prefetch.as_ref().unwrap()).to_equal(config::DEF_MQ_PREFETCH)?;
    expect(mq_channels_conf.device_route.is_some()).to_equal(true)?;
    let ctrl_conf = mq_channels_conf.device_route.as_ref().unwrap();
    expect(ctrl_conf.url.is_some()).to_equal(true)?;
    expect(ctrl_conf.url.as_ref().unwrap().as_str()).to_equal(config::DEF_MQ_CHANNEL_URL)?;
    expect(ctrl_conf.prefetch.is_some()).to_equal(true)?;
    expect(*ctrl_conf.prefetch.as_ref().unwrap()).to_equal(config::DEF_MQ_PREFETCH)?;
    expect(mq_channels_conf.network_route.is_some()).to_equal(true)?;
    let ctrl_conf = mq_channels_conf.network_route.as_ref().unwrap();
    expect(ctrl_conf.url.is_some()).to_equal(true)?;
    expect(ctrl_conf.url.as_ref().unwrap().as_str()).to_equal(config::DEF_MQ_CHANNEL_URL)?;
    expect(ctrl_conf.prefetch.is_some()).to_equal(true)?;
    expect(*ctrl_conf.prefetch.as_ref().unwrap()).to_equal(config::DEF_MQ_PREFETCH)?;
    expect(mq_channels_conf.data.is_none()).to_equal(true)?;
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
    expect(*mq_conf.prefetch.as_ref().unwrap()).to_equal(10)?;
    expect(mq_conf.shared_prefix.is_some()).to_equal(true)?;
    expect(mq_conf.shared_prefix.as_ref().unwrap().as_str()).to_equal("$shared/group/")?;
    let mq_channels_conf = conf.mq_channels.as_ref().unwrap();
    expect(mq_channels_conf.unit.is_some()).to_equal(true)?;
    let ctrl_conf = mq_channels_conf.unit.as_ref().unwrap();
    expect(ctrl_conf.url.is_some()).to_equal(true)?;
    expect(ctrl_conf.url.as_ref().unwrap().as_str()).to_equal("url3")?;
    expect(ctrl_conf.prefetch.is_some()).to_equal(true)?;
    expect(*ctrl_conf.prefetch.as_ref().unwrap()).to_equal(13)?;
    expect(mq_channels_conf.application.is_some()).to_equal(true)?;
    let ctrl_conf = mq_channels_conf.application.as_ref().unwrap();
    expect(ctrl_conf.url.is_some()).to_equal(true)?;
    expect(ctrl_conf.url.as_ref().unwrap().as_str()).to_equal("url4")?;
    expect(ctrl_conf.prefetch.is_some()).to_equal(true)?;
    expect(*ctrl_conf.prefetch.as_ref().unwrap()).to_equal(14)?;
    expect(mq_channels_conf.network.is_some()).to_equal(true)?;
    let ctrl_conf = mq_channels_conf.network.as_ref().unwrap();
    expect(ctrl_conf.url.is_some()).to_equal(true)?;
    expect(ctrl_conf.url.as_ref().unwrap().as_str()).to_equal("url5")?;
    expect(ctrl_conf.prefetch.is_some()).to_equal(true)?;
    expect(*ctrl_conf.prefetch.as_ref().unwrap()).to_equal(15)?;
    expect(mq_channels_conf.device.is_some()).to_equal(true)?;
    let ctrl_conf = mq_channels_conf.device.as_ref().unwrap();
    expect(ctrl_conf.url.is_some()).to_equal(true)?;
    expect(ctrl_conf.url.as_ref().unwrap().as_str()).to_equal("url6")?;
    expect(ctrl_conf.prefetch.is_some()).to_equal(true)?;
    expect(*ctrl_conf.prefetch.as_ref().unwrap()).to_equal(16)?;
    expect(mq_channels_conf.device_route.is_some()).to_equal(true)?;
    let ctrl_conf = mq_channels_conf.device_route.as_ref().unwrap();
    expect(ctrl_conf.url.is_some()).to_equal(true)?;
    expect(ctrl_conf.url.as_ref().unwrap().as_str()).to_equal("url7")?;
    expect(ctrl_conf.prefetch.is_some()).to_equal(true)?;
    expect(*ctrl_conf.prefetch.as_ref().unwrap()).to_equal(17)?;
    expect(mq_channels_conf.network_route.is_some()).to_equal(true)?;
    let ctrl_conf = mq_channels_conf.network_route.as_ref().unwrap();
    expect(ctrl_conf.url.is_some()).to_equal(true)?;
    expect(ctrl_conf.url.as_ref().unwrap().as_str()).to_equal("url8")?;
    expect(ctrl_conf.prefetch.is_some()).to_equal(true)?;
    expect(*ctrl_conf.prefetch.as_ref().unwrap()).to_equal(18)?;
    expect(mq_channels_conf.data.is_some()).to_equal(true)?;
    let data_conf = mq_channels_conf.data.as_ref().unwrap();
    expect(data_conf.url.is_some()).to_equal(true)?;
    expect(data_conf.url.as_ref().unwrap().as_str()).to_equal("url9")?;
    expect(conf.api_scopes.as_ref()).to_equal(Some(&api_scopes))
}
