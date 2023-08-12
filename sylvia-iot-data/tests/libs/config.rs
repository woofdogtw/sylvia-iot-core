use std::{env, ffi::OsStr};

use clap::Command;
use laboratory::{expect, SpecContext};

use sylvia_iot_corelib::constants::DbEngine;
use sylvia_iot_data::libs::config::{self, Config};

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
    expect(conf.broker.is_some()).to_equal(true)?;
    expect(conf.broker.as_ref().unwrap().as_str()).to_equal(config::DEF_BROKER)?;
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
    expect(conf.mq_channels.is_some()).to_equal(true)?;
    let mq_channels_conf = conf.mq_channels.as_ref().unwrap();
    expect(mq_channels_conf.broker.is_some()).to_equal(true)?;
    let data_conf = mq_channels_conf.broker.as_ref().unwrap();
    expect(data_conf.url.is_some()).to_equal(true)?;
    expect(data_conf.url.as_ref().unwrap().as_str()).to_equal(config::DEF_MQ_CHANNEL_URL)?;
    expect(data_conf.prefetch.is_some()).to_equal(true)?;
    expect(data_conf.prefetch.unwrap()).to_equal(config::DEF_MQ_PREFETCH)?;
    expect(data_conf.shared_prefix.is_some()).to_equal(true)?;
    expect(data_conf.shared_prefix.as_ref().unwrap().as_str())
        .to_equal(config::DEF_MQ_SHAREDPREFIX)?;
    expect(mq_channels_conf.coremgr.is_some()).to_equal(true)?;
    let data_conf = mq_channels_conf.coremgr.as_ref().unwrap();
    expect(data_conf.url.is_some()).to_equal(true)?;
    expect(data_conf.url.as_ref().unwrap().as_str()).to_equal(config::DEF_MQ_CHANNEL_URL)?;
    expect(data_conf.prefetch.is_some()).to_equal(true)?;
    expect(data_conf.prefetch.unwrap()).to_equal(config::DEF_MQ_PREFETCH)?;
    expect(data_conf.shared_prefix.is_some()).to_equal(true)?;
    expect(data_conf.shared_prefix.as_ref().unwrap().as_str())
        .to_equal(config::DEF_MQ_SHAREDPREFIX)?;

    // Modified default by command-line arguments.
    let args = vec![
        "test",
        "--data.auth",
        "sylvia11",
        "--data.broker",
        "sylvia12",
        "--data.db.engine",
        "mongodb",
        "--data.db.mongodb.url",
        "url11",
        "--data.db.mongodb.database",
        "db1",
        "--data.db.mongodb.poolsize",
        "11",
        "--data.db.sqlite.path",
        "path1",
        "--data.mq-channels.broker.url",
        "url12",
        "--data.mq-channels.broker.prefetch",
        "12",
        "--data.mq-channels.broker.sharedprefix",
        "prefix12",
        "--data.mq-channels.coremgr.url",
        "url13",
        "--data.mq-channels.coremgr.prefetch",
        "13",
        "--data.mq-channels.coremgr.sharedprefix",
        "prefix13",
    ];
    let args = config::reg_args(Command::new("test")).get_matches_from(args);
    let conf = config::read_args(&args);
    expect(conf.auth.is_some()).to_equal(true)?;
    expect(conf.auth.as_ref().unwrap().as_str()).to_equal("sylvia11")?;
    expect(conf.broker.is_some()).to_equal(true)?;
    expect(conf.broker.as_ref().unwrap().as_str()).to_equal("sylvia12")?;
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
    expect(conf.mq_channels.is_some()).to_equal(true)?;
    let mq_channels_conf = conf.mq_channels.as_ref().unwrap();
    expect(mq_channels_conf.broker.is_some()).to_equal(true)?;
    let data_conf = mq_channels_conf.broker.as_ref().unwrap();
    expect(data_conf.url.is_some()).to_equal(true)?;
    expect(data_conf.url.as_ref().unwrap().as_str()).to_equal("url12")?;
    expect(data_conf.prefetch.is_some()).to_equal(true)?;
    expect(data_conf.prefetch.unwrap()).to_equal(12)?;
    expect(data_conf.shared_prefix.is_some()).to_equal(true)?;
    expect(data_conf.shared_prefix.as_ref().unwrap().as_str()).to_equal("prefix12")?;
    expect(mq_channels_conf.coremgr.is_some()).to_equal(true)?;
    let data_conf = mq_channels_conf.coremgr.as_ref().unwrap();
    expect(data_conf.url.is_some()).to_equal(true)?;
    expect(data_conf.url.as_ref().unwrap().as_str()).to_equal("url13")?;
    expect(data_conf.prefetch.is_some()).to_equal(true)?;
    expect(data_conf.prefetch.unwrap()).to_equal(13)?;
    expect(data_conf.shared_prefix.is_some()).to_equal(true)?;
    expect(data_conf.shared_prefix.as_ref().unwrap().as_str()).to_equal("prefix13")?;

    // Clear command-line arguments.
    let args = config::reg_args(Command::new("test")).get_matches_from(vec!["test"]);

    env::set_var(&OsStr::new("DATA_AUTH"), "sylvia21");
    env::set_var(&OsStr::new("DATA_BROKER"), "sylvia22");
    env::set_var(&OsStr::new("DATA_DB_ENGINE"), "sqlite");
    env::set_var(&OsStr::new("DATA_DB_MONGODB_URL"), "url21");
    env::set_var(&OsStr::new("DATA_DB_MONGODB_DATABASE"), "db2");
    env::set_var(&OsStr::new("DATA_DB_MONGODB_POOLSIZE"), "21");
    env::set_var(&OsStr::new("DATA_DB_SQLITE_PATH"), "path2");
    env::set_var(&OsStr::new("DATA_MQCHANNELS_BROKER_URL"), "url22");
    env::set_var(&OsStr::new("DATA_MQCHANNELS_BROKER_PREFETCH"), "22");
    env::set_var(
        &OsStr::new("DATA_MQCHANNELS_BROKER_SHAREDPREFIX"),
        "prefix22",
    );
    env::set_var(&OsStr::new("DATA_MQCHANNELS_COREMGR_URL"), "url23");
    env::set_var(&OsStr::new("DATA_MQCHANNELS_COREMGR_PREFETCH"), "23");
    env::set_var(
        &OsStr::new("DATA_MQCHANNELS_COREMGR_SHAREDPREFIX"),
        "prefix23",
    );
    let conf = config::read_args(&args);
    expect(conf.auth.is_some()).to_equal(true)?;
    expect(conf.auth.as_ref().unwrap().as_str()).to_equal("sylvia21")?;
    expect(conf.broker.is_some()).to_equal(true)?;
    expect(conf.broker.as_ref().unwrap().as_str()).to_equal("sylvia22")?;
    expect(conf.db.is_some()).to_equal(true)?;
    expect(conf.db.as_ref().unwrap().engine.as_ref().unwrap().as_str())
        .to_equal(DbEngine::SQLITE)?;
    expect(conf.db.as_ref().unwrap().mongodb.is_some()).to_equal(true)?;
    expect(conf.db.as_ref().unwrap().sqlite.is_some()).to_equal(true)?;
    let db_conf = conf.db.as_ref().unwrap().mongodb.as_ref().unwrap();
    expect(db_conf.url.as_ref().unwrap().as_str()).to_equal("url21")?;
    expect(db_conf.database.as_ref().unwrap().as_str()).to_equal("db2")?;
    expect(db_conf.pool_size).to_equal(Some(21))?;
    let db_conf = conf.db.as_ref().unwrap().sqlite.as_ref().unwrap();
    expect(db_conf.path.as_ref().unwrap().as_str()).to_equal("path2")?;
    expect(conf.mq_channels.is_some()).to_equal(true)?;
    let mq_channels_conf = conf.mq_channels.as_ref().unwrap();
    expect(mq_channels_conf.broker.is_some()).to_equal(true)?;
    let data_conf = mq_channels_conf.broker.as_ref().unwrap();
    expect(data_conf.url.is_some()).to_equal(true)?;
    expect(data_conf.url.as_ref().unwrap().as_str()).to_equal("url22")?;
    expect(data_conf.prefetch.is_some()).to_equal(true)?;
    expect(data_conf.prefetch.unwrap()).to_equal(22)?;
    expect(data_conf.shared_prefix.is_some()).to_equal(true)?;
    expect(data_conf.shared_prefix.as_ref().unwrap().as_str()).to_equal("prefix22")?;
    expect(mq_channels_conf.coremgr.is_some()).to_equal(true)?;
    let data_conf = mq_channels_conf.coremgr.as_ref().unwrap();
    expect(data_conf.url.is_some()).to_equal(true)?;
    expect(data_conf.url.as_ref().unwrap().as_str()).to_equal("url23")?;
    expect(data_conf.prefetch.is_some()).to_equal(true)?;
    expect(data_conf.prefetch.unwrap()).to_equal(23)?;
    expect(data_conf.shared_prefix.is_some()).to_equal(true)?;
    expect(data_conf.shared_prefix.as_ref().unwrap().as_str()).to_equal("prefix23")?;

    // Test wrong environment variables.
    env::set_var(&OsStr::new("DATA_DB_ENGINE"), "mongodb1");
    env::set_var(&OsStr::new("DATA_DB_MONGODB_POOLSIZE"), "12_000");
    env::set_var(&OsStr::new("DATA_MQCHANNELS_BROKER_PREFETCH"), "12_000");
    env::set_var(&OsStr::new("DATA_MQCHANNELS_COREMGR_PREFETCH"), "12_000");
    let conf = config::read_args(&args);
    expect(conf.db.is_some()).to_equal(true)?;
    expect(conf.db.as_ref().unwrap().engine.as_ref().unwrap().as_str())
        .to_equal(config::DEF_ENGINE)?;
    let db_conf = conf.db.as_ref().unwrap().mongodb.as_ref().unwrap();
    expect(db_conf.pool_size).to_equal(None)?;
    expect(conf.mq_channels.is_some()).to_equal(true)?;
    let mq_channels_conf = conf.mq_channels.as_ref().unwrap();
    expect(mq_channels_conf.broker.is_some()).to_equal(true)?;
    let data_conf = mq_channels_conf.broker.as_ref().unwrap();
    expect(data_conf.prefetch.is_some()).to_equal(true)?;
    expect(data_conf.prefetch.unwrap()).to_equal(config::DEF_MQ_PREFETCH)?;
    expect(mq_channels_conf.coremgr.is_some()).to_equal(true)?;
    let data_conf = mq_channels_conf.coremgr.as_ref().unwrap();
    expect(data_conf.prefetch.is_some()).to_equal(true)?;
    expect(data_conf.prefetch.unwrap()).to_equal(config::DEF_MQ_PREFETCH)?;

    // Test command-line arguments overwrite environment variables.
    let args = vec![
        "test",
        "--data.auth",
        "sylvia31",
        "--data.broker",
        "sylvia32",
        "--data.db.engine",
        "mongodb",
        "--data.db.mongodb.url",
        "url31",
        "--data.db.mongodb.database",
        "db3",
        "--data.db.mongodb.poolsize",
        "31",
        "--data.db.sqlite.path",
        "path3",
        "--data.mq-channels.broker.url",
        "url32",
        "--data.mq-channels.broker.prefetch",
        "32",
        "--data.mq-channels.broker.sharedprefix",
        "prefix32",
        "--data.mq-channels.coremgr.url",
        "url33",
        "--data.mq-channels.coremgr.prefetch",
        "33",
        "--data.mq-channels.coremgr.sharedprefix",
        "prefix33",
    ];
    env::set_var(&OsStr::new("DATA_AUTH"), "sylvia41");
    env::set_var(&OsStr::new("DATA_BROKER"), "sylvia42");
    env::set_var(&OsStr::new("DATA_DB_ENGINE"), "sqlite");
    env::set_var(&OsStr::new("DATA_DB_MONGODB_URL"), "url41");
    env::set_var(&OsStr::new("DATA_DB_MONGODB_DATABASE"), "db4");
    env::set_var(&OsStr::new("DATA_DB_MONGODB_POOLSIZE"), "41");
    env::set_var(&OsStr::new("DATA_DB_SQLITE_PATH"), "path4");
    env::set_var(&OsStr::new("DATA_MQCHANNELS_BROKER_URL"), "url42");
    env::set_var(&OsStr::new("DATA_MQCHANNELS_BROKER_PREFETCH"), "42");
    env::set_var(
        &OsStr::new("DATA_MQCHANNELS_BROKER_SHAREDPREFIX"),
        "prefix42",
    );
    env::set_var(&OsStr::new("DATA_MQCHANNELS_COREMGR_URL"), "url43");
    env::set_var(&OsStr::new("DATA_MQCHANNELS_COREMGR_PREFETCH"), "43");
    env::set_var(
        &OsStr::new("DATA_MQCHANNELS_COREMGR_SHAREDPREFIX"),
        "prefix43",
    );
    let args = config::reg_args(Command::new("test")).get_matches_from(args);
    let conf = config::read_args(&args);
    expect(conf.auth.is_some()).to_equal(true)?;
    expect(conf.auth.as_ref().unwrap().as_str()).to_equal("sylvia31")?;
    expect(conf.broker.is_some()).to_equal(true)?;
    expect(conf.broker.as_ref().unwrap().as_str()).to_equal("sylvia32")?;
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
    expect(conf.mq_channels.is_some()).to_equal(true)?;
    let mq_channels_conf = conf.mq_channels.as_ref().unwrap();
    expect(mq_channels_conf.broker.is_some()).to_equal(true)?;
    let data_conf = mq_channels_conf.broker.as_ref().unwrap();
    expect(data_conf.url.is_some()).to_equal(true)?;
    expect(data_conf.url.as_ref().unwrap().as_str()).to_equal("url32")?;
    expect(data_conf.prefetch.is_some()).to_equal(true)?;
    expect(data_conf.prefetch.unwrap()).to_equal(32)?;
    expect(data_conf.shared_prefix.is_some()).to_equal(true)?;
    expect(data_conf.shared_prefix.as_ref().unwrap().as_str()).to_equal("prefix32")?;
    expect(mq_channels_conf.coremgr.is_some()).to_equal(true)?;
    let data_conf = mq_channels_conf.coremgr.as_ref().unwrap();
    expect(data_conf.url.is_some()).to_equal(true)?;
    expect(data_conf.url.as_ref().unwrap().as_str()).to_equal("url33")?;
    expect(data_conf.prefetch.is_some()).to_equal(true)?;
    expect(data_conf.prefetch.unwrap()).to_equal(33)?;
    expect(data_conf.shared_prefix.is_some()).to_equal(true)?;
    expect(data_conf.shared_prefix.as_ref().unwrap().as_str()).to_equal("prefix33")
}

