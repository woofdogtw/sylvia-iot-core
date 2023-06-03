//! Provides the operation log middleware by sending requests to the data channel.

use std::{
    cell::RefCell,
    collections::HashMap,
    rc::Rc,
    task::{Context, Poll},
};

use actix_http::h1::Payload;
use actix_service::{Service, Transform};
use actix_web::{
    body::BoxBody,
    dev::{ServiceRequest, ServiceResponse},
    http::Method,
    web::BytesMut,
    Error, HttpMessage,
};
use chrono::Utc;
use futures::future::{self, LocalBoxFuture, Ready};
use futures_util::StreamExt;
use reqwest;
use serde::{self, Deserialize, Serialize};
use serde_json::{Map, Value};

use general_mq::{queue::GmqQueue, Queue};
use sylvia_iot_corelib::{http as sylvia_http, strings};

pub struct LogService {
    auth_uri: String,
    queue: Option<Queue>,
}

pub struct LogMiddleware<S> {
    client: reqwest::Client,
    auth_uri: String,
    queue: Option<Queue>,
    service: Rc<RefCell<S>>,
}

/// The user/client information of the token.
#[derive(Deserialize)]
struct GetTokenInfo {
    data: GetTokenInfoData,
}

#[derive(Deserialize)]
struct GetTokenInfoData {
    #[serde(rename = "userId")]
    user_id: String,
    #[serde(rename = "account")]
    _account: String,
    #[serde(rename = "roles")]
    _roles: HashMap<String, bool>,
    #[serde(rename = "name")]
    _name: String,
    #[serde(rename = "clientId")]
    client_id: String,
    #[serde(rename = "scopes")]
    _scopes: Vec<String>,
}

#[derive(Serialize)]
struct SendDataMsg {
    kind: String,
    data: SendDataKind,
}

#[derive(Serialize)]
#[serde(untagged)]
enum SendDataKind {
    Operation {
        #[serde(rename = "dataId")]
        data_id: String,
        #[serde(rename = "reqTime")]
        req_time: String,
        #[serde(rename = "resTime")]
        res_time: String,
        #[serde(rename = "latencyMs")]
        latency_ms: i64,
        status: isize,
        #[serde(rename = "sourceIp")]
        source_ip: String,
        method: String,
        path: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        body: Option<Map<String, Value>>,
        #[serde(rename = "userId")]
        user_id: String,
        #[serde(rename = "clientId")]
        client_id: String,
        #[serde(rename = "errCode", skip_serializing_if = "Option::is_none")]
        err_code: Option<String>,
        #[serde(rename = "errMessage", skip_serializing_if = "Option::is_none")]
        err_message: Option<String>,
    },
}

struct DataMsgKind;

const DATA_ID_RAND_LEN: usize = 12;

impl DataMsgKind {
    const OPERATION: &'static str = "operation";
}

impl LogService {
    pub fn new(auth_uri: String, queue: Option<Queue>) -> Self {
        LogService { auth_uri, queue }
    }
}

impl<S> Transform<S, ServiceRequest> for LogService
where
    S: Service<ServiceRequest, Response = ServiceResponse<BoxBody>, Error = Error> + 'static,
    S::Future: 'static,
{
    type Response = ServiceResponse<BoxBody>;
    type Error = Error;
    type Transform = LogMiddleware<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        future::ok(LogMiddleware {
            client: reqwest::Client::new(),
            auth_uri: self.auth_uri.clone(),
            queue: self.queue.clone(),
            service: Rc::new(RefCell::new(service)),
        })
    }
}

