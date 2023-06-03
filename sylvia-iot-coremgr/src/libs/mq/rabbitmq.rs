//! Wrapper APIs for controlling RabbitMQ.
//!
//! - `hostname` of all APIs are host name or IP address of the broker.

use reqwest::{self, Client, Method, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use sylvia_iot_corelib::err::ErrResp;

use super::QueueType;

/// RabbitMQ management information.
#[derive(Clone)]
pub struct ManagementOpts {
    /// Management plugin administrator name.
    pub username: String,
    /// Management plugin administrator password.
    pub password: String,
    /// Default message TTL in milliseconds.
    pub ttl: Option<usize>,
    /// Default queue length.
    pub length: Option<usize>,
}

/// Policies for `broker.*` queues.
pub struct BrokerPolicies {
    /// Message TTL in milliseconds.
    pub ttl: Option<usize>,
    /// Queue length.
    pub length: Option<usize>,
}

/// Statistics.
#[derive(Default)]
pub struct Stats {
    /// Number of queue consumers.
    pub consumers: usize,
    /// Number of ready/unacked messages.
    pub messages: usize,
    /// Publish rate from the producer.
    pub publish_rate: f64,
    /// Deliver rate to the consumer.
    pub deliver_rate: f64,
}

#[derive(Serialize)]
struct PutUsersBody<'a> {
    password: &'a str,
    tags: &'a str,
}

#[derive(Serialize)]
struct PutPermissionsBody {
    configure: String,
    write: String,
    read: String,
}

#[derive(Deserialize, Serialize)]
struct Policies {
    pattern: String,
    definition: PoliciesDefinition,
    #[serde(rename = "apply-to")]
    apply_to: String,
}

#[derive(Deserialize, Serialize)]
struct PoliciesDefinition {
    #[serde(rename = "message-ttl", skip_serializing_if = "Option::is_none")]
    message_ttl: Option<usize>,
    #[serde(rename = "max-length", skip_serializing_if = "Option::is_none")]
    max_length: Option<usize>,
}

#[derive(Serialize)]
struct PostExchangesBody<'a> {
    properties: Map<String, Value>,
    routing_key: String,
    payload: String,
    payload_encoding: &'a str,
}

#[derive(Deserialize)]
struct GetQueuesResBody {
    consumers: Option<usize>,
    messages: Option<usize>,
    message_stats: Option<MessageStats>,
}

#[derive(Deserialize)]
struct MessageStats {
    deliver_details: Option<Details>,
    publish_details: Option<Details>,
}

#[derive(Deserialize)]
struct Details {
    rate: f64,
}

/// To create or update user account and its password.
pub async fn put_user(
    client: &Client,
    opts: &ManagementOpts,
    hostname: &str,
    username: &str,
    password: &str,
) -> Result<(), ErrResp> {
    let uri = format!("http://{}:15672/api/users/{}", hostname, username);
    let req = match client
        .request(Method::PUT, uri)
        .basic_auth(opts.username.as_str(), Some(opts.password.as_str()))
        .json(&PutUsersBody { password, tags: "" })
        .build()
    {
        Err(e) => {
            let e = format!("generate user request error: {}", e);
            return Err(ErrResp::ErrRsc(Some(e)));
        }
        Ok(req) => req,
    };
    match client.execute(req).await {
        Err(e) => {
            let e = format!("execute user request error: {}", e);
            Err(ErrResp::ErrIntMsg(Some(e)))
        }
        Ok(resp) => match resp.status() {
            StatusCode::CREATED | StatusCode::NO_CONTENT => Ok(()),
            _ => {
                let e = format!("execute user request with status: {}", resp.status());
                Err(ErrResp::ErrIntMsg(Some(e)))
            }
        },
    }
}

