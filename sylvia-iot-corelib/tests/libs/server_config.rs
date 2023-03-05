use std::{env, ffi::OsStr};

use clap::Command;
use laboratory::{expect, SpecContext};

use sylvia_iot_corelib::server_config::{self, Config};

use crate::TestState;

/// Test [`server_config::reg_args`].
pub fn reg_args(_context: &mut SpecContext<TestState>) -> Result<(), String> {
    server_config::reg_args(Command::new("test"));
    Ok(())
}

/// Test [`server_config::read_args`].
pub fn read_args(_context: &mut SpecContext<TestState>) -> Result<(), String> {
    let args = Command::new("test").get_matches();
    let conf = server_config::read_args(&args);
    expect(conf.http_port).to_equal(Some(server_config::DEF_HTTP_PORT))?;
    expect(conf.https_port).to_equal(Some(server_config::DEF_HTTPS_PORT))?;
    expect(conf.cacert_file).to_equal(None)?;
    expect(conf.cert_file).to_equal(None)?;
    expect(conf.key_file).to_equal(None)?;
    expect(conf.static_path).to_equal(None)?;

    env::set_var(&OsStr::new("SERVER_HTTP_PORT"), "1081");
    env::set_var(&OsStr::new("SERVER_HTTPS_PORT"), "1444");
    env::set_var(&OsStr::new("SERVER_CACERT_FILE"), "cacert");
    env::set_var(&OsStr::new("SERVER_CERT_FILE"), "cert");
    env::set_var(&OsStr::new("SERVER_KEY_FILE"), "key");
    env::set_var(&OsStr::new("SERVER_STATIC_PATH"), "static");
    let conf = server_config::read_args(&args);
    expect(conf.http_port).to_equal(Some(1081))?;
    expect(conf.https_port).to_equal(Some(1444))?;
    expect(conf.cacert_file.is_some()).to_equal(true)?;
    expect(conf.cacert_file.as_ref().unwrap().as_str()).to_equal("cacert")?;
    expect(conf.cert_file.is_some()).to_equal(true)?;
    expect(conf.cert_file.as_ref().unwrap().as_str()).to_equal("cert")?;
    expect(conf.key_file.is_some()).to_equal(true)?;
    expect(conf.key_file.as_ref().unwrap().as_str()).to_equal("key")?;
    expect(conf.static_path.is_some()).to_equal(true)?;
    expect(conf.static_path.as_ref().unwrap().as_str()).to_equal("static")
}

/// Test [`server_config::apply_default`].
pub fn apply_default(_context: &mut SpecContext<TestState>) -> Result<(), String> {
    let conf = Config {
        ..Default::default()
    };
    let conf = server_config::apply_default(&conf);
    expect(conf.http_port).to_equal(Some(server_config::DEF_HTTP_PORT))?;
    expect(conf.https_port).to_equal(Some(server_config::DEF_HTTPS_PORT))?;
    expect(conf.cacert_file).to_equal(None)?;
    expect(conf.cert_file).to_equal(None)?;
    expect(conf.key_file).to_equal(None)?;
    expect(conf.static_path).to_equal(None)?;

    let conf = Config {
        http_port: Some(1081),
        https_port: Some(1444),
        cacert_file: Some("cacert".to_string()),
        cert_file: Some("cert".to_string()),
        key_file: Some("key".to_string()),
        static_path: Some("static".to_string()),
    };
    let conf = server_config::apply_default(&conf);
    expect(conf.http_port).to_equal(Some(1081))?;
    expect(conf.https_port).to_equal(Some(1444))?;
    expect(conf.cacert_file.is_some()).to_equal(true)?;
    expect(conf.cacert_file.as_ref().unwrap().as_str()).to_equal("cacert")?;
    expect(conf.cert_file.is_some()).to_equal(true)?;
    expect(conf.cert_file.as_ref().unwrap().as_str()).to_equal("cert")?;
    expect(conf.key_file.is_some()).to_equal(true)?;
    expect(conf.key_file.as_ref().unwrap().as_str()).to_equal("key")?;
    expect(conf.static_path.is_some()).to_equal(true)?;
    expect(conf.static_path.as_ref().unwrap().as_str()).to_equal("static")
}
