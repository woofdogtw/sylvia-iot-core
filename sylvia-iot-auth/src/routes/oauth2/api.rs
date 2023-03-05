use std::{borrow::Cow, sync::Arc};

use actix_web::{
    body::BoxBody,
    http::header,
    web::{self, Query},
    HttpResponse, Responder,
};
use chrono::{Duration, Utc};
use log::{error, warn};
use oxide_auth::{
    code_grant::{
        accesstoken::{Error as AccessTokenError, Request, TokenResponse},
        authorization::Error as AuthorizationError,
        authorization::Request as OxideAuthorizationRequest,
        refresh::Error as RefreshTokenError,
    },
    primitives::{
        grant::{Extensions, Grant},
        scope::Scope,
    },
};
use oxide_auth_async::code_grant::{self, access_token::Endpoint as TokenEndpoint};
use serde_urlencoded;
use tera::{Context, Tera};
use url::Url;

use sylvia_iot_corelib::strings;

use super::{
    super::State,
    endpoint::Endpoint,
    request::{
        self, AccessTokenRequest, AuthorizationRequest, GetAuthRequest, GetLoginRequest,
        PostLoginRequest, RefreshTokenRequest,
    },
    response::OAuth2Error,
};
use crate::models::{
    client::QueryCond as ClientQueryCond, user::QueryCond as UserQueryCond, Model,
};

pub const TMPL_LOGIN: &'static str = "login";
pub const TMPL_GRANT: &'static str = "grant";

const CONTENT_TYPE_JSON: &'static str = "application/json";

/// `GET /{base}/oauth2/auth`
///
/// Authenticate client and redirect to the login page.
pub async fn get_auth(req: GetAuthRequest, state: web::Data<State>) -> impl Responder {
    const FN_NAME: &'static str = "get_auth";

    if let Err(resp) = check_auth_params(FN_NAME, &req, &state.model).await {
        return resp;
    }

    let login_state: String = match serde_urlencoded::to_string(&req) {
        Err(e) => {
            let err_str = e.to_string();
            error!(
                "[{}] encode authorize state error: {}",
                FN_NAME,
                err_str.as_str()
            );
            return redirect_server_error(
                FN_NAME,
                req.redirect_uri.as_str(),
                Some(err_str.as_str()),
            );
        }
        Ok(str) => match serde_urlencoded::to_string(GetLoginRequest { state: str }) {
            Err(e) => {
                let err_str = e.to_string();
                error!(
                    "[{}] encode login state error: {}",
                    FN_NAME,
                    err_str.as_str()
                );
                return redirect_server_error(
                    FN_NAME,
                    req.redirect_uri.as_str(),
                    Some(err_str.as_str()),
                );
            }
            Ok(str) => str,
        },
    };
    resp_found(format!("{}/oauth2/login?{}", state.scope_path, login_state).as_str())
}

/// `GET /{base}/oauth2/login`
///
/// To render the login page.
pub async fn get_login(
    req: GetLoginRequest,
    state: web::Data<State>,
    tera: web::Data<Tera>,
) -> impl Responder {
    const FN_NAME: &'static str = "get_login";

    if req.state.as_str().len() == 0 {
        warn!("[{}] empty state content", FN_NAME);
        return resp_invalid_request(Some("invalid state content"));
    }
    match Query::<GetAuthRequest>::from_query(req.state.as_str()) {
        Err(e) => {
            warn!(
                "[{}] parse state error: {}, content: {}",
                FN_NAME,
                e,
                req.state.as_str()
            );
            return resp_invalid_request(Some("invalid state content"));
        }
        Ok(inner_req) => {
            if let Err(resp) = check_auth_params(FN_NAME, &inner_req, &state.model).await {
                return resp;
            }
        }
    }

    let mut context = Context::new();
    context.insert("scope_path", &state.scope_path);
    context.insert("state", &req.state);
    let page = match tera.render(TMPL_LOGIN, &context) {
        Err(e) => {
            let err_str = e.to_string();
            error!(
                "[{}] render login template error: {}",
                FN_NAME,
                err_str.as_str()
            );
            return resp_temporary_unavailable(Some(err_str));
        }
        Ok(page) => page,
    };

    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(page)
}

