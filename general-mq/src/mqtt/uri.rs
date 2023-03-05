use std::str::FromStr;

use url::Url;
use urlencoding;

/// A MQTT Uri. This is used for manage parsed URI from the option.
///
/// This implementation refers [`lapin::uri::AMQPUri`].
#[derive(Clone)]
pub struct MQTTUri {
    pub scheme: MQTTScheme,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
}

/// Support MQTT scheme.
#[derive(Clone, PartialEq)]
pub enum MQTTScheme {
    MQTT,
    MQTTS,
}

impl Default for MQTTUri {
    fn default() -> Self {
        MQTTUri {
            scheme: Default::default(),
            host: "localhost".to_string(),
            port: MQTTScheme::default().default_port(),
            username: "".to_string(),
            password: "".to_string(),
        }
    }
}

impl FromStr for MQTTUri {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let url = Url::parse(s).map_err(|e| e.to_string())?;
        if url.cannot_be_a_base() {
            return Err(format!("Invalid URL: '{}'", s));
        }
        let default = MQTTUri::default();
        let scheme = url.scheme().parse::<MQTTScheme>()?;
        let username = match url.username() {
            "" => default.username,
            username => percent_decode(username)?,
        };
        let password = url
            .password()
            .map_or(Ok(default.password), percent_decode)?;
        let host = url.domain().map_or(Ok(default.host), percent_decode)?;
        let port = url.port().unwrap_or_else(|| scheme.default_port());

        Ok(MQTTUri {
            scheme,
            host,
            port,
            username,
            password,
        })
    }
}

impl MQTTScheme {
    pub fn default_port(&self) -> u16 {
        match *self {
            MQTTScheme::MQTT => 1883,
            MQTTScheme::MQTTS => 8883,
        }
    }
}

impl Default for MQTTScheme {
    fn default() -> Self {
        MQTTScheme::MQTT
    }
}

impl FromStr for MQTTScheme {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "mqtt" => Ok(MQTTScheme::MQTT),
            "mqtts" => Ok(MQTTScheme::MQTTS),
            s => Err(format!("Invalid MQTT scheme: {}", s)),
        }
    }
}

fn percent_decode(s: &str) -> Result<String, String> {
    match urlencoding::decode(s) {
        Err(e) => Err(e.to_string()),
        Ok(s) => Ok(s.into_owned()),
    }
}
