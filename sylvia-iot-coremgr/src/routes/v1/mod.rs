use actix_web::{
    http::header::{self, HeaderValue},
    web::{self, Bytes},
    HttpRequest, HttpResponse, HttpResponseBuilder, ResponseError,
};
use log::error;
use reqwest::{self, Client, Method, StatusCode};
use serde_urlencoded;
use url::Url;

use sylvia_iot_corelib::err::ErrResp;

use super::{AmqpState, MqttState, State};
use crate::libs::mq::{emqx, rabbitmq, QueueType};

pub mod application;
pub mod auth;
pub mod client;
pub mod device;
pub mod device_route;
pub mod dldata_buffer;
pub mod network;
pub mod network_route;
mod request;
mod response;
pub mod unit;
pub mod user;

enum ListResp {
    /// The complete response. Return this directly.
    ActixWeb(HttpResponse),
    /// Get body from [`reqwest::Response`]. Use [`HttpResponseBuilder`] to build response body.
    ArrayStream(reqwest::Response, HttpResponseBuilder),
}

struct CreateQueueResource<'a> {
    scheme: &'a str,
    host: &'a str,
    username: &'a str,
    password: &'a str,
    ttl: Option<usize>,
    length: Option<usize>,
    q_type: QueueType,
}

struct ClearQueueResource<'a> {
    scheme: &'a str,
    host: &'a str,
    username: &'a str,
}

struct PatchHost {
    host_uri: Url,
    username: String,
}

/// To launch a HTTP request as bridge from coremgr to auth/broker.
async fn api_bridge(
    fn_name: &str,
    client: &Client,
    req: &mut HttpRequest,
    api_path: &str,
    body: Option<Bytes>,
) -> HttpResponse {
    let token = match req.headers().get(header::AUTHORIZATION) {
        None => {
            return ErrResp::ErrParam(Some("missing Authorization".to_string())).error_response()
        }
        Some(value) => value.clone(),
    };

    let mut builder = client
        .request(req.method().clone(), api_path)
        .header(reqwest::header::AUTHORIZATION, token);
    if req.query_string().len() > 0 {
        let query = match serde_urlencoded::from_str::<Vec<(String, String)>>(req.query_string()) {
            Err(e) => {
                let e = format!("parse query error: {}", e);
                return ErrResp::ErrParam(Some(e)).error_response();
            }
            Ok(query) => query,
        };
        builder = builder.query(&query);
    }
    if let Some(content_type) = req.headers().get(header::CONTENT_TYPE) {
        builder = builder.header(reqwest::header::CONTENT_TYPE, content_type);
    }
    if let Some(body) = body {
        builder = builder.body(body);
    }
    let api_req = match builder.build() {
        Err(e) => {
            let e = format!("generate request error: {}", e);
            error!("[{}] {}", fn_name, e);
            return ErrResp::ErrRsc(Some(e)).error_response();
        }
        Ok(req) => req,
    };
    let api_resp = match client.execute(api_req).await {
        Err(e) => {
            let e = format!("execute request error: {}", e);
            error!("[{}] {}", fn_name, e);
            return ErrResp::ErrIntMsg(Some(e)).error_response();
        }
        Ok(resp) => resp,
    };

    let status_code = api_resp.status();
    let mut resp = HttpResponseBuilder::new(status_code);
    if let Some(content_type) = api_resp.headers().get(header::CONTENT_TYPE) {
        resp.insert_header((header::CONTENT_TYPE, content_type.clone()));
    }
    if let Some(auth) = api_resp.headers().get(header::WWW_AUTHENTICATE) {
        resp.insert_header((header::WWW_AUTHENTICATE, auth.clone()));
    }
    resp.streaming(api_resp.bytes_stream())
}

