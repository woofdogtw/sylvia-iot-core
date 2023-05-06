//! To configure the logger.

use std::env;

use anyhow::Result;
use chrono::{SecondsFormat, Utc};
use clap::{Arg, ArgMatches, Command};
use log::{Level, LevelFilter, Record};
use log4rs::{
    self,
    append::console::ConsoleAppender,
    config::{Appender, Root},
    encode::{Encode, Write},
};
use serde::{Deserialize, Serialize};

/// Logger configuration object.
#[derive(Default, Deserialize)]
pub struct Config {
    /// Log level. Can be `off`, `error`, `warn`, `info`, `debug`.
    ///
    /// Default is `info`.
    pub level: Option<String>,
    /// Log style. Can be `json`, `log4j`.
    ///
    /// Default is `json`.
    pub style: Option<String>,
}

/// The log4rs encoder for JSON format.
#[derive(Debug)]
struct JsonEncoder {
    _proj_name: String,
}

/// The log4rs encoder for log4j format.
#[derive(Debug)]
struct Log4jEncoder {
    _proj_name: String,
}

/// Normal log information.
#[derive(Debug, Serialize)]
struct JsonEncoderMsg {
    pub ts: String,
    pub level: String,
    pub module: String,
    pub msg: String,
}

/// HTTP log information.
#[derive(Debug, Serialize)]
struct JsonEncoderHttpMsg {
    pub ts: String,
    pub level: String,
    pub remote: String,
    pub status: String,
    pub method: String,
    pub url: String,
    #[serde(rename = "latencyMs")]
    pub latency_ms: String,
}

// remote address, status code, processing milliseconds, request URL, request line (method, resource, version)
pub const ACTIX_LOGGER_FORMAT: &'static str = "%a %s %D %U %r";
pub const ACTIX_LOGGER_NAME: &'static str = "actix_web::middleware::logger";

pub const LEVEL_OFF: &'static str = "off";
pub const LEVEL_ERROR: &'static str = "error";
pub const LEVEL_WARN: &'static str = "warn";
pub const LEVEL_INFO: &'static str = "info";
pub const LEVEL_DEBUG: &'static str = "debug";

pub const STYLE_JSON: &'static str = "json";
pub const STYLE_LOG4J: &'static str = "log4j";

pub const DEF_LEVEL: &'static str = LEVEL_INFO;
pub const DEF_STYLE: &'static str = STYLE_JSON;

