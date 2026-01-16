//! Provides the authentication middleware by sending the Bearer token to [`sylvia-iot-auth`].

use std::{
    collections::{HashMap, HashSet},
    task::{Context, Poll},
};

use axum::{
    extract::Request,
    http::Method,
    response::{IntoResponse, Response},
};
use futures::future::BoxFuture;
use reqwest;
use serde::{self, Deserialize};
use tower::{Layer, Service};

use sylvia_iot_corelib::{err::ErrResp, http as sylvia_http};

pub type RoleScopeType = (Vec<&'static str>, Vec<String>);
type RoleScopeInner = (HashSet<&'static str>, HashSet<String>);

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
    role_scopes: HashMap<Method, RoleScopeType>,
}

#[derive(Clone)]
pub struct AuthMiddleware<S> {
    client: reqwest::Client,
    auth_uri: String,
    role_scopes: HashMap<Method, RoleScopeInner>,
    service: S,
}

/// The user/client information of the token.
#[derive(Deserialize)]
struct GetTokenInfo {
    data: GetTokenInfoDataInner,
}

#[derive(Deserialize)]
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
    pub fn new(
        client: reqwest::Client,
        auth_uri: String,
        role_scopes: HashMap<Method, RoleScopeType>,
    ) -> Self {
        AuthService {
            client,
            auth_uri,
            role_scopes,
        }
    }
}

impl<S> Layer<S> for AuthService {
    type Service = AuthMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        let mut role_scopes: HashMap<Method, RoleScopeInner> = HashMap::new();
        for (k, (r, s)) in self.role_scopes.iter() {
            role_scopes.insert(
                k.clone(),
                (
                    r.iter().map(|&r| r).collect(),
                    s.iter().map(|s| s.clone()).collect(),
                ),
            );
        }

        AuthMiddleware {
            client: self.client.clone(),
            auth_uri: self.auth_uri.clone(),
            role_scopes,
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
        let role_scopes = self.role_scopes.clone();

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

            if let Some((api_roles, api_scopes)) = role_scopes.get(req.method()) {
                if api_roles.len() > 0 {
                    let roles: HashSet<&str> = token_info
                        .data
                        .roles
                        .iter()
                        .filter(|(_, v)| **v)
                        .map(|(k, _)| k.as_str())
                        .collect();
                    if api_roles.is_disjoint(&roles) {
                        let e = ErrResp::ErrPerm(Some("invalid role".to_string()));
                        return Ok(e.into_response());
                    }
                }
                if api_scopes.len() > 0 {
                    let api_scopes: HashSet<&str> = api_scopes.iter().map(|s| s.as_str()).collect();
                    let scopes: HashSet<&str> =
                        token_info.data.scopes.iter().map(|s| s.as_str()).collect();
                    if api_scopes.is_disjoint(&scopes) {
                        let e = ErrResp::ErrPerm(Some("invalid scope".to_string()));
                        return Ok(e.into_response());
                    }
                }
            }

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