/// `POST /{base}/oauth2/login`
///
/// Do the login process.
pub async fn post_login(req: PostLoginRequest, state: web::Data<State>) -> impl Responder {
    const FN_NAME: &'static str = "post_login";

    if req.state.as_str().len() == 0 {
        warn!("[{}] empty state content", FN_NAME);
        return resp_invalid_request(Some("invalid state content"));
    }
    match Query::<GetAuthRequest>::from_query(req.state.as_str()) {
        Err(e) => {
            warn!(
                "[{}] parse state error: {}, content: {}",
                FN_NAME,
                e,
                req.state.as_str()
            );
            return resp_invalid_request(Some("invalid state content"));
        }
        Ok(inner_req) => {
            if let Err(resp) = check_auth_params(FN_NAME, &inner_req, &state.model).await {
                return resp;
            }
        }
    }

    let user_cond = UserQueryCond {
        user_id: None,
        account: Some(req.account.as_str()),
    };
    let user_id = match state.model.user().get(&user_cond).await {
        Err(e) => {
            let err_str = e.to_string();
            error!("[{}] get user DB error: {}", FN_NAME, err_str.as_str());
            return resp_temporary_unavailable(Some(err_str));
        }
        Ok(user) => match user {
            None => {
                return resp_invalid_auth(None);
            }
            Some(user) => {
                let hash = strings::password_hash(req.password.as_str(), user.salt.as_str());
                if user.password != hash {
                    return resp_invalid_auth(None);
                }
                user.user_id
            }
        },
    };

    resp_found(
        format!(
            "{}/oauth2/authorize?{}&user_id={}",
            state.scope_path, req.state, user_id
        )
        .as_str(),
    )
}

/// `GET /{base}/oauth2/authorize` and `POST /{base}/oauth2/authorize`
///
/// To render the OAuth2 grant page or to authorize the client and grant.
pub async fn authorize(
    req: AuthorizationRequest,
    state: web::Data<State>,
    tera: web::Data<Tera>,
) -> impl Responder {
    const FN_NAME: &'static str = "authorize";

    let mut endpoint = Endpoint::new(state.model.clone(), None);
    let pending = match code_grant::authorization::authorization_code(&mut endpoint, &req).await {
        Err(e) => match e {
            AuthorizationError::Ignore => {
                return resp_invalid_request(None);
            }
            AuthorizationError::Redirect(url) => {
                let url: Url = url.into();
                return resp_found(url.as_str());
            }
            AuthorizationError::PrimitiveError => {
                error!("[{}] authorize() with primitive error", FN_NAME);
                return resp_temporary_unavailable(None);
            }
        },
        Ok(pending) => pending,
    };

    if let Some(allowed) = req.allowed() {
        match allowed {
            false => {
                if let Err(e) = pending.deny() {
                    match e {
                        AuthorizationError::Redirect(url) => {
                            let url: Url = url.into();
                            return resp_found(url.as_str());
                        }
                        _ => (),
                    }
                }
                let e = OAuth2Error::new("server_error", Some("deny error".to_string()));
                return HttpResponse::InternalServerError().json(e);
            }
            true => {
                let user_id = req.user_id();
                match pending.authorize(&mut endpoint, Cow::from(user_id)).await {
                    Err(_) => {
                        error!("[{}] authorize error", FN_NAME);
                        return resp_temporary_unavailable(None);
                    }
                    Ok(url) => {
                        return resp_found(url.as_str());
                    }
                }
            }
        }
    }

    let client_id = req.client_id().unwrap();
    let client_cond = ClientQueryCond {
        user_id: None,
        client_id: Some(client_id.as_ref()),
    };
    let client_name = match state.model.client().get(&client_cond).await {
        Err(e) => {
            let err_str = e.to_string();
            error!("[{}] get client DB error: {}", FN_NAME, err_str.as_str());
            return resp_temporary_unavailable(Some(err_str));
        }
        Ok(client) => match client {
            None => {
                return resp_invalid_request(Some("invalid client"));
            }
            Some(client) => client.name,
        },
    };

    let mut context = Context::new();
    context.insert("scope_path", &state.scope_path);
    context.insert("client_name", &client_name);
    context.insert("user_id", req.user_id());
    context.insert("client_id", client_id.as_ref());
    context.insert("response_type", req.response_type().unwrap().as_ref());
    context.insert("redirect_uri", req.redirect_uri().unwrap().as_ref());
    context.insert("allow_value", request::ALLOW_VALUE);
    let scope = req.scope();
    if let Some(ref scope) = scope {
        context.insert("scope", scope);
    }
    let state = req.state();
    if let Some(ref state) = state {
        context.insert("state", state);
    }
    let page = match tera.render(TMPL_GRANT, &context) {
        Err(e) => {
            let err_str = e.to_string();
            error!("[{}] get client DB error: {}", FN_NAME, err_str.as_str());
            return resp_temporary_unavailable(Some(err_str));
        }
        Ok(page) => page,
    };
    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(page)
}

