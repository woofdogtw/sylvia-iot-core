//! Provides the authentication middleware by sending the Bearer token to [`sylvia-iot-auth`].

use std::{
    collections::HashMap,
    task::{Context, Poll},
};

use axum::{
    extract::Request,
    response::{IntoResponse, Response},
};
use futures::future::BoxFuture;
use reqwest;
use serde::{self, Deserialize};
use tower::{Layer, Service};

use sylvia_iot_corelib::{err::ErrResp, http as sylvia_http};

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
    client: reqwest::Client,
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
    pub user_id: String,
    pub account: String,
    pub roles: HashMap<String, bool>,
    pub name: String,
    #[serde(rename = "clientId")]
    pub client_id: String,
    pub scopes: Vec<String>,
}

impl AuthService {
    pub fn new(client: reqwest::Client, auth_uri: String) -> Self {
        AuthService { client, auth_uri }
    }
}

impl<S> Layer<S> for AuthService {
    type Service = AuthMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        AuthMiddleware {
            client: self.client.clone(),
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
            let token = match sylvia_http::parse_header_auth(&req) {
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
