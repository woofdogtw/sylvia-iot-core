use std::error::Error as StdError;

use actix_web::{
    dev::HttpServiceFactory,
    http::header::{self, HeaderValue},
    web::{self, Bytes, BytesMut},
    HttpRequest, HttpResponse, Responder, ResponseError,
};
use csv::WriterBuilder;
use futures_util::StreamExt;
use log::error;
use serde::Deserialize;
use serde_json::Deserializer;
use url::Url;

use sylvia_iot_corelib::err::ErrResp;

use super::{
    super::{AmqpState, MqttState, State},
    api_bridge, get_stream_resp, get_unit_inner, list_api_bridge, response, ListResp,
};
use crate::libs::mq::{self, emqx, rabbitmq, QueueType};

#[derive(Deserialize)]
struct UnitIdPath {
    unit_id: String,
}

#[derive(Deserialize)]
struct Application {
    code: String,
    #[serde(rename = "hostUri")]
    host_uri: String,
}

#[derive(Deserialize)]
struct Network {
    code: String,
    #[serde(rename = "hostUri")]
    host_uri: String,
}

const CSV_FIELDS: &'static str = "unitId,code,createdAt,modifiedAt,ownerId,memberIds,name,info\n";

pub fn new_service(scope_path: &str) -> impl HttpServiceFactory {
    web::scope(scope_path)
        .service(web::resource("").route(web::post().to(post_unit)))
        .service(web::resource("/count").route(web::get().to(get_unit_count)))
        .service(web::resource("/list").route(web::get().to(get_unit_list)))
        .service(
            web::resource("/{unit_id}")
                .route(web::get().to(get_unit))
                .route(web::patch().to(patch_unit))
                .route(web::delete().to(delete_unit)),
        )
}

/// `POST /{base}/api/v1/unit`
async fn post_unit(mut req: HttpRequest, body: Bytes, state: web::Data<State>) -> impl Responder {
    const FN_NAME: &'static str = "post_unit";
    let api_path = format!("{}/api/v1/unit", state.broker_base);
    let client = state.client.clone();

    api_bridge(FN_NAME, &client, &mut req, api_path.as_str(), Some(body)).await
}

/// `GET /{base}/api/v1/unit/count`
async fn get_unit_count(mut req: HttpRequest, state: web::Data<State>) -> impl Responder {
    const FN_NAME: &'static str = "get_unit_count";
    let api_path = format!("{}/api/v1/unit/count", state.broker_base.as_str());
    let client = state.client.clone();

    api_bridge(FN_NAME, &client, &mut req, api_path.as_str(), None).await
}

/// `GET /{base}/api/v1/unit/list`
async fn get_unit_list(mut req: HttpRequest, state: web::Data<State>) -> impl Responder {
    const FN_NAME: &'static str = "get_unit_list";
    let api_path = format!("{}/api/v1/unit/list", state.broker_base.as_str());
    let api_path = api_path.as_str();
    let client = state.client.clone();

    let (api_resp, mut resp) =
        match list_api_bridge(FN_NAME, &client, &mut req, api_path, false, "unit").await {
            ListResp::ActixWeb(resp) => return resp,
            ListResp::ArrayStream(api_resp, resp) => (api_resp, resp),
        };

    let mut resp_stream = api_resp.bytes_stream();
    let stream = async_stream::stream! {
        yield Ok(Bytes::from(vec![0xEF, 0xBB, 0xBF])); // BOM
        yield Ok(Bytes::from(CSV_FIELDS));

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

            let mut json_stream = Deserializer::from_slice(&buffer[..]).into_iter::<response::Unit>();
            let mut index = 0;
            let mut finish = false;
            loop {
                if let Some(Ok(mut v)) = json_stream.next() {
                    if let Ok(member_ids_str) = serde_json::to_string(&v.member_ids) {
                        v.member_ids_str = Some(member_ids_str);
                    }
                    if let Ok(info_str) = serde_json::to_string(&v.info) {
                        v.info_str = Some(info_str);
                    }
                    let mut writer = WriterBuilder::new().has_headers(false).from_writer(vec![]);
                    if let Err(e) = writer.serialize(v) {
                        let err: Box<dyn StdError> = Box::new(e);
                        yield Err(err);
                        finish = true;
                        break;
                    }
                    match writer.into_inner() {
                        Err(e) => {
                            let err: Box<dyn StdError> = Box::new(e);
                            yield Err(err);
                            finish = true;
                            break;
                        }
                        Ok(row) => yield Ok(Bytes::copy_from_slice(row.as_slice())),
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
                            Deserializer::from_slice(&buffer[index..])
                                .into_iter::<response::Unit>();
                    }
                    b']' => {
                        finish = true;
                        break;
                    }
                    _ => break,
                }
            }
            if finish {
                break;
            }
            buffer = buffer.split_off(index);
        }
    };
    resp.streaming(stream)
}

