use std::error::Error as StdError;

use actix_web::{
    dev::HttpServiceFactory,
    http::{
        header::{self, HeaderValue},
        StatusCode,
    },
    web::{self, Bytes, BytesMut},
    HttpRequest, HttpResponse, HttpResponseBuilder, Responder, ResponseError,
};
use base64::{engine::general_purpose, Engine};
use chrono::Utc;
use csv::WriterBuilder;
use futures_util::StreamExt;
use hex;
use log::error;
use reqwest;
use serde::{Deserialize, Serialize};
use serde_json::{Deserializer, Map, Value};
use url::Url;

use sylvia_iot_corelib::{err::ErrResp, role::Role, strings};

use super::{
    super::{AmqpState, ErrReq, MqttState, State},
    api_bridge, clear_patch_host, clear_queue_rsc, cmp_host_uri, create_queue_rsc,
    get_device_inner, get_stream_resp, get_tokeninfo_inner, get_unit_inner, list_api_bridge,
    request, response, transfer_host_uri, trunc_host_uri, ClearQueueResource, CreateQueueResource,
    ListResp, PatchHost,
};
use crate::libs::mq::{self, emqx, rabbitmq, QueueType};

enum ListFormat {
    Array,
    Csv,
    Data,
}

#[derive(Deserialize)]
struct NetworkIdPath {
    network_id: String,
}

#[derive(Deserialize, Serialize)]
struct Network {
    #[serde(rename = "networkId")]
    network_id: String,
    code: String,
    #[serde(rename = "unitId")]
    unit_id: Option<String>,
    #[serde(rename = "unitCode")]
    unit_code: Option<String>,
    #[serde(rename = "createdAt")]
    created_at: String,
    #[serde(rename = "modifiedAt")]
    modified_at: String,
    #[serde(rename = "hostUri")]
    host_uri: String,
    name: String,
    info: Map<String, Value>,
}

#[derive(Deserialize, Serialize)]
struct CsvItem {
    #[serde(rename = "networkId")]
    network_id: String,
    code: String,
    #[serde(rename = "unitId")]
    unit_id: Option<String>,
    #[serde(rename = "unitCode")]
    unit_code: Option<String>,
    #[serde(rename = "createdAt")]
    created_at: String,
    #[serde(rename = "modifiedAt")]
    modified_at: String,
    #[serde(rename = "hostUri")]
    host_uri: String,
    name: String,
    info: Option<String>,
}

/// Downlink data from application to broker.
#[derive(Default, Serialize)]
struct UlData {
    time: String,
    #[serde(rename = "networkAddr")]
    network_addr: String,
    data: String,
    extension: Option<Map<String, Value>>,
}

const CSV_FIELDS: &'static [u8] =
    b"\xEF\xBB\xBFnetworkId,code,unitId,createdAt,modifiedAt,hostUri,name,info\n";

pub fn new_service(scope_path: &str) -> impl HttpServiceFactory {
    web::scope(scope_path)
        .service(web::resource("").route(web::post().to(post_network)))
        .service(web::resource("/count").route(web::get().to(get_network_count)))
        .service(web::resource("/list").route(web::get().to(get_network_list)))
        .service(
            web::resource("/{network_id}")
                .route(web::get().to(get_network))
                .route(web::patch().to(patch_network))
                .route(web::delete().to(delete_network)),
        )
        .service(web::resource("/{network_id}/stats").route(web::get().to(get_network_stats)))
        .service(web::resource("/{network_id}/uldata").route(web::post().to(post_network_uldata)))
}