/// `POST /{base}/oauth2/token`
///
/// To generate an access token with the authorization code or client credentials.
pub async fn post_token(req: AccessTokenRequest, state: web::Data<State>) -> impl Responder {
    let mut endpoint = Endpoint::new(state.model.clone(), None);

    if let Some(grant_type) = req.grant_type() {
        if grant_type.eq("client_credentials") {
            return client_credentials_token(&req, state, &mut endpoint).await;
        }
    }

    let token = match code_grant::access_token::access_token(&mut endpoint, &req).await {
        Err(e) => match e {
            AccessTokenError::Invalid(desc) => {
                return HttpResponse::BadRequest()
                    .content_type(CONTENT_TYPE_JSON)
                    .body(desc.to_json())
            }
            AccessTokenError::Unauthorized(desc, authtype) => {
                return HttpResponse::Unauthorized()
                    .content_type(CONTENT_TYPE_JSON)
                    .insert_header((header::WWW_AUTHENTICATE, authtype))
                    .body(desc.to_json());
            }
            // TODO: handle this
            AccessTokenError::Primitive(e) => return HttpResponse::ServiceUnavailable().finish(),
        },
        Ok(token) => token,
    };
    HttpResponse::Ok()
        .content_type(CONTENT_TYPE_JSON)
        .body(token.to_json())
}

/// `POST /{base}/oauth2/refresh`
///
/// To refresh an access token.
pub async fn post_refresh(req: RefreshTokenRequest, state: web::Data<State>) -> impl Responder {
    let mut endpoint = Endpoint::new(state.model.clone(), None);
    let token = match code_grant::refresh::refresh(&mut endpoint, &req).await {
        Err(e) => match e {
            RefreshTokenError::Invalid(desc) => {
                return HttpResponse::BadRequest()
                    .content_type(CONTENT_TYPE_JSON)
                    .body(desc.to_json())
            }
            RefreshTokenError::Unauthorized(desc, authtype) => {
                return HttpResponse::Unauthorized()
                    .content_type(CONTENT_TYPE_JSON)
                    .insert_header((header::WWW_AUTHENTICATE, authtype))
                    .body(desc.to_json());
            }
            RefreshTokenError::Primitive => return HttpResponse::ServiceUnavailable().finish(),
        },
        Ok(token) => token,
    };
    HttpResponse::Ok()
        .content_type(CONTENT_TYPE_JSON)
        .body(token.to_json())
}

async fn client_credentials_token(
    req: &AccessTokenRequest,
    state: web::Data<State>,
    endpoint: &mut Endpoint,
) -> HttpResponse<BoxBody> {
    // Validate the client.
    let client = match req.authorization() {
        None => return resp_invalid_request(None),
        Some(auth) => {
            let cond = ClientQueryCond {
                client_id: Some(auth.0.as_ref()),
                ..Default::default()
            };
            let client = match state.model.client().get(&cond).await {
                Err(e) => return resp_temporary_unavailable(Some(format!("{}", e))),
                Ok(client) => match client {
                    None => return resp_invalid_client(None),
                    Some(client) => client,
                },
            };
            match client.client_secret.as_ref() {
                None => return resp_invalid_client(None),
                Some(secret) => match secret.as_bytes().eq(auth.1.as_ref()) {
                    false => return resp_invalid_client(None),
                    true => client,
                },
            }
        }
    };

    // Reuse the issuer to generate tokens.
    let grant = Grant {
        owner_id: client.user_id,
        client_id: client.client_id,
        scope: match client.scopes.as_slice().join(" ").parse() {
            Err(_) => return resp_invalid_client(Some("no valid scope")),
            Ok(scope) => scope,
        },
        redirect_uri: match client.redirect_uris.get(0) {
            None => return resp_invalid_client(Some("no valid redirect_uri")),
            Some(uri) => match Url::parse(uri.as_str()) {
                Err(_) => return resp_invalid_client(Some("invalid redirect_uri")),
                Ok(uri) => uri,
            },
        },
        until: Utc::now() + Duration::minutes(10),
        extensions: Extensions::new(),
    };
    let token = match endpoint.issuer().issue(grant).await {
        Err(_) => return resp_temporary_unavailable(None),
        Ok(token) => token,
    };

    HttpResponse::Ok().json(&TokenResponse {
        access_token: Some(token.token),
        refresh_token: token.refresh,
        token_type: Some("bearer".to_string()),
        expires_in: Some(token.until.signed_duration_since(Utc::now()).num_seconds()),
        scope: Some(client.scopes.as_slice().join(" ")),
        error: None,
    })
}

