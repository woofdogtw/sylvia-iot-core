use std::{env, ffi::OsStr};

use clap::Command;
use laboratory::{expect, SpecContext};

use sylvia_iot_corelib::logger::{self, Config};

use crate::TestState;

/// Test [`logger::reg_args`].
pub fn reg_args(_context: &mut SpecContext<TestState>) -> Result<(), String> {
    logger::reg_args(Command::new("test"));
    Ok(())
}

/// Test [`logger::read_args`].
pub fn read_args(_context: &mut SpecContext<TestState>) -> Result<(), String> {
    let args = Command::new("test").get_matches();
    let conf = logger::read_args(&args);
    expect(conf.level.is_some()).to_equal(true)?;
    expect(conf.level.as_ref().unwrap().as_str()).to_equal(logger::DEF_LEVEL)?;
    expect(conf.style.is_some()).to_equal(true)?;
    expect(conf.style.as_ref().unwrap().as_str()).to_equal(logger::DEF_STYLE)?;

    env::set_var(&OsStr::new("LOG_LEVEL"), "level");
    env::set_var(&OsStr::new("LOG_STYLE"), "style");
    let conf = logger::read_args(&args);
    expect(conf.level.is_some()).to_equal(true)?;
    expect(conf.level.as_ref().unwrap().as_str()).to_equal(logger::DEF_LEVEL)?;
    expect(conf.style.is_some()).to_equal(true)?;
    expect(conf.style.as_ref().unwrap().as_str()).to_equal(logger::DEF_STYLE)?;

    env::set_var(&OsStr::new("LOG_LEVEL"), "off");
    env::set_var(&OsStr::new("LOG_STYLE"), "json");
    expect(conf.level.is_some()).to_equal(true)?;
    expect(conf.level.as_ref().unwrap().as_str()).to_equal("off")?;
    expect(conf.style.is_some()).to_equal(true)?;
    expect(conf.style.as_ref().unwrap().as_str()).to_equal("json")?;

    env::set_var(&OsStr::new("LOG_LEVEL"), "info");
    env::set_var(&OsStr::new("LOG_STYLE"), "log4j");
    expect(conf.level.is_some()).to_equal(true)?;
    expect(conf.level.as_ref().unwrap().as_str()).to_equal("info")?;
    expect(conf.style.is_some()).to_equal(true)?;
    expect(conf.style.as_ref().unwrap().as_str()).to_equal("log4j")
}

/// Test [`logger::apply_default`].
pub fn apply_default(_context: &mut SpecContext<TestState>) -> Result<(), String> {
    let conf = Config {
        ..Default::default()
    };
    let conf = logger::apply_default(&conf);
    expect(conf.level.is_some()).to_equal(true)?;
    expect(conf.level.as_ref().unwrap().as_str()).to_equal(logger::DEF_LEVEL)?;
    expect(conf.style.is_some()).to_equal(true)?;
    expect(conf.style.as_ref().unwrap().as_str()).to_equal(logger::DEF_STYLE)?;

    let conf = Config {
        level: Some("level".to_string()),
        style: Some("style".to_string()),
    };
    let conf = logger::apply_default(&conf);
    expect(conf.level.is_some()).to_equal(true)?;
    expect(conf.level.as_ref().unwrap().as_str()).to_equal(logger::DEF_LEVEL)?;
    expect(conf.style.is_some()).to_equal(true)?;
    expect(conf.style.as_ref().unwrap().as_str()).to_equal(logger::DEF_STYLE)?;

    let conf = Config {
        level: Some("off".to_string()),
        style: Some("json".to_string()),
    };
    let conf = logger::apply_default(&conf);
    expect(conf.level.is_some()).to_equal(true)?;
    expect(conf.level.as_ref().unwrap().as_str()).to_equal("off")?;
    expect(conf.style.is_some()).to_equal(true)?;
    expect(conf.style.as_ref().unwrap().as_str()).to_equal("json")?;

    let conf = Config {
        level: Some("info".to_string()),
        style: Some("log4j".to_string()),
    };
    let conf = logger::apply_default(&conf);
    expect(conf.level.is_some()).to_equal(true)?;
    expect(conf.level.as_ref().unwrap().as_str()).to_equal("info")?;
    expect(conf.style.is_some()).to_equal(true)?;
    expect(conf.style.as_ref().unwrap().as_str()).to_equal("log4j")
}