/// `POST /{base}/api/v1/network`
async fn post_network(
    req: HttpRequest,
    mut body: web::Json<request::PostNetworkBody>,
    state: web::Data<State>,
) -> impl Responder {
    const FN_NAME: &'static str = "post_network";
    let broker_base = state.broker_base.as_str();
    let api_path = format!("{}/api/v1/network", broker_base);
    let client = state.client.clone();
    let token = match req.headers().get(header::AUTHORIZATION) {
        None => {
            let e = "missing Authorization".to_string();
            return ErrResp::ErrParam(Some(e)).error_response();
        }
        Some(value) => value.clone(),
    };

    if body.data.unit_id.is_none() {
        let auth_base = state.auth_base.as_str();
        let token_info = match get_tokeninfo_inner(FN_NAME, &client, auth_base, &token).await {
            Err(e) => return e,
            Ok(info) => info,
        };
        if !Role::is_role(&token_info.roles, Role::ADMIN)
            && !Role::is_role(&token_info.roles, Role::MANAGER)
        {
            let e = "missing `unitId`".to_string();
            return ErrResp::ErrParam(Some(e)).error_response();
        }
    }

    // Get unit information to create queue information.
    let unit_code = match body.data.unit_id.as_ref() {
        None => "".to_string(),
        Some(unit_id) => {
            if unit_id.len() == 0 {
                return ErrResp::ErrParam(Some(
                    "`unitId` must with at least one character".to_string(),
                ))
                .error_response();
            }
            let unit = match get_unit_inner(FN_NAME, &client, broker_base, unit_id, &token).await {
                Err(e) => return e,
                Ok(unit) => match unit {
                    None => {
                        return ErrResp::Custom(
                            ErrReq::UNIT_NOT_EXIST.0,
                            ErrReq::UNIT_NOT_EXIST.1,
                            None,
                        )
                        .error_response()
                    }
                    Some(unit) => unit,
                },
            };
            unit.code
        }
    };
    let code = body.data.code.as_str();
    if !strings::is_code(code) {
        return ErrResp::ErrParam(Some(
            "`code` must be [A-Za-z0-9]{1}[A-Za-z0-9-_]*".to_string(),
        ))
        .error_response();
    }
    let q_type = QueueType::Network;
    let username = mq::to_username(q_type, unit_code.as_str(), code);
    let password = strings::randomstring(8);
    let uri = match Url::parse(body.data.host_uri.as_str()) {
        Err(e) => {
            return ErrResp::ErrParam(Some(format!("invalid `hostUri`: {}", e))).error_response();
        }
        Ok(uri) => uri,
    };
    let host = match uri.host() {
        None => {
            let e = "invalid `hostUri`".to_string();
            return ErrResp::ErrParam(Some(e)).error_response();
        }
        Some(host) => host.to_string(),
    };
    let scheme = uri.scheme();
    let host = host.as_str();
    let username = username.as_str();
    let password = password.as_str();

    // Create message broker resources.
    let create_rsc = CreateQueueResource {
        scheme,
        host,
        username,
        password,
        ttl: body.data.ttl,
        length: body.data.length,
        q_type: QueueType::Network,
    };
    if let Err(e) = create_queue_rsc(FN_NAME, &state, &create_rsc).await {
        return e;
    }
    let clear_rsc = ClearQueueResource {
        scheme,
        host,
        username,
    };

    // Create network instance.
    let mut body_uri = uri.clone();
    transfer_host_uri(&state, &mut body_uri, username);
    body.data.host_uri = body_uri.to_string();
    let mut builder = client
        .request(reqwest::Method::POST, api_path)
        .header(reqwest::header::AUTHORIZATION, token)
        .json(&body);
    if let Some(content_type) = req.headers().get(header::CONTENT_TYPE) {
        builder = builder.header(reqwest::header::CONTENT_TYPE, content_type);
    }
    let api_req = match builder.build() {
        Err(e) => {
            let _ = clear_queue_rsc(FN_NAME, &state, &clear_rsc);
            let e = format!("generate request error: {}", e);
            error!("[{}] {}", FN_NAME, e);
            return ErrResp::ErrRsc(Some(e)).error_response();
        }
        Ok(req) => req,
    };
    let api_resp = match client.execute(api_req).await {
        Err(e) => {
            let _ = clear_queue_rsc(FN_NAME, &state, &clear_rsc);
            let e = format!("execute request error: {}", e);
            error!("[{}] {}", FN_NAME, e);
            return ErrResp::ErrIntMsg(Some(e)).error_response();
        }
        Ok(resp) => match resp.status() {
            StatusCode::OK => resp,
            _ => return HttpResponseBuilder::new(resp.status()).streaming(resp.bytes_stream()),
        },
    };
    let mut body = match api_resp.json::<response::PostNetwork>().await {
        Err(e) => {
            let _ = clear_queue_rsc(FN_NAME, &state, &clear_rsc);
            let e = format!("unexpected response: {}", e);
            return ErrResp::ErrUnknown(Some(e)).error_response();
        }
        Ok(body) => body,
    };
    body.data.password = Some(password.to_string());

    HttpResponse::Ok().json(&body)
}

