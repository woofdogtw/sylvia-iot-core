//! Provides the authentication middleware by sending the Bearer token to [`sylvia-auth`].

use std::{
    cell::RefCell,
    collections::HashMap,
    rc::Rc,
    task::{Context, Poll},
};

use actix_service::{Service, Transform};
use actix_web::{
    body::BoxBody,
    dev::{ServiceRequest, ServiceResponse},
    http::header,
    Error, HttpMessage, HttpRequest,
};
use futures::future::{self, LocalBoxFuture, Ready};
use reqwest;
use serde::{self, Deserialize};

use crate::util::err::ErrResp;

/// The information contains [`GetTokenInfoData`] and access token.
#[derive(Clone)]
pub struct FullTokenInfo {
    pub token: String,
    pub info: GetTokenInfoData,
}

/// The user/client information of the token.
#[derive(Clone, Deserialize)]
pub struct GetTokenInfo {
    pub data: GetTokenInfoData,
}

#[derive(Clone, Deserialize)]
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
}

pub struct AuthMiddleware<S> {
    client: reqwest::Client,
    auth_uri: String,
    service: Rc<RefCell<S>>,
}

impl AuthService {
    pub fn new(auth_uri: String) -> Self {
        AuthService { auth_uri }
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
        future::ok(AuthMiddleware {
            client: reqwest::Client::new(),
            auth_uri: self.auth_uri.clone(),
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

        Box::pin(async move {
            let (http_req, _) = req.parts_mut();
            let token = match parse_header_auth(&http_req) {
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

            req.extensions_mut().insert(FullTokenInfo {
                token,
                info: token_info.data,
            });

            let res = svc.call(req).await?;
            Ok(res)
        })
    }
}

/// Parse Authorization header content. Returns `None` means no Authorization header.
pub fn parse_header_auth(req: &HttpRequest) -> Result<Option<String>, ErrResp> {
    let mut auth_all = req.headers().get_all(header::AUTHORIZATION);
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
