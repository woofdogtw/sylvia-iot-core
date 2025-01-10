use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    str,
    sync::Arc,
    task::{Context, Poll},
};

use axum::{
    extract::{FromRequest, Request},
    http::{header, Method, StatusCode},
    response::{IntoResponse, Response},
};
use futures::future::BoxFuture;
use oxide_auth::code_grant::resource::{Error as ResourceError, Request as OxideResourceRequest};
use oxide_auth_async::code_grant;
use tower::{Layer, Service};

use sylvia_iot_corelib::{err::ErrResp, http::parse_header_auth};

use super::endpoint::Endpoint;
use crate::models::{
    client::QueryCond as ClientQueryCond, user::QueryCond as UserQueryCond, Model,
};

pub type RoleScopeType = (Vec<&'static str>, Vec<String>);
type RoleScopeInner = (HashSet<&'static str>, HashSet<String>);

#[derive(Clone)]
pub struct AuthService {
    model: Arc<dyn Model>,
    role_scopes: HashMap<Method, RoleScopeType>,
}

#[derive(Clone)]
pub struct AuthMiddleware<S> {
    endpoint: Endpoint,
    model: Arc<dyn Model>,
    role_scopes: HashMap<Method, RoleScopeInner>,
    service: S,
}

struct ResourceRequest {
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
            endpoint: Endpoint::new(self.model.clone(), Some("")),
            model: self.model.clone(),
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
        let mut endpoint = self.endpoint.clone();
        let model = self.model.clone();
        let role_scopes = self.role_scopes.clone();

        Box::pin(async move {
            let auth_req = match ResourceRequest::new(&req) {
                Err(e) => return Ok(e.into_response()),
                Ok(req) => match req.token().is_none() {
                    false => req,
                    true => {
                        let e = ErrResp::ErrParam(Some("missing token".to_string()));
                        return Ok(e.into_response());
                    }
                },
            };
            let grant = match code_grant::resource::protect(&mut endpoint, &auth_req).await {
                Err(e) => match e {
                    ResourceError::PrimitiveError => {
                        return Ok(ErrResp::ErrDb(None).into_response());
                    }
                    _ => {
                        return Ok((
                            StatusCode::UNAUTHORIZED,
                            [(header::WWW_AUTHENTICATE, e.www_authenticate())],
                        )
                            .into_response());
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
                    return Ok(ErrResp::ErrDb(Some(e.to_string())).into_response());
                }
                Ok(user) => match user {
                    None => {
                        let e = ErrResp::ErrPerm(Some("user not exist".to_string()));
                        return Ok(e.into_response());
                    }
                    Some(user) => {
                        if let Some((api_roles, api_scopes)) = role_scopes.get(req.method()) {
                            if api_roles.len() > 0 {
                                let roles: HashSet<&str> = user
                                    .roles
                                    .iter()
                                    .filter(|(_, &v)| v)
                                    .map(|(k, _)| k.as_str())
                                    .collect();
                                if api_roles.is_disjoint(&roles) {
                                    let e = ErrResp::ErrPerm(Some("invalid role".to_string()));
                                    return Ok(e.into_response());
                                }
                            }
                            if api_scopes.len() > 0 {
                                let api_scopes: HashSet<&str> =
                                    api_scopes.iter().map(|s| s.as_str()).collect();
                                let scopes: HashSet<&str> = grant.scope.iter().map(|s| s).collect();
                                if api_scopes.is_disjoint(&scopes) {
                                    return Ok(ErrResp::ErrPerm(Some("invalid scope".to_string()))
                                        .into_response());
                                }
                            }
                        }
                        user
                    }
                },
            };
            req.extensions_mut().insert(user);

            let cond = ClientQueryCond {
                client_id: Some(grant.client_id.as_str()),
                ..Default::default()
            };
            let client = match model.client().get(&cond).await {
                Err(e) => {
                    return Ok(ErrResp::ErrDb(Some(e.to_string())).into_response());
                }
                Ok(client) => match client {
                    None => {
                        let e = ErrResp::ErrPerm(Some("client not exist".to_string()));
                        return Ok(e.into_response());
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
    fn new(req: &Request) -> Result<Self, ErrResp> {
        match parse_header_auth(req) {
            Err(e) => Err(e),
            Ok(auth) => Ok(ResourceRequest {
                authorization: auth,
            }),
        }
    }
}

impl<S> FromRequest<S> for ResourceRequest
where
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request(req: Request, _state: &S) -> Result<Self, Self::Rejection> {
        match parse_header_auth(&req) {
            Err(e) => Err(e.into_response()),
            Ok(auth) => Ok(ResourceRequest {
                authorization: auth,
            }),
        }
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
