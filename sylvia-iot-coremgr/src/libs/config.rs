//! Program configurations.

use std::env;

use clap::{builder::RangedU64ValueParser, Arg, ArgMatches, Command};
use serde::Deserialize;

use sylvia_iot_corelib::constants::MqEngine;

/// Configuration file object.
#[derive(Default, Deserialize)]
pub struct Config {
    /// **sylvia-auth** API base path with host. For example: `http://localhost:1080/auth`.
    pub auth: Option<String>,
    /// **sylvia-broker** API base path with host. For example: `http://localhost:2080/broker`.
    pub broker: Option<String>,
    /// Message queue broker configurations.
    pub mq: Option<Mq>,
    #[serde(rename = "mqChannels")]
    pub mq_channels: Option<MqChannels>,
}

/// Message queue engine configurations.
#[derive(Default, Deserialize)]
pub struct Mq {
    /// Engine selections.
    pub engine: Option<Engine>,
    /// RabbitMQ configurations.
    pub rabbitmq: Option<RabbitMq>,
    /// EMQX configurations.
    pub emqx: Option<Emqx>,
    /// rumqttd configurations.
    pub rumqttd: Option<Rumqttd>,
}

/// Engine selections.
#[derive(Default, Deserialize)]
pub struct Engine {
    /// AMQP message broker. Now we only support `rabbitmq`.
    pub amqp: Option<String>,
    /// MQTT message broker. `emqx` and `rumqttd` are supported. Default is `emqx`.
    pub mqtt: Option<String>,
}

/// Configurations for RabbitMQ.
#[derive(Default, Deserialize)]
pub struct RabbitMq {
    /// Management user name. Default is **guest**.
    pub username: Option<String>,
    /// Management password. Default is **guest**.
    pub password: Option<String>,
    /// Message TTL in milliseconds. Default is **3600000**.
    pub ttl: Option<usize>,
    /// Queue length. Default is **10000**.
    pub length: Option<usize>,
    /// Available hosts. None or empty means unlimited.
    pub hosts: Option<Vec<MqHost>>,
}

/// Configurations for EMQX.
#[derive(Default, Deserialize)]
pub struct Emqx {
    /// Management API key.
    #[serde(rename = "apiKey")]
    pub api_key: Option<String>,
    /// Management password. Default is **public**.
    #[serde(rename = "apiSecret")]
    pub api_secret: Option<String>,
    /// Available hosts. None or empty means unlimited.
    pub hosts: Option<Vec<MqHost>>,
}

/// Configurations for rumqttd.
#[derive(Default, Deserialize)]
pub struct Rumqttd {
    /// MQTT listen port. Default is 1883.
    #[serde(rename = "mqttPort")]
    pub mqtt_port: Option<u16>,
    /// MQTTS listen port. Default is 8883. This must used with both `cert_file` and `key_file` in
    /// [`sylvia_iot_corelib::server_config::Config`].
    #[serde(rename = "mqttsPort")]
    pub mqtts_port: Option<u16>,
    /// Console listen port. Default is 18083.
    #[serde(rename = "consolePort")]
    pub console_port: Option<u16>,
}

#[derive(Clone, Default, Deserialize)]
pub struct MqHost {
    /// Display name.
    pub name: String,
    /// Internal host name or IP address of the message broker for sylvia-broker to access.
    pub host: String,
    /// External host name or IP address of the message broker for developer to access.
    pub external: String,
    /// Is active broker in operation or to be stopped.
    pub active: bool,
}

/// Message channels configuration object.
#[derive(Default, Deserialize)]
pub struct MqChannels {
    pub data: Option<CoremgrData>,
}

/// Channel `coremgr.data` configuration object.
#[derive(Default, Deserialize)]
pub struct CoremgrData {
    /// Queue connection URL of the data channel.
    pub url: Option<String>,
}

pub const DEF_AUTH: &'static str = "http://localhost:1080/auth";
pub const DEF_BROKER: &'static str = "http://localhost:2080/broker";
pub const DEF_ENGINE_AMQP: &'static str = MqEngine::RABBITMQ;
pub const DEF_ENGINE_MQTT: &'static str = MqEngine::EMQX;
pub const DEF_RABBITMQ_USERNAME: &'static str = "guest";
pub const DEF_RABBITMQ_PASSWORD: &'static str = "guest";
pub const DEF_EMQX_API_KEY: &'static str = "";
pub const DEF_EMQX_API_SECRET: &'static str = "";
pub const DEF_RUMQTTD_MQTT_PORT: u16 = 1883;
pub const DEF_RUMQTTD_MQTT_PORT_STR: &'static str = "1883";
pub const DEF_RUMQTTD_MQTTS_PORT: u16 = 8883;
pub const DEF_RUMQTTD_MQTTS_PORT_STR: &'static str = "8883";
pub const DEF_RUMQTTD_CONSOLE_PORT: u16 = 18083;
pub const DEF_RUMQTTD_CONSOLE_PORT_STR: &'static str = "18083";
pub const DEF_MQ_CHANNEL_URL: &'static str = "amqp://localhost";

