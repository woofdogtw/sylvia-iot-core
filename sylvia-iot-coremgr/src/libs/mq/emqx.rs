//! Wrapper APIs for controlling EMQX.
//!
//! - `hostname` of all APIs are host name or IP address of the broker.

use reqwest::{self, Client, Method, StatusCode};
use serde::{Deserialize, Serialize};

use sylvia_iot_corelib::{err::ErrResp, strings::randomstring};

use super::QueueType;

/// EMQX management information.
#[derive(Clone)]
pub struct ManagementOpts {
    /// Management plugin API key.
    pub api_key: String,
    /// Management plugin API secret.
    pub api_secret: String,
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

#[derive(Deserialize)]
struct Meta {
    count: usize,
}

#[derive(Serialize)]
struct PostAuthUsersBody<'a> {
    user_id: &'a str,
    password: &'a str,
    is_superuser: bool,
}

#[derive(Serialize)]
struct PutAuthUsersBody<'a> {
    password: &'a str,
}

#[derive(Serialize)]
struct PostAclBodyItem<'a> {
    username: &'a str,
    rules: Vec<PostAclRuleItem<'a>>,
}

#[derive(Clone, Serialize)]
struct PostAclRuleItem<'a> {
    topic: String,
    action: &'a str,
    permission: &'a str,
}

#[derive(Serialize)]
struct PostPublishBody<'a> {
    topic: String,
    clientid: String,
    payload: String,
    payload_encoding: &'a str,
    qos: usize,
}

#[derive(Serialize)]
struct PostTopicMetricsBody {
    topic: String,
}

#[derive(Deserialize)]
struct GetSubscriptionsResBody {
    meta: Meta,
}

#[derive(Default, Deserialize)]
struct GetTopicMetricsResBody {
    metrics: TopicMetrics,
}

#[derive(Default, Deserialize)]
struct TopicMetrics {
    #[serde(rename = "messages.in.rate")]
    messages_in_rate: Option<f64>,
    #[serde(rename = "messages.out.rate")]
    messages_out_rate: Option<f64>,
}

#[derive(Deserialize)]
struct ErrResBody {
    code: String,
    message: Option<String>,
}

/// Authenticator ID.
const AUTH_ID: &'static str = "password_based:built_in_database";

/// To create an account.
pub async fn post_user(
    client: &Client,
    opts: &ManagementOpts,
    hostname: &str,
    username: &str,
    password: &str,
    is_superuser: bool,
) -> Result<(), ErrResp> {
    let uri = format!(
        "http://{}:18083/api/v5/authentication/{}/users",
        hostname, AUTH_ID
    );
    let req = match client
        .request(Method::POST, uri)
        .basic_auth(opts.api_key.as_str(), Some(opts.api_secret.as_str()))
        .json(&PostAuthUsersBody {
            user_id: username,
            password,
            is_superuser,
        })
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
            StatusCode::CREATED => Ok(()),
            StatusCode::CONFLICT => put_user(client, opts, hostname, username, password).await,
            _ => {
                let e = format!("execute user request with status: {}", resp.status());
                Err(ErrResp::ErrIntMsg(Some(e)))
            }
        },
    }
}