/// To launch a HTTP request for one `/list` API as bridge from coremgr to auth/broker.
async fn list_api_bridge(
    fn_name: &str,
    client: &Client,
    req: &mut HttpRequest,
    api_path: &str,
    force_array: bool,
    csv_file: &str,
) -> ListResp {
    let token = match req.headers().get(header::AUTHORIZATION) {
        None => {
            let msg = "missing Authorization".to_string();
            return ListResp::ActixWeb(ErrResp::ErrParam(Some(msg)).error_response());
        }
        Some(value) => value.clone(),
    };

    let mut is_csv = false;
    let mut builder = client
        .request(req.method().clone(), api_path)
        .header(reqwest::header::AUTHORIZATION, token);
    if req.query_string().len() > 0 {
        let query = match serde_urlencoded::from_str::<Vec<(String, String)>>(req.query_string()) {
            Err(e) => {
                let e = format!("parse query error: {}", e);
                return ListResp::ActixWeb(ErrResp::ErrParam(Some(e)).error_response());
            }
            Ok(query) => query,
        };
        let mut has_format = false;
        let mut query: Vec<_> = query
            .iter()
            .map(|(k, v)| {
                if k.as_str().eq("format") {
                    has_format = true;
                    if v.as_str().eq("csv") {
                        is_csv = true;
                        ("format".to_string(), "array".to_string())
                    } else {
                        (k.clone(), v.clone())
                    }
                } else {
                    (k.clone(), v.clone())
                }
            })
            .collect();
        if force_array && !has_format {
            query.push(("format".to_string(), "array".to_string()));
        }
        builder = builder.query(&query);
    } else if force_array {
        builder = builder.query(&vec![("format", "array")]);
    }
    let api_req = match builder.build() {
        Err(e) => {
            let e = format!("generate request error: {}", e);
            error!("[{}] {}", fn_name, e);
            return ListResp::ActixWeb(ErrResp::ErrRsc(Some(e)).error_response());
        }
        Ok(req) => req,
    };
    let api_resp = match client.execute(api_req).await {
        Err(e) => {
            let e = format!("execute request error: {}", e);
            error!("[{}] {}", fn_name, e);
            return ListResp::ActixWeb(ErrResp::ErrIntMsg(Some(e)).error_response());
        }
        Ok(resp) => resp,
    };

    let status_code = api_resp.status();
    let mut resp = HttpResponseBuilder::new(status_code);
    if is_csv {
        resp.insert_header((header::CONTENT_TYPE, "text/csv"));
        let csv_file = format!("attachment;filename={}.csv", csv_file);
        resp.insert_header((header::CONTENT_DISPOSITION, csv_file));
    } else if let Some(content_type) = api_resp.headers().get(header::CONTENT_TYPE) {
        resp.insert_header((header::CONTENT_TYPE, content_type.clone()));
    }
    if let Some(auth) = api_resp.headers().get(header::WWW_AUTHENTICATE) {
        resp.insert_header((header::WWW_AUTHENTICATE, auth.clone()));
    }
    if status_code == reqwest::StatusCode::OK && (is_csv || force_array) {
        return ListResp::ArrayStream(api_resp, resp);
    }
    ListResp::ActixWeb(resp.streaming(api_resp.bytes_stream()))
}

async fn get_tokeninfo_inner(
    fn_name: &str,
    client: &Client,
    auth_base: &str,
    token: &HeaderValue,
) -> Result<response::TokenInfo, HttpResponse> {
    let uri = format!("{}/api/v1/auth/tokeninfo", auth_base);
    let resp = get_stream_resp(fn_name, token, &client, uri.as_str()).await?;
    match resp.json::<response::GetTokenInfo>().await {
        Err(e) => {
            let e = format!("wrong response of token info: {}", e);
            error!("[{}] {}", fn_name, e);
            Err(ErrResp::ErrIntMsg(Some(e)).error_response())
        }
        Ok(info) => Ok(info.data),
    }
}

