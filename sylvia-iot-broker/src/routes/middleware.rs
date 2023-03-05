//! Provides the authentication middleware by sending the Bearer token to [`sylvia-auth`].

use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    rc::Rc,
    task::{Context, Poll},
};

use actix_service::{Service, Transform};
use actix_web::{
    body::BoxBody,
    dev::{ServiceRequest, ServiceResponse},
    http::Method,
    Error, HttpMessage,
};
use futures::future::{self, LocalBoxFuture, Ready};
use reqwest;
use serde::{self, Deserialize};

use sylvia_iot_corelib::{err::ErrResp, http as sylvia_http};

pub type RoleScopeType = (Vec<&'static str>, Vec<String>);
type RoleScopeInner = (HashSet<&'static str>, HashSet<String>);

/// The user/client information of the token.
#[derive(Deserialize)]
pub struct GetTokenInfo {
    pub data: GetTokenInfoData,
}

#[derive(Deserialize)]
pub struct GetTokenInfoData {
    #[serde(rename = "userId")]
    pub user_id: String,
    pub account: String,
    pub roles: HashMap<String, bool>,
    pub name: String,
    #[serde(rename = "clientId")]
    pub client_id: String,
    pub scopes: Vec<String>,
}

pub struct AuthService {
    auth_uri: String,
    role_scopes: HashMap<Method, RoleScopeType>,
}

pub struct AuthMiddleware<S> {
    client: reqwest::Client,
    auth_uri: String,
    role_scopes: HashMap<Method, RoleScopeInner>,
    service: Rc<RefCell<S>>,
}

impl AuthService {
    pub fn new(auth_uri: String, role_scopes: HashMap<Method, RoleScopeType>) -> Self {
        AuthService {
            role_scopes,
            auth_uri,
        }
    }
}

impl<S> Transform<S, ServiceRequest> for AuthService
where
    S: Service<ServiceRequest, Response = ServiceResponse<BoxBody>, Error = Error> + 'static,
    S::Future: 'static,
{
    type Response = ServiceResponse<BoxBody>;
    type Error = Error;
    type Transform = AuthMiddleware<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
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

        future::ok(AuthMiddleware {
            client: reqwest::Client::new(),
            auth_uri: self.auth_uri.clone(),
            role_scopes,
            service: Rc::new(RefCell::new(service)),
        })
    }
}

impl<S> Service<ServiceRequest> for AuthMiddleware<S>
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
        let role_scopes = self.role_scopes.clone();

        Box::pin(async move {
            let (http_req, _) = req.parts_mut();
            let token = match sylvia_http::parse_header_auth(&http_req) {
                Err(e) => {
                    return Ok(ServiceResponse::from_err(e, http_req.clone()));
                }
                Ok(token) => match token {
                    None => {
                        let e = ErrResp::ErrParam(Some("missing token".to_string()));
                        return Ok(ServiceResponse::from_err(e, http_req.clone()));
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
                    return Ok(ServiceResponse::from_err(e, http_req.clone()));
                }
                Ok(req) => req,
            };
            let resp = match client.execute(token_req).await {
                Err(e) => {
                    let e = ErrResp::ErrIntMsg(Some(format!("auth error: {}", e)));
                    return Ok(ServiceResponse::from_err(e, http_req.clone()));
                }
                Ok(resp) => match resp.status() {
                    reqwest::StatusCode::UNAUTHORIZED => {
                        let e = ErrResp::ErrAuth(None);
                        return Ok(ServiceResponse::from_err(e, http_req.clone()));
                    }
                    reqwest::StatusCode::OK => resp,
                    _ => {
                        let e = ErrResp::ErrIntMsg(Some(format!(
                            "auth error with status code: {}",
                            resp.status()
                        )));
                        return Ok(ServiceResponse::from_err(e, http_req.clone()));
                    }
                },
            };
            let token_info = match resp.json::<GetTokenInfo>().await {
                Err(e) => {
                    let e = ErrResp::ErrIntMsg(Some(format!("read auth body error: {}", e)));
                    return Ok(ServiceResponse::from_err(e, http_req.clone()));
                }
                Ok(info) => info,
            };

            if let Some((api_roles, api_scopes)) = role_scopes.get(http_req.method()) {
                if api_roles.len() > 0 {
                    let roles: HashSet<&str> = token_info
                        .data
                        .roles
                        .iter()
                        .filter(|(_, &v)| v)
                        .map(|(k, _)| k.as_str())
                        .collect();
                    if api_roles.is_disjoint(&roles) {
                        return Ok(ServiceResponse::from_err(
                            ErrResp::ErrPerm(Some("invalid role".to_string())),
                            http_req.clone(),
                        ));
                    }
                }
                if api_scopes.len() > 0 {
                    let api_scopes: HashSet<&str> = api_scopes.iter().map(|s| s.as_str()).collect();
                    let scopes: HashSet<&str> =
                        token_info.data.scopes.iter().map(|s| s.as_str()).collect();
                    if api_scopes.is_disjoint(&scopes) {
                        return Ok(ServiceResponse::from_err(
                            ErrResp::ErrPerm(Some("invalid scope".to_string())),
                            http_req.clone(),
                        ));
                    }
                }
            }
            req.extensions_mut().insert(token_info.data);

            let res = svc.call(req).await?;
            Ok(res)
        })
    }
}
