use std::{collections::HashMap, env, ffi::OsStr};

use clap::Command;
use laboratory::{SpecContext, expect};

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
    // Default.
    let args = config::reg_args(Command::new("test")).get_matches_from(vec!["test"]);
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

    // Modified default by command-line arguments.
    let args = vec![
        "test",
        "--auth.db.engine",
        "mongodb",
        "--auth.db.mongodb.url",
        "url1",
        "--auth.db.mongodb.database",
        "db1",
        "--auth.db.mongodb.poolsize",
        "11",
        "--auth.db.sqlite.path",
        "path1",
        "--auth.api-scopes",
        "{\"key11\":[\"value11\"]}",
        "--auth.templates",
        "{\"key12\":\"value12\"}",
    ];
    let args = config::reg_args(Command::new("test")).get_matches_from(args);
    let conf = config::read_args(&args);
    expect(conf.db.is_some()).to_equal(true)?;
    expect(conf.db.as_ref().unwrap().engine.as_ref().unwrap().as_str())
        .to_equal(DbEngine::MONGODB)?;
    expect(conf.db.as_ref().unwrap().mongodb.is_some()).to_equal(true)?;
    expect(conf.db.as_ref().unwrap().sqlite.is_some()).to_equal(true)?;
    let db_conf = conf.db.as_ref().unwrap().mongodb.as_ref().unwrap();
    expect(db_conf.url.as_ref().unwrap().as_str()).to_equal("url1")?;
    expect(db_conf.database.as_ref().unwrap().as_str()).to_equal("db1")?;
    expect(db_conf.pool_size).to_equal(Some(11))?;
    let db_conf = conf.db.as_ref().unwrap().sqlite.as_ref().unwrap();
    expect(db_conf.path.as_ref().unwrap().as_str()).to_equal("path1")?;
    let mut map: HashMap<String, Vec<String>> = HashMap::new();
    map.insert("key11".to_string(), vec!["value11".to_string()]);
    expect(conf.api_scopes.as_ref()).to_equal(Some(&map))?;
    let mut map: HashMap<String, String> = HashMap::new();
    map.insert("key12".to_string(), "value12".to_string());
    expect(conf.templates.as_ref()).to_equal(Some(&map))?;

    let args = vec![
        "test",
        "--auth.db.engine",
        "sqlite",
        "--auth.api-scopes",
        "",
        "--auth.templates",
        "",
    ];
    let args = config::reg_args(Command::new("test")).get_matches_from(args);
    let conf = config::read_args(&args);
    expect(conf.db.is_some()).to_equal(true)?;
    expect(conf.db.as_ref().unwrap().engine.as_ref().unwrap().as_str())
        .to_equal(DbEngine::SQLITE)?;
    expect(conf.api_scopes.as_ref()).to_equal(Some(&HashMap::new()))?;
    expect(conf.templates.as_ref()).to_equal(Some(&HashMap::new()))?;

    // Test wrong command-line arguments.
    let args = vec!["test", "--auth.api-scopes", "{", "--auth.templates", "}"];
    let args = config::reg_args(Command::new("test")).get_matches_from(args);
    let conf = config::read_args(&args);
    expect(conf.api_scopes.as_ref()).to_equal(Some(&HashMap::new()))?;
    expect(conf.templates.as_ref()).to_equal(Some(&HashMap::new()))?;

    // Clear command-line arguments.
    let args = config::reg_args(Command::new("test")).get_matches_from(vec!["test"]);

    // Modified default by environment variables.
    set_env_var("AUTH_DB_ENGINE", "mongodb");
    set_env_var("AUTH_DB_MONGODB_URL", "url2");
    set_env_var("AUTH_DB_MONGODB_DATABASE", "db2");
    set_env_var("AUTH_DB_MONGODB_POOLSIZE", "12");
    set_env_var("AUTH_DB_SQLITE_PATH", "path2");
    set_env_var("AUTH_API_SCOPES", "{\"key21\":[\"value21\"]}");
    set_env_var("AUTH_TEMPLATES", "{\"key22\":\"value22\"}");
    let conf = config::read_args(&args);
    expect(conf.db.is_some()).to_equal(true)?;
    expect(conf.db.as_ref().unwrap().engine.as_ref().unwrap().as_str())
        .to_equal(DbEngine::MONGODB)?;
    expect(conf.db.as_ref().unwrap().mongodb.is_some()).to_equal(true)?;
    expect(conf.db.as_ref().unwrap().sqlite.is_some()).to_equal(true)?;
    let db_conf = conf.db.as_ref().unwrap().mongodb.as_ref().unwrap();
    expect(db_conf.url.as_ref().unwrap().as_str()).to_equal("url2")?;
    expect(db_conf.database.as_ref().unwrap().as_str()).to_equal("db2")?;
    expect(db_conf.pool_size).to_equal(Some(12))?;
    let db_conf = conf.db.as_ref().unwrap().sqlite.as_ref().unwrap();
    expect(db_conf.path.as_ref().unwrap().as_str()).to_equal("path2")?;
    let mut map: HashMap<String, Vec<String>> = HashMap::new();
    map.insert("key21".to_string(), vec!["value21".to_string()]);
    expect(conf.api_scopes.as_ref()).to_equal(Some(&map))?;
    let mut map: HashMap<String, String> = HashMap::new();
    map.insert("key22".to_string(), "value22".to_string());
    expect(conf.templates.as_ref()).to_equal(Some(&map))?;

    set_env_var("AUTH_DB_ENGINE", "sqlite");
    set_env_var("AUTH_API_SCOPES", "");
    set_env_var("AUTH_TEMPLATES", "");
    let conf = config::read_args(&args);
    expect(conf.db.is_some()).to_equal(true)?;
    expect(conf.db.as_ref().unwrap().engine.as_ref().unwrap().as_str())
        .to_equal(DbEngine::SQLITE)?;
    expect(conf.api_scopes.as_ref()).to_equal(Some(&HashMap::new()))?;
    expect(conf.templates.as_ref()).to_equal(Some(&HashMap::new()))?;

    // Test wrong environment variables.
    set_env_var("AUTH_DB_ENGINE", "test2");
    set_env_var("AUTH_DB_MONGODB_POOLSIZE", "12_000");
    set_env_var("AUTH_API_SCOPES", "}");
    set_env_var("AUTH_TEMPLATES", "{");
    let args = config::reg_args(Command::new("test")).get_matches_from(vec!["test"]);
    let conf = config::read_args(&args);
    expect(conf.db.is_some()).to_equal(true)?;
    expect(conf.db.as_ref().unwrap().engine.as_ref().unwrap().as_str())
        .to_equal(config::DEF_ENGINE)?;
    expect(conf.db.as_ref().unwrap().mongodb.is_some()).to_equal(true)?;
    expect(conf.db.as_ref().unwrap().sqlite.is_some()).to_equal(true)?;
    let db_conf = conf.db.as_ref().unwrap().mongodb.as_ref().unwrap();
    expect(db_conf.pool_size).to_equal(None)?;
    expect(conf.api_scopes.as_ref()).to_equal(Some(&HashMap::new()))?;
    expect(conf.templates.as_ref()).to_equal(Some(&HashMap::new()))?;

    // Test command-line arguments overwrite environment variables.
    let args = vec![
        "test",
        "--auth.db.engine",
        "mongodb",
        "--auth.db.mongodb.url",
        "url3",
        "--auth.db.mongodb.database",
        "db3",
        "--auth.db.mongodb.poolsize",
        "13",
        "--auth.db.sqlite.path",
        "path3",
        "--auth.api-scopes",
        "{\"key31\":[\"value31\"]}",
        "--auth.templates",
        "{\"key32\":\"value32\"}",
    ];
    set_env_var("AUTH_DB_ENGINE", "sqlite");
    set_env_var("AUTH_DB_MONGODB_URL", "url4");
    set_env_var("AUTH_DB_MONGODB_DATABASE", "db4");
    set_env_var("AUTH_DB_MONGODB_POOLSIZE", "14");
    set_env_var("AUTH_DB_SQLITE_PATH", "path4");
    set_env_var("AUTH_API_SCOPES", "{\"key41\":[\"value41\"]}");
    set_env_var("AUTH_TEMPLATES", "{\"key42\":\"value42\"}");
    let args = config::reg_args(Command::new("test")).get_matches_from(args);
    let conf = config::read_args(&args);
    expect(conf.db.is_some()).to_equal(true)?;
    expect(conf.db.as_ref().unwrap().engine.as_ref().unwrap().as_str())
        .to_equal(DbEngine::MONGODB)?;
    expect(conf.db.as_ref().unwrap().mongodb.is_some()).to_equal(true)?;
    expect(conf.db.as_ref().unwrap().sqlite.is_some()).to_equal(true)?;
    let db_conf = conf.db.as_ref().unwrap().mongodb.as_ref().unwrap();
    expect(db_conf.url.as_ref().unwrap().as_str()).to_equal("url3")?;
    expect(db_conf.database.as_ref().unwrap().as_str()).to_equal("db3")?;
    expect(db_conf.pool_size).to_equal(Some(13))?;
    let db_conf = conf.db.as_ref().unwrap().sqlite.as_ref().unwrap();
    expect(db_conf.path.as_ref().unwrap().as_str()).to_equal("path3")?;
    let mut map: HashMap<String, Vec<String>> = HashMap::new();
    map.insert("key31".to_string(), vec!["value31".to_string()]);
    expect(conf.api_scopes.as_ref()).to_equal(Some(&map))?;
    let mut map: HashMap<String, String> = HashMap::new();
    map.insert("key32".to_string(), "value32".to_string());
    expect(conf.templates.as_ref()).to_equal(Some(&map))
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

fn set_env_var(key: &str, val: &str) {
    unsafe {
        env::set_var(&OsStr::new(key), val);
    }
}
