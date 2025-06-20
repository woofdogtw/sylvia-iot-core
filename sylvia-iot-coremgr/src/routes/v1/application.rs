use std::error::Error as StdError;

use axum::{
    Router,
    body::Body,
    extract::{Request, State},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
    routing,
};
use base64::{Engine, engine::general_purpose};
use bytes::{Bytes, BytesMut};
use csv::WriterBuilder;
use futures_util::StreamExt;
use hex;
use log::error;
use reqwest;
use serde::{Deserialize, Serialize};
use serde_json::{Deserializer, Map, Value};
use url::Url;

use sylvia_iot_corelib::{
    err::ErrResp,
    http::{Json, Path},
    strings,
};

use super::{
    super::{AmqpState, ErrReq, MqttState, State as AppState},
    ClearQueueResource, CreateQueueResource, ListResp, PatchHost, api_bridge, clear_patch_host,
    clear_queue_rsc, cmp_host_uri, create_queue_rsc, get_device_inner, get_stream_resp,
    get_unit_inner, list_api_bridge, request, response, transfer_host_uri, trunc_host_uri,
};
use crate::libs::mq::{self, QueueType, emqx, rabbitmq};

enum ListFormat {
    Array,
    Csv,
    Data,
}

#[derive(Deserialize)]
struct ApplicationIdPath {
    application_id: String,
}

