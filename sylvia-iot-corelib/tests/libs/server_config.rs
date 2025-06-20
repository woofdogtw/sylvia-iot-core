use clap::Command;
use laboratory::{SpecContext, expect};

use sylvia_iot_corelib::server_config::{self, Config};

use super::set_env_var;
use crate::TestState;

/// Test [`server_config::reg_args`].
pub fn reg_args(_context: &mut SpecContext<TestState>) -> Result<(), String> {
    server_config::reg_args(Command::new("test"));
    Ok(())
}

/// Test [`server_config::read_args`].
pub fn read_args(_context: &mut SpecContext<TestState>) -> Result<(), String> {
    // Default.
    let args = server_config::reg_args(Command::new("test")).get_matches_from(vec!["test"]);
    let conf = server_config::read_args(&args);
    expect(conf.http_port).to_equal(Some(server_config::DEF_HTTP_PORT))?;
    expect(conf.https_port).to_equal(Some(server_config::DEF_HTTPS_PORT))?;
    expect(conf.cacert_file).to_equal(None)?;
    expect(conf.cert_file).to_equal(None)?;
    expect(conf.key_file).to_equal(None)?;
    expect(conf.static_path).to_equal(None)?;

    // Modified default by command-line arguments.
    let args = vec![
        "test",
        "--server.httpport",
        "1082",
        "--server.httpsport",
        "1445",
        "--server.cacertfile",
        "cacert1",
        "--server.certfile",
        "cert1",
        "--server.keyfile",
        "key1",
        "--server.static",
        "static1",
    ];
    let args = server_config::reg_args(Command::new("test")).get_matches_from(args);
    let conf = server_config::read_args(&args);
    expect(conf.http_port).to_equal(Some(1082))?;
    expect(conf.https_port).to_equal(Some(1445))?;
    expect(conf.cacert_file.is_some()).to_equal(true)?;
    expect(conf.cacert_file.as_ref().unwrap().as_str()).to_equal("cacert1")?;
    expect(conf.cert_file.is_some()).to_equal(true)?;
    expect(conf.cert_file.as_ref().unwrap().as_str()).to_equal("cert1")?;
    expect(conf.key_file.is_some()).to_equal(true)?;
    expect(conf.key_file.as_ref().unwrap().as_str()).to_equal("key1")?;
    expect(conf.static_path.is_some()).to_equal(true)?;
    expect(conf.static_path.as_ref().unwrap().as_str()).to_equal("static1")?;

    // Clear command-line arguments.
    let args = server_config::reg_args(Command::new("test")).get_matches_from(vec!["test"]);

    // Modified default by environment variables.
    set_env_var("SERVER_HTTP_PORT", "1081");
    set_env_var("SERVER_HTTPS_PORT", "1444");
    set_env_var("SERVER_CACERT_FILE", "cacert");
    set_env_var("SERVER_CERT_FILE", "cert");
    set_env_var("SERVER_KEY_FILE", "key");
    set_env_var("SERVER_STATIC_PATH", "static");
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
    expect(conf.static_path.as_ref().unwrap().as_str()).to_equal("static")?;

    // Test command-line arguments overwrite environment variables.
    let args = vec![
        "test",
        "--server.httpport",
        "1083",
        "--server.httpsport",
        "1446",
        "--server.cacertfile",
        "cacert2",
        "--server.certfile",
        "cert2",
        "--server.keyfile",
        "key2",
        "--server.static",
        "static2",
    ];
    let args = server_config::reg_args(Command::new("test")).get_matches_from(args);
    set_env_var("SERVER_HTTP_PORT", "1084");
    set_env_var("SERVER_HTTPS_PORT", "1447");
    set_env_var("SERVER_CACERT_FILE", "cacert3");
    set_env_var("SERVER_CERT_FILE", "cert3");
    set_env_var("SERVER_KEY_FILE", "key3");
    set_env_var("SERVER_STATIC_PATH", "static3");
    let conf = server_config::read_args(&args);
    expect(conf.http_port).to_equal(Some(1083))?;
    expect(conf.https_port).to_equal(Some(1446))?;
    expect(conf.cacert_file.is_some()).to_equal(true)?;
    expect(conf.cacert_file.as_ref().unwrap().as_str()).to_equal("cacert2")?;
    expect(conf.cert_file.is_some()).to_equal(true)?;
    expect(conf.cert_file.as_ref().unwrap().as_str()).to_equal("cert2")?;
    expect(conf.key_file.is_some()).to_equal(true)?;
    expect(conf.key_file.as_ref().unwrap().as_str()).to_equal("key2")?;
    expect(conf.static_path.is_some()).to_equal(true)?;
    expect(conf.static_path.as_ref().unwrap().as_str()).to_equal("static2")
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
