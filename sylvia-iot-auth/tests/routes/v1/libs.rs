use axum::{
    http::{header, HeaderValue, Method, StatusCode},
    Router,
};
use axum_test::TestServer;
use base64::Engine;
use laboratory::expect;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use tokio::runtime::Runtime;

use sylvia_iot_auth::routes;
use sylvia_iot_corelib::err;

use super::super::read_location;

#[derive(Deserialize)]
pub struct ApiError {
    pub code: String,
    pub message: Option<String>,
}

#[derive(Debug, Serialize)]
struct PostLoginRequest<'a> {
    account: &'a str,
    password: &'a str,
    state: &'a str,
}

#[derive(Debug, Serialize)]
struct PostLoginStateParam<'a> {
    response_type: &'a str,
    redirect_uri: &'a str,
    client_id: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    scope: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    state: Option<&'a str>,
}

#[derive(Debug, Serialize)]
struct PostAuthorizeRequest<'a> {
    response_type: &'a str,
    redirect_uri: &'a str,
    client_id: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    scope: Option<&'a str>,
    session_id: &'a str,
    allow: &'a str,
}

#[derive(Deserialize)]
struct PostLoginLocation {
    session_id: String,
}

#[derive(Deserialize)]
struct PostAuthorizeLocation {
    code: String,
}

#[derive(Debug, Serialize)]
struct PostTokenRequest<'a> {
    grant_type: &'a str,
    code: &'a str,
    redirect_uri: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    client_id: Option<&'a str>,
}

#[derive(Deserialize)]
struct OAuth2Error {
    error: String,
    #[serde(rename = "error_description")]
    _error_description: Option<String>,
}

#[derive(Deserialize)]
struct AccessToken {
    access_token: String,
    #[serde(rename = "refresh_token")]
    _refresh_token: String,
    #[serde(rename = "token_type")]
    _token_type: String,
    #[serde(rename = "expires_in")]
    _expires_in: u64,
}

pub fn get_token(
    runtime: &Runtime,
    state: &routes::State,
    user_id: &str,
) -> Result<String, String> {
    get_token_client_id(runtime, state, user_id, "public", None, None)
}

pub fn get_token_client_id(
    runtime: &Runtime,
    state: &routes::State,
    user_id: &str,
    client_id: &str,
    client_secret: Option<&str>,
    scope: Option<&str>,
) -> Result<String, String> {
    let app = Router::new().merge(routes::new_service(&state));
    let server = match TestServer::new(app) {
        Err(e) => return Err(format!("new server error: {}", e)),
        Ok(server) => server,
    };

    // Login to get session ID.
    let state = PostLoginStateParam {
        response_type: "code",
        redirect_uri: &crate::TEST_REDIRECT_URI,
        client_id,
        scope,
        state: Some("/"),
    };
    let state = serde_urlencoded::to_string(state).unwrap();
    let params = PostLoginRequest {
        account: user_id,
        password: user_id,
        state: state.as_str(),
    };
    let req = server.post("/auth/oauth2/login").form(&params);
    let resp = runtime.block_on(async { req.await });
    let status = resp.status_code();
    if status != StatusCode::FOUND {
        let body = resp.text();
        let body = String::from_utf8(body.as_bytes().to_vec()).unwrap();
        return Err(format!("post login response not 302, {} {}", status, body));
    }
    let location = read_location(&resp)?;
    let session_id = match location.query() {
        None => return Err("302 with no query content".to_string()),
        Some(query) => {
            if let Ok(resp) = serde_urlencoded::from_str::<OAuth2Error>(query) {
                if !location.as_str().starts_with(crate::TEST_REDIRECT_URI) {
                    return Err(format!("redirect wrong URI: {}", location.as_str()));
                }
                return Err(format!("login error: {}", resp.error));
            } else if let Ok(resp) = serde_urlencoded::from_str::<PostLoginLocation>(query) {
                if resp.session_id.len() == 0 {
                    return Err("session_id length zero".to_string());
                }
                resp.session_id
            } else {
                return Err(format!("unexpected 302 query: {}", query));
            }
        }
    };

    // Get authorization code.
    let params = PostAuthorizeRequest {
        response_type: "code",
        client_id,
        scope,
        redirect_uri: crate::TEST_REDIRECT_URI,
        session_id: session_id.as_str(),
        allow: "yes",
    };
    let req = server.post("/auth/oauth2/authorize").form(&params);
    let resp = runtime.block_on(async { req.await });
    let status = resp.status_code();
    if status != StatusCode::FOUND {
        return Err(format!("post authorize response not 302, {}", status));
    }
    let location = read_location(&resp)?;
    let code = match location.query() {
        None => return Err("302 with no query content".to_string()),
        Some(query) => {
            if let Ok(resp) = serde_urlencoded::from_str::<OAuth2Error>(query) {
                if !location.as_str().starts_with(crate::TEST_REDIRECT_URI) {
                    return Err(format!("redirect wrong URI: {}", location.as_str()));
                }
                return Err(format!("authorize error: {}", resp.error));
            } else if let Ok(resp) = serde_urlencoded::from_str::<PostAuthorizeLocation>(query) {
                if !location.as_str().starts_with(crate::TEST_REDIRECT_URI) {
                    return Err(format!("redirect wrong URI: {}", location.as_str()));
                } else if resp.code.len() == 0 {
                    return Err("code length zero".to_string());
                }
                resp.code
            } else {
                return Err(format!("unexpected 302 query: {}", query));
            }
        }
    };

    // Get access token.
    let params = PostTokenRequest {
        grant_type: "authorization_code",
        code: code.as_str(),
        redirect_uri: crate::TEST_REDIRECT_URI,
        client_id: match client_secret {
            None => Some(client_id),
            Some(_) => None,
        },
    };
    let mut req = server.post("/auth/oauth2/token").form(&params);
    if let Some(secret) = client_secret {
        let auth = base64::engine::general_purpose::STANDARD
            .encode(format!("{}:{}", client_id, secret).as_str());
        req = req.add_header(
            header::AUTHORIZATION,
            HeaderValue::from_str(format!("Basic {}", auth).as_str()).unwrap(),
        );
    }
    let resp = runtime.block_on(async { req.await });
    let status = resp.status_code();
    if status != StatusCode::OK {
        let body: OAuth2Error = resp.json();
        if body.error.len() == 0 {
            return Err(format!("post token response not 200, {}", status));
        }
        return Err(format!(
            "client {} secret {} get token error: {}",
            client_id,
            client_secret.is_some(),
            body.error
        ));
    }
    let body: AccessToken = resp.json();
    if body.access_token.len() == 0 {
        return Err("post token unexpected 200".to_string());
    }
    return Ok(body.access_token);
}

