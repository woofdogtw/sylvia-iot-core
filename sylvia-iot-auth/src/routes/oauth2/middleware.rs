use std::{
    borrow::Cow,
    cell::RefCell,
    collections::{HashMap, HashSet},
    rc::Rc,
    str,
    sync::Arc,
    task::{Context, Poll},
};

use actix_service::{Service, Transform};
use actix_web::{
    body::BoxBody,
    dev::{Payload, ServiceRequest, ServiceResponse},
    http::{header, Method},
    Error, FromRequest, HttpMessage, HttpRequest, HttpResponse,
};
use futures::future::{self, FutureExt, LocalBoxFuture, Ready};
use oxide_auth::code_grant::resource::{Error as ResourceError, Request as OxideResourceRequest};
use oxide_auth_async::code_grant;

use sylvia_iot_corelib::err::ErrResp;

use super::endpoint::Endpoint;
use crate::models::{
    client::QueryCond as ClientQueryCond, user::QueryCond as UserQueryCond, Model,
};

pub type RoleScopeType = (Vec<&'static str>, Vec<String>);
type RoleScopeInner = (HashSet<&'static str>, HashSet<String>);

pub struct AuthService {
    model: Arc<dyn Model>,
    role_scopes: HashMap<Method, RoleScopeType>,
}

pub struct AuthMiddleware<S> {
    endpoint: Endpoint,
    model: Arc<dyn Model>,
    role_scopes: HashMap<Method, RoleScopeInner>,
    service: Rc<RefCell<S>>,
}

pub struct ResourceRequest {
    authorization: Option<String>,
}

impl AuthService {
    pub fn new(model: &Arc<dyn Model>, role_scopes: HashMap<Method, RoleScopeType>) -> Self {
        AuthService {
            model: model.clone(),
            role_scopes,
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
            endpoint: Endpoint::new(self.model.clone(), Some("")),
            model: self.model.clone(),
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
        let mut endpoint = self.endpoint.clone();
        let model = self.model.clone();
        let role_scopes = self.role_scopes.clone();

        Box::pin(async move {
            let (http_req, _) = req.parts_mut();
            let auth_req = match ResourceRequest::new(&http_req) {
                Err(e) => return Ok(ServiceResponse::from_err(e, http_req.clone())),
                Ok(req) => req,
            };
            let grant = match code_grant::resource::protect(&mut endpoint, &auth_req).await {
                Err(e) => match e {
                    ResourceError::PrimitiveError => {
                        return Ok(ServiceResponse::from_err(
                            ErrResp::ErrDb(None),
                            http_req.clone(),
                        ));
                    }
                    _ => {
                        return Ok(ServiceResponse::new(
                            http_req.clone(),
                            HttpResponse::Unauthorized()
                                .insert_header((header::WWW_AUTHENTICATE, e.www_authenticate()))
                                .finish(),
                        ));
                    }
                },
                Ok(grant) => grant,
            };

            let cond = UserQueryCond {
                user_id: Some(grant.owner_id.as_str()),
                account: None,
            };
            let user = match model.user().get(&cond).await {
                Err(e) => {
                    return Ok(ServiceResponse::from_err(
                        ErrResp::ErrDb(Some(e.to_string())),
                        http_req.clone(),
                    ))
                }
                Ok(user) => match user {
                    None => {
                        return Ok(ServiceResponse::from_err(
                            ErrResp::ErrPerm(Some("user not exist".to_string())),
                            http_req.clone(),
                        ))
                    }
                    Some(user) => {
                        if let Some((api_roles, api_scopes)) = role_scopes.get(http_req.method()) {
                            if api_roles.len() > 0 {
                                let roles: HashSet<&str> = user
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
                                let api_scopes: HashSet<&str> =
                                    api_scopes.iter().map(|s| s.as_str()).collect();
                                let scopes: HashSet<&str> = grant.scope.iter().map(|s| s).collect();
                                if api_scopes.is_disjoint(&scopes) {
                                    return Ok(ServiceResponse::from_err(
                                        ErrResp::ErrPerm(Some("invalid scope".to_string())),
                                        http_req.clone(),
                                    ));
                                }
                            }
                        }
                        user
                    }
                },
            };
            req.extensions_mut().insert(user);

            let (http_req, _) = req.parts_mut();
            let cond = ClientQueryCond {
                client_id: Some(grant.client_id.as_str()),
                ..Default::default()
            };
            let client = match model.client().get(&cond).await {
                Err(e) => {
                    return Ok(ServiceResponse::from_err(
                        ErrResp::ErrDb(Some(e.to_string())),
                        http_req.clone(),
                    ))
                }
                Ok(client) => match client {
                    None => {
                        return Ok(ServiceResponse::from_err(
                            ErrResp::ErrPerm(Some("client not exist".to_string())),
                            http_req.clone(),
                        ))
                    }
                    Some(client) => client,
                },
            };
            req.extensions_mut().insert(client);

            let res = svc.call(req).await?;
            Ok(res)
        })
    }
}

impl ResourceRequest {
    pub fn new(req: &HttpRequest) -> Result<Self, ErrResp> {
        match parse_bearer_auth(req) {
            Err(e) => Err(e),
            Ok(auth) => Ok(ResourceRequest {
                authorization: auth,
            }),
        }
    }

    pub async fn new_async(req: HttpRequest) -> Result<Self, ErrResp> {
        match parse_bearer_auth(&req) {
            Err(e) => Err(e),
            Ok(auth) => Ok(ResourceRequest {
                authorization: auth,
            }),
        }
    }
}

impl FromRequest for ResourceRequest {
    type Error = ErrResp;
    type Future = LocalBoxFuture<'static, Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        Self::new_async(req.clone()).boxed_local()
    }
}

impl OxideResourceRequest for ResourceRequest {
    fn valid(&self) -> bool {
        true
    }

    fn token(&self) -> Option<Cow<str>> {
        match self.authorization.as_deref() {
            None => None,
            Some(auth) => Some(Cow::from(auth)),
        }
    }
}

fn parse_bearer_auth(req: &HttpRequest) -> Result<Option<String>, ErrResp> {
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