/// `GET /{base}/api/v1/network/count`
async fn get_network_count(mut req: HttpRequest, state: web::Data<State>) -> impl Responder {
    const FN_NAME: &'static str = "get_network_count";
    let api_path = format!("{}/api/v1/network/count", state.broker_base.as_str());
    let client = state.client.clone();

    api_bridge(FN_NAME, &client, &mut req, api_path.as_str(), None).await
}

/// `GET /{base}/api/v1/network/list`
async fn get_network_list(mut req: HttpRequest, state: web::Data<State>) -> impl Responder {
    const FN_NAME: &'static str = "get_network_list";
    let api_path = format!("{}/api/v1/network/list", state.broker_base.as_str());
    let api_path = api_path.as_str();
    let client = state.client.clone();

    let mut list_format = ListFormat::Data;
    if req.query_string().len() > 0 {
        let query = match serde_urlencoded::from_str::<Vec<(String, String)>>(req.query_string()) {
            Err(e) => {
                let e = format!("parse query error: {}", e);
                return ErrResp::ErrParam(Some(e)).error_response();
            }
            Ok(query) => query,
        };
        for (k, v) in query.iter() {
            if k.as_str().eq("format") {
                if v.as_str().eq("array") {
                    list_format = ListFormat::Array;
                } else if v.as_str().eq("csv") {
                    list_format = ListFormat::Csv;
                }
            }
        }
    }

    let (api_resp, mut resp) =
        match list_api_bridge(FN_NAME, &client, &mut req, api_path, true, "network").await {
            ListResp::ActixWeb(resp) => return resp,
            ListResp::ArrayStream(api_resp, resp) => (api_resp, resp),
        };

    let mut resp_stream = api_resp.bytes_stream();
    let stream = async_stream::stream! {
        match list_format {
            ListFormat::Array => yield Ok(Bytes::from("[")),
            ListFormat::Csv => yield Ok(Bytes::from(CSV_FIELDS)),
            ListFormat::Data => yield Ok(Bytes::from("{\"data\":[")),
        }
        let mut first_sent = false;

        let mut buffer = BytesMut::new();
        while let Some(body) = resp_stream.next().await {
            match body {
                Err(e) => {
                    error!("[{}] get body error: {}", FN_NAME, e);
                    let err: Box<dyn StdError> = Box::new(e);
                    yield Err(err);
                    break;
                }
                Ok(body) => buffer.extend_from_slice(&body[..]),
            }

            let mut json_stream = Deserializer::from_slice(&buffer[..]).into_iter::<Network>();
            let mut index = 0;
            let mut finish = false;
            loop {
                if let Some(Ok(mut v)) = json_stream.next() {
                    v.host_uri = match Url::parse(v.host_uri.as_str()) {
                        Err(e) => {
                            error!("[{}] parse body hostUri error: {}", FN_NAME, e);
                            yield Err(Box::new(e));
                            finish = true;
                            break;
                        }
                        Ok(uri) => trunc_host_uri(&uri),
                    };
                    match list_format {
                        ListFormat::Array | ListFormat::Data => match serde_json::to_string(&v) {
                            Err(e) =>{
                                error!("[{}] serialize JSON error: {}", FN_NAME, e);
                                let err: Box<dyn StdError> = Box::new(e);
                                yield Err(err);
                                finish = true;
                                break;
                            }
                            Ok(v) => {
                                match first_sent {
                                    false => first_sent = true,
                                    true => yield Ok(Bytes::from(",")),
                                }
                                yield Ok(Bytes::copy_from_slice(v.as_str().as_bytes()));
                            }
                        }
                        ListFormat::Csv => {
                            let mut item = CsvItem{
                                network_id: v.network_id,
                                code: v.code,
                                unit_id: v.unit_id,
                                unit_code: v.unit_code,
                                created_at: v.created_at,
                                modified_at: v.modified_at,
                                host_uri: v.host_uri,
                                name: v.name,
                                info: None,
                            };
                            if let Ok(info_str) = serde_json::to_string(&v.info) {
                                item.info = Some(info_str);
                            }
                            let mut writer =
                                WriterBuilder::new().has_headers(false).from_writer(vec![]);
                            if let Err(e) = writer.serialize(item) {
                                error!("[{}] serialize CSV error: {}", FN_NAME, e);
                                let err: Box<dyn StdError> = Box::new(e);
                                yield Err(err);
                                finish = true;
                                break;
                            }
                            match writer.into_inner() {
                                Err(e) => {
                                    error!("[{}] serialize bytes error: {}", FN_NAME, e);
                                    let err: Box<dyn StdError> = Box::new(e);
                                    yield Err(err);
                                    finish = true;
                                    break;
                                }
                                Ok(row) => yield Ok(Bytes::copy_from_slice(row.as_slice())),
                            }
                        }
                    }
                    continue;
                }
                let offset = json_stream.byte_offset();
                if buffer.len() <= index + offset {
                    index = buffer.len();
                    break;
                }
                match buffer[index+offset] {
                    b'[' | b',' => {
                        index += offset + 1;
                        if buffer.len() <= index {
                            break;
                        }
                        json_stream =
                            Deserializer::from_slice(&buffer[index..]).into_iter::<Network>();
                    }
                    b']' => {
                        finish = true;
                        break;
                    }
                    _ => break,
                }
            }
            if finish {
                match list_format {
                    ListFormat::Array => yield Ok(Bytes::from("]")),
                    ListFormat::Csv => (),
                    ListFormat::Data => yield Ok(Bytes::from("]}")),
                }
                break;
            }
            buffer = buffer.split_off(index);
        }
    };
    resp.streaming(stream)
}

