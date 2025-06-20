//! Provides the authentication middleware by sending the Bearer token to `sylvia-iot-auth`.
//!
//! Here is an example to wrap the auth middleware and how to get token information:
//!
//! ```rust
//! use axum::{
//!     extract::Request,
//!     http::{header, StatusCode},
//!     response::{IntoResponse, Response},
//!     routing, Extension, Router,
//! };
//! use sylvia_iot_sdk::middlewares::auth::{AuthService, GetTokenInfoData};
//!
//! fn new_service() -> Router {
//!     let auth_uri = "http://localhost:1080/auth/api/v1/auth/tokeninfo";
//!     Router::new()
//!         .route("/api", routing::get(api))
//!         .layer(AuthService::new(auth_uri.clone()))
//! }
//!
//! async fn api(Extension(token_info): Extension<GetTokenInfoData>) -> impl IntoResponse {
//!     StatusCode::NO_CONTENT
//! }
//! ```

use axum::{
    extract::Request,
    http::header,
    response::{IntoResponse, Response},
};
use futures::future::BoxFuture;
use reqwest;
use serde::{self, Deserialize};
use std::{
    collections::HashMap,
    task::{Context, Poll},
};
use tower::{Layer, Service};

use crate::util::err::ErrResp;

#[derive(Clone)]
pub struct GetTokenInfoData {
    /// The access token.
    pub token: String,
    pub user_id: String,
    pub account: String,
    pub roles: HashMap<String, bool>,
    pub name: String,
    pub client_id: String,
    pub scopes: Vec<String>,
}

#[derive(Clone)]
pub struct AuthService {
    auth_uri: String,
}

#[derive(Clone)]
pub struct AuthMiddleware<S> {
    client: reqwest::Client,
    auth_uri: String,
    service: S,
}

/// The user/client information of the token.
#[derive(Clone, Deserialize)]
struct GetTokenInfo {
    data: GetTokenInfoDataInner,
}

#[derive(Clone, Deserialize)]
struct GetTokenInfoDataInner {
    #[serde(rename = "userId")]
    user_id: String,
    account: String,
    roles: HashMap<String, bool>,
    name: String,
    #[serde(rename = "clientId")]
    client_id: String,
    scopes: Vec<String>,
}

impl AuthService {
    pub fn new(auth_uri: String) -> Self {
        AuthService { auth_uri }
    }
}

impl<S> Layer<S> for AuthService {
    type Service = AuthMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        AuthMiddleware {
            client: reqwest::Client::new(),
            auth_uri: self.auth_uri.clone(),
            service: inner,
        }
    }
}

impl<S> Service<Request> for AuthMiddleware<S>
where
    S: Service<Request, Response = Response> + Clone + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx)
    }

    fn call(&mut self, mut req: Request) -> Self::Future {
        let mut svc = self.service.clone();
        let client = self.client.clone();
        let auth_uri = self.auth_uri.clone();

        Box::pin(async move {
            let token = match parse_header_auth(&req) {
                Err(e) => return Ok(e.into_response()),
                Ok(token) => match token {
                    None => {
                        let e = ErrResp::ErrParam(Some("missing token".to_string()));
                        return Ok(e.into_response());
                    }
                    Some(token) => token,
                },
            };

            let token_req = match client
                .request(reqwest::Method::GET, auth_uri.as_str())
                .header(reqwest::header::AUTHORIZATION, token.as_str())
                .build()
            {
                Err(e) => {
                    let e = ErrResp::ErrRsc(Some(format!("request auth error: {}", e)));
                    return Ok(e.into_response());
                }
                Ok(req) => req,
            };
            let resp = match client.execute(token_req).await {
                Err(e) => {
                    let e = ErrResp::ErrIntMsg(Some(format!("auth error: {}", e)));
                    return Ok(e.into_response());
                }
                Ok(resp) => match resp.status() {
                    reqwest::StatusCode::UNAUTHORIZED => {
                        return Ok(ErrResp::ErrAuth(None).into_response());
                    }
                    reqwest::StatusCode::OK => resp,
                    _ => {
                        let e = ErrResp::ErrIntMsg(Some(format!(
                            "auth error with status code: {}",
                            resp.status()
                        )));
                        return Ok(e.into_response());
                    }
                },
            };
            let token_info = match resp.json::<GetTokenInfo>().await {
                Err(e) => {
                    let e = ErrResp::ErrIntMsg(Some(format!("read auth body error: {}", e)));
                    return Ok(e.into_response());
                }
                Ok(info) => info,
            };

            let mut split = token.split_whitespace();
            split.next(); // skip "Bearer".
            let token = match split.next() {
                None => {
                    let e = ErrResp::ErrUnknown(Some("parse token error".to_string()));
                    return Ok(e.into_response());
                }
                Some(token) => token.to_string(),
            };

            req.extensions_mut().insert(GetTokenInfoData {
                token,
                user_id: token_info.data.user_id,
                account: token_info.data.account,
                roles: token_info.data.roles,
                name: token_info.data.name,
                client_id: token_info.data.client_id,
                scopes: token_info.data.scopes,
            });

            let res = svc.call(req).await?;
            Ok(res)
        })
    }
}

/// Parse Authorization header content. Returns `None` means no Authorization header.
pub fn parse_header_auth(req: &Request) -> Result<Option<String>, ErrResp> {
    let mut auth_all = req.headers().get_all(header::AUTHORIZATION).iter();
    let auth = match auth_all.next() {
        None => return Ok(None),
        Some(auth) => match auth.to_str() {
            Err(e) => return Err(ErrResp::ErrParam(Some(e.to_string()))),
            Ok(auth) => auth,
        },
    };
    if auth_all.next() != None {
        return Err(ErrResp::ErrParam(Some(
            "invalid multiple Authorization header".to_string(),
        )));
    }
    Ok(Some(auth.to_string()))
}