async fn get_unit_inner(
    fn_name: &str,
    client: &Client,
    broker_base: &str,
    unit_id: &str,
    token: &HeaderValue,
) -> Result<Option<response::Unit>, HttpResponse> {
    let uri = format!("{}/api/v1/unit/{}", broker_base, unit_id);
    match get_stream_resp(fn_name, token, &client, uri.as_str()).await {
        Err(resp) => match resp.status() {
            StatusCode::NOT_FOUND => Ok(None),
            _ => Err(resp),
        },
        Ok(resp) => match resp.json::<response::GetUnit>().await {
            Err(e) => {
                let e = format!("wrong response of unit: {}", e);
                error!("[{}] {}", fn_name, e);
                Err(ErrResp::ErrIntMsg(Some(e)).error_response())
            }
            Ok(unit) => Ok(Some(unit.data)),
        },
    }
}

async fn get_device_inner(
    fn_name: &str,
    client: &Client,
    broker_base: &str,
    device_id: &str,
    token: &HeaderValue,
) -> Result<Option<response::Device>, HttpResponse> {
    let uri = format!("{}/api/v1/device/{}", broker_base, device_id);
    match get_stream_resp(fn_name, token, &client, uri.as_str()).await {
        Err(resp) => match resp.status() {
            StatusCode::NOT_FOUND => Ok(None),
            _ => Err(resp),
        },
        Ok(resp) => match resp.json::<response::GetDevice>().await {
            Err(e) => {
                let e = format!("wrong response of device: {}", e);
                error!("[{}] {}", fn_name, e);
                Err(ErrResp::ErrIntMsg(Some(e)).error_response())
            }
            Ok(device) => Ok(Some(device.data)),
        },
    }
}

async fn get_stream_resp(
    fn_name: &str,
    token: &HeaderValue,
    client: &Client,
    uri: &str,
) -> Result<reqwest::Response, HttpResponse> {
    match client
        .request(Method::GET, uri)
        .header(reqwest::header::AUTHORIZATION, token)
        .build()
    {
        Err(e) => {
            let e = format!("generate request error: {}", e);
            error!("[{}] {}", fn_name, e);
            Err(ErrResp::ErrRsc(Some(e)).error_response())
        }
        Ok(req) => match client.execute(req).await {
            Err(e) => {
                let e = format!("execute request error: {}", e);
                error!("[{}] {}", fn_name, e);
                Err(ErrResp::ErrIntMsg(Some(e)).error_response())
            }
            Ok(resp) => match resp.status() {
                StatusCode::OK => Ok(resp),
                _ => {
                    let mut new_resp = HttpResponseBuilder::new(resp.status());
                    if let Some(content_type) = resp.headers().get(header::CONTENT_TYPE) {
                        new_resp.insert_header((header::CONTENT_TYPE, content_type.clone()));
                    }
                    if let Some(auth) = resp.headers().get(header::WWW_AUTHENTICATE) {
                        new_resp.insert_header((header::WWW_AUTHENTICATE, auth.clone()));
                    }
                    Err(new_resp.streaming(resp.bytes_stream()))
                }
            },
        },
    }
}

/// To compare if two broker hosts are the same.
///
/// For example:
/// - `amqp://localhost` is the same as `amqp://localhost:5672`
/// - `mqtts://localhost` is the same as `mqtts://localhost:8883`
fn cmp_host_uri(src: &str, dst: &str) -> bool {
    let src_uri = match Url::parse(src) {
        Err(_) => return false,
        Ok(uri) => uri,
    };
    let dst_uri = match Url::parse(dst) {
        Err(_) => return false,
        Ok(uri) => uri,
    };
    if src_uri.scheme() != dst_uri.scheme() || src_uri.host_str() != dst_uri.host_str() {
        return false;
    }
    let src_port = match src_uri.port() {
        None => match src_uri.scheme() {
            "amqp" => 5672,
            "amqps" => 5671,
            "mqtt" => 1883,
            "mqtts" => 8883,
            _ => return false,
        },
        Some(port) => port,
    };
    let dst_port = match dst_uri.port() {
        None => match dst_uri.scheme() {
            "amqp" => 5672,
            "amqps" => 5671,
            "mqtt" => 1883,
            "mqtts" => 8883,
            _ => return false,
        },
        Some(port) => port,
    };
    src_port == dst_port
}