pub const FILTER_ONLY: [&'static str; 2] = ["/auth/oauth2/", "/api/"];

impl JsonEncoder {
    pub fn new(proj_name: &str) -> Self {
        JsonEncoder {
            _proj_name: proj_name.to_string(),
        }
    }
}

impl Log4jEncoder {
    pub fn new(proj_name: &str) -> Self {
        Log4jEncoder {
            _proj_name: proj_name.to_string(),
        }
    }
}

impl Encode for Log4jEncoder {
    fn encode(&self, w: &mut dyn Write, record: &Record<'_>) -> Result<()> {
        let module = match get_module_name(record) {
            None => return Ok(()),
            Some(module) => module,
        };

        let str = match module.eq(ACTIX_LOGGER_NAME) {
            false => format!(
                "{} {} [{}] {}\n",
                Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
                record.level(),
                module,
                record.args().to_string().replace("\n", "\\n")
            ),
            true => {
                let msg = match get_http_msg(record) {
                    None => return Ok(()),
                    Some(msg) => msg,
                };
                let mut found = false;
                for filter in FILTER_ONLY {
                    if msg.url.contains(filter) {
                        found = true;
                        break;
                    }
                }
                if !found {
                    return Ok(());
                }
                format!(
                    "{} {} [{}] {} {} {} ({} ms)\n",
                    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
                    match &msg.status.chars().next() {
                        Some('4') => Level::Warn.as_str(),
                        Some('5') => Level::Error.as_str(),
                        _ => Level::Info.as_str(),
                    },
                    msg.remote,
                    msg.status,
                    msg.method,
                    msg.url,
                    msg.latency_ms,
                )
            }
        };
        w.write_all(str.as_bytes())?;
        Ok(())
    }
}

impl Encode for JsonEncoder {
    fn encode(&self, w: &mut dyn Write, record: &Record<'_>) -> Result<()> {
        let module = match get_module_name(record) {
            None => return Ok(()),
            Some(module) => module,
        };

        let str = match module.eq(ACTIX_LOGGER_NAME) {
            false => {
                let msg = JsonEncoderMsg {
                    ts: Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
                    level: record.level().to_string().to_lowercase(),
                    module,
                    msg: record.args().to_string(),
                };
                serde_json::to_string(&msg)? + "\n"
            }
            true => {
                let mut msg = match get_http_msg(record) {
                    None => return Ok(()),
                    Some(msg) => msg,
                };
                let mut found = false;
                for filter in FILTER_ONLY {
                    if msg.url.contains(filter) {
                        found = true;
                        break;
                    }
                }
                if !found {
                    return Ok(());
                }
                msg.level = match &msg.status.chars().next() {
                    Some('4') => Level::Warn.as_str().to_lowercase(),
                    Some('5') => Level::Error.as_str().to_lowercase(),
                    _ => Level::Info.as_str().to_lowercase(),
                };
                serde_json::to_string(&msg)? + "\n"
            }
        };
        w.write_all(str.as_bytes())?;
        Ok(())
    }
}

/// To initialize the logger with configurations.
pub fn init(proj_name: &str, conf: &Config) {
    let conf = apply_default(&conf);

    let level = match conf.level.as_ref() {
        None => DEF_LEVEL,
        Some(v) => v.as_str(),
    };
    let level = match level {
        LEVEL_OFF => LevelFilter::Off,
        LEVEL_ERROR => LevelFilter::Error,
        LEVEL_WARN => LevelFilter::Warn,
        LEVEL_INFO => LevelFilter::Info,
        LEVEL_DEBUG => LevelFilter::Debug,
        _ => LevelFilter::Info,
    };
    let style = match conf.style.as_ref() {
        None => DEF_STYLE,
        Some(v) => v.as_str(),
    };

    let log4j_encoder = ConsoleAppender::builder()
        .encoder(Box::new(Log4jEncoder::new(proj_name)))
        .build();
    let json_encoder = ConsoleAppender::builder()
        .encoder(Box::new(JsonEncoder::new(proj_name)))
        .build();
    let _ = log4rs::init_config(
        log4rs::Config::builder()
            .appender(Appender::builder().build("log4j", Box::new(log4j_encoder)))
            .appender(Appender::builder().build("json", Box::new(json_encoder)))
            .build(Root::builder().appender(style).build(level))
            .unwrap(),
    )
    .unwrap();
}

/// To register Clap arguments.
pub fn reg_args(cmd: Command) -> Command {
    cmd.arg(
        Arg::new("log.level")
            .long("log.level")
            .help("log level")
            .num_args(1)
            .value_parser([LEVEL_OFF, LEVEL_ERROR, LEVEL_WARN, LEVEL_INFO, LEVEL_DEBUG]),
    )
    .arg(
        Arg::new("log.style")
            .long("log.style")
            .help("log style")
            .num_args(1)
            .value_parser([STYLE_JSON, STYLE_LOG4J]),
    )
}

/// To read input arguments from command-line arguments and environment variables.
///
/// This function will call [`apply_default()`] to fill missing values so you do not need call it
/// again.
pub fn read_args(args: &ArgMatches) -> Config {
    apply_default(&Config {
        level: match args.get_one::<String>("log.level") {
            None => match env::var("LOG_LEVEL") {
                Err(_) => None,
                Ok(v) => match v.as_str() {
                    "off" => Some("off".to_string()),
                    "error" => Some("error".to_string()),
                    "warn" => Some("warn".to_string()),
                    "info" => Some("info".to_string()),
                    "debug" => Some("debug".to_string()),
                    _ => None,
                },
            },
            Some(v) => match v.as_str() {
                "off" => Some("off".to_string()),
                "error" => Some("error".to_string()),
                "warn" => Some("warn".to_string()),
                "info" => Some("info".to_string()),
                "debug" => Some("debug".to_string()),
                _ => None,
            },
        },
        style: match args.get_one::<String>("log.style") {
            None => match env::var("LOG_STYLE") {
                Err(_) => None,
                Ok(v) => match v.as_str() {
                    STYLE_JSON => Some(STYLE_JSON.to_string()),
                    STYLE_LOG4J => Some(STYLE_LOG4J.to_string()),
                    _ => None,
                },
            },
            Some(v) => match v.as_str() {
                STYLE_JSON => Some(STYLE_JSON.to_string()),
                STYLE_LOG4J => Some(STYLE_LOG4J.to_string()),
                _ => None,
            },
        },
    })
}

/// Fill missing configuration with default values.
pub fn apply_default(config: &Config) -> Config {
    Config {
        level: match config.level.as_ref() {
            None => Some(DEF_LEVEL.to_string()),
            Some(v) => match v.as_str() {
                "off" => Some("off".to_string()),
                "error" => Some("error".to_string()),
                "warn" => Some("warn".to_string()),
                "info" => Some("info".to_string()),
                "debug" => Some("debug".to_string()),
                _ => Some(DEF_LEVEL.to_string()),
            },
        },
        style: match config.style.as_ref() {
            None => Some(DEF_STYLE.to_string()),
            Some(v) => match v.as_str() {
                STYLE_LOG4J => Some(STYLE_LOG4J.to_string()),
                _ => Some(STYLE_JSON.to_string()),
            },
        },
    }
}

/// To filter `actix_` prefix and try to get the module name for printing logs.
fn get_module_name(record: &Record<'_>) -> Option<String> {
    match record.module_path() {
        None => None,
        Some(module) => match module.starts_with("actix_") {
            false => match record.file() {
                None => Some(module.to_string()),
                Some(file) => match file.contains("/.cargo/") {
                    false => match record.line() {
                        None => Some(file.to_string()),
                        Some(line) => Some(format!("{}:{}", file, line)),
                    },
                    true => None,
                },
            },
            true => Some(module.to_string()),
        },
    }
}

/// Parse Actix-Web HTTP log for generating logs.
fn get_http_msg(record: &Record<'_>) -> Option<JsonEncoderHttpMsg> {
    let msg = record.args().to_string();
    let mut split = msg.split(' ');
    let remote = match split.next() {
        None => return None,
        Some(remote) => remote,
    };
    let status = match split.next() {
        None => return None,
        Some(status) => status,
    };
    let latency_ms = match split.next() {
        None => return None,
        Some(latency) => latency,
    };
    let url = match split.next() {
        None => return None,
        Some(url) => url,
    };
    let method = match split.next() {
        None => return None,
        Some(method) => method,
    };
    Some(JsonEncoderHttpMsg {
        ts: Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
        level: record.level().to_string(),
        remote: remote.to_string(),
        status: status.to_string(),
        method: method.to_string(),
        url: url.to_string(),
        latency_ms: latency_ms.to_string(),
    })
}