#[derive(Deserialize, Serialize)]
struct Application {
    #[serde(rename = "applicationId")]
    application_id: String,
    code: String,
    #[serde(rename = "unitId")]
    unit_id: String,
    #[serde(rename = "unitCode")]
    unit_code: String,
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
    #[serde(rename = "applicationId")]
    application_id: String,
    code: String,
    #[serde(rename = "unitId")]
    unit_id: String,
    #[serde(rename = "unitCode")]
    unit_code: String,
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
struct DlData {
    #[serde(rename = "correlationId")]
    correlation_id: String,
    #[serde(rename = "deviceId")]
    device_id: Option<String>,
    #[serde(rename = "networkCode")]
    network_code: Option<String>,
    #[serde(rename = "networkAddr")]
    network_addr: Option<String>,
    data: String,
    extension: Option<Map<String, Value>>,
}

const CSV_FIELDS: &'static [u8] =
    b"\xEF\xBB\xBFapplicationId,code,unitId,unitCode,createdAt,modifiedAt,hostUri,name,info\n";

pub fn new_service(scope_path: &str, state: &AppState) -> Router {
    Router::new().nest(
        scope_path,
        Router::new()
            .route("/", routing::post(post_application))
            .route("/count", routing::get(get_application_count))
            .route("/list", routing::get(get_application_list))
            .route(
                "/{application_id}",
                routing::get(get_application)
                    .patch(patch_application)
                    .delete(delete_application),
            )
            .route(
                "/{application_id}/stats",
                routing::get(get_application_stats),
            )
            .route(
                "/{application_id}/dldata",
                routing::post(post_application_dldata),
            )
            .with_state(state.clone()),
    )
}

/// `POST /{base}/api/v1/application`
async fn post_application(
    State(state): State<AppState>,
    mut headers: HeaderMap,
    Json(mut body): Json<request::PostApplicationBody>,
) -> impl IntoResponse {
    const FN_NAME: &'static str = "post_application";
    let broker_base = state.broker_base.as_str();
    let api_path = format!("{}/api/v1/application", broker_base);
    let client = state.client.clone();
    let token = match headers.get(header::AUTHORIZATION) {
        None => {
            let e = "missing Authorization".to_string();
            return ErrResp::ErrParam(Some(e)).into_response();
        }
        Some(value) => value.clone(),
    };

    // Get unit information to create queue information.
    let unit_id = body.data.unit_id.as_str();
    if unit_id.len() == 0 {
        return ErrResp::ErrParam(Some(
            "`unitId` must with at least one character".to_string(),
        ))
        .into_response();
    }
    let unit = match get_unit_inner(FN_NAME, &client, broker_base, unit_id, &token).await {
        Err(e) => return e,
        Ok(unit) => match unit {
            None => {
                return ErrResp::Custom(ErrReq::UNIT_NOT_EXIST.0, ErrReq::UNIT_NOT_EXIST.1, None)
                    .into_response();
            }
            Some(unit) => unit,
        },
    };
    let unit_code = unit.code.as_str();
    let code = body.data.code.as_str();
    if !strings::is_code(code) {
        return ErrResp::ErrParam(Some(
            "`code` must be [A-Za-z0-9]{1}[A-Za-z0-9-_]*".to_string(),
        ))
        .into_response();
    }
    match check_application_code_inner(FN_NAME, &client, broker_base, unit_id, code, &token).await {
        Err(e) => return e,
        Ok(count) => match count {
            0 => (),
            _ => {
                return ErrResp::Custom(
                    ErrReq::APPLICATION_EXIST.0,
                    ErrReq::APPLICATION_EXIST.1,
                    None,
                )
                .into_response();
            }
        },
    }
    let q_type = QueueType::Application;
    let username = mq::to_username(q_type, unit_code, code);
    let password = strings::randomstring(8);
    let uri = match Url::parse(body.data.host_uri.as_str()) {
        Err(e) => {
            return ErrResp::ErrParam(Some(format!("invalid `hostUri`: {}", e))).into_response();
        }
        Ok(uri) => uri,
    };
    let host = match uri.host() {
        None => {
            let e = "invalid `hostUri`".to_string();
            return ErrResp::ErrParam(Some(e)).into_response();
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
        q_type: QueueType::Application,
    };
    if let Err(e) = create_queue_rsc(FN_NAME, &state, &create_rsc).await {
        return e;
    }
    let clear_rsc = ClearQueueResource {
        scheme,
        host,
        username,
    };

    // Create application instance.
    let mut body_uri = uri.clone();
    transfer_host_uri(&state, &mut body_uri, username);
    body.data.host_uri = body_uri.to_string();
    headers.remove(header::CONTENT_LENGTH);
    let builder = client
        .request(reqwest::Method::POST, api_path)
        .headers(headers)
        .json(&body);
    let api_req = match builder.build() {
        Err(e) => {
            let _ = clear_queue_rsc(FN_NAME, &state, &clear_rsc);
            let e = format!("generate request error: {}", e);
            error!("[{}] {}", FN_NAME, e);
            return ErrResp::ErrRsc(Some(e)).into_response();
        }
        Ok(req) => req,
    };
    let api_resp = match client.execute(api_req).await {
        Err(e) => {
            let _ = clear_queue_rsc(FN_NAME, &state, &clear_rsc);
            let e = format!("execute request error: {}", e);
            error!("[{}] {}", FN_NAME, e);
            return ErrResp::ErrIntMsg(Some(e)).into_response();
        }
        Ok(resp) => match resp.status() {
            reqwest::StatusCode::OK => resp,
            _ => {
                let mut resp_builder = Response::builder().status(resp.status());
                for (k, v) in resp.headers() {
                    resp_builder = resp_builder.header(k, v);
                }
                match resp_builder.body(Body::from_stream(resp.bytes_stream())) {
                    Err(e) => {
                        let e = format!("wrap response body error: {}", e);
                        error!("[{}] {}", FN_NAME, e);
                        return ErrResp::ErrIntMsg(Some(e)).into_response();
                    }
                    Ok(resp) => return resp,
                }
            }
        },
    };
    let mut body = match api_resp.json::<response::PostApplication>().await {
        Err(e) => {
            let _ = clear_queue_rsc(FN_NAME, &state, &clear_rsc);
            let e = format!("unexpected response: {}", e);
            return ErrResp::ErrUnknown(Some(e)).into_response();
        }
        Ok(body) => body,
    };
    body.data.password = Some(password.to_string());

    Json(&body).into_response()
}

/// `GET /{base}/api/v1/application/count`
async fn get_application_count(state: State<AppState>, req: Request) -> impl IntoResponse {
    const FN_NAME: &'static str = "get_application_count";
    let api_path = format!("{}/api/v1/application/count", state.broker_base.as_str());
    let client = state.client.clone();

    api_bridge(FN_NAME, &client, req, api_path.as_str()).await
}

/// `GET /{base}/api/v1/application/list`
async fn get_application_list(state: State<AppState>, req: Request) -> impl IntoResponse {
    const FN_NAME: &'static str = "get_application_list";
    let api_path = format!("{}/api/v1/application/list", state.broker_base.as_str());
    let api_path = api_path.as_str();
    let client = state.client.clone();

    let mut list_format = ListFormat::Data;
    if let Some(query_str) = req.uri().query() {
        let query = match serde_urlencoded::from_str::<Vec<(String, String)>>(query_str) {
            Err(e) => {
                let e = format!("parse query error: {}", e);
                return ErrResp::ErrParam(Some(e)).into_response();
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

    let (api_resp, resp_builder) =
        match list_api_bridge(FN_NAME, &client, req, api_path, true, "application").await {
            ListResp::Axum(resp) => return resp,
            ListResp::ArrayStream(api_resp, resp_builder) => (api_resp, resp_builder),
        };

    let mut resp_stream = api_resp.bytes_stream();
    let body = Body::from_stream(async_stream::stream! {
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
                    let err: Box<dyn StdError + Send + Sync> = Box::new(e);
                    yield Err(err);
                    break;
                }
                Ok(body) => buffer.extend_from_slice(&body[..]),
            }

            let mut json_stream = Deserializer::from_slice(&buffer[..]).into_iter::<Application>();
            let mut index = 0;
            let mut finish = false;
            loop {
                if let Some(Ok(mut v)) = json_stream.next() {
                    v.host_uri = match Url::parse(v.host_uri.as_str()) {
                        Err(e) => {
                            error!("[{}] parse body hostUri error: {}", FN_NAME, e);
                            let err: Box<dyn StdError + Send + Sync> = Box::new(e);
                            yield Err(err);
                            finish = true;
                            break;
                        }
                        Ok(uri) => trunc_host_uri(&uri),
                    };
                    match list_format {
                        ListFormat::Array | ListFormat::Data => match serde_json::to_string(&v) {
                            Err(e) =>{
                                error!("[{}] serialize JSON error: {}", FN_NAME, e);
                                let err: Box<dyn StdError + Send + Sync> = Box::new(e);
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
                                application_id: v.application_id,
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
                                let err: Box<dyn StdError + Send + Sync> = Box::new(e);
                                yield Err(err);
                                finish = true;
                                break;
                            }
                            match writer.into_inner() {
                                Err(e) => {
                                    error!("[{}] serialize bytes error: {}", FN_NAME, e);
                                    let err: Box<dyn StdError + Send + Sync> = Box::new(e);
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
                            Deserializer::from_slice(&buffer[index..]).into_iter::<Application>();
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
    });
    match resp_builder.body(body) {
        Err(e) => ErrResp::ErrRsc(Some(e.to_string())).into_response(),
        Ok(resp) => resp,
    }
}

/// `GET /{base}/api/v1/application/{applicationId}`
async fn get_application(
    state: State<AppState>,
    Path(param): Path<ApplicationIdPath>,
    req: Request,
) -> impl IntoResponse {
    const FN_NAME: &'static str = "get_application";
    let broker_base = state.broker_base.as_str();
    let client = state.client.clone();
    let token = match req.headers().get(header::AUTHORIZATION) {
        None => {
            let e = "missing Authorization".to_string();
            return ErrResp::ErrParam(Some(e)).into_response();
        }
        Some(value) => value.clone(),
    };

    let (mut application, uri, host) = match get_application_inner(
        FN_NAME,
        &client,
        broker_base,
        param.application_id.as_str(),
        &token,
    )
    .await
    {
        Err(e) => return e,
        Ok((application, uri, host)) => (application, uri, host),
    };

    let host = host.as_str();
    let scheme = uri.scheme();
    if scheme.eq("amqp") || scheme.eq("amqps") {
        let AmqpState::RabbitMq(opts) = &state.amqp;
        let username = mq::to_username(
            QueueType::Application,
            application.unit_code.as_str(),
            application.code.as_str(),
        );
        let username = username.as_str();
        match rabbitmq::get_policies(&client, opts, host, username).await {
            Err(e) => {
                error!("[{}] get {} policies error: {}", FN_NAME, username, e);
                return e.into_response();
            }
            Ok(policies) => {
                application.ttl = policies.ttl;
                application.length = policies.length;
            }
        }
    }
    application.host_uri = trunc_host_uri(&uri);
    Json(&response::GetApplication { data: application }).into_response()
}

/// `PATCH /{base}/api/v1/application/{applicationId}`
async fn patch_application(
    state: State<AppState>,
    headers: HeaderMap,
    Path(param): Path<ApplicationIdPath>,
    Json(body): Json<request::PatchApplicationBody>,
) -> impl IntoResponse {
    const FN_NAME: &'static str = "patch_application";
    let broker_base = state.broker_base.as_str();
    let client = state.client.clone();
    let token = match headers.get(header::AUTHORIZATION) {
        None => {
            let e = "missing Authorization".to_string();
            return ErrResp::ErrParam(Some(e)).into_response();
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
        return ErrResp::ErrParam(Some("at least one parameter".to_string())).into_response();
    }

    let (application, uri, hostname) = match get_application_inner(
        FN_NAME,
        &client,
        broker_base,
        param.application_id.as_str(),
        &token,
    )
    .await
    {
        Err(e) => return e,
        Ok((application, uri, hostname)) => (application, uri, hostname),
    };

    let mut patch_data = request::PatchApplicationData {
        name: data.name.clone(),
        info: data.info.clone(),
        ..Default::default()
    };
    let mut patch_host: Option<PatchHost> = None;
    if let Some(host) = data.host_uri.as_ref() {
        if !strings::is_uri(host) {
            return ErrResp::ErrParam(Some("invalid `hostUri`".to_string())).into_response();
        }
        // Change to the new broker host.
        if !cmp_host_uri(application.host_uri.as_str(), host.as_str()) {
            let password = match data.password.as_ref() {
                None => {
                    let e = "missing `password`".to_string();
                    return ErrResp::ErrParam(Some(e)).into_response();
                }
                Some(password) => match password.len() {
                    0 => {
                        let e = "missing `password`".to_string();
                        return ErrResp::ErrParam(Some(e)).into_response();
                    }
                    _ => password,
                },
            };
            let mut new_host_uri = match Url::parse(host.as_str()) {
                Err(e) => {
                    let e = format!("invalid `hostUri`: {}", e);
                    return ErrResp::ErrParam(Some(e)).into_response();
                }
                Ok(uri) => match uri.host_str() {
                    None => {
                        let e = "invalid `hostUri`".to_string();
                        return ErrResp::ErrParam(Some(e)).into_response();
                    }
                    Some(_) => uri,
                },
            };

            let unit_code = application.unit_code.as_str();
            let code = application.code.as_str();
            let username = mq::to_username(QueueType::Application, unit_code, code);
            let resource = CreateQueueResource {
                scheme: new_host_uri.scheme(),
                host: new_host_uri.host_str().unwrap(),
                username: username.as_str(),
                password: password.as_str(),
                ttl: data.ttl,
                length: data.length,
                q_type: QueueType::Application,
            };
            if let Err(e) = create_queue_rsc(FN_NAME, &state, &resource).await {
                return e;
            }

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
        let application_id = param.application_id.as_str();
        let uri = format!("{}/api/v1/application/{}", broker_base, application_id);
        let mut builder = client
            .request(reqwest::Method::PATCH, uri)
            .header(reqwest::header::AUTHORIZATION, &token)
            .json(&request::PatchApplicationBody { data: patch_data });
        if let Some(content_type) = headers.get(header::CONTENT_TYPE) {
            builder = builder.header(reqwest::header::CONTENT_TYPE, content_type);
        }
        let api_req = match builder.build() {
            Err(e) => {
                clear_patch_host(FN_NAME, &state, &patch_host).await;
                let e = format!("generate request error: {}", e);
                error!("[{}] {}", FN_NAME, e);
                return ErrResp::ErrRsc(Some(e)).into_response();
            }
            Ok(req) => req,
        };
        let api_resp = match client.execute(api_req).await {
            Err(e) => {
                clear_patch_host(FN_NAME, &state, &patch_host).await;
                let e = format!("execute request error: {}", e);
                error!("[{}] {}", FN_NAME, e);
                return ErrResp::ErrIntMsg(Some(e)).into_response();
            }
            Ok(resp) => resp,
        };

        let status_code = api_resp.status();
        if status_code != StatusCode::NO_CONTENT {
            clear_patch_host(FN_NAME, &state, &patch_host).await;
            let mut resp_builder = Response::builder().status(status_code);
            for (k, v) in api_resp.headers() {
                resp_builder = resp_builder.header(k, v);
            }
            match resp_builder.body(Body::from_stream(api_resp.bytes_stream())) {
                Err(e) => {
                    let e = format!("wrap response body error: {}", e);
                    error!("[{}] {}", FN_NAME, e);
                    return ErrResp::ErrIntMsg(Some(e)).into_response();
                }
                Ok(resp) => return resp,
            }
        }
    }

    if let Some(host) = patch_host {
        let resource = ClearQueueResource {
            scheme: uri.scheme(),
            host: uri.host_str().unwrap(),
            username: host.username.as_str(),
        };
        let _ = clear_queue_rsc(FN_NAME, &state, &resource).await;
        return StatusCode::NO_CONTENT.into_response();
    } else if data.ttl.is_none() && data.length.is_none() && data.password.is_none() {
        return StatusCode::NO_CONTENT.into_response();
    }

    // Update broker information without changing hostUri.
    if let Some(password) = data.password.as_ref() {
        if password.len() == 0 {
            let e = "missing `password`".to_string();
            return ErrResp::ErrParam(Some(e)).into_response();
        }
    }
    let unit_code = application.unit_code.as_str();
    let code = application.code.as_str();
    let hostname = hostname.as_str();
    let username = mq::to_username(QueueType::Application, unit_code, code);
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
                        return ErrResp::ErrIntMsg(Some(e)).into_response();
                    }
                }
                if let Some(password) = data.password.as_ref() {
                    let password = password.as_str();
                    if let Err(e) =
                        rabbitmq::put_user(&client, opts, hostname, username, password).await
                    {
                        let e = format!("patch RabbitMQ password error: {}", e);
                        error!("[{}] {}", FN_NAME, e);
                        return ErrResp::ErrIntMsg(Some(e)).into_response();
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
                        return ErrResp::ErrIntMsg(Some(e)).into_response();
                    }
                }
            }
            MqttState::Rumqttd => {}
        },
        _ => {}
    }

    StatusCode::NO_CONTENT.into_response()
}

/// `DELETE /{base}/api/v1/application/{applicationId}`
async fn delete_application(
    state: State<AppState>,
    Path(param): Path<ApplicationIdPath>,
    req: Request,
) -> impl IntoResponse {
    const FN_NAME: &'static str = "delete_application";
    let broker_base = state.broker_base.as_str();
    let application_id = param.application_id.as_str();
    let api_path = format!("{}/api/v1/application/{}", broker_base, application_id);
    let client = state.client.clone();
    let token = match req.headers().get(header::AUTHORIZATION) {
        None => {
            let e = "missing Authorization".to_string();
            return ErrResp::ErrParam(Some(e)).into_response();
        }
        Some(value) => value.clone(),
    };

    let (application, uri, host) =
        match get_application_inner(FN_NAME, &client, broker_base, application_id, &token).await {
            Err(e) => return e,
            Ok((application, uri, host)) => (application, uri, host),
        };

    let resp = api_bridge(FN_NAME, &client, req, api_path.as_str()).await;
    if !resp.status().is_success() {
        return resp;
    }

    let unit_code = application.unit_code.as_str();
    let code = application.code.as_str();
    let username = mq::to_username(QueueType::Application, unit_code, code);
    let clear_rsc = ClearQueueResource {
        scheme: uri.scheme(),
        host: host.as_str(),
        username: username.as_str(),
    };
    if let Err(e) = clear_queue_rsc(FN_NAME, &state, &clear_rsc).await {
        return e;
    }

    StatusCode::NO_CONTENT.into_response()
}

/// `GET /{base}/api/v1/application/{applicationId}/stats`
async fn get_application_stats(
    state: State<AppState>,
    Path(param): Path<ApplicationIdPath>,
    req: Request,
) -> impl IntoResponse {
    const FN_NAME: &'static str = "get_application";
    let broker_base = state.broker_base.as_str();
    let client = state.client.clone();
    let token = match req.headers().get(header::AUTHORIZATION) {
        None => {
            let e = "missing Authorization".to_string();
            return ErrResp::ErrParam(Some(e)).into_response();
        }
        Some(value) => value.clone(),
    };

    let (application, uri, host) = match get_application_inner(
        FN_NAME,
        &client,
        broker_base,
        param.application_id.as_str(),
        &token,
    )
    .await
    {
        Err(e) => return e,
        Ok((application, uri, host)) => (application, uri, host),
    };

    let host = host.as_str();
    let scheme = uri.scheme();
    let data = match scheme {
        "amqp" | "amqps" => {
            let AmqpState::RabbitMq(opts) = &state.amqp;
            let username = mq::to_username(
                QueueType::Application,
                application.unit_code.as_str(),
                application.code.as_str(),
            );
            let username = username.as_str();
            response::GetApplicationStatsData {
                uldata: match rabbitmq::stats(&client, opts, host, username, "uldata").await {
                    Err(ErrResp::ErrNotFound(_)) => response::Stats {
                        consumers: 0,
                        messages: 0,
                        publish_rate: 0.0,
                        deliver_rate: 0.0,
                    },
                    Err(e) => {
                        error!("[{}] get uldata stats error: {}", FN_NAME, e);
                        return e.into_response();
                    }
                    Ok(stats) => response::Stats {
                        consumers: stats.consumers,
                        messages: stats.messages,
                        publish_rate: stats.publish_rate,
                        deliver_rate: stats.deliver_rate,
                    },
                },
                dldata_resp: match rabbitmq::stats(&client, opts, host, username, "dldata-resp")
                    .await
                {
                    Err(ErrResp::ErrNotFound(_)) => response::Stats {
                        consumers: 0,
                        messages: 0,
                        publish_rate: 0.0,
                        deliver_rate: 0.0,
                    },
                    Err(e) => {
                        error!("[{}] get dldata-resp stats error: {}", FN_NAME, e);
                        return e.into_response();
                    }
                    Ok(stats) => response::Stats {
                        consumers: stats.consumers,
                        messages: stats.messages,
                        publish_rate: stats.publish_rate,
                        deliver_rate: stats.deliver_rate,
                    },
                },
                dldata_result: match rabbitmq::stats(&client, opts, host, username, "dldata-result")
                    .await
                {
                    Err(ErrResp::ErrNotFound(_)) => response::Stats {
                        consumers: 0,
                        messages: 0,
                        publish_rate: 0.0,
                        deliver_rate: 0.0,
                    },
                    Err(e) => {
                        error!("[{}] get dldata-result stats error: {}", FN_NAME, e);
                        return e.into_response();
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
                let username = mq::to_username(
                    QueueType::Application,
                    application.unit_code.as_str(),
                    application.code.as_str(),
                );
                let username = username.as_str();
                response::GetApplicationStatsData {
                    uldata: match emqx::stats(&client, opts, host, username, "uldata").await {
                        Err(e) => {
                            error!("[{}] get uldata stats error: {}", FN_NAME, e);
                            return e.into_response();
                        }
                        Ok(stats) => response::Stats {
                            consumers: stats.consumers,
                            messages: stats.messages,
                            publish_rate: stats.publish_rate,
                            deliver_rate: stats.deliver_rate,
                        },
                    },
                    dldata_resp: match emqx::stats(&client, opts, host, username, "dldata-resp")
                        .await
                    {
                        Err(e) => {
                            error!("[{}] get dldata-resp stats error: {}", FN_NAME, e);
                            return e.into_response();
                        }
                        Ok(stats) => response::Stats {
                            consumers: stats.consumers,
                            messages: stats.messages,
                            publish_rate: stats.publish_rate,
                            deliver_rate: stats.deliver_rate,
                        },
                    },
                    dldata_result: match emqx::stats(&client, opts, host, username, "dldata-result")
                        .await
                    {
                        Err(e) => {
                            error!("[{}] get dldata-result stats error: {}", FN_NAME, e);
                            return e.into_response();
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
            MqttState::Rumqttd => response::GetApplicationStatsData {
                uldata: response::Stats {
                    ..Default::default()
                },
                dldata_resp: response::Stats {
                    ..Default::default()
                },
                dldata_result: response::Stats {
                    ..Default::default()
                },
            },
        },
        _ => {
            let e = format!("unsupport scheme {}", scheme);
            error!("[{}] {}", FN_NAME, e);
            return ErrResp::ErrUnknown(Some(e)).into_response();
        }
    };
    Json(&response::GetApplicationStats { data }).into_response()
}

/// `POST /{base}/api/v1/application/{applicationId}/dldata`
async fn post_application_dldata(
    state: State<AppState>,
    headers: HeaderMap,
    Path(param): Path<ApplicationIdPath>,
    Json(body): Json<request::PostApplicationDlDataBody>,
) -> impl IntoResponse {
    const FN_NAME: &'static str = "post_application_dldata";
    let broker_base = state.broker_base.as_str();
    let client = state.client.clone();
    let token = match headers.get(header::AUTHORIZATION) {
        None => {
            let e = "missing Authorization".to_string();
            return ErrResp::ErrParam(Some(e)).into_response();
        }
        Some(value) => value.clone(),
    };

    if body.data.device_id.len() == 0 {
        let e = "empty `deviceId` is invalid".to_string();
        return ErrResp::ErrParam(Some(e)).into_response();
    }
    if let Err(e) = hex::decode(body.data.payload.as_str()) {
        let e = format!("`payload` is not hexadecimal string: {}", e);
        return ErrResp::ErrParam(Some(e)).into_response();
    }

    let (application, uri, hostname) = match get_application_inner(
        FN_NAME,
        &client,
        broker_base,
        param.application_id.as_str(),
        &token,
    )
    .await
    {
        Err(e) => return e,
        Ok((application, uri, hostname)) => (application, uri, hostname),
    };
    match get_device_inner(
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
                .into_response();
            }
            Some(_) => (),
        },
    };

    let hostname = hostname.as_str();
    let scheme = uri.scheme();
    let payload = match serde_json::to_string(&DlData {
        correlation_id: "1".to_string(),
        device_id: Some(body.data.device_id.clone()),
        data: body.data.payload.clone(),
        ..Default::default()
    }) {
        Err(e) => {
            let e = format!("encode JSON error: {}", e);
            error!("[{}] {}", FN_NAME, e);
            return ErrResp::ErrRsc(Some(e)).into_response();
        }
        Ok(payload) => general_purpose::STANDARD.encode(payload),
    };
    match scheme {
        "amqp" | "amqps" => {
            let AmqpState::RabbitMq(opts) = &state.amqp;
            let username = mq::to_username(
                QueueType::Application,
                application.unit_code.as_str(),
                application.code.as_str(),
            );
            let username = username.as_str();
            if let Err(e) =
                rabbitmq::publish_message(&client, opts, hostname, username, "dldata", payload)
                    .await
            {
                return e.into_response();
            }
        }
        "mqtt" | "mqtts" => match &state.mqtt {
            MqttState::Emqx(opts) => {
                let username = mq::to_username(
                    QueueType::Application,
                    application.unit_code.as_str(),
                    application.code.as_str(),
                );
                let username = username.as_str();
                if let Err(e) =
                    emqx::publish_message(&client, opts, hostname, username, "dldata", payload)
                        .await
                {
                    return e.into_response();
                }
            }
            MqttState::Rumqttd => {
                let e = "not support now".to_string();
                return ErrResp::ErrParam(Some(e)).into_response();
            }
        },
        _ => {
            let e = format!("unsupport scheme {}", scheme);
            error!("[{}] {}", FN_NAME, e);
            return ErrResp::ErrUnknown(Some(e)).into_response();
        }
    }
    StatusCode::NO_CONTENT.into_response()
}

async fn get_application_inner(
    fn_name: &str,
    client: &reqwest::Client,
    broker_base: &str,
    application_id: &str,
    token: &HeaderValue,
) -> Result<(response::GetApplicationData, Url, String), Response> {
    let uri = format!("{}/api/v1/application/{}", broker_base, application_id);
    let resp = get_stream_resp(fn_name, token, &client, uri.as_str()).await?;

    let application = match resp.json::<response::GetApplication>().await {
        Err(e) => {
            let e = format!("wrong response of application: {}", e);
            error!("[{}] {}", fn_name, e);
            return Err(ErrResp::ErrIntMsg(Some(e)).into_response());
        }
        Ok(application) => application.data,
    };
    let uri = match Url::parse(application.host_uri.as_str()) {
        Err(e) => {
            let e = format!("unexpected hostUri: {}", e);
            error!("[{}] {}", fn_name, e);
            return Err(ErrResp::ErrUnknown(Some(e)).into_response());
        }
        Ok(uri) => uri,
    };
    let host = match uri.host_str() {
        None => {
            let e = "unexpected hostUri".to_string();
            error!("[{}] {}", fn_name, e);
            return Err(ErrResp::ErrUnknown(Some(e)).into_response());
        }
        Some(host) => host.to_string(),
    };
    Ok((application, uri, host))
}

async fn check_application_code_inner(
    fn_name: &str,
    client: &reqwest::Client,
    broker_base: &str,
    unit_id: &str,
    code: &str,
    token: &HeaderValue,
) -> Result<u64, Response> {
    let uri = format!("{}/api/v1/application/count", broker_base);
    let req = match client
        .request(reqwest::Method::GET, uri)
        .header(reqwest::header::AUTHORIZATION, token)
        .query(&[("unit", unit_id), ("code", code)])
        .build()
    {
        Err(e) => {
            let e = format!("generate request error: {}", e);
            error!("[{}] {}", fn_name, e);
            return Err(ErrResp::ErrRsc(Some(e)).into_response());
        }
        Ok(req) => req,
    };
    let resp = match client.execute(req).await {
        Err(e) => {
            let e = format!("execute request error: {}", e);
            error!("[{}] {}", fn_name, e);
            return Err(ErrResp::ErrIntMsg(Some(e)).into_response());
        }
        Ok(resp) => resp,
    };

    match resp.json::<response::GetCount>().await {
        Err(e) => {
            let e = format!("wrong response of application: {}", e);
            error!("[{}] {}", fn_name, e);
            Err(ErrResp::ErrIntMsg(Some(e)).into_response())
        }
        Ok(data) => Ok(data.data.count),
    }
}
