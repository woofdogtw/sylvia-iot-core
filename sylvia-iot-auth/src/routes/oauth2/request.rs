use std::{borrow::Cow, str};

use async_trait::async_trait;
use axum::{
    body::Bytes,
    extract::{Form, FromRequest, Query, Request},
    http::{header, Method},
    response::{IntoResponse, Response},
};
use base64::{engine::general_purpose, Engine};
use oxide_auth::code_grant::{
    accesstoken::{Authorization, Request as OxideAccessTokenRequest},
    authorization::Request as OxideAuthorizationRequest,
    refresh::Request as OxideRefreshTokenRequest,
};
use serde::{Deserialize, Serialize};

use super::response::OAuth2Error;

#[derive(Deserialize, Serialize)]
pub struct GetAuthRequest {
    pub response_type: String,
    pub client_id: String,
    pub redirect_uri: String,
    pub scope: Option<String>,
    pub state: Option<String>,
}

#[derive(Deserialize, Serialize)]
pub struct GetLoginRequest {
    pub state: String,
}

#[derive(Deserialize)]
pub struct PostLoginRequest {
    pub account: String,
    pub password: String,
    pub state: String,
}

#[derive(Deserialize, Serialize)]
pub struct AuthorizationRequest {
    response_type: String,
    client_id: String,
    redirect_uri: String,
    scope: Option<String>,
    state: Option<String>,
    session_id: String,
    allow: Option<String>,
}

#[derive(Deserialize, Serialize)]
pub struct AccessTokenRequest {
    #[serde(skip)]
    authorization: Option<(String, Vec<u8>)>,
    grant_type: String,
    code: Option<String>,         // for authorization code grant flow
    redirect_uri: Option<String>, // for authorization code grant flow
    client_id: Option<String>,
    scope: Option<String>, // for client credentials grant flow
}

#[derive(Deserialize, Serialize)]
pub struct RefreshTokenRequest {
    #[serde(skip)]
    authorization: Option<(String, Vec<u8>)>,
    grant_type: String,
    refresh_token: String,
    scope: Option<String>,
    client_id: Option<String>,
}

pub const ALLOW_VALUE: &'static str = "yes";

#[async_trait]
impl<S> FromRequest<S> for GetAuthRequest
where
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        match Query::<GetAuthRequest>::from_request(req, state).await {
            Err(e) => Err(OAuth2Error::new_request(Some(e.to_string())).into_response()),
            Ok(request) => Ok(request.0),
        }
    }
}

#[async_trait]
impl<S> FromRequest<S> for GetLoginRequest
where
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        match Query::<GetLoginRequest>::from_request(req, state).await {
            Err(e) => Err(OAuth2Error::new_request(Some(e.to_string())).into_response()),
            Ok(request) => Ok(request.0),
        }
    }
}

#[async_trait]
impl<S> FromRequest<S> for PostLoginRequest
where
    Bytes: FromRequest<S>,
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        match Form::<PostLoginRequest>::from_request(req, state).await {
            Err(e) => Err(OAuth2Error::new_request(Some(e.to_string())).into_response()),
            Ok(body) => Ok(body.0),
        }
    }
}

impl AuthorizationRequest {
    pub fn session_id(&self) -> &str {
        self.session_id.as_str()
    }

    pub fn allowed(&self) -> Option<bool> {
        if let Some(allow_str) = self.allow.as_deref() {
            return Some(allow_str == ALLOW_VALUE);
        }
        None
    }
}

#[async_trait]
impl<S> FromRequest<S> for AuthorizationRequest
where
    Bytes: FromRequest<S>,
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        match *req.method() {
            Method::GET => match Query::<AuthorizationRequest>::from_request(req, state).await {
                Err(e) => Err(OAuth2Error::new_request(Some(e.to_string())).into_response()),
                Ok(request) => Ok(request.0),
            },
            Method::POST => match Form::<AuthorizationRequest>::from_request(req, state).await {
                Err(e) => Err(OAuth2Error::new_request(Some(e.to_string())).into_response()),
                Ok(request) => Ok(request.0),
            },
            _ => Err(OAuth2Error::new_request(Some("invalid method".to_string())).into_response()),
        }
    }
}

impl OxideAuthorizationRequest for AuthorizationRequest {
    fn valid(&self) -> bool {
        true
    }

    fn client_id(&self) -> Option<Cow<str>> {
        Some(Cow::from(self.client_id.as_str()))
    }

    fn scope(&self) -> Option<Cow<str>> {
        match self.scope.as_ref() {
            None => None,
            Some(scope) => Some(Cow::from(scope)),
        }
    }

    fn redirect_uri(&self) -> Option<Cow<str>> {
        Some(Cow::from(&self.redirect_uri))
    }

    fn state(&self) -> Option<Cow<str>> {
        match self.state.as_ref() {
            None => None,
            Some(state) => Some(Cow::from(state)),
        }
    }

    fn response_type(&self) -> Option<Cow<str>> {
        Some(Cow::from(&self.response_type))
    }

    fn extension(&self, _key: &str) -> Option<Cow<str>> {
        None
    }
}

