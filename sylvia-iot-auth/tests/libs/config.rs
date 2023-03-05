use std::{collections::HashMap, env, ffi::OsStr};

use clap::Command;
use laboratory::{expect, SpecContext};

use sylvia_iot_auth::libs::config::{self, Config};
use sylvia_iot_corelib::constants::DbEngine;

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
    expect(conf.api_scopes.as_ref()).to_equal(Some(&HashMap::new()))?;
    expect(conf.templates.as_ref()).to_equal(Some(&HashMap::new()))?;

    env::set_var(&OsStr::new("AUTH_DB_ENGINE"), "test2");
    env::set_var(&OsStr::new("AUTH_DB_MONGODB_URL"), "url2");
    env::set_var(&OsStr::new("AUTH_DB_MONGODB_DATABASE"), "db2");
    env::set_var(&OsStr::new("AUTH_DB_MONGODB_POOLSIZE"), "12");
    env::set_var(&OsStr::new("AUTH_DB_SQLITE_PATH"), "path2");
    let args = Command::new("test").get_matches();
    let conf = config::read_args(&args);
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

    env::set_var(&OsStr::new("AUTH_DB_MONGODB_POOLSIZE"), "12_000");
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
    expect(conf.api_scopes.as_ref()).to_equal(Some(&HashMap::new()))?;
    expect(conf.templates.as_ref()).to_equal(Some(&HashMap::new()))?;

    let conf = Config {
        db: Some(config::Db {
            ..Default::default()
        }),
        ..Default::default()
    };
    let conf = config::apply_default(&conf);
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
    expect(conf.api_scopes.as_ref()).to_equal(Some(&HashMap::new()))?;
    expect(conf.templates.as_ref()).to_equal(Some(&HashMap::new()))?;

    let conf = Config {
        db: Some(config::Db {
            engine: Some(DbEngine::MONGODB.to_string()),
            ..Default::default()
        }),
        ..Default::default()
    };
    let conf = config::apply_default(&conf);
    expect(conf.db.is_some()).to_equal(true)?;
    expect(conf.db.as_ref().unwrap().engine.as_ref().unwrap().as_str())
        .to_equal(DbEngine::MONGODB)?;

    let mut api_scopes: HashMap<String, Vec<String>> = HashMap::new();
    api_scopes.insert("api".to_string(), vec!["scope1".to_string()]);
    let mut templates: HashMap<String, String> = HashMap::new();
    templates.insert("template".to_string(), "path".to_string());
    let conf = Config {
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
        api_scopes: Some(api_scopes.clone()),
        templates: Some(templates.clone()),
    };
    let conf = config::apply_default(&conf);
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
    expect(conf.api_scopes.as_ref()).to_equal(Some(&api_scopes))?;
    expect(conf.templates.as_ref()).to_equal(Some(&templates))
}