/// To delete a user.
pub async fn delete_user(
    client: &Client,
    opts: &ManagementOpts,
    hostname: &str,
    username: &str,
) -> Result<(), ErrResp> {
    let uri = format!("http://{}:15672/api/users/{}", hostname, username);
    let req = match client
        .request(Method::DELETE, uri)
        .basic_auth(opts.username.as_str(), Some(opts.password.as_str()))
        .build()
    {
        Err(e) => {
            let e = format!("generate user request error: {}", e);
            return Err(ErrResp::ErrRsc(Some(e)));
        }
        Ok(req) => req,
    };
    match client.execute(req).await {
        Err(e) => {
            let e = format!("execute user request error: {}", e);
            Err(ErrResp::ErrIntMsg(Some(e)))
        }
        Ok(resp) => match resp.status() {
            StatusCode::NO_CONTENT | StatusCode::NOT_FOUND => Ok(()),
            _ => {
                let e = format!("execute user request with status: {}", resp.status());
                Err(ErrResp::ErrIntMsg(Some(e)))
            }
        },
    }
}

/// To create a virtual host.
pub async fn put_vhost(
    client: &Client,
    opts: &ManagementOpts,
    hostname: &str,
    username: &str,
) -> Result<(), ErrResp> {
    let uri = format!("http://{}:15672/api/vhosts/{}", hostname, username);
    let req = match client
        .request(Method::PUT, uri)
        .basic_auth(opts.username.as_str(), Some(opts.password.as_str()))
        .build()
    {
        Err(e) => {
            let e = format!("generate vhost request error: {}", e);
            return Err(ErrResp::ErrRsc(Some(e)));
        }
        Ok(req) => req,
    };
    match client.execute(req).await {
        Err(e) => {
            let e = format!("execute vhost request error: {}", e);
            Err(ErrResp::ErrIntMsg(Some(e)))
        }
        Ok(resp) => match resp.status() {
            StatusCode::CREATED | StatusCode::NO_CONTENT => Ok(()),
            _ => {
                let e = format!("execute vhost request with status: {}", resp.status());
                Err(ErrResp::ErrIntMsg(Some(e)))
            }
        },
    }
}

/// To delete a virtual host.
pub async fn delete_vhost(
    client: &Client,
    opts: &ManagementOpts,
    hostname: &str,
    username: &str,
) -> Result<(), ErrResp> {
    let uri = format!("http://{}:15672/api/vhosts/{}", hostname, username);
    let req = match client
        .request(Method::DELETE, uri)
        .basic_auth(opts.username.as_str(), Some(opts.password.as_str()))
        .build()
    {
        Err(e) => {
            let e = format!("generate vhost request error: {}", e);
            return Err(ErrResp::ErrRsc(Some(e)));
        }
        Ok(req) => req,
    };
    match client.execute(req).await {
        Err(e) => {
            let e = format!("execute vhost request error: {}", e);
            Err(ErrResp::ErrIntMsg(Some(e)))
        }
        Ok(resp) => match resp.status() {
            StatusCode::NO_CONTENT | StatusCode::NOT_FOUND => Ok(()),
            _ => {
                let e = format!("execute vhost request with status: {}", resp.status());
                Err(ErrResp::ErrIntMsg(Some(e)))
            }
        },
    }
}

/// To set-up permissions of a group of application/network queues in a virtual host for the user.
pub async fn put_permissions(
    client: &Client,
    opts: &ManagementOpts,
    hostname: &str,
    q_type: QueueType,
    username: &str,
) -> Result<(), ErrResp> {
    let uri = format!(
        "http://{}:15672/api/permissions/{}/{}",
        hostname, username, username
    );
    let config_pattern = match q_type {
        QueueType::Application => format!(
            "^broker.{}.(uldata|dldata|dldata-resp|dldata-result)$",
            username
        )
        .replace(".", "\\."),
        QueueType::Network => {
            format!("^broker.{}.(uldata|dldata|dldata-result|ctrl)$", username).replace(".", "\\.")
        }
    };
    let read_pattern = match q_type {
        QueueType::Application => {
            format!("^broker.{}.(uldata|dldata-resp|dldata-result)$", username).replace(".", "\\.")
        }
        QueueType::Network => format!("^broker.{}.(dldata|ctrl)$", username).replace(".", "\\."),
    };
    let body = PutPermissionsBody {
        configure: config_pattern.to_string(),
        write: ".*".to_string(),
        read: read_pattern,
    };
    let req = match client
        .request(Method::PUT, uri)
        .basic_auth(opts.username.as_str(), Some(opts.password.as_str()))
        .json(&body)
        .build()
    {
        Err(e) => {
            let e = format!("generate permissions request error: {}", e);
            return Err(ErrResp::ErrRsc(Some(e)));
        }
        Ok(req) => req,
    };
    match client.execute(req).await {
        Err(e) => {
            let e = format!("execute permissions request error: {}", e);
            Err(ErrResp::ErrIntMsg(Some(e)))
        }
        Ok(resp) => match resp.status() {
            StatusCode::CREATED | StatusCode::NO_CONTENT => Ok(()),
            _ => {
                let e = format!("execute permissions request with status: {}", resp.status());
                Err(ErrResp::ErrIntMsg(Some(e)))
            }
        },
    }
}