/// `GET /{base}/api/v1/unit/{unitId}`
async fn get_unit(
    mut req: HttpRequest,
    param: web::Path<UnitIdPath>,
    state: web::Data<State>,
) -> impl Responder {
    const FN_NAME: &'static str = "get_unit";
    let api_path = format!("{}/api/v1/unit/{}", state.broker_base, param.unit_id);
    let client = state.client.clone();

    api_bridge(FN_NAME, &client, &mut req, api_path.as_str(), None).await
}

/// `PATCH /{base}/api/v1/unit/{unitId}`
async fn patch_unit(
    mut req: HttpRequest,
    param: web::Path<UnitIdPath>,
    body: Bytes,
    state: web::Data<State>,
) -> impl Responder {
    const FN_NAME: &'static str = "patch_unit";
    let api_path = format!("{}/api/v1/unit/{}", state.broker_base, param.unit_id);
    let client = state.client.clone();

    api_bridge(FN_NAME, &client, &mut req, api_path.as_str(), Some(body)).await
}

/// `DELETE /{base}/api/v1/unit/{unitId}`
async fn delete_unit(
    mut req: HttpRequest,
    param: web::Path<UnitIdPath>,
    state: web::Data<State>,
) -> impl Responder {
    const FN_NAME: &'static str = "delete_unit";
    let api_path = format!("{}/api/v1/unit/{}", state.broker_base, param.unit_id);
    let client = state.client.clone();

    // Delete all underlaying broker resources before deleting the unit.
    let token = match req.headers().get(header::AUTHORIZATION) {
        None => {
            let msg = "missing Authorization".to_string();
            return ErrResp::ErrParam(Some(msg)).error_response();
        }
        Some(value) => value.clone(),
    };
    let unit = match get_unit_inner(
        FN_NAME,
        &client,
        state.broker_base.as_str(),
        param.unit_id.as_str(),
        &token,
    )
    .await
    {
        Err(e) => return e,
        Ok(unit) => unit,
    };
    if let Some(unit) = unit {
        let unit_id = param.unit_id.as_str();
        let unit_code = unit.code.as_str();
        if let Err(e) =
            delete_application_resources(FN_NAME, &token, &state, unit_id, unit_code).await
        {
            return e;
        }
        if let Err(e) = delete_network_resources(FN_NAME, &token, &state, unit_id, unit_code).await
        {
            return e;
        }
    }

    api_bridge(FN_NAME, &client, &mut req, api_path.as_str(), None).await
}

