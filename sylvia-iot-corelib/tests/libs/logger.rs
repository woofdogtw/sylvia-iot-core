use clap::Command;
use laboratory::{SpecContext, expect};

use sylvia_iot_corelib::logger::{self, Config};

use super::set_env_var;
use crate::TestState;

/// Test [`logger::reg_args`].
pub fn reg_args(_context: &mut SpecContext<TestState>) -> Result<(), String> {
    logger::reg_args(Command::new("test"));
    Ok(())
}

/// Test [`logger::read_args`].
pub fn read_args(_context: &mut SpecContext<TestState>) -> Result<(), String> {
    // Default.
    let args = logger::reg_args(Command::new("test")).get_matches_from(vec!["test"]);
    let conf = logger::read_args(&args);
    expect(conf.level.is_some()).to_equal(true)?;
    expect(conf.level.as_ref().unwrap().as_str()).to_equal(logger::DEF_LEVEL)?;
    expect(conf.style.is_some()).to_equal(true)?;
    expect(conf.style.as_ref().unwrap().as_str()).to_equal(logger::DEF_STYLE)?;

    // Modified default by command-line arguments.
    let args = vec!["test", "--log.level", "off", "--log.style", "json"];
    let args = logger::reg_args(Command::new("test")).get_matches_from(args);
    let conf = logger::read_args(&args);
    expect(conf.level.is_some()).to_equal(true)?;
    expect(conf.level.as_ref().unwrap().as_str()).to_equal(logger::LEVEL_OFF)?;
    expect(conf.style.is_some()).to_equal(true)?;
    expect(conf.style.as_ref().unwrap().as_str()).to_equal(logger::STYLE_JSON)?;

    let args = vec!["test", "--log.level", "error", "--log.style", "log4j"];
    let args = logger::reg_args(Command::new("test")).get_matches_from(args);
    let conf = logger::read_args(&args);
    expect(conf.level.is_some()).to_equal(true)?;
    expect(conf.level.as_ref().unwrap().as_str()).to_equal(logger::LEVEL_ERROR)?;
    expect(conf.style.is_some()).to_equal(true)?;
    expect(conf.style.as_ref().unwrap().as_str()).to_equal(logger::STYLE_LOG4J)?;

    let args = vec!["test", "--log.level", "warn"];
    let args = logger::reg_args(Command::new("test")).get_matches_from(args);
    let conf = logger::read_args(&args);
    expect(conf.level.is_some()).to_equal(true)?;
    expect(conf.level.as_ref().unwrap().as_str()).to_equal(logger::LEVEL_WARN)?;

    let args = vec!["test", "--log.level", "info"];
    let args = logger::reg_args(Command::new("test")).get_matches_from(args);
    let conf = logger::read_args(&args);
    expect(conf.level.is_some()).to_equal(true)?;
    expect(conf.level.as_ref().unwrap().as_str()).to_equal(logger::LEVEL_INFO)?;

    let args = vec!["test", "--log.level", "debug"];
    let args = logger::reg_args(Command::new("test")).get_matches_from(args);
    let conf = logger::read_args(&args);
    expect(conf.level.is_some()).to_equal(true)?;
    expect(conf.level.as_ref().unwrap().as_str()).to_equal(logger::LEVEL_DEBUG)?;

    // Clear command-line arguments.
    let args = logger::reg_args(Command::new("test")).get_matches_from(vec!["test"]);

    // Test wrong environment variables.
    set_env_var("LOG_LEVEL", "level");
    set_env_var("LOG_STYLE", "style");
    let conf = logger::read_args(&args);
    expect(conf.level.is_some()).to_equal(true)?;
    expect(conf.level.as_ref().unwrap().as_str()).to_equal(logger::DEF_LEVEL)?;
    expect(conf.style.is_some()).to_equal(true)?;
    expect(conf.style.as_ref().unwrap().as_str()).to_equal(logger::DEF_STYLE)?;

    // Modified default by environment variables.
    set_env_var("LOG_LEVEL", "off");
    set_env_var("LOG_STYLE", "json");
    let conf = logger::read_args(&args);
    expect(conf.level.is_some()).to_equal(true)?;
    expect(conf.level.as_ref().unwrap().as_str()).to_equal(logger::LEVEL_OFF)?;
    expect(conf.style.is_some()).to_equal(true)?;
    expect(conf.style.as_ref().unwrap().as_str()).to_equal(logger::STYLE_JSON)?;

    set_env_var("LOG_LEVEL", "error");
    set_env_var("LOG_STYLE", "log4j");
    let conf = logger::read_args(&args);
    expect(conf.level.is_some()).to_equal(true)?;
    expect(conf.level.as_ref().unwrap().as_str()).to_equal(logger::LEVEL_ERROR)?;
    expect(conf.style.is_some()).to_equal(true)?;
    expect(conf.style.as_ref().unwrap().as_str()).to_equal(logger::STYLE_LOG4J)?;

    set_env_var("LOG_LEVEL", "warn");
    let conf = logger::read_args(&args);
    expect(conf.level.is_some()).to_equal(true)?;
    expect(conf.level.as_ref().unwrap().as_str()).to_equal(logger::LEVEL_WARN)?;

    set_env_var("LOG_LEVEL", "info");
    let conf = logger::read_args(&args);
    expect(conf.level.is_some()).to_equal(true)?;
    expect(conf.level.as_ref().unwrap().as_str()).to_equal(logger::LEVEL_INFO)?;

    set_env_var("LOG_LEVEL", "debug");
    let conf = logger::read_args(&args);
    expect(conf.level.is_some()).to_equal(true)?;
    expect(conf.level.as_ref().unwrap().as_str()).to_equal(logger::LEVEL_DEBUG)?;

    // Test command-line arguments overwrite environment variables.
    let args = vec!["test", "--log.level", "warn", "--log.style", "log4j"];
    set_env_var("LOG_LEVEL", "debug");
    set_env_var("LOG_STYLE", "json");
    let args = logger::reg_args(Command::new("test")).get_matches_from(args);
    let conf = logger::read_args(&args);
    expect(conf.level.is_some()).to_equal(true)?;
    expect(conf.level.as_ref().unwrap().as_str()).to_equal(logger::LEVEL_WARN)?;
    expect(conf.style.is_some()).to_equal(true)?;
    expect(conf.style.as_ref().unwrap().as_str()).to_equal(logger::STYLE_LOG4J)
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