#[async_trait]
impl<S> FromRequest<S> for AccessTokenRequest
where
    Bytes: FromRequest<S>,
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let authorization = match parse_basic_auth(&req) {
            Err(e) => return Err(e.into_response()),
            Ok(auth) => auth,
        };
        let mut request = match Form::<AccessTokenRequest>::from_request(req, state).await {
            Err(e) => return Err(OAuth2Error::new_request(Some(e.to_string())).into_response()),
            Ok(request) => request.0,
        };
        request.authorization = authorization;
        Ok(request)
    }
}

impl OxideAccessTokenRequest for AccessTokenRequest {
    fn valid(&self) -> bool {
        true
    }

    fn code(&self) -> Option<Cow<str>> {
        match self.code.as_ref() {
            None => None,
            Some(code) => Some(Cow::from(code)),
        }
    }

    fn authorization(&self) -> Authorization {
        match self.authorization.as_ref() {
            None => Authorization::None,
            Some(auth) => match auth.1.len() {
                0 => Authorization::Username(Cow::from(auth.0.as_str())),
                _ => Authorization::UsernamePassword(
                    Cow::from(auth.0.as_str()),
                    Cow::from(auth.1.as_slice()),
                ),
            },
        }
    }

    fn client_id(&self) -> Option<Cow<str>> {
        match self.client_id.as_ref() {
            None => None,
            Some(id) => Some(Cow::from(id)),
        }
    }

    fn redirect_uri(&self) -> Option<Cow<str>> {
        match self.redirect_uri.as_ref() {
            None => None,
            Some(uri) => Some(Cow::from(uri)),
        }
    }

    fn grant_type(&self) -> Option<Cow<str>> {
        Some(Cow::from(&self.grant_type))
    }

    fn extension(&self, _key: &str) -> Option<Cow<str>> {
        None
    }

    fn allow_credentials_in_body(&self) -> bool {
        false
    }
}

#[async_trait]
impl<S> FromRequest<S> for RefreshTokenRequest
where
    Bytes: FromRequest<S>,
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let authorization = match parse_basic_auth(&req) {
            Err(e) => return Err(e.into_response()),
            Ok(auth) => auth,
        };
        let mut request = match Form::<RefreshTokenRequest>::from_request(req, state).await {
            Err(e) => {
                return Err(OAuth2Error::new_request(Some(e.to_string())).into_response());
            }
            Ok(request) => request.0,
        };
        request.authorization = authorization;
        Ok(request)
    }
}

impl OxideRefreshTokenRequest for RefreshTokenRequest {
    fn valid(&self) -> bool {
        true
    }

    fn refresh_token(&self) -> Option<Cow<str>> {
        Some(Cow::from(&self.refresh_token))
    }

    fn scope(&self) -> Option<Cow<str>> {
        match self.scope.as_ref() {
            None => None,
            Some(scope) => Some(Cow::from(scope)),
        }
    }

    fn grant_type(&self) -> Option<Cow<str>> {
        Some(Cow::from(&self.grant_type))
    }

    fn authorization(&self) -> Option<(Cow<str>, Cow<[u8]>)> {
        match self.authorization.as_ref() {
            None => None,
            Some(auth) => Some((Cow::from(auth.0.as_str()), Cow::from(auth.1.as_slice()))),
        }
    }

    fn extension(&self, _key: &str) -> Option<Cow<str>> {
        None
    }
}

fn parse_basic_auth(req: &Request) -> Result<Option<(String, Vec<u8>)>, OAuth2Error> {
    let mut auth_all = req.headers().get_all(header::AUTHORIZATION).iter();
    let auth = match auth_all.next() {
        None => return Ok(None),
        Some(auth) => match auth.to_str() {
            Err(e) => return Err(OAuth2Error::new_request(Some(e.to_string()))),
            Ok(auth) => auth,
        },
    };
    if auth_all.next() != None {
        return Err(OAuth2Error::new_request(Some(
            "invalid multiple Authorization header".to_string(),
        )));
    } else if !auth.starts_with("Basic ") || auth.len() < 7 {
        return Err(OAuth2Error::new_request(Some(
            "not a Basic header".to_string(),
        )));
    }
    let auth = match general_purpose::STANDARD.decode(&auth[6..]) {
        Err(e) => match general_purpose::STANDARD_NO_PAD.decode(&auth[6..]) {
            Err(_) => {
                return Err(OAuth2Error::new_request(Some(format!(
                    "invalid Basic content: {}",
                    e
                ))))
            }
            Ok(auth) => auth,
        },
        Ok(auth) => auth,
    };
    let mut split = auth.splitn(2, |&c| c == b':');
    let user = match split.next() {
        None => {
            return Err(OAuth2Error::new_request(Some(
                "invalid Basic content".to_string(),
            )))
        }
        Some(user) => user,
    };
    let pass = match split.next() {
        None => {
            return Err(OAuth2Error::new_request(Some(
                "invalid Basic content".to_string(),
            )))
        }
        Some(pass) => pass,
    };
    let user = match str::from_utf8(user) {
        Err(e) => {
            return Err(OAuth2Error::new_request(Some(format!(
                "invalid Basic content: {}",
                e
            ))))
        }
        Ok(user) => user,
    };
    Ok(Some((user.to_string(), pass.to_vec())))
}