/// To register Clap arguments.
pub fn reg_args(cmd: Command) -> Command {
    cmd.arg(
        Arg::new("coremgr.auth")
            .long("coremgr.auth")
            .help("sylvia-auth host (ex: http://localhost:1080/auth)")
            .num_args(1)
            .default_value(DEF_AUTH),
    )
    .arg(
        Arg::new("coremgr.broker")
            .long("coremgr.broker")
            .help("sylvia-broker host (ex: http://localhost:2080/broker)")
            .num_args(1)
            .default_value(DEF_BROKER),
    )
    .arg(
        Arg::new("coremgr.mq.engine.amqp")
            .long("coremgr.mq.engine.amqp")
            .help("AMQP broker")
            .num_args(1)
            .value_parser([MqEngine::RABBITMQ])
            .default_value(DEF_ENGINE_AMQP),
    )
    .arg(
        Arg::new("coremgr.mq.engine.mqtt")
            .long("coremgr.mq.engine.mqtt")
            .help("MQTT broker")
            .num_args(1)
            .value_parser([MqEngine::EMQX, MqEngine::RUMQTTD])
            .default_value(DEF_ENGINE_MQTT),
    )
    .arg(
        Arg::new("coremgr.mq.rabbitmq.username")
            .long("coremgr.mq.rabbitmq.username")
            .help("RabbitMQ configurations: management user name")
            .num_args(1)
            .default_value(DEF_RABBITMQ_USERNAME),
    )
    .arg(
        Arg::new("coremgr.mq.rabbitmq.password")
            .long("coremgr.mq.rabbitmq.password")
            .help("RabbitMQ configurations: management password")
            .num_args(1)
            .default_value(DEF_RABBITMQ_PASSWORD),
    )
    .arg(
        Arg::new("coremgr.mq.rabbitmq.ttl")
            .long("coremgr.mq.rabbitmq.ttl")
            .help("RabbitMQ configurations: message TTL in milliseconds")
            .num_args(1)
            .value_parser(RangedU64ValueParser::<u64>::new().range(1..=usize::MAX as u64)),
    )
    .arg(
        Arg::new("coremgr.mq.rabbitmq.length")
            .long("coremgr.mq.rabbitmq.length")
            .help("RabbitMQ configurations: message length in milliseconds")
            .num_args(1)
            .value_parser(RangedU64ValueParser::<u64>::new().range(1..=usize::MAX as u64)),
    )
    .arg(
        Arg::new("coremgr.mq.rabbitmq.hosts")
            .long("coremgr.mq.rabbitmq.hosts")
            .help("RabbitMQ hosts")
            .num_args(1),
    )
    .arg(
        Arg::new("coremgr.mq.emqx.apikey")
            .long("coremgr.mq.emqx.apikey")
            .help("EMQX configurations: management API key")
            .num_args(1),
    )
    .arg(
        Arg::new("coremgr.mq.emqx.apisecret")
            .long("coremgr.mq.emqx.apisecret")
            .help("EMQX configurations: management API secret")
            .num_args(1),
    )
    .arg(
        Arg::new("coremgr.mq.emqx.hosts")
            .long("coremgr.mq.emqx.hosts")
            .help("EMQX hosts")
            .num_args(1),
    )
    .arg(
        Arg::new("coremgr.mq.rumqttd.mqtt-port")
            .long("coremgr.mq.rumqttd.mqtt-port")
            .help("rumqttd MQTT listen port")
            .num_args(1)
            .value_parser(RangedU64ValueParser::<u64>::new().range(1..=65535))
            .default_value(DEF_RUMQTTD_MQTT_PORT_STR),
    )
    .arg(
        Arg::new("coremgr.mq.rumqttd.mqtts-port")
            .long("coremgr.mq.rumqttd.mqtts-port")
            .help("rumqttd MQTTS listen port")
            .num_args(1)
            .value_parser(RangedU64ValueParser::<u64>::new().range(1..=65535))
            .default_value(DEF_RUMQTTD_MQTTS_PORT_STR),
    )
    .arg(
        Arg::new("coremgr.mq.rumqttd.console-port")
            .long("coremgr.mq.rumqttd.console-port")
            .help("rumqttd console listen port")
            .num_args(1)
            .value_parser(RangedU64ValueParser::<u64>::new().range(1..=65535))
            .default_value(DEF_RUMQTTD_CONSOLE_PORT_STR),
    )
    .arg(
        Arg::new("coremgr.mq-channels.data.url")
            .long("coremgr.mq-channels.data.url")
            .help("URL of `coremgr.data` channel")
            .num_args(1),
    )
}