/// To set-up queue resources (vhost, ACL, ...) in the broker.
async fn create_queue_rsc<'a>(
    fn_name: &str,
    state: &web::Data<State>,
    rsc: &CreateQueueResource<'a>,
) -> Result<(), HttpResponse> {
    let scheme = rsc.scheme;
    match scheme {
        "amqp" | "amqps" => match &state.amqp {
            AmqpState::RabbitMq(opts) => {
                let client = state.client.clone();
                let host = rsc.host;
                let username = rsc.username;
                let password = rsc.password;
                let clear_rsc = ClearQueueResource {
                    scheme,
                    host,
                    username,
                };
                if let Err(e) = rabbitmq::put_user(&client, opts, host, username, password).await {
                    error!("[{}] add RabbitMQ user {} error: {}", fn_name, username, e);
                    return Err(e.error_response());
                }
                if let Err(e) = rabbitmq::put_vhost(&client, opts, host, username).await {
                    let _ = clear_queue_rsc(fn_name, &state, &clear_rsc);
                    error!("[{}] add RabbitMQ vhost {} error: {}", fn_name, username, e);
                    return Err(e.error_response());
                }
                if let Err(e) =
                    rabbitmq::put_permissions(&client, opts, host, rsc.q_type, username).await
                {
                    let _ = clear_queue_rsc(fn_name, &state, &clear_rsc);
                    error!(
                        "[{}] add RabbitMQ permission {} error: {}",
                        fn_name, username, e
                    );
                    return Err(e.error_response());
                }
                if rsc.ttl.is_some() && rsc.ttl.is_some() {
                    let policies = rabbitmq::BrokerPolicies {
                        ttl: rsc.ttl,
                        length: rsc.length,
                    };
                    if let Err(e) =
                        rabbitmq::put_policies(&client, opts, host, username, &policies).await
                    {
                        error!("[{}] patch RabbitMQ {} error: {}", fn_name, username, e);
                        return Err(e.error_response());
                    }
                }
            }
        },
        "mqtt" | "mqtts" => match &state.mqtt {
            MqttState::Emqx(opts) => {
                let client = state.client.clone();
                let host = rsc.host;
                let username = rsc.username;
                let password = rsc.password;
                let clear_rsc = ClearQueueResource {
                    scheme,
                    host,
                    username,
                };
                if let Err(e) = emqx::post_user(
                    &client,
                    opts,
                    host,
                    opts.api_key.as_str(),
                    opts.api_secret.as_str(),
                    true,
                )
                .await
                {
                    error!("[{}] add EMQX user {} error: {}", fn_name, username, e);
                    return Err(e.error_response());
                }
                if let Err(e) =
                    emqx::post_user(&client, opts, host, username, password, false).await
                {
                    error!("[{}] add EMQX user {} error: {}", fn_name, username, e);
                    return Err(e.error_response());
                }
                let q_type = QueueType::Application;
                if let Err(e) = emqx::post_acl(&client, opts, host, q_type, username).await {
                    let _ = clear_queue_rsc(fn_name, &state, &clear_rsc);
                    error!("[{}] add EMQX ACL {} error: {}", fn_name, username, e);
                    return Err(e.error_response());
                }
                if let Err(e) =
                    emqx::post_topic_metrics(&client, opts, host, q_type, username).await
                {
                    let _ = clear_queue_rsc(fn_name, &state, &clear_rsc);
                    error!("[{}] add EMQX metrics {} error: {}", fn_name, username, e);
                    return Err(e.error_response());
                }
            }
            MqttState::Rumqttd => {}
        },
        _ => return Err(ErrResp::ErrParam(Some("unsupport scheme".to_string())).error_response()),
    }
    Ok(())
}

