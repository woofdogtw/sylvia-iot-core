use std::{borrow::Cow, str};

use actix_web::{
    dev::Payload,
    http::{header, Method},
    web::{Form, Query},
    FromRequest, HttpRequest,
};
use base64::{engine::general_purpose, Engine};
use futures::future::{FutureExt, LocalBoxFuture};
use oxide_auth::code_grant::{
    accesstoken::Request as OxideAccessTokenRequest,
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

impl GetAuthRequest {
    pub async fn new(req: HttpRequest) -> Result<Self, OAuth2Error> {
        match Query::<GetAuthRequest>::from_query(req.query_string()) {
            Err(e) => Err(OAuth2Error::new_request(Some(e.to_string()))),
            Ok(request) => Ok(request.into_inner()),
        }
    }
}

impl FromRequest for GetAuthRequest {
    type Error = OAuth2Error;
    type Future = LocalBoxFuture<'static, Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        Self::new(req.clone()).boxed_local()
    }
}

impl GetLoginRequest {
    pub async fn new(req: HttpRequest) -> Result<Self, OAuth2Error> {
        match Query::<GetLoginRequest>::from_query(req.query_string()) {
            Err(e) => Err(OAuth2Error::new_request(Some(e.to_string()))),
            Ok(request) => Ok(request.into_inner()),
        }
    }
}

impl FromRequest for GetLoginRequest {
    type Error = OAuth2Error;
    type Future = LocalBoxFuture<'static, Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        Self::new(req.clone()).boxed_local()
    }
}

impl PostLoginRequest {
    pub async fn new(req: HttpRequest, mut payload: Payload) -> Result<Self, OAuth2Error> {
        match Form::<PostLoginRequest>::from_request(&req, &mut payload).await {
            Err(e) => Err(OAuth2Error::new_request(Some(e.to_string()))),
            Ok(request) => Ok(request.into_inner()),
        }
    }
}

impl FromRequest for PostLoginRequest {
    type Error = OAuth2Error;
    type Future = LocalBoxFuture<'static, Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, payload: &mut Payload) -> Self::Future {
        Self::new(req.clone(), payload.take()).boxed_local()
    }
}

impl AuthorizationRequest {
    pub async fn new(req: HttpRequest, mut payload: Payload) -> Result<Self, OAuth2Error> {
        let request = match *req.method() {
            Method::GET => {
                match Query::<AuthorizationRequest>::from_request(&req, &mut payload).await {
                    Err(e) => {
                        return Err(OAuth2Error::new_request(Some(e.to_string())));
                    }
                    Ok(request) => request.into_inner(),
                }
            }
            Method::POST => {
                match Form::<AuthorizationRequest>::from_request(&req, &mut payload).await {
                    Err(e) => {
                        return Err(OAuth2Error::new_request(Some(e.to_string())));
                    }
                    Ok(request) => request.into_inner(),
                }
            }
            _ => return Err(OAuth2Error::new_request(Some("invalid method".to_string()))),
        };
        Ok(request)
    }

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

impl FromRequest for AuthorizationRequest {
    type Error = OAuth2Error;
    type Future = LocalBoxFuture<'static, Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, payload: &mut Payload) -> Self::Future {
        Self::new(req.clone(), payload.take()).boxed_local()
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

impl AccessTokenRequest {
    pub async fn new(req: HttpRequest, mut payload: Payload) -> Result<Self, OAuth2Error> {
        let mut request = match Form::<AccessTokenRequest>::from_request(&req, &mut payload).await {
            Err(e) => {
                return Err(OAuth2Error::new_request(Some(e.to_string())));
            }
            Ok(request) => request.into_inner(),
        };
        request.authorization = match parse_basic_auth(&req) {
            Err(e) => return Err(e),
            Ok(auth) => auth,
        };
        Ok(request)
    }
}

impl FromRequest for AccessTokenRequest {
    type Error = OAuth2Error;
    type Future = LocalBoxFuture<'static, Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, payload: &mut Payload) -> Self::Future {
        Self::new(req.clone(), payload.take()).boxed_local()
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

    fn authorization(&self) -> Option<(Cow<str>, Cow<[u8]>)> {
        match self.authorization.as_ref() {
            None => None,
            Some(auth) => Some((Cow::from(auth.0.as_str()), Cow::from(auth.1.as_slice()))),
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

impl RefreshTokenRequest {
    pub async fn new(req: HttpRequest, mut payload: Payload) -> Result<Self, OAuth2Error> {
        let mut request = match Form::<RefreshTokenRequest>::from_request(&req, &mut payload).await
        {
            Err(e) => {
                return Err(OAuth2Error::new_request(Some(e.to_string())));
            }
            Ok(request) => request.into_inner(),
        };
        request.authorization = match parse_basic_auth(&req) {
            Err(e) => return Err(e),
            Ok(auth) => auth,
        };
        Ok(request)
    }
}

impl FromRequest for RefreshTokenRequest {
    type Error = OAuth2Error;
    type Future = LocalBoxFuture<'static, Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, payload: &mut Payload) -> Self::Future {
        Self::new(req.clone(), payload.take()).boxed_local()
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

fn parse_basic_auth(req: &HttpRequest) -> Result<Option<(String, Vec<u8>)>, OAuth2Error> {
    let mut auth_all = req.headers().get_all(header::AUTHORIZATION);
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
