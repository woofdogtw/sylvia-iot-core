//! The top level configuration `server`.

use std::env;

use clap::{Arg, ArgMatches, Command, builder::RangedU64ValueParser};
use serde::Deserialize;

/// Server configuration object.
#[derive(Default, Deserialize)]
pub struct Config {
    /// HTTP port.
    ///
    /// Default is `1080`.
    #[serde(rename = "httpPort")]
    pub http_port: Option<u16>,
    /// HTTPS port.
    ///
    /// Default is `1443`.
    #[serde(rename = "httpsPort")]
    pub https_port: Option<u16>,
    /// HTTPS CA certificate file path.
    #[serde(rename = "cacertFile")]
    pub cacert_file: Option<String>,
    /// HTTPS certificate file path. Missing this to disable HTTPS.
    #[serde(rename = "certFile")]
    pub cert_file: Option<String>,
    /// HTTPS private key file path. Missing this to disable HTTPS.
    #[serde(rename = "keyFile")]
    pub key_file: Option<String>,
    /// Static file path.
    #[serde(rename = "staticPath")]
    pub static_path: Option<String>,
}

pub const DEF_HTTP_PORT: u16 = 1080;
pub const DEF_HTTPS_PORT: u16 = 1443;

/// To register Clap arguments.
pub fn reg_args(cmd: Command) -> Command {
    cmd.arg(
        Arg::new("server.httpport")
            .long("server.httpport")
            .help("HTTP port")
            .num_args(1)
            .value_parser(RangedU64ValueParser::<u64>::new().range(1..=65535)),
    )
    .arg(
        Arg::new("server.httpsport")
            .long("server.httpsport")
            .help("HTTPS port")
            .num_args(1)
            .value_parser(RangedU64ValueParser::<u64>::new().range(1..=65535)),
    )
    .arg(
        Arg::new("server.cacertfile")
            .long("server.cacertfile")
            .help("HTTPS CA certificate file")
            .num_args(1),
    )
    .arg(
        Arg::new("server.certfile")
            .long("server.certfile")
            .help("HTTPS certificate file")
            .num_args(1),
    )
    .arg(
        Arg::new("server.keyfile")
            .long("server.keyfile")
            .help("HTTPS private key file")
            .num_args(1),
    )
    .arg(
        Arg::new("server.static")
            .long("server.static")
            .help("Static files directory path")
            .num_args(1),
    )
}

/// To read input arguments from command-line arguments and environment variables.
///
/// This function will call [`apply_default()`] to fill missing values so you do not need call it
/// again.
pub fn read_args(args: &ArgMatches) -> Config {
    apply_default(&Config {
        http_port: match args.get_one::<u64>("server.httpport") {
            None => match env::var("SERVER_HTTP_PORT") {
                Err(_) => Some(DEF_HTTP_PORT),
                Ok(v) => match v.parse::<u16>() {
                    Err(_) => Some(DEF_HTTP_PORT),
                    Ok(v) => Some(v),
                },
            },
            Some(v) => Some(*v as u16),
        },
        https_port: match args.get_one::<u64>("server.httpsport") {
            None => match env::var("SERVER_HTTPS_PORT") {
                Err(_) => Some(DEF_HTTPS_PORT),
                Ok(v) => match v.parse::<u16>() {
                    Err(_) => Some(DEF_HTTPS_PORT),
                    Ok(v) => Some(v),
                },
            },
            Some(v) => Some(*v as u16),
        },
        cacert_file: match args.get_one::<String>("server.cacertfile") {
            None => match env::var("SERVER_CACERT_FILE") {
                Err(_) => None,
                Ok(v) => Some(v),
            },
            Some(v) => Some(v.clone()),
        },
        cert_file: match args.get_one::<String>("server.certfile") {
            None => match env::var("SERVER_CERT_FILE") {
                Err(_) => None,
                Ok(v) => Some(v),
            },
            Some(v) => Some(v.clone()),
        },
        key_file: match args.get_one::<String>("server.keyfile") {
            None => match env::var("SERVER_KEY_FILE") {
                Err(_) => None,
                Ok(v) => Some(v),
            },
            Some(v) => Some(v.clone()),
        },
        static_path: match args.get_one::<String>("server.static") {
            None => match env::var("SERVER_STATIC_PATH") {
                Err(_) => None,
                Ok(v) => Some(v),
            },
            Some(v) => Some(v.clone()),
        },
    })
}

/// Fill missing configuration with default values.
pub fn apply_default(config: &Config) -> Config {
    Config {
        http_port: Some(config.http_port.unwrap_or(DEF_HTTP_PORT)),
        https_port: Some(config.https_port.unwrap_or(DEF_HTTPS_PORT)),
        cacert_file: config.cacert_file.clone(),
        cert_file: config.cert_file.clone(),
        key_file: config.key_file.clone(),
        static_path: config.static_path.clone(),
    }
}