/// To read input arguments from command-line arguments and environment variables.
///
/// This function will call [`apply_default()`] to fill missing values so you do not need call it
/// again.
pub fn read_args(args: &ArgMatches) -> Config {
    apply_default(&Config {
        auth: match args.get_one::<String>("coremgr.auth") {
            None => match env::var("COREMGR_AUTH") {
                Err(_) => None,
                Ok(v) => Some(v),
            },
            Some(v) => Some(v.clone()),
        },
        broker: match args.get_one::<String>("coremgr.broker") {
            None => match env::var("COREMGR_BROKER") {
                Err(_) => None,
                Ok(v) => Some(v),
            },
            Some(v) => Some(v.clone()),
        },
        mq: Some(Mq {
            engine: Some(Engine {
                amqp: match args.get_one::<String>("coremgr.mq.engine.amqp") {
                    None => match env::var("COREMGR_MQ_ENGINE_AMQP") {
                        Err(_) => None,
                        Ok(v) => Some(v),
                    },
                    Some(v) => Some(v.clone()),
                },
                mqtt: match args.get_one::<String>("coremgr.mq.engine.mqtt") {
                    None => match env::var("COREMGR_MQ_ENGINE_MQTT") {
                        Err(_) => None,
                        Ok(v) => Some(v),
                    },
                    Some(v) => Some(v.clone()),
                },
            }),
            rabbitmq: Some(RabbitMq {
                username: match args.get_one::<String>("coremgr.mq.rabbitmq.username") {
                    None => match env::var("COREMGR_MQ_RABBITMQ_USERNAME") {
                        Err(_) => None,
                        Ok(v) => Some(v),
                    },
                    Some(v) => Some(v.clone()),
                },
                password: match args.get_one::<String>("coremgr.mq.rabbitmq.password") {
                    None => match env::var("COREMGR_MQ_RABBITMQ_PASSWORD") {
                        Err(_) => None,
                        Ok(v) => Some(v),
                    },
                    Some(v) => Some(v.clone()),
                },
                ttl: match args.get_one::<u64>("coremgr.mq.rabbitmq.ttl") {
                    None => match env::var("COREMGR_MQ_RABBITMQ_TTL") {
                        Err(_) => None,
                        Ok(v) => match v.parse::<usize>() {
                            Err(_) => None,
                            Ok(v) => Some(v),
                        },
                    },
                    Some(v) => Some(*v as usize),
                },
                length: match args.get_one::<u64>("coremgr.mq.rabbitmq.length") {
                    None => match env::var("COREMGR_MQ_RABBITMQ_LENGTH") {
                        Err(_) => None,
                        Ok(v) => match v.parse::<usize>() {
                            Err(_) => None,
                            Ok(v) => Some(v),
                        },
                    },
                    Some(v) => Some(*v as usize),
                },
                hosts: match args.get_one::<String>("coremgr.mq.rabbitmq.hosts") {
                    None => match env::var("COREMGR_MQ_RABBITMQ_HOSTS") {
                        Err(_) => None,
                        Ok(v) => match v.len() {
                            0 => None,
                            _ => match serde_json::from_str::<Vec<MqHost>>(v.as_str()) {
                                Err(_) => None,
                                Ok(v) => Some(v),
                            },
                        },
                    },
                    Some(v) => match v.len() {
                        0 => None,
                        _ => match serde_json::from_str::<Vec<MqHost>>(v.as_str()) {
                            Err(_) => None,
                            Ok(v) => Some(v),
                        },
                    },
                },
            }),
            emqx: Some(Emqx {
                api_key: match args.get_one::<String>("coremgr.mq.emqx.apikey") {
                    None => match env::var("COREMGR_MQ_EMQX_APIKEY") {
                        Err(_) => None,
                        Ok(v) => Some(v),
                    },
                    Some(v) => Some(v.clone()),
                },
                api_secret: match args.get_one::<String>("coremgr.mq.emqx.apisecret") {
                    None => match env::var("COREMGR_MQ_EMQX_APISECRET") {
                        Err(_) => None,
                        Ok(v) => Some(v),
                    },
                    Some(v) => Some(v.clone()),
                },
                hosts: match args.get_one::<String>("coremgr.mq.emqx.hosts") {
                    None => match env::var("COREMGR_MQ_EMQX_HOSTS") {
                        Err(_) => None,
                        Ok(v) => match v.len() {
                            0 => None,
                            _ => match serde_json::from_str::<Vec<MqHost>>(v.as_str()) {
                                Err(_) => None,
                                Ok(v) => Some(v),
                            },
                        },
                    },
                    Some(v) => match v.len() {
                        0 => None,
                        _ => match serde_json::from_str::<Vec<MqHost>>(v.as_str()) {
                            Err(_) => None,
                            Ok(v) => Some(v),
                        },
                    },
                },
            }),
            rumqttd: Some(Rumqttd {
                mqtt_port: match args.get_one::<u64>("coremgr.mq.rumqttd.mqtt-port") {
                    None => match env::var("COREMGR_MQ_RUMQTTD_MQTT_PORT") {
                        Err(_) => None,
                        Ok(v) => match v.parse::<u16>() {
                            Err(_) => None,
                            Ok(v) => Some(v),
                        },
                    },
                    Some(v) => Some(*v as u16),
                },
                mqtts_port: match args.get_one::<u64>("coremgr.mq.rumqttd.mqtts-port") {
                    None => match env::var("COREMGR_MQ_RUMQTTD_MQTTS_PORT") {
                        Err(_) => None,
                        Ok(v) => match v.parse::<u16>() {
                            Err(_) => None,
                            Ok(v) => Some(v),
                        },
                    },
                    Some(v) => Some(*v as u16),
                },
                console_port: match args.get_one::<u64>("coremgr.mq.rumqttd.console-port") {
                    None => match env::var("COREMGR_MQ_RUMQTTD_CONSOLE_PORT") {
                        Err(_) => None,
                        Ok(v) => match v.parse::<u16>() {
                            Err(_) => None,
                            Ok(v) => Some(v),
                        },
                    },
                    Some(v) => Some(*v as u16),
                },
            }),
        }),
        mq_channels: Some(MqChannels {
            data: match args.get_one::<String>("coremgr.mq-channels.data.url") {
                None => match env::var("COREMGR_MQCHANNELS_DATA_URL") {
                    Err(_) => None,
                    Ok(v) => Some(CoremgrData { url: Some(v) }),
                },
                Some(v) => Some(CoremgrData {
                    url: Some(v.clone()),
                }),
            },
        }),
    })
}