impl<S> Service<ServiceRequest> for LogMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<BoxBody>, Error = Error> + 'static,
    S::Future: 'static,
{
    type Response = ServiceResponse<BoxBody>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx)
    }

    fn call(&self, mut req: ServiceRequest) -> Self::Future {
        let svc = self.service.clone();
        let client = self.client.clone();
        let auth_uri = self.auth_uri.clone();
        let method = req.method().clone();
        let queue = match method {
            Method::DELETE | Method::PATCH | Method::POST | Method::PUT => self.queue.clone(),
            _ => None,
        };

        Box::pin(async move {
            // Only log for DELETE/PATCH/POST/PUT methods.
            let q = match queue.as_ref() {
                None => {
                    let res = svc.call(req).await?;
                    return Ok(res);
                }
                Some(q) => q,
            };

            let req_time = Utc::now();

            // Collect body (and generate a new stream) and information for logging the operation.
            let source_ip = match req.connection_info().realip_remote_addr() {
                None => "".to_string(),
                Some(addr) => addr.to_string(),
            };
            let method = req.method().to_string();
            let path = req.path().to_string();
            let (http_req, _) = req.parts();
            let auth_token = match sylvia_http::parse_header_auth(http_req) {
                Err(_) => None,
                Ok(token) => match token {
                    None => None,
                    Some(token) => Some(token),
                },
            };
            let mut req_body = BytesMut::new();
            while let Some(chunk) = req.take_payload().next().await {
                req_body.extend_from_slice(&chunk?);
            }
            let body_bytes = req_body.freeze();
            let log_body = match serde_json::from_slice::<Map<String, Value>>(&body_bytes.to_vec())
            {
                Err(_) => None,
                Ok(mut body) => {
                    // Remove secret contents.
                    if let Some(data) = body.get_mut("data") {
                        if let Some(data) = data.as_object_mut() {
                            if data.contains_key("password") {
                                data.insert("password".to_string(), Value::String("".to_string()));
                            }
                        }
                    }
                    Some(body)
                }
            };
            let (_, mut orig_payload) = Payload::create(true);
            orig_payload.unread_data(body_bytes);
            req.set_payload(orig_payload.into());

            // Do the request.
            let res = svc.call(req).await?;
            let (err_code, err_message) = match res.status().is_success() {
                false => {
                    // TODO: extract (code, message) pair of response body.
                    (None, None)
                }
                true => (None, None),
            };

            // Send log.
            let auth_token = match auth_token.as_ref() {
                None => return Ok(res),
                Some(auth_token) => auth_token,
            };
            let token_info = match get_token(client, auth_token.as_str(), auth_uri.as_str()).await {
                Err(_) => return Ok(res),
                Ok(token_info) => token_info,
            };
            let res_time = Utc::now();
            let msg = SendDataMsg {
                kind: DataMsgKind::OPERATION.to_string(),
                data: SendDataKind::Operation {
                    data_id: strings::random_id(&req_time, DATA_ID_RAND_LEN),
                    req_time: strings::time_str(&req_time),
                    res_time: strings::time_str(&res_time),
                    latency_ms: res_time.timestamp_millis() - req_time.timestamp_millis(),
                    status: res.status().as_u16() as isize,
                    source_ip,
                    method,
                    path,
                    body: log_body,
                    user_id: token_info.data.user_id,
                    client_id: token_info.data.client_id,
                    err_code,
                    err_message,
                },
            };
            if let Ok(payload) = serde_json::to_vec(&msg) {
                let _ = q.send_msg(payload).await;
            }
            Ok(res)
        })
    }
}

async fn get_token(
    client: reqwest::Client,
    auth_token: &str,
    auth_uri: &str,
) -> Result<GetTokenInfo, String> {
    let token_req = match client
        .request(reqwest::Method::GET, auth_uri)
        .header(reqwest::header::AUTHORIZATION, auth_token)
        .build()
    {
        Err(e) => return Err(format!("request auth error: {}", e)),
        Ok(req) => req,
    };
    let resp = match client.execute(token_req).await {
        Err(e) => return Err(format!("auth error: {}", e)),
        Ok(resp) => match resp.status() {
            reqwest::StatusCode::UNAUTHORIZED => return Err("unauthorized".to_string()),
            reqwest::StatusCode::OK => resp,
            _ => return Err(format!("auth error with status code: {}", resp.status())),
        },
    };
    match resp.json::<GetTokenInfo>().await {
        Err(e) => Err(format!("read auth body error: {}", e)),
        Ok(info) => Ok(info),
    }
}