/// To get TTL/length policies for the user.
pub async fn get_policies(
    client: &Client,
    opts: &ManagementOpts,
    hostname: &str,
    username: &str,
) -> Result<BrokerPolicies, ErrResp> {
    let uri = format!(
        "http://{}:15672/api/policies/{}/sylvia-iot-broker",
        hostname, username
    );
    let req = match client
        .request(Method::GET, uri)
        .basic_auth(opts.username.as_str(), Some(opts.password.as_str()))
        .build()
    {
        Err(e) => {
            let e = format!("generate policies request error: {}", e);
            return Err(ErrResp::ErrRsc(Some(e)));
        }
        Ok(req) => req,
    };
    let resp = match client.execute(req).await {
        Err(e) => {
            let e = format!("execute policies request error: {}", e);
            return Err(ErrResp::ErrIntMsg(Some(e)));
        }
        Ok(resp) => match resp.status() {
            StatusCode::OK => resp,
            StatusCode::NOT_FOUND => {
                return Ok(BrokerPolicies {
                    ttl: Some(0),
                    length: Some(0),
                })
            }
            _ => {
                let e = format!("execute request with status: {}", resp.status());
                return Err(ErrResp::ErrIntMsg(Some(e)));
            }
        },
    };
    match resp.json::<Policies>().await {
        Err(e) => {
            let e = format!("not expected policies body: {}", e);
            Err(ErrResp::ErrUnknown(Some(e)))
        }
        Ok(body) => Ok(BrokerPolicies {
            ttl: match body.definition.message_ttl {
                None => Some(0),
                _ => body.definition.message_ttl,
            },
            length: match body.definition.max_length {
                None => Some(0),
                _ => body.definition.max_length,
            },
        }),
    }
}

/// To update TTL/length policies for the user.
pub async fn put_policies(
    client: &Client,
    opts: &ManagementOpts,
    hostname: &str,
    username: &str,
    policies: &BrokerPolicies,
) -> Result<(), ErrResp> {
    let uri = format!(
        "http://{}:15672/api/policies/{}/sylvia-iot-broker",
        hostname, username
    );
    let is_delete = match policies.ttl {
        None | Some(0) => match policies.length {
            None | Some(0) => true,
            _ => false,
        },
        _ => false,
    };
    let builder = if is_delete {
        client
            .request(Method::DELETE, uri)
            .basic_auth(opts.username.as_str(), Some(opts.password.as_str()))
    } else {
        let definition = PoliciesDefinition {
            message_ttl: match policies.ttl {
                Some(0) => None,
                _ => policies.ttl,
            },
            max_length: match policies.length {
                Some(0) => None,
                _ => policies.length,
            },
        };
        let body = Policies {
            pattern: "^broker.".to_string(),
            definition,
            apply_to: "queues".to_string(),
        };
        client
            .request(Method::PUT, uri)
            .basic_auth(opts.username.as_str(), Some(opts.password.as_str()))
            .json(&body)
    };
    let req = match builder.build() {
        Err(e) => {
            let e = format!("generate policies request error: {}", e);
            return Err(ErrResp::ErrRsc(Some(e)));
        }
        Ok(req) => req,
    };
    match client.execute(req).await {
        Err(e) => {
            let e = format!("execute policies request error: {}", e);
            Err(ErrResp::ErrIntMsg(Some(e)))
        }
        Ok(resp) => match resp.status() {
            StatusCode::CREATED | StatusCode::NO_CONTENT => Ok(()),
            StatusCode::NOT_FOUND => match is_delete {
                false => Err(ErrResp::ErrNotFound(None)),
                true => Ok(()),
            },
            _ => {
                let e = format!("execute request with status: {}", resp.status());
                Err(ErrResp::ErrIntMsg(Some(e)))
            }
        },
    }
}