/// `GET /{base}/api/v1/network/{networkId}`
async fn get_network(
    req: HttpRequest,
    param: web::Path<NetworkIdPath>,
    state: web::Data<State>,
) -> impl Responder {
    const FN_NAME: &'static str = "get_network";
    let broker_base = state.broker_base.as_str();
    let client = state.client.clone();
    let token = match req.headers().get(header::AUTHORIZATION) {
        None => {
            let e = "missing Authorization".to_string();
            return ErrResp::ErrParam(Some(e)).error_response();
        }
        Some(value) => value.clone(),
    };

    let (mut network, uri, host) = match get_network_inner(
        FN_NAME,
        &client,
        broker_base,
        param.network_id.as_str(),
        &token,
    )
    .await
    {
        Err(e) => return e,
        Ok((network, uri, host)) => (network, uri, host),
    };

    let host = host.as_str();
    let scheme = uri.scheme();
    if scheme.eq("amqp") || scheme.eq("amqps") {
        let AmqpState::RabbitMq(opts) = &state.amqp;
        let unit_code = match network.unit_code.as_ref() {
            None => "",
            Some(unit_code) => unit_code.as_str(),
        };
        let username = mq::to_username(QueueType::Network, unit_code, network.code.as_str());
        let username = username.as_str();
        match rabbitmq::get_policies(&client, opts, host, username).await {
            Err(e) => {
                error!("[{}] get {} policies error: {}", FN_NAME, username, e);
                return e.error_response();
            }
            Ok(policies) => {
                network.ttl = policies.ttl;
                network.length = policies.length;
            }
        }
    }
    network.host_uri = trunc_host_uri(&uri);
    HttpResponse::Ok().json(&response::GetNetwork { data: network })
}