/// Test [`config::apply_default`].
pub fn apply_default(_context: &mut SpecContext<TestState>) -> Result<(), String> {
    let conf = Config {
        ..Default::default()
    };
    let conf = config::apply_default(&conf);
    expect(conf.auth.is_some()).to_equal(true)?;
    expect(conf.auth.as_ref().unwrap().as_str()).to_equal(config::DEF_AUTH)?;
    expect(conf.broker.is_some()).to_equal(true)?;
    expect(conf.broker.as_ref().unwrap().as_str()).to_equal(config::DEF_BROKER)?;
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
    expect(conf.mq_channels.is_some()).to_equal(true)?;
    let mq_channels_conf = conf.mq_channels.as_ref().unwrap();
    expect(mq_channels_conf.broker.is_some()).to_equal(true)?;
    let data_conf = mq_channels_conf.broker.as_ref().unwrap();
    expect(data_conf.url.is_some()).to_equal(true)?;
    expect(data_conf.url.as_ref().unwrap().as_str()).to_equal(config::DEF_MQ_CHANNEL_URL)?;
    expect(data_conf.prefetch.is_some()).to_equal(true)?;
    expect(data_conf.prefetch.unwrap()).to_equal(config::DEF_MQ_PREFETCH)?;
    expect(data_conf.shared_prefix.is_some()).to_equal(true)?;
    expect(data_conf.shared_prefix.as_ref().unwrap().as_str())
        .to_equal(config::DEF_MQ_SHAREDPREFIX)?;
    expect(mq_channels_conf.coremgr.is_some()).to_equal(true)?;
    let data_conf = mq_channels_conf.coremgr.as_ref().unwrap();
    expect(data_conf.url.is_some()).to_equal(true)?;
    expect(data_conf.url.as_ref().unwrap().as_str()).to_equal(config::DEF_MQ_CHANNEL_URL)?;
    expect(data_conf.prefetch.is_some()).to_equal(true)?;
    expect(data_conf.prefetch.unwrap()).to_equal(config::DEF_MQ_PREFETCH)?;
    expect(data_conf.shared_prefix.is_some()).to_equal(true)?;
    expect(data_conf.shared_prefix.as_ref().unwrap().as_str())
        .to_equal(config::DEF_MQ_SHAREDPREFIX)?;

    let conf = Config {
        db: Some(config::Db {
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
    expect(conf.broker.is_some()).to_equal(true)?;
    expect(conf.broker.as_ref().unwrap().as_str()).to_equal(config::DEF_BROKER)?;
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
    expect(conf.mq_channels.is_some()).to_equal(true)?;
    let mq_channels_conf = conf.mq_channels.as_ref().unwrap();
    expect(mq_channels_conf.broker.is_some()).to_equal(true)?;
    let data_conf = mq_channels_conf.broker.as_ref().unwrap();
    expect(data_conf.url.is_some()).to_equal(true)?;
    expect(data_conf.url.as_ref().unwrap().as_str()).to_equal(config::DEF_MQ_CHANNEL_URL)?;
    expect(data_conf.prefetch.is_some()).to_equal(true)?;
    expect(data_conf.prefetch.unwrap()).to_equal(config::DEF_MQ_PREFETCH)?;
    expect(data_conf.shared_prefix.is_some()).to_equal(true)?;
    expect(data_conf.shared_prefix.as_ref().unwrap().as_str())
        .to_equal(config::DEF_MQ_SHAREDPREFIX)?;
    expect(mq_channels_conf.coremgr.is_some()).to_equal(true)?;
    let data_conf = mq_channels_conf.coremgr.as_ref().unwrap();
    expect(data_conf.url.is_some()).to_equal(true)?;
    expect(data_conf.url.as_ref().unwrap().as_str()).to_equal(config::DEF_MQ_CHANNEL_URL)?;
    expect(data_conf.prefetch.is_some()).to_equal(true)?;
    expect(data_conf.prefetch.unwrap()).to_equal(config::DEF_MQ_PREFETCH)?;
    expect(data_conf.shared_prefix.is_some()).to_equal(true)?;
    expect(data_conf.shared_prefix.as_ref().unwrap().as_str())
        .to_equal(config::DEF_MQ_SHAREDPREFIX)?;

    let conf = Config {
        auth: Some("sylvia2".to_string()),
        broker: Some("sylvia3".to_string()),
        db: Some(config::Db {
            engine: Some(DbEngine::MONGODB.to_string()),
            ..Default::default()
        }),
        mq_channels: Some(config::MqChannels {
            broker: Some(config::DataData {
                ..Default::default()
            }),
            coremgr: Some(config::DataData {
                ..Default::default()
            }),
        }),
    };
    let conf = config::apply_default(&conf);
    expect(conf.auth.is_some()).to_equal(true)?;
    expect(conf.auth.as_ref().unwrap().as_str()).to_equal("sylvia2")?;
    expect(conf.broker.is_some()).to_equal(true)?;
    expect(conf.broker.as_ref().unwrap().as_str()).to_equal("sylvia3")?;
    expect(conf.db.is_some()).to_equal(true)?;
    expect(conf.db.as_ref().unwrap().engine.as_ref().unwrap().as_str())
        .to_equal(DbEngine::MONGODB)?;
    expect(conf.mq_channels.is_some()).to_equal(true)?;
    let mq_channels_conf = conf.mq_channels.as_ref().unwrap();
    expect(mq_channels_conf.broker.is_some()).to_equal(true)?;
    let data_conf = mq_channels_conf.broker.as_ref().unwrap();
    expect(data_conf.url.is_some()).to_equal(true)?;
    expect(data_conf.url.as_ref().unwrap().as_str()).to_equal(config::DEF_MQ_CHANNEL_URL)?;
    expect(data_conf.prefetch.is_some()).to_equal(true)?;
    expect(data_conf.prefetch.unwrap()).to_equal(config::DEF_MQ_PREFETCH)?;
    expect(data_conf.shared_prefix.is_some()).to_equal(true)?;
    expect(data_conf.shared_prefix.as_ref().unwrap().as_str())
        .to_equal(config::DEF_MQ_SHAREDPREFIX)?;
    expect(mq_channels_conf.coremgr.is_some()).to_equal(true)?;
    let data_conf = mq_channels_conf.coremgr.as_ref().unwrap();
    expect(data_conf.url.is_some()).to_equal(true)?;
    expect(data_conf.url.as_ref().unwrap().as_str()).to_equal(config::DEF_MQ_CHANNEL_URL)?;
    expect(data_conf.prefetch.is_some()).to_equal(true)?;
    expect(data_conf.prefetch.unwrap()).to_equal(config::DEF_MQ_PREFETCH)?;
    expect(data_conf.shared_prefix.is_some()).to_equal(true)?;
    expect(data_conf.shared_prefix.as_ref().unwrap().as_str())
        .to_equal(config::DEF_MQ_SHAREDPREFIX)?;

    let conf = Config {
        auth: Some("sylvia3".to_string()),
        broker: Some("sylvia2".to_string()),
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
        mq_channels: Some(config::MqChannels {
            broker: Some(config::DataData {
                url: Some("url3".to_string()),
                prefetch: Some(13),
                shared_prefix: Some("$shared/group3".to_string()),
            }),
            coremgr: Some(config::DataData {
                url: Some("url4".to_string()),
                prefetch: Some(14),
                shared_prefix: Some("$shared/group4".to_string()),
            }),
        }),
    };
    let conf = config::apply_default(&conf);
    expect(conf.auth.is_some()).to_equal(true)?;
    expect(conf.auth.as_ref().unwrap().as_str()).to_equal("sylvia3")?;
    expect(conf.broker.is_some()).to_equal(true)?;
    expect(conf.broker.as_ref().unwrap().as_str()).to_equal("sylvia2")?;
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
    expect(conf.mq_channels.is_some()).to_equal(true)?;
    let mq_channels_conf = conf.mq_channels.as_ref().unwrap();
    expect(mq_channels_conf.broker.is_some()).to_equal(true)?;
    let data_conf = mq_channels_conf.broker.as_ref().unwrap();
    expect(data_conf.url.is_some()).to_equal(true)?;
    expect(data_conf.url.as_ref().unwrap().as_str()).to_equal("url3")?;
    expect(data_conf.prefetch.is_some()).to_equal(true)?;
    expect(data_conf.prefetch.unwrap()).to_equal(13)?;
    expect(data_conf.shared_prefix.is_some()).to_equal(true)?;
    expect(data_conf.shared_prefix.as_ref().unwrap().as_str()).to_equal("$shared/group3")?;
    expect(mq_channels_conf.coremgr.is_some()).to_equal(true)?;
    let data_conf = mq_channels_conf.coremgr.as_ref().unwrap();
    expect(data_conf.url.is_some()).to_equal(true)?;
    expect(data_conf.url.as_ref().unwrap().as_str()).to_equal("url4")?;
    expect(data_conf.prefetch.is_some()).to_equal(true)?;
    expect(data_conf.prefetch.unwrap()).to_equal(14)?;
    expect(data_conf.shared_prefix.is_some()).to_equal(true)?;
    expect(data_conf.shared_prefix.as_ref().unwrap().as_str()).to_equal("$shared/group4")
}