pub fn test_invalid_perm(
    runtime: &Runtime,
    state: &routes::State,
    token: &str,
    method: Method,
    uri: &str,
) -> Result<(), String> {
    let app = Router::new().merge(routes::new_service(&state));
    let server = match TestServer::new(app) {
        Err(e) => return Err(format!("new server error: {}", e)),
        Ok(server) => server,
    };

    let req = server.method(method, uri).add_header(
        header::AUTHORIZATION,
        HeaderValue::from_str(format!("Bearer {}", token).as_str()).unwrap(),
    );
    let resp = runtime.block_on(async { req.await });
    expect(resp.status_code()).to_equal(StatusCode::FORBIDDEN)?;
    let body: ApiError = resp.json();
    if body.code.as_str() != err::E_PERM {
        return Err(format!("unexpected 403 error: {}", body.code.as_str()));
    }
    Ok(())
}

pub fn test_invalid_token(
    runtime: &Runtime,
    state: &routes::State,
    method: Method,
    uri: &str,
) -> Result<(), String> {
    let app = Router::new().merge(routes::new_service(&state));
    let server = match TestServer::new(app) {
        Err(e) => return Err(format!("new server error: {}", e)),
        Ok(server) => server,
    };

    let req = server.method(method, uri).add_header(
        header::AUTHORIZATION,
        HeaderValue::from_str("Bearer token").unwrap(),
    );
    let resp = runtime.block_on(async { req.await });
    expect(resp.status_code()).to_equal(StatusCode::UNAUTHORIZED)
}

pub fn test_get_list_invalid_param(
    runtime: &Runtime,
    state: &routes::State,
    token: &str,
    uri: &str,
    param: &Map<String, Value>,
) -> Result<(), String> {
    let app = Router::new().merge(routes::new_service(&state));
    let server = match TestServer::new(app) {
        Err(e) => return Err(format!("new server error: {}", e)),
        Ok(server) => server,
    };

    let req = server.get(uri).add_query_params(param).add_header(
        header::AUTHORIZATION,
        HeaderValue::from_str(format!("Bearer {}", token).as_str()).unwrap(),
    );
    let resp = runtime.block_on(async { req.await });
    expect(resp.status_code()).to_equal(StatusCode::BAD_REQUEST)?;
    let body: ApiError = resp.json();
    if body.code.as_str() != err::E_PARAM {
        return Err(format!("unexpected 400 error: {}", body.code.as_str()));
    }
    Ok(())
}