/// `PATCH /{base}/api/v1/network/{networkId}`
async fn patch_network(
    req: HttpRequest,
    param: web::Path<NetworkIdPath>,
    body: web::Json<request::PatchNetworkBody>,
    state: web::Data<State>,
) -> impl Responder {
    const FN_NAME: &'static str = "patch_network";
    let broker_base = state.broker_base.as_str();
    let client = state.client.clone();
    let token = match req.headers().get(header::AUTHORIZATION) {
        None => {
            let e = "missing Authorization".to_string();
            return ErrResp::ErrParam(Some(e)).error_response();
        }
        Some(value) => value.clone(),
    };

    let data = &body.data;
    if data.host_uri.is_none()
        && data.name.is_none()
        && data.info.is_none()
        && data.ttl.is_none()
        && data.length.is_none()
        && data.password.is_none()
    {
        return ErrResp::ErrParam(Some("at least one parameter".to_string())).error_response();
    }

    let (network, uri, hostname) = match get_network_inner(
        FN_NAME,
        &client,
        broker_base,
        param.network_id.as_str(),
        &token,
    )
    .await
    {
        Err(e) => return e,
        Ok((network, uri, hostname)) => (network, uri, hostname),
    };

    let mut patch_data = request::PatchNetworkData {
        name: data.name.clone(),
        info: data.info.clone(),
        ..Default::default()
    };
    let mut patch_host: Option<PatchHost> = None;
    if let Some(host) = data.host_uri.as_ref() {
        if !strings::is_uri(host) {
            return ErrResp::ErrParam(Some("invalid `hostUri`".to_string())).error_response();
        }
        // Change to the new broker host.
        if !cmp_host_uri(network.host_uri.as_str(), host.as_str()) {
            let password = match data.password.as_ref() {
                None => {
                    let e = "missing `password`".to_string();
                    return ErrResp::ErrParam(Some(e)).error_response();
                }
                Some(password) => match password.len() {
                    0 => {
                        let e = "missing `password`".to_string();
                        return ErrResp::ErrParam(Some(e)).error_response();
                    }
                    _ => password,
                },
            };
            let mut new_host_uri = match Url::parse(host.as_str()) {
                Err(e) => {
                    let e = format!("invalid `hostUri`: {}", e);
                    return ErrResp::ErrParam(Some(e)).error_response();
                }
                Ok(uri) => match uri.host_str() {
                    None => {
                        let e = "invalid `hostUri`".to_string();
                        return ErrResp::ErrParam(Some(e)).error_response();
                    }
                    Some(_) => uri,
                },
            };

            let unit_code = match network.unit_code.as_ref() {
                None => "",
                Some(unit_code) => unit_code.as_str(),
            };
            let code = network.code.as_str();
            let username = mq::to_username(QueueType::Network, unit_code, code);
            let resource = CreateQueueResource {
                scheme: new_host_uri.scheme(),
                host: new_host_uri.host_str().unwrap(),
                username: username.as_str(),
                password: password.as_str(),
                ttl: data.ttl,
                length: data.length,
                q_type: QueueType::Network,
            };
            if let Err(e) = create_queue_rsc(FN_NAME, &state, &resource).await {
                return e;
            }
            let resource = ClearQueueResource {
                scheme: uri.scheme(),
                host: uri.host_str().unwrap(),
                username: username.as_str(),
            };
            let _ = clear_queue_rsc(FN_NAME, &state, &resource).await;

            transfer_host_uri(&state, &mut new_host_uri, username.as_str());
            patch_data.host_uri = Some(new_host_uri.to_string());
            patch_host = Some(PatchHost {
                host_uri: new_host_uri,
                username,
            });
        }
    }

    // Send request body to the sylvia-iot-broker.
    if patch_data.host_uri.is_some() || patch_data.name.is_some() || patch_data.info.is_some() {
        let network_id = param.network_id.as_str();
        let uri = format!("{}/api/v1/network/{}", broker_base, network_id);
        let mut builder = client
            .request(reqwest::Method::PATCH, uri)
            .header(reqwest::header::AUTHORIZATION, &token)
            .json(&request::PatchNetworkBody { data: patch_data });
        if let Some(content_type) = req.headers().get(header::CONTENT_TYPE) {
            builder = builder.header(reqwest::header::CONTENT_TYPE, content_type);
        }
        let api_req = match builder.build() {
            Err(e) => {
                clear_patch_host(FN_NAME, &state, &patch_host).await;
                let e = format!("generate request error: {}", e);
                error!("[{}] {}", FN_NAME, e);
                return ErrResp::ErrRsc(Some(e)).error_response();
            }
            Ok(req) => req,
        };
        let api_resp = match client.execute(api_req).await {
            Err(e) => {
                clear_patch_host(FN_NAME, &state, &patch_host).await;
                let e = format!("execute request error: {}", e);
                error!("[{}] {}", FN_NAME, e);
                return ErrResp::ErrIntMsg(Some(e)).error_response();
            }
            Ok(resp) => resp,
        };

        let status_code = api_resp.status();
        if status_code != StatusCode::NO_CONTENT {
            clear_patch_host(FN_NAME, &state, &patch_host).await;
            let mut resp = HttpResponseBuilder::new(status_code);
            if let Some(content_type) = api_resp.headers().get(header::CONTENT_TYPE) {
                resp.insert_header((header::CONTENT_TYPE, content_type.clone()));
            }
            if let Some(auth) = api_resp.headers().get(header::WWW_AUTHENTICATE) {
                resp.insert_header((header::WWW_AUTHENTICATE, auth.clone()));
            }
            return resp.streaming(api_resp.bytes_stream());
        }
    }

    if let Some(host) = patch_host {
        let resource = ClearQueueResource {
            scheme: uri.scheme(),
            host: uri.host_str().unwrap(),
            username: host.username.as_str(),
        };
        let _ = clear_queue_rsc(FN_NAME, &state, &resource).await;
        return HttpResponse::NoContent().finish();
    } else if data.ttl.is_none() && data.length.is_none() && data.password.is_none() {
        return HttpResponse::NoContent().finish();
    }

    // Update broker information without changing hostUri.
    if let Some(password) = data.password.as_ref() {
        if password.len() == 0 {
            let e = "missing `password`".to_string();
            return ErrResp::ErrParam(Some(e)).error_response();
        }
    }
    let unit_code = match network.unit_code.as_ref() {
        None => "",
        Some(unit_code) => unit_code.as_str(),
    };
    let code = network.code.as_str();
    let hostname = hostname.as_str();
    let username = mq::to_username(QueueType::Network, unit_code, code);
    let username = username.as_str();
    match uri.scheme() {
        "amqp" | "amqps" => match &state.amqp {
            AmqpState::RabbitMq(opts) => {
                if data.ttl.is_some() || data.length.is_some() {
                    let policies = rabbitmq::BrokerPolicies {
                        ttl: data.ttl,
                        length: data.length,
                    };
                    if let Err(e) =
                        rabbitmq::put_policies(&client, opts, hostname, username, &policies).await
                    {
                        let e = format!("patch RabbitMQ error: {}", e);
                        error!("[{}] {}", FN_NAME, e);
                        return ErrResp::ErrIntMsg(Some(e)).error_response();
                    }
                }
                if let Some(password) = data.password.as_ref() {
                    let password = password.as_str();
                    if let Err(e) =
                        rabbitmq::put_user(&client, opts, hostname, username, password).await
                    {
                        let e = format!("patch RabbitMQ password error: {}", e);
                        error!("[{}] {}", FN_NAME, e);
                        return ErrResp::ErrIntMsg(Some(e)).error_response();
                    }
                }
            }
        },
        "mqtt" | "mqtts" => match &state.mqtt {
            MqttState::Emqx(opts) => {
                if let Some(password) = data.password.as_ref() {
                    let password = password.as_str();
                    if let Err(e) =
                        emqx::put_user(&client, opts, hostname, username, password).await
                    {
                        let e = format!("patch EMQX password error: {}", e);
                        error!("[{}] {}", FN_NAME, e);
                        return ErrResp::ErrIntMsg(Some(e)).error_response();
                    }
                }
            }
            MqttState::Rumqttd => {}
        },
        _ => {}
    }

    HttpResponse::NoContent().finish()
}