/// To update the user's password.
pub async fn put_user(
    client: &Client,
    opts: &ManagementOpts,
    hostname: &str,
    username: &str,
    password: &str,
) -> Result<(), ErrResp> {
    let uri = format!(
        "http://{}:18083/api/v5/authentication/{}/users/{}",
        hostname, AUTH_ID, username
    );
    let req = match client
        .request(Method::PUT, uri)
        .basic_auth(opts.api_key.as_str(), Some(opts.api_secret.as_str()))
        .json(&PutAuthUsersBody { password })
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
            StatusCode::OK => Ok(()),
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
    let uri = format!(
        "http://{}:18083/api/v5/authentication/{}/users/{}",
        hostname, AUTH_ID, username
    );
    let req = match client
        .request(Method::DELETE, uri)
        .basic_auth(opts.api_key.as_str(), Some(opts.api_secret.as_str()))
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

/// To create an ACL rule of a topic for the user.
pub async fn post_acl(
    client: &Client,
    opts: &ManagementOpts,
    hostname: &str,
    q_type: QueueType,
    username: &str,
) -> Result<(), ErrResp> {
    let uri = format!(
        "http://{}:18083/api/v5/authorization/sources/built_in_database/rules/users",
        hostname
    );
    let rules = match q_type {
        QueueType::Application => vec![
            PostAclRuleItem {
                topic: format!("broker.{}.uldata", username),
                action: "subscribe",
                permission: "allow",
            },
            PostAclRuleItem {
                topic: format!("broker.{}.dldata", username),
                action: "publish",
                permission: "allow",
            },
            PostAclRuleItem {
                topic: format!("broker.{}.dldata-resp", username),
                action: "subscribe",
                permission: "allow",
            },
            PostAclRuleItem {
                topic: format!("broker.{}.dldata-result", username),
                action: "subscribe",
                permission: "allow",
            },
        ],
        QueueType::Network => vec![
            PostAclRuleItem {
                topic: format!("broker.{}.uldata", username),
                action: "publish",
                permission: "allow",
            },
            PostAclRuleItem {
                topic: format!("broker.{}.dldata", username),
                action: "subscribe",
                permission: "allow",
            },
            PostAclRuleItem {
                topic: format!("broker.{}.dldata-result", username),
                action: "publish",
                permission: "allow",
            },
            PostAclRuleItem {
                topic: format!("broker.{}.ctrl", username),
                action: "subscribe",
                permission: "allow",
            },
        ],
    };
    let req = match client
        .request(Method::POST, uri.clone())
        .basic_auth(opts.api_key.as_str(), Some(opts.api_secret.as_str()))
        .json(&vec![PostAclBodyItem {
            username,
            rules: rules.clone(),
        }])
        .build()
    {
        Err(e) => {
            let e = format!("generate acl request error: {}", e);
            return Err(ErrResp::ErrRsc(Some(e)));
        }
        Ok(req) => req,
    };
    match client.execute(req).await {
        Err(e) => {
            let e = format!("execute acl request error: {}", e);
            return Err(ErrResp::ErrIntMsg(Some(e)));
        }
        Ok(resp) => match resp.status() {
            StatusCode::NO_CONTENT => return Ok(()),
            StatusCode::CONFLICT => (),
            _ => {
                let e = format!("execute acl request with status: {}", resp.status());
                return Err(ErrResp::ErrIntMsg(Some(e)));
            }
        },
    }

    let req = match client
        .request(Method::PUT, format!("{}/{}", uri, username))
        .basic_auth(opts.api_key.as_str(), Some(opts.api_secret.as_str()))
        .json(&PostAclBodyItem { username, rules })
        .build()
    {
        Err(e) => {
            let e = format!("generate put acl request error: {}", e);
            return Err(ErrResp::ErrRsc(Some(e)));
        }
        Ok(req) => req,
    };
    match client.execute(req).await {
        Err(e) => {
            let e = format!("execute put acl request error: {}", e);
            Err(ErrResp::ErrIntMsg(Some(e)))
        }
        Ok(resp) => match resp.status() {
            StatusCode::NO_CONTENT => Ok(()),
            _ => {
                let e = format!("execute put acl request with status: {}", resp.status());
                Err(ErrResp::ErrIntMsg(Some(e)))
            }
        },
    }
}

/// To delete an ACL rule of a group of topics of an application/network for the user.
pub async fn delete_acl(
    client: &Client,
    opts: &ManagementOpts,
    hostname: &str,
    username: &str,
) -> Result<(), ErrResp> {
    let uri = format!(
        "http://{}:18083/api/v5/authorization/sources/built_in_database/rules/users/{}",
        hostname, username
    );
    let req = match client
        .request(Method::DELETE, uri)
        .basic_auth(opts.api_key.as_str(), Some(opts.api_secret.as_str()))
        .build()
    {
        Err(e) => {
            let e = format!("generate acl request error: {}", e);
            return Err(ErrResp::ErrRsc(Some(e)));
        }
        Ok(req) => req,
    };
    match client.execute(req).await {
        Err(e) => {
            let e = format!("execute acl request error: {}", e);
            Err(ErrResp::ErrIntMsg(Some(e)))
        }
        Ok(resp) => match resp.status() {
            StatusCode::NO_CONTENT | StatusCode::NOT_FOUND => Ok(()),
            _ => {
                let e = format!("execute acl request with status: {}", resp.status());
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
    let uri = format!("http://{}:18083/api/v5/publish", hostname);
    let body = PostPublishBody {
        topic: format!("broker.{}.{}", username, queue),
        clientid: format!("sylvia-{}", randomstring(12)),
        payload,
        payload_encoding: "base64",
        qos: 0,
    };
    let req = match client
        .request(Method::POST, uri)
        .basic_auth(opts.api_key.as_str(), Some(opts.api_secret.as_str()))
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
            StatusCode::OK | StatusCode::ACCEPTED => Ok(()), // 200 for <= 5.0.8, 202 for >= 5.0.9
            _ => {
                let e = format!("execute publish request with status: {}", resp.status());
                Err(ErrResp::ErrIntMsg(Some(e)))
            }
        },
    }
}

/// To enable metrics for a queue.
pub async fn post_topic_metrics(
    client: &Client,
    opts: &ManagementOpts,
    hostname: &str,
    q_type: QueueType,
    username: &str,
) -> Result<(), ErrResp> {
    let uri = format!("http://{}:18083/api/v5/mqtt/topic_metrics", hostname);
    let q_name_prefix = format!("broker.{}.", username);
    let queues = match q_type {
        QueueType::Application => vec!["uldata", "dldata", "dldata-resp", "dldata-result"],
        QueueType::Network => vec!["uldata", "dldata", "dldata-result", "ctrl"],
    };
    for queue in queues {
        let req = match client
            .request(Method::POST, uri.as_str())
            .basic_auth(opts.api_key.as_str(), Some(opts.api_secret.as_str()))
            .json(&PostTopicMetricsBody {
                topic: format!("{}{}", q_name_prefix, queue),
            })
            .build()
        {
            Err(e) => {
                let e = format!("generate topic_metrics request error: {}", e);
                return Err(ErrResp::ErrRsc(Some(e)));
            }
            Ok(req) => req,
        };
        match client.execute(req).await {
            Err(e) => {
                let e = format!("execute topic_metrics request error: {}", e);
                return Err(ErrResp::ErrIntMsg(Some(e)));
            }
            Ok(resp) => match resp.status() {
                StatusCode::OK => (),
                StatusCode::BAD_REQUEST => {
                    match resp.json::<ErrResBody>().await {
                        Err(e) => {
                            let e = format!("execute topic_metrics read 400 body error: {}", e);
                            return Err(ErrResp::ErrIntMsg(Some(e)));
                        }
                        Ok(body) => match body.code.as_str() {
                            "BAD_TOPIC" => (),
                            _ => {
                                let e = format!(
                                    "execute topic_metrics request with unexpected 400 code: {}, message: {:?}",
                                    body.code, body.message
                                );
                                return Err(ErrResp::ErrIntMsg(Some(e)));
                            }
                        },
                    };
                }
                _ => {
                    let e = format!(
                        "execute topic_metrics request with status: {}",
                        resp.status()
                    );
                    return Err(ErrResp::ErrIntMsg(Some(e)));
                }
            },
        }
    }
    Ok(())
}

/// To disable metrics for a queue.
pub async fn delete_topic_metrics(
    client: &Client,
    opts: &ManagementOpts,
    hostname: &str,
    q_type: QueueType,
    username: &str,
) -> Result<(), ErrResp> {
    let uri_prefix = format!(
        "http://{}:18083/api/v5/mqtt/topic_metrics/broker.{}.",
        hostname, username
    );
    let queues = match q_type {
        QueueType::Application => vec!["uldata", "dldata", "dldata-resp", "dldata-result"],
        QueueType::Network => vec!["uldata", "dldata", "dldata-result", "ctrl"],
    };
    for queue in queues {
        let req = match client
            .request(Method::DELETE, format!("{}{}", uri_prefix, queue).as_str())
            .basic_auth(opts.api_key.as_str(), Some(opts.api_secret.as_str()))
            .build()
        {
            Err(e) => {
                let e = format!("generate topic_metrics request error: {}", e);
                return Err(ErrResp::ErrRsc(Some(e)));
            }
            Ok(req) => req,
        };
        match client.execute(req).await {
            Err(e) => {
                let e = format!("execute topic_metrics request error: {}", e);
                return Err(ErrResp::ErrIntMsg(Some(e)));
            }
            Ok(resp) => match resp.status() {
                StatusCode::NO_CONTENT | StatusCode::NOT_FOUND => (),
                _ => {
                    let e = format!(
                        "execute topic_metrics request with status: {}",
                        resp.status()
                    );
                    return Err(ErrResp::ErrIntMsg(Some(e)));
                }
            },
        }
    }
    Ok(())
}

/// Get statistics of a queue.
pub async fn stats(
    client: &Client,
    opts: &ManagementOpts,
    hostname: &str,
    username: &str,
    queue: &str, // uldata,dldata,dldata-resp,dldata-result,ctrl
) -> Result<Stats, ErrResp> {
    let queue_name = format!("broker.{}.{}", username, queue);
    let uri = format!(
        "http://{}:18083/api/v5/subscriptions?topic={}",
        hostname, queue_name
    );
    let req = match client
        .request(Method::GET, uri)
        .basic_auth(opts.api_key.as_str(), Some(opts.api_secret.as_str()))
        .build()
    {
        Err(e) => {
            let e = format!("generate stats subscriptions request error: {}", e);
            return Err(ErrResp::ErrRsc(Some(e)));
        }
        Ok(req) => req,
    };
    let resp = match client.execute(req).await {
        Err(e) => {
            let e = format!("execute stats subscriptions request error: {}", e);
            return Err(ErrResp::ErrIntMsg(Some(e)));
        }
        Ok(resp) => match resp.status() {
            StatusCode::OK => resp,
            _ => {
                let e = format!(
                    "execute stats subscriptions request with status: {}",
                    resp.status()
                );
                return Err(ErrResp::ErrIntMsg(Some(e)));
            }
        },
    };
    let resp_stats = match resp.json::<GetSubscriptionsResBody>().await {
        Err(e) => {
            let e = format!("read stats subscriptions body error: {}", e);
            return Err(ErrResp::ErrIntMsg(Some(e)));
        }
        Ok(stats) => stats,
    };
    let mut stats = Stats {
        consumers: resp_stats.meta.count,
        ..Default::default()
    };

    let uri = format!(
        "http://{}:18083/api/v5/mqtt/topic_metrics/{}",
        hostname, queue_name
    );
    let req = match client
        .request(Method::GET, uri.as_str())
        .basic_auth(opts.api_key.as_str(), Some(opts.api_secret.as_str()))
        .build()
    {
        Err(e) => {
            let e = format!("generate stats topic_metrics request error: {}", e);
            return Err(ErrResp::ErrRsc(Some(e)));
        }
        Ok(req) => req,
    };
    let resp_stats = match client.execute(req).await {
        Err(e) => {
            let e = format!("execute stats topic_metrics request error: {}", e);
            return Err(ErrResp::ErrIntMsg(Some(e)));
        }
        Ok(resp) => match resp.status() {
            StatusCode::OK => match resp.json::<GetTopicMetricsResBody>().await {
                Err(e) => {
                    let e = format!("read stats topic_metrics body error: {}", e);
                    return Err(ErrResp::ErrIntMsg(Some(e)));
                }
                Ok(stats) => stats,
            },
            StatusCode::NOT_FOUND => GetTopicMetricsResBody::default(),
            _ => {
                let e = format!(
                    "execute stats topic_metrics request with status: {}",
                    resp.status()
                );
                return Err(ErrResp::ErrIntMsg(Some(e)));
            }
        },
    };
    stats.publish_rate = match resp_stats.metrics.messages_in_rate {
        None => 0.0,
        Some(rate) => rate,
    };
    stats.deliver_rate = match resp_stats.metrics.messages_out_rate {
        None => 0.0,
        Some(rate) => rate,
    };

    Ok(stats)
}