/// To publish a message to the specified queue (such as `uldata` and `dldata`).
///
/// The `payload` MUST be Base64 encoded string.
pub async fn publish_message(
    client: &Client,
    opts: &ManagementOpts,
    hostname: &str,
    username: &str,
    queue: &str,     // uldata,dldata
    payload: String, // Base64
) -> Result<(), ErrResp> {
    let uri = format!(
        "http://{}:15672/api/exchanges/{}/amq.default/publish",
        hostname, username
    );
    let body = PostExchangesBody {
        properties: Map::<String, Value>::new(),
        routing_key: format!("broker.{}.{}", username, queue),
        payload,
        payload_encoding: "base64",
    };
    let req = match client
        .request(Method::POST, uri)
        .basic_auth(opts.username.as_str(), Some(opts.password.as_str()))
        .json(&body)
        .build()
    {
        Err(e) => {
            let e = format!("generate publish request error: {}", e);
            return Err(ErrResp::ErrRsc(Some(e)));
        }
        Ok(req) => req,
    };
    match client.execute(req).await {
        Err(e) => {
            let e = format!("execute publish request error: {}", e);
            Err(ErrResp::ErrIntMsg(Some(e)))
        }
        Ok(resp) => match resp.status() {
            StatusCode::OK => Ok(()),
            _ => {
                let e = format!("execute publish request with status: {}", resp.status());
                Err(ErrResp::ErrIntMsg(Some(e)))
            }
        },
    }
}

/// Get statistics of a queue.
pub async fn stats(
    client: &Client,
    opts: &ManagementOpts,
    hostname: &str,
    username: &str,
    queue: &str, // uldata,dldata,dldata-resp,dldata-result,ctrl
) -> Result<Stats, ErrResp> {
    let uri = format!(
        "http://{}:15672/api/queues/{}/broker.{}.{}?msg_rates_age=60&msg_rates_incr=5",
        hostname, username, username, queue
    );
    let req = match client
        .request(Method::GET, uri)
        .basic_auth(opts.username.as_str(), Some(opts.password.as_str()))
        .build()
    {
        Err(e) => {
            let e = format!("generate stats request error: {}", e);
            return Err(ErrResp::ErrRsc(Some(e)));
        }
        Ok(req) => req,
    };
    let resp = match client.execute(req).await {
        Err(e) => {
            let e = format!("execute stats request error: {}", e);
            return Err(ErrResp::ErrIntMsg(Some(e)));
        }
        Ok(resp) => match resp.status() {
            StatusCode::OK => resp,
            StatusCode::NOT_FOUND => return Err(ErrResp::ErrNotFound(None)),
            _ => {
                let e = format!("execute stats request with status: {}", resp.status());
                return Err(ErrResp::ErrIntMsg(Some(e)));
            }
        },
    };
    let resp_stats = match resp.json::<GetQueuesResBody>().await {
        Err(e) => {
            let e = format!("read stats body error: {}", e);
            return Err(ErrResp::ErrIntMsg(Some(e)));
        }
        Ok(stats) => stats,
    };
    let mut ret_stats = Stats {
        ..Default::default()
    };
    if let Some(consumers) = resp_stats.consumers {
        ret_stats.consumers = consumers;
    }
    if let Some(messages) = resp_stats.messages {
        ret_stats.messages = messages;
    }
    if let Some(stats) = resp_stats.message_stats {
        if let Some(details) = stats.publish_details.as_ref() {
            ret_stats.publish_rate = details.rate;
        }
        if let Some(details) = stats.deliver_details.as_ref() {
            ret_stats.deliver_rate = details.rate;
        }
    }
    Ok(ret_stats)
}