/// `DELETE /{base}/api/v1/network/{networkId}`
async fn delete_network(
    mut req: HttpRequest,
    param: web::Path<NetworkIdPath>,
    state: web::Data<State>,
) -> impl Responder {
    const FN_NAME: &'static str = "delete_network";
    let broker_base = state.broker_base.as_str();
    let network_id = param.network_id.as_str();
    let api_path = format!("{}/api/v1/network/{}", broker_base, network_id);
    let client = state.client.clone();
    let token = match req.headers().get(header::AUTHORIZATION) {
        None => {
            let e = "missing Authorization".to_string();
            return ErrResp::ErrParam(Some(e)).error_response();
        }
        Some(value) => value.clone(),
    };

    let (network, uri, host) =
        match get_network_inner(FN_NAME, &client, broker_base, network_id, &token).await {
            Err(e) => return e,
            Ok((network, uri, host)) => (network, uri, host),
        };

    let resp = api_bridge(FN_NAME, &client, &mut req, api_path.as_str(), None).await;
    if !resp.status().is_success() {
        return resp;
    }

    let unit_code = match network.unit_code.as_ref() {
        None => "",
        Some(unit_code) => unit_code.as_str(),
    };
    let code = network.code.as_str();
    let username = mq::to_username(QueueType::Network, unit_code, code);
    let clear_rsc = ClearQueueResource {
        scheme: uri.scheme(),
        host: host.as_str(),
        username: username.as_str(),
    };
    if let Err(e) = clear_queue_rsc(FN_NAME, &state, &clear_rsc).await {
        return e;
    }

    HttpResponse::NoContent().finish()
}