/// To check the authorization grant flow parameters.
async fn check_auth_params(
    fn_name: &str,
    req: &GetAuthRequest,
    model: &Arc<dyn Model>,
) -> Result<(), HttpResponse> {
    if req.response_type != "code" {
        return Err(resp_invalid_request(Some("unsupport response_type")));
    }

    let client_cond = ClientQueryCond {
        user_id: None,
        client_id: Some(req.client_id.as_str()),
    };
    match model.client().get(&client_cond).await {
        Err(e) => {
            let err_str = e.to_string();
            error!("[{}] get client DB error: {}", fn_name, err_str.as_str());
            return Err(resp_temporary_unavailable(Some(err_str)));
        }
        Ok(client) => match client {
            None => {
                return Err(resp_invalid_request(Some("invalid client")));
            }
            Some(client) => {
                if !client.redirect_uris.contains(&req.redirect_uri) {
                    return Err(resp_invalid_request(Some("invalid redirect_uri")));
                } else if client.scopes.len() > 0 {
                    if req.scope.is_none() {
                        return Err(redirect_invalid_scope(&req.redirect_uri));
                    }
                    let req_scopes = match req.scope.as_ref().unwrap().parse::<Scope>() {
                        Err(e) => {
                            return Err(redirect_invalid_scope(&req.redirect_uri));
                        }
                        Ok(scopes) => scopes,
                    };
                    let client_scopes = match client.scopes.join(" ").parse::<Scope>() {
                        Err(e) => {
                            error!("[{}] parse client scopes error: {}", fn_name, e);
                            return Err(redirect_server_error(fn_name, &req.redirect_uri, None));
                        }
                        Ok(scopes) => scopes,
                    };
                    if !req_scopes.allow_access(&client_scopes) {
                        return Err(redirect_invalid_scope(&req.redirect_uri));
                    }
                }
            }
        },
    }
    Ok(())
}

fn redirect_invalid_scope(redirect_uri: &str) -> HttpResponse {
    resp_found(format!("{}?error=invalid_scope", redirect_uri).as_str())
}

fn redirect_server_error(
    fn_name: &str,
    redirect_uri: &str,
    description: Option<&str>,
) -> HttpResponse {
    let location = match description {
        None => format!("{}?error=server_error", redirect_uri),
        Some(desc) => {
            let err_desc = [("error_description", desc)];
            match serde_urlencoded::to_string(&err_desc) {
                Err(e) => {
                    error!("[{}] redirect server_error encode error: {}", fn_name, e);
                    format!("{}?error=server_error", redirect_uri)
                }
                Ok(qs) => format!("{}?error=server_error&{}", redirect_uri, qs),
            }
        }
    };
    resp_found(location.as_str())
}

fn resp_found<'a>(location: &'a str) -> HttpResponse {
    HttpResponse::Found()
        .insert_header((header::LOCATION, location))
        .finish()
}

fn resp_invalid_auth<'a>(description: Option<&'a str>) -> HttpResponse {
    let description = match description {
        None => None,
        Some(description) => Some(description.to_string()),
    };
    HttpResponse::BadRequest().json(OAuth2Error::new("invalid_auth", description))
}

fn resp_invalid_client<'a>(description: Option<&'a str>) -> HttpResponse {
    let description = match description {
        None => None,
        Some(description) => Some(description.to_string()),
    };
    HttpResponse::Unauthorized().json(OAuth2Error::new("invalid_client", description))
}

fn resp_invalid_request<'a>(description: Option<&'a str>) -> HttpResponse {
    let description = match description {
        None => None,
        Some(description) => Some(description.to_string()),
    };
    HttpResponse::BadRequest().json(OAuth2Error::new("invalid_request", description))
}

fn resp_temporary_unavailable(description: Option<String>) -> HttpResponse {
    HttpResponse::ServiceUnavailable()
        .json(OAuth2Error::new("temporarily_unavailable", description))
}