/// To clear queue resources (vhost, ACL, ...) in the broker.
async fn clear_queue_rsc<'a>(
    fn_name: &str,
    state: &web::Data<State>,
    rsc: &ClearQueueResource<'a>,
) -> Result<(), HttpResponse> {
    match rsc.scheme {
        "amqp" | "amqps" => match &state.amqp {
            AmqpState::RabbitMq(opts) => {
                let client = state.client.clone();
                if let Err(e) = rabbitmq::delete_user(&client, opts, rsc.host, rsc.username).await {
                    error!(
                        "[{}] clear RabbitMQ user {} error: {}",
                        fn_name, rsc.username, e
                    );
                    return Err(e.error_response());
                }
                if let Err(e) = rabbitmq::delete_vhost(&client, opts, rsc.host, rsc.username).await
                {
                    error!(
                        "[{}] clear RabbitMQ vhost {} error: {}",
                        fn_name, rsc.username, e
                    );
                    return Err(e.error_response());
                }
            }
        },
        "mqtt" | "mqtts" => match &state.mqtt {
            MqttState::Emqx(opts) => {
                let client = state.client.clone();
                if let Err(e) = emqx::delete_user(&client, opts, rsc.host, rsc.username).await {
                    error!(
                        "[{}] clear EMQX user {} error: {}",
                        fn_name, rsc.username, e
                    );
                    return Err(e.error_response());
                }
                let q_type = QueueType::Application;
                if let Err(e) = emqx::delete_acl(&client, opts, rsc.host, rsc.username).await {
                    error!("[{}] clear EMQX ACL {} error: {}", fn_name, rsc.username, e);
                    return Err(e.error_response());
                }
                if let Err(e) =
                    emqx::delete_topic_metrics(&client, opts, rsc.host, q_type, rsc.username).await
                {
                    error!(
                        "[{}] clear EMQX topic metrics {} error: {}",
                        fn_name, rsc.username, e
                    );
                    return Err(e.error_response());
                }
            }
            MqttState::Rumqttd => {}
        },
        _ => {}
    }
    Ok(())
}

/// To clear new resources after something wrong when patching the application/network.
async fn clear_patch_host(fn_name: &str, state: &web::Data<State>, patch_host: &Option<PatchHost>) {
    if let Some(patch_host) = patch_host {
        if let Some(host) = patch_host.host_uri.host_str() {
            let clear_rsc = ClearQueueResource {
                scheme: patch_host.host_uri.scheme(),
                host,
                username: patch_host.username.as_str(),
            };
            let _ = clear_queue_rsc(fn_name, &state, &clear_rsc);
        }
    }
}

/// To composite management plugin's information in the URI for sylvia-broker.
fn transfer_host_uri(state: &web::Data<State>, host_uri: &mut Url, mq_username: &str) {
    match host_uri.scheme() {
        "amqp" | "amqps" => match &state.amqp {
            AmqpState::RabbitMq(opts) => {
                let _ = host_uri.set_username(opts.username.as_str());
                let _ = host_uri.set_password(Some(opts.password.as_str()));
                let _ = host_uri.set_path(mq_username);
            }
        },
        "mqtt" | "mqtts" => match &state.mqtt {
            MqttState::Emqx(opts) => {
                let _ = host_uri.set_username(opts.api_key.as_str());
                let _ = host_uri.set_password(Some(opts.api_secret.as_str()));
            }
            MqttState::Rumqttd => {}
        },
        _ => {}
    }
}

/// Truncates the host (from sylvia-broker) to `scheme://host:port`.
fn trunc_host_uri(host_uri: &Url) -> String {
    let mut new_uri = host_uri.clone();
    let _ = new_uri.set_username("");
    let _ = new_uri.set_password(None);
    new_uri.set_path("");
    new_uri.to_string()
}