/// `GET /{base}/api/v1/network/{networkId}/stats`
async fn get_network_stats(
    req: HttpRequest,
    param: web::Path<NetworkIdPath>,
    state: web::Data<State>,
) -> impl Responder {
    const FN_NAME: &'static str = "get_network";
    let broker_base = state.broker_base.as_str();
    let client = state.client.clone();
    let token = match req.headers().get(header::AUTHORIZATION) {
        None => {
            let e = "missing Authorization".to_string();
            return ErrResp::ErrParam(Some(e)).error_response();
        }
        Some(value) => value.clone(),
    };

    let (network, uri, host) = match get_network_inner(
        FN_NAME,
        &client,
        broker_base,
        param.network_id.as_str(),
        &token,
    )
    .await
    {
        Err(e) => return e,
        Ok((network, uri, host)) => (network, uri, host),
    };

    let host = host.as_str();
    let scheme = uri.scheme();
    let data = match scheme {
        "amqp" | "amqps" => {
            let AmqpState::RabbitMq(opts) = &state.amqp;
            let unit_code = match network.unit_code.as_ref() {
                None => "",
                Some(unit_code) => unit_code.as_str(),
            };
            let username = mq::to_username(QueueType::Network, unit_code, network.code.as_str());
            let username = username.as_str();
            response::GetNetworkStatsData {
                dldata: match rabbitmq::stats(&client, opts, host, username, "dldata").await {
                    Err(ErrResp::ErrNotFound(_)) => response::Stats {
                        consumers: 0,
                        messages: 0,
                        publish_rate: 0.0,
                        deliver_rate: 0.0,
                    },
                    Err(e) => {
                        error!("[{}] get dldata stats error: {}", FN_NAME, e);
                        return e.error_response();
                    }
                    Ok(stats) => response::Stats {
                        consumers: stats.consumers,
                        messages: stats.messages,
                        publish_rate: stats.publish_rate,
                        deliver_rate: stats.deliver_rate,
                    },
                },
            }
        }
        "mqtt" | "mqtts" => match &state.mqtt {
            MqttState::Emqx(opts) => {
                let unit_code = match network.unit_code.as_ref() {
                    None => "",
                    Some(unit_code) => unit_code.as_str(),
                };
                let username =
                    mq::to_username(QueueType::Network, unit_code, network.code.as_str());
                let username = username.as_str();
                response::GetNetworkStatsData {
                    dldata: match emqx::stats(&client, opts, host, username, "dldata").await {
                        Err(e) => {
                            error!("[{}] get dldata stats error: {}", FN_NAME, e);
                            return e.error_response();
                        }
                        Ok(stats) => response::Stats {
                            consumers: stats.consumers,
                            messages: stats.messages,
                            publish_rate: stats.publish_rate,
                            deliver_rate: stats.deliver_rate,
                        },
                    },
                }
            }
            MqttState::Rumqttd => response::GetNetworkStatsData {
                dldata: response::Stats {
                    ..Default::default()
                },
            },
        },
        _ => {
            let e = format!("unsupport scheme {}", scheme);
            error!("[{}] {}", FN_NAME, e);
            return ErrResp::ErrUnknown(Some(e)).error_response();
        }
    };
    HttpResponse::Ok().json(&response::GetNetworkStats { data })
}