/// Fill missing configuration with default values.
pub fn apply_default(config: &Config) -> Config {
    Config {
        auth: match config.auth.as_ref() {
            None => Some(DEF_AUTH.to_string()),
            Some(auth) => Some(auth.clone()),
        },
        broker: match config.broker.as_ref() {
            None => Some(DEF_BROKER.to_string()),
            Some(broker) => Some(broker.clone()),
        },
        mq: match config.mq.as_ref() {
            None => Some(Mq {
                engine: Some(Engine {
                    amqp: Some(DEF_ENGINE_AMQP.to_string()),
                    mqtt: Some(DEF_ENGINE_MQTT.to_string()),
                }),
                rabbitmq: Some(RabbitMq {
                    username: Some(DEF_RABBITMQ_USERNAME.to_string()),
                    password: Some(DEF_RABBITMQ_PASSWORD.to_string()),
                    ttl: None,
                    length: None,
                    hosts: None,
                }),
                emqx: Some(Emqx {
                    api_key: Some(DEF_EMQX_API_KEY.to_string()),
                    api_secret: Some(DEF_EMQX_API_SECRET.to_string()),
                    hosts: None,
                }),
                rumqttd: Some(Rumqttd {
                    mqtt_port: Some(DEF_RUMQTTD_MQTT_PORT),
                    mqtts_port: Some(DEF_RUMQTTD_MQTTS_PORT),
                    console_port: Some(DEF_RUMQTTD_CONSOLE_PORT),
                }),
            }),
            Some(mq) => Some(Mq {
                engine: match mq.engine.as_ref() {
                    None => Some(Engine {
                        amqp: Some(DEF_ENGINE_AMQP.to_string()),
                        mqtt: Some(DEF_ENGINE_MQTT.to_string()),
                    }),
                    Some(engine) => Some(Engine {
                        amqp: match engine.amqp.as_ref() {
                            None => Some(DEF_ENGINE_AMQP.to_string()),
                            Some(amqp) => match amqp.as_str() {
                                MqEngine::RABBITMQ => Some(MqEngine::RABBITMQ.to_string()),
                                _ => Some(DEF_ENGINE_AMQP.to_string()),
                            },
                        },
                        mqtt: match engine.mqtt.as_ref() {
                            None => Some(DEF_ENGINE_MQTT.to_string()),
                            Some(mqtt) => match mqtt.as_str() {
                                MqEngine::EMQX => Some(MqEngine::EMQX.to_string()),
                                MqEngine::RUMQTTD => Some(MqEngine::RUMQTTD.to_string()),
                                _ => Some(DEF_ENGINE_MQTT.to_string()),
                            },
                        },
                    }),
                },
                rabbitmq: match mq.rabbitmq.as_ref() {
                    None => Some(RabbitMq {
                        username: Some(DEF_RABBITMQ_USERNAME.to_string()),
                        password: Some(DEF_RABBITMQ_PASSWORD.to_string()),
                        ttl: None,
                        length: None,
                        hosts: None,
                    }),
                    Some(rabbitmq) => Some(RabbitMq {
                        username: match rabbitmq.username.as_ref() {
                            None => Some(DEF_RABBITMQ_USERNAME.to_string()),
                            Some(username) => Some(username.clone()),
                        },
                        password: match rabbitmq.password.as_ref() {
                            None => Some(DEF_RABBITMQ_PASSWORD.to_string()),
                            Some(password) => Some(password.clone()),
                        },
                        ttl: rabbitmq.ttl,
                        length: rabbitmq.length,
                        hosts: match rabbitmq.hosts.as_ref() {
                            None => None,
                            Some(hosts) => Some(hosts.clone()),
                        },
                    }),
                },
                emqx: match mq.emqx.as_ref() {
                    None => Some(Emqx {
                        api_key: Some(DEF_EMQX_API_KEY.to_string()),
                        api_secret: Some(DEF_EMQX_API_SECRET.to_string()),
                        hosts: None,
                    }),
                    Some(emqx) => Some(Emqx {
                        api_key: match emqx.api_key.as_ref() {
                            None => Some(DEF_EMQX_API_KEY.to_string()),
                            Some(api_key) => Some(api_key.clone()),
                        },
                        api_secret: match emqx.api_secret.as_ref() {
                            None => Some(DEF_EMQX_API_SECRET.to_string()),
                            Some(api_secret) => Some(api_secret.clone()),
                        },
                        hosts: match emqx.hosts.as_ref() {
                            None => None,
                            Some(hosts) => Some(hosts.clone()),
                        },
                    }),
                },
                rumqttd: match mq.rumqttd.as_ref() {
                    None => Some(Rumqttd {
                        mqtt_port: Some(DEF_RUMQTTD_MQTT_PORT),
                        mqtts_port: Some(DEF_RUMQTTD_MQTTS_PORT),
                        console_port: Some(DEF_RUMQTTD_CONSOLE_PORT),
                    }),
                    Some(rumqttd) => Some(Rumqttd {
                        mqtt_port: match rumqttd.mqtt_port {
                            None => Some(DEF_RUMQTTD_MQTT_PORT),
                            Some(port) => Some(port),
                        },
                        mqtts_port: match rumqttd.mqtts_port {
                            None => Some(DEF_RUMQTTD_MQTTS_PORT),
                            Some(port) => Some(port),
                        },
                        console_port: match rumqttd.console_port {
                            None => Some(DEF_RUMQTTD_CONSOLE_PORT),
                            Some(port) => Some(port),
                        },
                    }),
                },
            }),
        },
        mq_channels: match config.mq_channels.as_ref() {
            None => Some(MqChannels { data: None }),
            Some(mq_channels) => Some(MqChannels {
                data: match mq_channels.data.as_ref() {
                    None => None,
                    Some(channel) => match channel.url.as_ref() {
                        None => None,
                        Some(url) => Some(CoremgrData {
                            url: Some(url.to_string()),
                        }),
                    },
                },
            }),
        },
    }
}