async fn delete_application_resources(
    fn_name: &str,
    token: &HeaderValue,
    state: &web::Data<State>,
    unit_id: &str,
    unit_code: &str,
) -> Result<(), HttpResponse> {
    // Get application from stream and delete broker resources.
    let client = state.client.clone();
    let uri = format!(
        "{}/api/v1/application/list?limit=0&format=array&unit={}",
        state.broker_base.as_str(),
        unit_id
    );

    let mut buffer = BytesMut::new();
    let mut stream = get_stream_resp(fn_name, token, &client, uri.as_str())
        .await?
        .bytes_stream();
    while let Some(body) = stream.next().await {
        match body {
            Err(e) => {
                let msg = format!("get application body error: {}", e);
                error!("[{}] {}", fn_name, msg);
                return Err(ErrResp::ErrIntMsg(Some(msg)).error_response());
            }
            Ok(body) => buffer.extend_from_slice(&body[..]),
        }

        let mut json_stream = Deserializer::from_slice(&buffer[..]).into_iter::<Application>();
        let mut index = 0;
        let mut finish = false;
        loop {
            if let Some(Ok(v)) = json_stream.next() {
                if v.host_uri.starts_with("amqp") {
                    match &state.amqp {
                        AmqpState::RabbitMq(opts) => {
                            let host = match Url::parse(v.host_uri.as_str()) {
                                Err(e) => {
                                    let msg = format!("{} is not valid URI: {}", v.host_uri, e);
                                    error!("[{}] {}", fn_name, msg);
                                    return Err(ErrResp::ErrUnknown(Some(msg)).error_response());
                                }
                                Ok(url) => match url.host_str() {
                                    None => {
                                        let msg = format!("{} is not valid URI", v.host_uri);
                                        error!("[{}] {}", fn_name, msg);
                                        return Err(ErrResp::ErrUnknown(Some(msg)).error_response());
                                    }
                                    Some(host) => host.to_string(),
                                },
                            };
                            let username =
                                mq::to_username(QueueType::Application, unit_code, v.code.as_str());
                            if let Err(e) = rabbitmq::delete_user(
                                &client,
                                opts,
                                host.as_str(),
                                username.as_str(),
                            )
                            .await
                            {
                                let msg = format!("delete RabbitMQ user {} error: {}", username, e);
                                error!("[{}] {}", fn_name, msg);
                                return Err(ErrResp::ErrIntMsg(Some(msg)).error_response());
                            }
                        }
                    }
                } else if v.host_uri.starts_with("mqtt") {
                    match &state.mqtt {
                        MqttState::Emqx(opts) => {
                            let host = match Url::parse(v.host_uri.as_str()) {
                                Err(e) => {
                                    let msg = format!("{} is not valid URI: {}", v.host_uri, e);
                                    error!("[{}] {}", fn_name, msg);
                                    return Err(ErrResp::ErrUnknown(Some(msg)).error_response());
                                }
                                Ok(url) => match url.host_str() {
                                    None => {
                                        let msg = format!("{} is not valid URI", v.host_uri);
                                        error!("[{}] {}", fn_name, msg);
                                        return Err(ErrResp::ErrUnknown(Some(msg)).error_response());
                                    }
                                    Some(host) => host.to_string(),
                                },
                            };
                            let username =
                                mq::to_username(QueueType::Application, unit_code, v.code.as_str());
                            if let Err(e) =
                                emqx::delete_user(&client, opts, host.as_str(), username.as_str())
                                    .await
                            {
                                let msg = format!("delete RabbitMQ user {} error: {}", username, e);
                                error!("[{}] {}", fn_name, msg);
                                return Err(ErrResp::ErrIntMsg(Some(msg)).error_response());
                            }
                        }
                        MqttState::Rumqttd => {}
                    }
                }
            }
            let offset = json_stream.byte_offset();
            if buffer.len() <= index + offset {
                index = buffer.len();
                break;
            }
            match buffer[index + offset] {
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
            break;
        }
        buffer = buffer.split_off(index);
    }

    Ok(())
}

async fn delete_network_resources(
    fn_name: &str,
    token: &HeaderValue,
    state: &web::Data<State>,
    unit_id: &str,
    unit_code: &str,
) -> Result<(), HttpResponse> {
    // Get network from stream and delete broker resources.
    let client = state.client.clone();
    let uri = format!(
        "{}/api/v1/network/list?limit=0&format=array&unit={}",
        state.broker_base.as_str(),
        unit_id
    );

    let mut buffer = BytesMut::new();
    let mut stream = get_stream_resp(fn_name, token, &client, uri.as_str())
        .await?
        .bytes_stream();
    while let Some(body) = stream.next().await {
        match body {
            Err(e) => {
                let msg = format!("get network body error: {}", e);
                error!("[{}] {}", fn_name, msg);
                return Err(ErrResp::ErrIntMsg(Some(msg)).error_response());
            }
            Ok(body) => buffer.extend_from_slice(&body[..]),
        }

        let mut json_stream = Deserializer::from_slice(&buffer[..]).into_iter::<Network>();
        let mut index = 0;
        let mut finish = false;
        loop {
            if let Some(Ok(v)) = json_stream.next() {
                if v.host_uri.starts_with("amqp") {
                    match &state.amqp {
                        AmqpState::RabbitMq(opts) => {
                            let host = match Url::parse(v.host_uri.as_str()) {
                                Err(e) => {
                                    let msg = format!("{} is not valid URI: {}", v.host_uri, e);
                                    error!("[{}] {}", fn_name, msg);
                                    return Err(ErrResp::ErrUnknown(Some(msg)).error_response());
                                }
                                Ok(url) => match url.host_str() {
                                    None => {
                                        let msg = format!("{} is not valid URI", v.host_uri);
                                        error!("[{}] {}", fn_name, msg);
                                        return Err(ErrResp::ErrUnknown(Some(msg)).error_response());
                                    }
                                    Some(host) => host.to_string(),
                                },
                            };
                            let username =
                                mq::to_username(QueueType::Network, unit_code, v.code.as_str());
                            if let Err(e) = rabbitmq::delete_user(
                                &client,
                                opts,
                                host.as_str(),
                                username.as_str(),
                            )
                            .await
                            {
                                let msg = format!("delete RabbitMQ user {} error: {}", username, e);
                                error!("[{}] {}", fn_name, msg);
                                return Err(ErrResp::ErrIntMsg(Some(msg)).error_response());
                            }
                        }
                    }
                } else if v.host_uri.starts_with("mqtt") {
                    match &state.mqtt {
                        MqttState::Emqx(opts) => {
                            let host = match Url::parse(v.host_uri.as_str()) {
                                Err(e) => {
                                    let msg = format!("{} is not valid URI: {}", v.host_uri, e);
                                    error!("[{}] {}", fn_name, msg);
                                    return Err(ErrResp::ErrUnknown(Some(msg)).error_response());
                                }
                                Ok(url) => match url.host_str() {
                                    None => {
                                        let msg = format!("{} is not valid URI", v.host_uri);
                                        error!("[{}] {}", fn_name, msg);
                                        return Err(ErrResp::ErrUnknown(Some(msg)).error_response());
                                    }
                                    Some(host) => host.to_string(),
                                },
                            };
                            let username =
                                mq::to_username(QueueType::Network, unit_code, v.code.as_str());
                            if let Err(e) =
                                emqx::delete_user(&client, opts, host.as_str(), username.as_str())
                                    .await
                            {
                                let msg = format!("delete RabbitMQ user {} error: {}", username, e);
                                error!("[{}] {}", fn_name, msg);
                                return Err(ErrResp::ErrIntMsg(Some(msg)).error_response());
                            }
                        }
                        MqttState::Rumqttd => {}
                    }
                }
            }
            let offset = json_stream.byte_offset();
            if buffer.len() <= index + offset {
                index = buffer.len();
                break;
            }
            match buffer[index + offset] {
                b'[' | b',' => {
                    index += offset + 1;
                    if buffer.len() <= index {
                        break;
                    }
                    json_stream = Deserializer::from_slice(&buffer[index..]).into_iter::<Network>();
                }
                b']' => {
                    finish = true;
                    break;
                }
                _ => break,
            }
        }
        if finish {
            break;
        }
        buffer = buffer.split_off(index);
    }

    Ok(())
}