/// `POST /{base}/api/v1/network/{networkId}/uldata`
async fn post_network_uldata(
    req: HttpRequest,
    param: web::Path<NetworkIdPath>,
    body: web::Json<request::PostNetworkUlDataBody>,
    state: web::Data<State>,
) -> impl Responder {
    const FN_NAME: &'static str = "post_network_uldata";
    let broker_base = state.broker_base.as_str();
    let client = state.client.clone();
    let token = match req.headers().get(header::AUTHORIZATION) {
        None => {
            let e = "missing Authorization".to_string();
            return ErrResp::ErrParam(Some(e)).error_response();
        }
        Some(value) => value.clone(),
    };

    if body.data.device_id.len() == 0 {
        let e = "empty `deviceId` is invalid".to_string();
        return ErrResp::ErrParam(Some(e)).error_response();
    }
    if let Err(e) = hex::decode(body.data.payload.as_str()) {
        let e = format!("`payload` is not hexadecimal string: {}", e);
        return ErrResp::ErrParam(Some(e)).error_response();
    }

    let (network, uri, hostname) = match get_network_inner(
        FN_NAME,
        &client,
        broker_base,
        param.network_id.as_str(),
        &token,
    )
    .await
    {
        Err(e) => return e,
        Ok((network, uri, hostname)) => (network, uri, hostname),
    };
    let device = match get_device_inner(
        FN_NAME,
        &client,
        broker_base,
        body.data.device_id.as_str(),
        &token,
    )
    .await
    {
        Err(e) => return e,
        Ok(device) => match device {
            None => {
                return ErrResp::Custom(
                    ErrReq::DEVICE_NOT_EXIST.0,
                    ErrReq::DEVICE_NOT_EXIST.1,
                    None,
                )
                .error_response()
            }
            Some(device) => device,
        },
    };

    let hostname = hostname.as_str();
    let scheme = uri.scheme();
    let payload = match serde_json::to_string(&UlData {
        time: strings::time_str(&Utc::now()),
        network_addr: device.network_addr,
        data: body.data.payload.clone(),
        ..Default::default()
    }) {
        Err(e) => {
            let e = format!("encode JSON error: {}", e);
            error!("[{}] {}", FN_NAME, e);
            return ErrResp::ErrRsc(Some(e)).error_response();
        }
        Ok(payload) => general_purpose::STANDARD.encode(payload),
    };
    match scheme {
        "amqp" | "amqps" => {
            let AmqpState::RabbitMq(opts) = &state.amqp;
            let unit_code = match network.unit_code.as_ref() {
                None => "",
                Some(unit_code) => unit_code.as_str(),
            };
            let username = mq::to_username(QueueType::Network, unit_code, network.code.as_str());
            let username = username.as_str();
            if let Err(e) =
                rabbitmq::publish_message(&client, opts, hostname, username, "uldata", payload)
                    .await
            {
                return e.error_response();
            }
        }
        "mqtt" | "mqtts" => match &state.mqtt {
            MqttState::Emqx(opts) => {
                let unit_code = match network.unit_code.as_ref() {
                    None => "",
                    Some(unit_code) => unit_code.as_str(),
                };
                let username =
                    mq::to_username(QueueType::Network, unit_code, network.code.as_str());
                let username = username.as_str();
                if let Err(e) =
                    emqx::publish_message(&client, opts, hostname, username, "uldata", payload)
                        .await
                {
                    return e.error_response();
                }
            }
            MqttState::Rumqttd => {
                let e = "not support now".to_string();
                return ErrResp::ErrParam(Some(e)).error_response();
            }
        },
        _ => {
            let e = format!("unsupport scheme {}", scheme);
            error!("[{}] {}", FN_NAME, e);
            return ErrResp::ErrUnknown(Some(e)).error_response();
        }
    }
    HttpResponse::NoContent().finish()
}

async fn get_network_inner(
    fn_name: &str,
    client: &reqwest::Client,
    broker_base: &str,
    network_id: &str,
    token: &HeaderValue,
) -> Result<(response::GetNetworkData, Url, String), HttpResponse> {
    let uri = format!("{}/api/v1/network/{}", broker_base, network_id);
    let resp = get_stream_resp(fn_name, token, &client, uri.as_str()).await?;

    let network = match resp.json::<response::GetNetwork>().await {
        Err(e) => {
            let e = format!("wrong response of network: {}", e);
            error!("[{}] {}", fn_name, e);
            return Err(ErrResp::ErrIntMsg(Some(e)).error_response());
        }
        Ok(network) => network.data,
    };
    let uri = match Url::parse(network.host_uri.as_str()) {
        Err(e) => {
            let e = format!("unexpected hostUri: {}", e);
            error!("[{}] {}", fn_name, e);
            return Err(ErrResp::ErrUnknown(Some(e)).error_response());
        }
        Ok(uri) => uri,
    };
    let host = match uri.host_str() {
        None => {
            let e = "unexpected hostUri".to_string();
            error!("[{}] {}", fn_name, e);
            return Err(ErrResp::ErrUnknown(Some(e)).error_response());
        }
        Some(host) => host.to_string(),
    };
    Ok((network, uri, host))
}
