use std::collections::HashMap;

use actix_web::{
    http::{header, Method, StatusCode},
    middleware::NormalizePath,
    test::{self, TestRequest},
    web, App, HttpResponse, Responder,
};
use laboratory::{expect, SpecContext};
use tokio::runtime::Runtime;

use sylvia_iot_auth::routes::{
    self,
    oauth2::middleware::{AuthService, RoleScopeType},
};

use super::{
    super::read_location,
    request,
    response::{self, OAuth2Error},
    TestState, STATE,
};

const ACCESS_DENIED: &'static str = "access_denied";
const INVALID_AUTH: &'static str = "invalid_auth";
const INVALID_CLIENT: &'static str = "invalid_client";
const INVALID_GRANT: &'static str = "invalid_grant";
const INVALID_REQUEST: &'static str = "invalid_request";
const INVALID_SCOPE: &'static str = "invalid_scope";
const UNSUPPORTED_GRANT_TYPE: &'static str = "unsupported_grant_type";
const UNSUPPORTED_RESPONSE_TYPE: &'static str = "unsupported_response_type";
const ALLOW_VALUE: &'static str = "yes";

pub fn get_auth(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    test_get_auth(
        runtime,
        routes_state,
        None,
        StatusCode::BAD_REQUEST,
        INVALID_REQUEST,
    )?;

    let params = request::GetAuthRequest {
        response_type: "type".to_string(),
        redirect_uri: crate::TEST_REDIRECT_URI.to_string(),
        client_id: "public".to_string(),
        scope: None,
        state: None,
    };
    test_get_auth(
        runtime,
        routes_state,
        Some(&params),
        StatusCode::BAD_REQUEST,
        INVALID_REQUEST,
    )?;

    let params = request::GetAuthRequest {
        response_type: "code".to_string(),
        redirect_uri: "uri".to_string(),
        client_id: "public".to_string(),
        scope: None,
        state: None,
    };
    test_get_auth(
        runtime,
        routes_state,
        Some(&params),
        StatusCode::BAD_REQUEST,
        INVALID_REQUEST,
    )?;

    let params = request::GetAuthRequest {
        response_type: "code".to_string(),
        redirect_uri: crate::TEST_REDIRECT_URI.to_string(),
        client_id: "id".to_string(),
        scope: None,
        state: None,
    };
    test_get_auth(
        runtime,
        routes_state,
        Some(&params),
        StatusCode::BAD_REQUEST,
        INVALID_REQUEST,
    )?;

    let params = request::GetAuthRequest {
        response_type: "code".to_string(),
        redirect_uri: crate::TEST_REDIRECT_URI.to_string(),
        client_id: "private".to_string(),
        scope: None,
        state: None,
    };
    test_get_auth(
        runtime,
        routes_state,
        Some(&params),
        StatusCode::FOUND,
        INVALID_SCOPE,
    )?;

    let params = request::GetAuthRequest {
        response_type: "code".to_string(),
        redirect_uri: crate::TEST_REDIRECT_URI.to_string(),
        client_id: "private".to_string(),
        scope: Some("scope3".to_string()),
        state: None,
    };
    test_get_auth(
        runtime,
        routes_state,
        Some(&params),
        StatusCode::FOUND,
        INVALID_SCOPE,
    )?;

    let params = request::GetAuthRequest {
        response_type: "code".to_string(),
        redirect_uri: crate::TEST_REDIRECT_URI.to_string(),
        client_id: "private".to_string(),
        scope: Some("\\".to_string()),
        state: None,
    };
    test_get_auth(
        runtime,
        routes_state,
        Some(&params),
        StatusCode::FOUND,
        INVALID_SCOPE,
    )?;

    let params = request::GetAuthRequest {
        response_type: "code".to_string(),
        redirect_uri: crate::TEST_REDIRECT_URI.to_string(),
        client_id: "public".to_string(),
        scope: None,
        state: None,
    };
    test_get_auth(runtime, routes_state, Some(&params), StatusCode::FOUND, "")?;

    let params = request::GetAuthRequest {
        response_type: "code".to_string(),
        redirect_uri: crate::TEST_REDIRECT_URI.to_string(),
        client_id: "private".to_string(),
        scope: Some("scope1".to_string()),
        state: None,
    };
    test_get_auth(runtime, routes_state, Some(&params), StatusCode::FOUND, "")
}

pub fn get_login(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    test_get_login(
        runtime,
        routes_state,
        None,
        None,
        StatusCode::BAD_REQUEST,
        INVALID_REQUEST,
    )?;

    test_get_login(
        runtime,
        routes_state,
        None,
        Some(""),
        StatusCode::BAD_REQUEST,
        INVALID_REQUEST,
    )?;

    test_get_login(
        runtime,
        routes_state,
        None,
        Some("state"),
        StatusCode::BAD_REQUEST,
        INVALID_REQUEST,
    )?;

    let params = request::GetAuthRequest {
        response_type: "type".to_string(),
        redirect_uri: crate::TEST_REDIRECT_URI.to_string(),
        client_id: "public".to_string(),
        scope: None,
        state: None,
    };
    test_get_login(
        runtime,
        routes_state,
        Some(&params),
        None,
        StatusCode::BAD_REQUEST,
        INVALID_REQUEST,
    )?;

    let params = request::GetAuthRequest {
        response_type: "code".to_string(),
        redirect_uri: crate::TEST_REDIRECT_URI.to_string(),
        client_id: "private".to_string(),
        scope: None,
        state: None,
    };
    test_get_login(
        runtime,
        routes_state,
        Some(&params),
        None,
        StatusCode::FOUND,
        INVALID_SCOPE,
    )?;

    let params = request::GetAuthRequest {
        response_type: "code".to_string(),
        redirect_uri: crate::TEST_REDIRECT_URI.to_string(),
        client_id: "public".to_string(),
        scope: None,
        state: None,
    };
    test_get_login(
        runtime,
        routes_state,
        Some(&params),
        None,
        StatusCode::OK,
        "",
    )?;

    let params = request::GetAuthRequest {
        response_type: "code".to_string(),
        redirect_uri: crate::TEST_REDIRECT_URI.to_string(),
        client_id: "private".to_string(),
        scope: Some("scope1".to_string()),
        state: None,
    };
    test_get_login(
        runtime,
        routes_state,
        Some(&params),
        None,
        StatusCode::OK,
        "",
    )
}

pub fn post_login(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    test_post_login(
        runtime,
        routes_state,
        None,
        None,
        ("user", "user"),
        StatusCode::BAD_REQUEST,
        INVALID_REQUEST,
    )?;

    test_post_login(
        runtime,
        routes_state,
        None,
        Some(""),
        ("user", "user"),
        StatusCode::BAD_REQUEST,
        INVALID_REQUEST,
    )?;

    test_post_login(
        runtime,
        routes_state,
        None,
        Some("state"),
        ("user", "user"),
        StatusCode::BAD_REQUEST,
        INVALID_REQUEST,
    )?;

    let params = request::GetAuthRequest {
        response_type: "type".to_string(),
        redirect_uri: crate::TEST_REDIRECT_URI.to_string(),
        client_id: "public".to_string(),
        scope: None,
        state: None,
    };
    test_post_login(
        runtime,
        routes_state,
        Some(&params),
        None,
        ("user", "user"),
        StatusCode::BAD_REQUEST,
        INVALID_REQUEST,
    )?;

    let params = request::GetAuthRequest {
        response_type: "code".to_string(),
        redirect_uri: crate::TEST_REDIRECT_URI.to_string(),
        client_id: "private".to_string(),
        scope: None,
        state: None,
    };
    test_post_login(
        runtime,
        routes_state,
        Some(&params),
        None,
        ("user", "user"),
        StatusCode::FOUND,
        INVALID_SCOPE,
    )?;

    let params = request::GetAuthRequest {
        response_type: "code".to_string(),
        redirect_uri: crate::TEST_REDIRECT_URI.to_string(),
        client_id: "public".to_string(),
        scope: None,
        state: None,
    };
    test_post_login(
        runtime,
        routes_state,
        Some(&params),
        None,
        ("u", "user"),
        StatusCode::BAD_REQUEST,
        INVALID_AUTH,
    )?;
    test_post_login(
        runtime,
        routes_state,
        Some(&params),
        None,
        ("user", "password"),
        StatusCode::BAD_REQUEST,
        INVALID_AUTH,
    )?;

    let params = request::GetAuthRequest {
        response_type: "code".to_string(),
        redirect_uri: crate::TEST_REDIRECT_URI.to_string(),
        client_id: "public".to_string(),
        scope: None,
        state: None,
    };
    test_post_login(
        runtime,
        routes_state,
        Some(&params),
        None,
        ("user", "user"),
        StatusCode::FOUND,
        "",
    )?;

    let params = request::GetAuthRequest {
        response_type: "code".to_string(),
        redirect_uri: crate::TEST_REDIRECT_URI.to_string(),
        client_id: "private".to_string(),
        scope: Some("scope1".to_string()),
        state: None,
    };
    test_post_login(
        runtime,
        routes_state,
        Some(&params),
        None,
        ("user", "user"),
        StatusCode::FOUND,
        "",
    )?;

    Ok(())
}

pub fn get_authorize(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    let params = request::GetAuthRequest {
        response_type: "code".to_string(),
        redirect_uri: crate::TEST_REDIRECT_URI.to_string(),
        client_id: "public".to_string(),
        scope: None,
        state: None,
    };
    let session_id = match test_post_login(
        runtime,
        routes_state,
        Some(&params),
        None,
        ("user", "user"),
        StatusCode::FOUND,
        "",
    )? {
        None => return Err("get no session ID".to_string()),
        Some(session_id) => session_id,
    };

    test_get_authorize(
        runtime,
        routes_state,
        None,
        StatusCode::BAD_REQUEST,
        INVALID_REQUEST,
    )?;

    let params = request::GetAuthorizeRequest {
        response_type: "type".to_string(),
        redirect_uri: crate::TEST_REDIRECT_URI.to_string(),
        client_id: "public".to_string(),
        scope: None,
        state: None,
        session_id: session_id.clone(),
    };
    test_get_authorize(
        runtime,
        routes_state,
        Some(&params),
        StatusCode::FOUND,
        UNSUPPORTED_RESPONSE_TYPE,
    )?;

    let params = request::GetAuthorizeRequest {
        response_type: "code".to_string(),
        redirect_uri: "uri".to_string(),
        client_id: "public".to_string(),
        scope: None,
        state: None,
        session_id: session_id.clone(),
    };
    test_get_authorize(
        runtime,
        routes_state,
        Some(&params),
        StatusCode::BAD_REQUEST,
        INVALID_REQUEST,
    )?;

    let params = request::GetAuthorizeRequest {
        response_type: "code".to_string(),
        redirect_uri: crate::TEST_REDIRECT_URI.to_string(),
        client_id: "id".to_string(),
        scope: None,
        state: None,
        session_id: session_id.clone(),
    };
    test_get_authorize(
        runtime,
        routes_state,
        Some(&params),
        StatusCode::BAD_REQUEST,
        INVALID_REQUEST,
    )?;

    let params = request::GetAuthorizeRequest {
        response_type: "code".to_string(),
        redirect_uri: crate::TEST_REDIRECT_URI.to_string(),
        client_id: "private".to_string(),
        scope: None,
        state: None,
        session_id: session_id.clone(),
    };
    test_get_authorize(
        runtime,
        routes_state,
        Some(&params),
        StatusCode::FOUND,
        INVALID_SCOPE,
    )?;

    let params = request::GetAuthorizeRequest {
        response_type: "code".to_string(),
        redirect_uri: crate::TEST_REDIRECT_URI.to_string(),
        client_id: "private".to_string(),
        scope: Some("scope3".to_string()),
        state: None,
        session_id: session_id.clone(),
    };
    test_get_authorize(
        runtime,
        routes_state,
        Some(&params),
        StatusCode::FOUND,
        INVALID_SCOPE,
    )?;

    let params = request::GetAuthorizeRequest {
        response_type: "code".to_string(),
        redirect_uri: crate::TEST_REDIRECT_URI.to_string(),
        client_id: "public".to_string(),
        scope: None,
        state: None,
        session_id: session_id.clone(),
    };
    test_get_authorize(runtime, routes_state, Some(&params), StatusCode::OK, "")?;

    let params = request::GetAuthorizeRequest {
        response_type: "code".to_string(),
        redirect_uri: crate::TEST_REDIRECT_URI.to_string(),
        client_id: "private".to_string(),
        scope: Some("scope1".to_string()),
        state: Some("public-state".to_string()),
        session_id: session_id.clone(),
    };
    test_get_authorize(runtime, routes_state, Some(&params), StatusCode::OK, "")?;

    let params = request::GetAuthorizeRequest {
        response_type: "code".to_string(),
        redirect_uri: crate::TEST_REDIRECT_URI.to_string(),
        client_id: "private".to_string(),
        scope: Some("scope1".to_string()),
        state: None,
        session_id: session_id.clone(),
    };
    test_get_authorize(runtime, routes_state, Some(&params), StatusCode::OK, "")
}

pub fn post_authorize(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    let params = request::GetAuthRequest {
        response_type: "code".to_string(),
        redirect_uri: crate::TEST_REDIRECT_URI.to_string(),
        client_id: "public".to_string(),
        scope: None,
        state: None,
    };
    let session_id = match test_post_login(
        runtime,
        routes_state,
        Some(&params),
        None,
        ("user", "user"),
        StatusCode::FOUND,
        "",
    )? {
        None => return Err("get no session ID".to_string()),
        Some(session_id) => session_id,
    };

    test_post_authorize(
        runtime,
        routes_state,
        None,
        StatusCode::BAD_REQUEST,
        INVALID_REQUEST,
    )?;

    let params = request::PostAuthorizeRequest {
        response_type: "type".to_string(),
        redirect_uri: crate::TEST_REDIRECT_URI.to_string(),
        client_id: "public".to_string(),
        scope: None,
        state: None,
        session_id: session_id.clone(),
        allow: None,
    };
    test_post_authorize(
        runtime,
        routes_state,
        Some(&params),
        StatusCode::FOUND,
        UNSUPPORTED_RESPONSE_TYPE,
    )?;

    let params = request::PostAuthorizeRequest {
        response_type: "code".to_string(),
        redirect_uri: "uri".to_string(),
        client_id: "public".to_string(),
        scope: None,
        state: None,
        session_id: session_id.clone(),
        allow: None,
    };
    test_post_authorize(
        runtime,
        routes_state,
        Some(&params),
        StatusCode::BAD_REQUEST,
        INVALID_REQUEST,
    )?;

    let params = request::PostAuthorizeRequest {
        response_type: "code".to_string(),
        redirect_uri: crate::TEST_REDIRECT_URI.to_string(),
        client_id: "id".to_string(),
        scope: None,
        state: None,
        session_id: session_id.clone(),
        allow: None,
    };
    test_post_authorize(
        runtime,
        routes_state,
        Some(&params),
        StatusCode::BAD_REQUEST,
        INVALID_REQUEST,
    )?;

    let params = request::PostAuthorizeRequest {
        response_type: "code".to_string(),
        redirect_uri: crate::TEST_REDIRECT_URI.to_string(),
        client_id: "public".to_string(),
        scope: None,
        state: None,
        session_id: session_id.clone(),
        allow: None,
    };
    test_post_authorize(runtime, routes_state, Some(&params), StatusCode::OK, "")?;

    let params = request::GetAuthRequest {
        response_type: "code".to_string(),
        redirect_uri: crate::TEST_REDIRECT_URI.to_string(),
        client_id: "public".to_string(),
        scope: None,
        state: None,
    };
    let session_id = match test_post_login(
        runtime,
        routes_state,
        Some(&params),
        None,
        ("user", "user"),
        StatusCode::FOUND,
        "",
    )? {
        None => return Err("get no session ID".to_string()),
        Some(session_id) => session_id,
    };

    let params = request::PostAuthorizeRequest {
        response_type: "code".to_string(),
        redirect_uri: crate::TEST_REDIRECT_URI.to_string(),
        client_id: "private".to_string(),
        scope: None,
        state: None,
        session_id: session_id.clone(),
        allow: None,
    };
    test_post_authorize(
        runtime,
        routes_state,
        Some(&params),
        StatusCode::FOUND,
        INVALID_SCOPE,
    )?;

    let params = request::PostAuthorizeRequest {
        response_type: "code".to_string(),
        redirect_uri: crate::TEST_REDIRECT_URI.to_string(),
        client_id: "private".to_string(),
        scope: Some("scope3".to_string()),
        state: None,
        session_id: session_id.clone(),
        allow: None,
    };
    test_post_authorize(
        runtime,
        routes_state,
        Some(&params),
        StatusCode::FOUND,
        INVALID_SCOPE,
    )?;

    let params = request::PostAuthorizeRequest {
        response_type: "code".to_string(),
        redirect_uri: crate::TEST_REDIRECT_URI.to_string(),
        client_id: "private".to_string(),
        scope: Some("scope1".to_string()),
        state: None,
        session_id: session_id.clone(),
        allow: None,
    };
    test_post_authorize(runtime, routes_state, Some(&params), StatusCode::OK, "")?;

    let params = request::GetAuthRequest {
        response_type: "code".to_string(),
        redirect_uri: crate::TEST_REDIRECT_URI.to_string(),
        client_id: "public".to_string(),
        scope: None,
        state: None,
    };
    let session_id = match test_post_login(
        runtime,
        routes_state,
        Some(&params),
        None,
        ("user", "user"),
        StatusCode::FOUND,
        "",
    )? {
        None => return Err("get no session ID".to_string()),
        Some(session_id) => session_id,
    };

    let params = request::PostAuthorizeRequest {
        response_type: "code".to_string(),
        redirect_uri: crate::TEST_REDIRECT_URI.to_string(),
        client_id: "public".to_string(),
        scope: None,
        state: None,
        session_id: session_id.clone(),
        allow: Some("no".to_string()),
    };
    test_post_authorize(
        runtime,
        routes_state,
        Some(&params),
        StatusCode::FOUND,
        ACCESS_DENIED,
    )?;

    let params = request::PostAuthorizeRequest {
        response_type: "code".to_string(),
        redirect_uri: crate::TEST_REDIRECT_URI.to_string(),
        client_id: "private".to_string(),
        scope: Some("scope1".to_string()),
        state: None,
        session_id: session_id.clone(),
        allow: Some("no".to_string()),
    };
    test_post_authorize(
        runtime,
        routes_state,
        Some(&params),
        StatusCode::FOUND,
        ACCESS_DENIED,
    )?;

    let params = request::PostAuthorizeRequest {
        response_type: "code".to_string(),
        redirect_uri: crate::TEST_REDIRECT_URI.to_string(),
        client_id: "public".to_string(),
        scope: None,
        state: Some("public-state".to_string()),
        session_id: session_id.clone(),
        allow: Some(ALLOW_VALUE.to_string()),
    };
    test_post_authorize(runtime, routes_state, Some(&params), StatusCode::FOUND, "")?;

    let params = request::GetAuthRequest {
        response_type: "code".to_string(),
        redirect_uri: crate::TEST_REDIRECT_URI.to_string(),
        client_id: "public".to_string(),
        scope: None,
        state: None,
    };
    let session_id = match test_post_login(
        runtime,
        routes_state,
        Some(&params),
        None,
        ("user", "user"),
        StatusCode::FOUND,
        "",
    )? {
        None => return Err("get no session ID".to_string()),
        Some(session_id) => session_id,
    };

    let params = request::PostAuthorizeRequest {
        response_type: "code".to_string(),
        redirect_uri: crate::TEST_REDIRECT_URI.to_string(),
        client_id: "private".to_string(),
        scope: Some("scope1".to_string()),
        state: None,
        session_id: session_id.clone(),
        allow: Some(ALLOW_VALUE.to_string()),
    };
    test_post_authorize(runtime, routes_state, Some(&params), StatusCode::FOUND, "")?;

    test_post_authorize(
        runtime,
        routes_state,
        Some(&params),
        StatusCode::BAD_REQUEST,
        "invalid_auth",
    )?;

    Ok(())
}

pub fn post_token(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    test_post_token(
        runtime,
        routes_state,
        None,
        None,
        StatusCode::BAD_REQUEST,
        INVALID_REQUEST,
    )?;

    let mut params = request::PostTokenRequest {
        grant_type: "type".to_string(),
        code: "public".to_string(),
        redirect_uri: crate::TEST_REDIRECT_URI.to_string(),
        client_id: Some("public".to_string()),
    };
    test_post_token(
        runtime,
        routes_state,
        Some(&mut params),
        None,
        StatusCode::BAD_REQUEST,
        UNSUPPORTED_GRANT_TYPE,
    )?;

    let mut params = request::PostTokenRequest {
        grant_type: "authorization_code".to_string(),
        code: "code".to_string(),
        redirect_uri: crate::TEST_REDIRECT_URI.to_string(),
        client_id: Some("public".to_string()),
    };
    test_post_token(
        runtime,
        routes_state,
        Some(&mut params),
        None,
        StatusCode::BAD_REQUEST,
        INVALID_REQUEST,
    )?;

    let mut params = request::PostTokenRequest {
        grant_type: "authorization_code".to_string(),
        code: "public".to_string(),
        redirect_uri: "uri".to_string(),
        client_id: Some("public".to_string()),
    };
    test_post_token(
        runtime,
        routes_state,
        Some(&mut params),
        None,
        StatusCode::BAD_REQUEST,
        INVALID_REQUEST,
    )?;

    let mut params = request::PostTokenRequest {
        grant_type: "authorization_code".to_string(),
        code: "public".to_string(),
        redirect_uri: crate::TEST_REDIRECT_URI.to_string(),
        client_id: None,
    };
    test_post_token(
        runtime,
        routes_state,
        Some(&mut params),
        None,
        StatusCode::BAD_REQUEST,
        INVALID_REQUEST,
    )?;

    let mut params = request::PostTokenRequest {
        grant_type: "authorization_code".to_string(),
        code: "public".to_string(),
        redirect_uri: crate::TEST_REDIRECT_URI.to_string(),
        client_id: None,
    };
    test_post_token(
        runtime,
        routes_state,
        Some(&mut params),
        Some("Basic cHVibGljOg=="),
        StatusCode::UNAUTHORIZED,
        INVALID_CLIENT,
    )?;

    let mut params = request::PostTokenRequest {
        grant_type: "authorization_code".to_string(),
        code: "public".to_string(),
        redirect_uri: crate::TEST_REDIRECT_URI.to_string(),
        client_id: Some("id".to_string()),
    };
    test_post_token(
        runtime,
        routes_state,
        Some(&mut params),
        None,
        StatusCode::UNAUTHORIZED,
        INVALID_CLIENT,
    )?;

    let mut params = request::PostTokenRequest {
        grant_type: "authorization_code".to_string(),
        code: "public".to_string(),
        redirect_uri: crate::TEST_REDIRECT_URI.to_string(),
        client_id: Some("private".to_string()),
    };
    test_post_token(
        runtime,
        routes_state,
        Some(&mut params),
        None,
        StatusCode::UNAUTHORIZED,
        INVALID_CLIENT,
    )?;

    let mut params = request::PostTokenRequest {
        grant_type: "authorization_code".to_string(),
        code: "public".to_string(),
        redirect_uri: crate::TEST_REDIRECT_URI.to_string(),
        client_id: Some("public".to_string()),
    };
    test_post_token(
        runtime,
        routes_state,
        Some(&mut params),
        None,
        StatusCode::OK,
        "",
    )?;

    let mut params = request::PostTokenRequest {
        grant_type: "type".to_string(),
        code: "private".to_string(),
        redirect_uri: crate::TEST_REDIRECT_URI.to_string(),
        client_id: None,
    };
    test_post_token(
        runtime,
        routes_state,
        Some(&mut params),
        Some("Basic cHJpdmF0ZTpwcml2YXRl"),
        StatusCode::BAD_REQUEST,
        UNSUPPORTED_GRANT_TYPE,
    )?;

    let mut params = request::PostTokenRequest {
        grant_type: "authorization_code".to_string(),
        code: "code".to_string(),
        redirect_uri: crate::TEST_REDIRECT_URI.to_string(),
        client_id: None,
    };
    test_post_token(
        runtime,
        routes_state,
        Some(&mut params),
        Some("Basic cHJpdmF0ZTpwcml2YXRl"),
        StatusCode::BAD_REQUEST,
        INVALID_REQUEST,
    )?;

    let mut params = request::PostTokenRequest {
        grant_type: "authorization_code".to_string(),
        code: "private".to_string(),
        redirect_uri: "uri".to_string(),
        client_id: None,
    };
    test_post_token(
        runtime,
        routes_state,
        Some(&mut params),
        Some("Basic cHJpdmF0ZTpwcml2YXRl"),
        StatusCode::BAD_REQUEST,
        INVALID_REQUEST,
    )?;

    let mut params = request::PostTokenRequest {
        grant_type: "authorization_code".to_string(),
        code: "private".to_string(),
        redirect_uri: crate::TEST_REDIRECT_URI.to_string(),
        client_id: None,
    };
    test_post_token(
        runtime,
        routes_state,
        Some(&mut params),
        None,
        StatusCode::BAD_REQUEST,
        INVALID_REQUEST,
    )?;

    let mut params = request::PostTokenRequest {
        grant_type: "authorization_code".to_string(),
        code: "private".to_string(),
        redirect_uri: crate::TEST_REDIRECT_URI.to_string(),
        client_id: Some("id".to_string()),
    };
    test_post_token(
        runtime,
        routes_state,
        Some(&mut params),
        None,
        StatusCode::UNAUTHORIZED,
        INVALID_CLIENT,
    )?;

    let mut params = request::PostTokenRequest {
        grant_type: "authorization_code".to_string(),
        code: "private".to_string(),
        redirect_uri: crate::TEST_REDIRECT_URI.to_string(),
        client_id: Some("public".to_string()),
    };
    test_post_token(
        runtime,
        routes_state,
        Some(&mut params),
        None,
        StatusCode::BAD_REQUEST,
        INVALID_GRANT,
    )?;

    let mut params = request::PostTokenRequest {
        grant_type: "authorization_code".to_string(),
        code: "private".to_string(),
        redirect_uri: crate::TEST_REDIRECT_URI.to_string(),
        client_id: Some("private".to_string()),
    };
    test_post_token(
        runtime,
        routes_state,
        Some(&mut params),
        None,
        StatusCode::UNAUTHORIZED,
        INVALID_CLIENT,
    )?;

    let mut params = request::PostTokenRequest {
        grant_type: "authorization_code".to_string(),
        code: "private".to_string(),
        redirect_uri: crate::TEST_REDIRECT_URI.to_string(),
        client_id: None,
    };
    test_post_token(
        runtime,
        routes_state,
        Some(&mut params),
        Some("Basic cHJpdmF0ZTpwcml2YXRl"),
        StatusCode::OK,
        "",
    )?;

    Ok(())
}

pub fn post_token_client(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    test_post_token_client(
        runtime,
        routes_state,
        None,
        None,
        StatusCode::BAD_REQUEST,
        INVALID_REQUEST,
    )?;

    test_post_token_client(
        runtime,
        routes_state,
        None,
        Some("Basic dGVzdDo="), // test:
        StatusCode::UNAUTHORIZED,
        INVALID_CLIENT,
    )?;

    test_post_token_client(
        runtime,
        routes_state,
        None,
        Some("Basic cHJpdmF0ZTo="), // private:
        StatusCode::UNAUTHORIZED,
        INVALID_CLIENT,
    )?;

    test_post_token_client(
        runtime,
        routes_state,
        None,
        Some("Basic cHVibGljOg=="), // public:
        StatusCode::UNAUTHORIZED,
        INVALID_CLIENT,
    )?;

    test_post_token_client(
        runtime,
        routes_state,
        None,
        Some("Basic cHJpdmF0ZTpwcml2YXQ="), // private:privat
        StatusCode::UNAUTHORIZED,
        INVALID_CLIENT,
    )?;

    test_post_token_client(
        runtime,
        routes_state,
        None,
        Some("Basic bm8tcmVkaXJlY3Q6bm8tcmVkaXJlY3Q="), // no-redirect:no-redirect
        StatusCode::UNAUTHORIZED,
        INVALID_CLIENT,
    )?;

    test_post_token_client(
        runtime,
        routes_state,
        None,
        Some("Basic YmFkLXJlZGlyZWN0OmJhZC1yZWRpcmVjdA=="), // bad-redirect:bad-redirect
        StatusCode::UNAUTHORIZED,
        INVALID_CLIENT,
    )?;

    test_post_token_client(
        runtime,
        routes_state,
        None,
        Some("Basic cHJpdmF0ZTpwcml2YXRl"),
        StatusCode::OK,
        "",
    )?;

    Ok(())
}

pub fn post_refresh(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    test_post_refresh(
        runtime,
        routes_state,
        None,
        None,
        StatusCode::BAD_REQUEST,
        INVALID_REQUEST,
    )?;

    let mut params = request::PostRefreshRequest {
        grant_type: "type".to_string(),
        refresh_token: "public".to_string(),
        scope: Some("scope".to_string()),
        client_id: Some("public".to_string()),
    };
    test_post_refresh(
        runtime,
        routes_state,
        Some(&mut params),
        None,
        StatusCode::BAD_REQUEST,
        UNSUPPORTED_GRANT_TYPE,
    )?;

    let mut params = request::PostRefreshRequest {
        grant_type: "refresh_token".to_string(),
        refresh_token: "token".to_string(),
        scope: Some("scope".to_string()),
        client_id: Some("public".to_string()),
    };
    test_post_refresh(
        runtime,
        routes_state,
        Some(&mut params),
        None,
        StatusCode::BAD_REQUEST,
        INVALID_GRANT,
    )?;

    let mut params = request::PostRefreshRequest {
        grant_type: "refresh_token".to_string(),
        refresh_token: "public".to_string(),
        scope: None,
        client_id: None,
    };
    test_post_refresh(
        runtime,
        routes_state,
        Some(&mut params),
        None,
        StatusCode::OK,
        "",
    )?;

    let mut params = request::PostRefreshRequest {
        grant_type: "refresh_token".to_string(),
        refresh_token: "public".to_string(),
        scope: Some("scope".to_string()),
        client_id: None,
    };
    test_post_refresh(
        runtime,
        routes_state,
        Some(&mut params),
        None,
        StatusCode::BAD_REQUEST,
        INVALID_SCOPE,
    )?;

    let mut params = request::PostRefreshRequest {
        grant_type: "refresh_token".to_string(),
        refresh_token: "public".to_string(),
        scope: None,
        client_id: Some("id".to_string()),
    };
    test_post_refresh(
        runtime,
        routes_state,
        Some(&mut params),
        None,
        StatusCode::OK,
        "",
    )?;

    let mut params = request::PostRefreshRequest {
        grant_type: "refresh_token".to_string(),
        refresh_token: "public".to_string(),
        scope: None,
        client_id: Some("private".to_string()),
    };
    test_post_refresh(
        runtime,
        routes_state,
        Some(&mut params),
        None,
        StatusCode::OK,
        "",
    )?;

    let mut params = request::PostRefreshRequest {
        grant_type: "refresh_token".to_string(),
        refresh_token: "public".to_string(),
        scope: None,
        client_id: Some("public".to_string()),
    };
    test_post_refresh(
        runtime,
        routes_state,
        Some(&mut params),
        None,
        StatusCode::OK,
        "",
    )?;

    let mut params = request::PostRefreshRequest {
        grant_type: "type".to_string(),
        refresh_token: "private".to_string(),
        scope: Some("scope1".to_string()),
        client_id: Some("private".to_string()),
    };
    test_post_refresh(
        runtime,
        routes_state,
        Some(&mut params),
        None,
        StatusCode::BAD_REQUEST,
        UNSUPPORTED_GRANT_TYPE,
    )?;

    let mut params = request::PostRefreshRequest {
        grant_type: "refresh_token".to_string(),
        refresh_token: "token".to_string(),
        scope: Some("scope1".to_string()),
        client_id: Some("public".to_string()),
    };
    test_post_refresh(
        runtime,
        routes_state,
        Some(&mut params),
        None,
        StatusCode::BAD_REQUEST,
        INVALID_GRANT,
    )?;

    let mut params = request::PostRefreshRequest {
        grant_type: "refresh_token".to_string(),
        refresh_token: "private".to_string(),
        scope: None,
        client_id: None,
    };
    test_post_refresh(
        runtime,
        routes_state,
        Some(&mut params),
        None,
        StatusCode::UNAUTHORIZED,
        INVALID_CLIENT,
    )?;

    let mut params = request::PostRefreshRequest {
        grant_type: "refresh_token".to_string(),
        refresh_token: "private".to_string(),
        scope: Some("scope1".to_string()),
        client_id: None,
    };
    test_post_refresh(
        runtime,
        routes_state,
        Some(&mut params),
        None,
        StatusCode::UNAUTHORIZED,
        INVALID_CLIENT,
    )?;

    let mut params = request::PostRefreshRequest {
        grant_type: "refresh_token".to_string(),
        refresh_token: "private".to_string(),
        scope: None,
        client_id: Some("id".to_string()),
    };
    test_post_refresh(
        runtime,
        routes_state,
        Some(&mut params),
        None,
        StatusCode::UNAUTHORIZED,
        INVALID_CLIENT,
    )?;

    let mut params = request::PostRefreshRequest {
        grant_type: "refresh_token".to_string(),
        refresh_token: "private".to_string(),
        scope: None,
        client_id: Some("public".to_string()),
    };
    test_post_refresh(
        runtime,
        routes_state,
        Some(&mut params),
        None,
        StatusCode::UNAUTHORIZED,
        INVALID_CLIENT,
    )?;

    let mut params = request::PostRefreshRequest {
        grant_type: "refresh_token".to_string(),
        refresh_token: "private".to_string(),
        scope: None,
        client_id: Some("private".to_string()),
    };
    test_post_refresh(
        runtime,
        routes_state,
        Some(&mut params),
        None,
        StatusCode::UNAUTHORIZED,
        INVALID_CLIENT,
    )?;

    let mut params = request::PostRefreshRequest {
        grant_type: "refresh_token".to_string(),
        refresh_token: "private".to_string(),
        scope: None,
        client_id: None,
    };
    test_post_refresh(
        runtime,
        routes_state,
        Some(&mut params),
        Some("Basic cHJpdmF0ZTpwcml2YXRl"),
        StatusCode::OK,
        "",
    )?;

    let mut params = request::PostRefreshRequest {
        grant_type: "refresh_token".to_string(),
        refresh_token: "private".to_string(),
        scope: Some("scope1".to_string()),
        client_id: None,
    };
    test_post_refresh(
        runtime,
        routes_state,
        Some(&mut params),
        Some("Basic cHJpdmF0ZTpwcml2YXRl"),
        StatusCode::OK,
        "",
    )?;

    let mut params = request::PostRefreshRequest {
        grant_type: "refresh_token".to_string(),
        refresh_token: "private".to_string(),
        scope: Some("scope3".to_string()),
        client_id: None,
    };
    test_post_refresh(
        runtime,
        routes_state,
        Some(&mut params),
        Some("Basic cHJpdmF0ZTpwcml2YXRl"),
        StatusCode::BAD_REQUEST,
        INVALID_SCOPE,
    )?;

    Ok(())
}

pub fn middleware_api_scope(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let model = state.routes_state.as_ref().unwrap().model.clone();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    let mut params = request::PostTokenRequest {
        grant_type: "authorization_code".to_string(),
        code: "public".to_string(),
        redirect_uri: crate::TEST_REDIRECT_URI.to_string(),
        client_id: Some("public".to_string()),
    };
    let public_token = test_post_token(
        runtime,
        routes_state,
        Some(&mut params),
        None,
        StatusCode::OK,
        "",
    )?;
    let public_token = match public_token {
        None => return Err("get no public token".to_string()),
        Some(token) => token.access_token,
    };
    params.code = "private".to_string();
    params.client_id = None;
    let private_token = test_post_token(
        runtime,
        routes_state,
        Some(&mut params),
        Some("Basic cHJpdmF0ZTpwcml2YXRl"),
        StatusCode::OK,
        "",
    )?;
    let private_token = match private_token {
        None => return Err("get no private token".to_string()),
        Some(token) => token.access_token,
    };

    let mut role_scopes_root: HashMap<Method, RoleScopeType> = HashMap::new();
    role_scopes_root.insert(Method::GET, (vec![], vec![]));
    role_scopes_root.insert(Method::POST, (vec![], vec!["scope1".to_string()]));
    let mut app = runtime.block_on(async {
        test::init_service(
            App::new()
                .wrap(NormalizePath::trim())
                .wrap(AuthService::new(&model, role_scopes_root))
                .service(
                    web::resource("/")
                        .route(web::get().to(dummy_handler))
                        .route(web::post().to(dummy_handler)),
                ),
        )
        .await
    });

    let req = TestRequest::get()
        .uri("/")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", public_token)))
        .to_request();
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    let status = resp.status();
    if status != StatusCode::NO_CONTENT {
        let body = runtime.block_on(async { test::read_body(resp).await });
        return Err(format!("public case status {}, body: {:?}", status, body));
    }
    let req = TestRequest::post()
        .uri("/")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", public_token)))
        .to_request();
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    let status = resp.status();
    if status != StatusCode::FORBIDDEN {
        let body = runtime.block_on(async { test::read_body(resp).await });
        return Err(format!("public case status {}, body: {:?}", status, body));
    }

    let req = TestRequest::get()
        .uri("/")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", private_token)))
        .to_request();
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    let status = resp.status();
    if status != StatusCode::NO_CONTENT {
        let body = runtime.block_on(async { test::read_body(resp).await });
        return Err(format!("private case status {}, body: {:?}", status, body));
    }
    let req = TestRequest::post()
        .uri("/")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", private_token)))
        .to_request();
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    let status = resp.status();
    if status != StatusCode::NO_CONTENT {
        let body = runtime.block_on(async { test::read_body(resp).await });
        return Err(format!("private case status {}, body: {:?}", status, body));
    }

    Ok(())
}

fn test_get_auth(
    runtime: &Runtime,
    state: &routes::State,
    params: Option<&request::GetAuthRequest>,
    expect_status: StatusCode,
    expect_error: &str,
) -> Result<(), String> {
    let mut app = runtime.block_on(async {
        test::init_service(
            App::new()
                .wrap(NormalizePath::trim())
                .service(routes::new_service(&state)),
        )
        .await
    });

    let uri = match params {
        None => "/auth/oauth2/auth".to_string(),
        Some(params) => format!(
            "/auth/oauth2/auth?{}",
            serde_urlencoded::to_string(params).unwrap()
        ),
    };
    let req = TestRequest::get().uri(uri.as_str()).to_request();
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    if resp.status() != expect_status {
        return Err(format!(
            "get auth response not {}, {}",
            expect_status,
            resp.status()
        ));
    }
    match resp.status() {
        StatusCode::FOUND => {
            let location = read_location(&resp)?;
            match location.query() {
                None => return Err("302 with no query content".to_string()),
                Some(query) => {
                    if let Ok(resp) = serde_urlencoded::from_str::<OAuth2Error>(query) {
                        if !location.as_str().starts_with(crate::TEST_REDIRECT_URI) {
                            return Err(format!("redirect wrong URI: {}", location.as_str()));
                        } else if resp.error == expect_error {
                            return Ok(());
                        }
                    } else if let Ok(resp) =
                        serde_urlencoded::from_str::<request::GetLoginRequest>(query)
                    {
                        if let Ok(resp) = serde_urlencoded::from_str::<request::GetAuthRequest>(
                            resp.state.as_str(),
                        ) {
                            expect(resp.response_type.as_str())
                                .to_equal(params.unwrap().response_type.as_str())?;
                            expect(resp.client_id.as_str())
                                .to_equal(params.unwrap().client_id.as_str())?;
                            expect(resp.redirect_uri.as_str())
                                .to_equal(params.unwrap().redirect_uri.as_str())?;
                            expect(resp.scope.as_deref())
                                .to_equal(params.unwrap().scope.as_deref())?;
                            expect(resp.state.as_deref())
                                .to_equal(params.unwrap().state.as_deref())?;
                            return Ok(());
                        }
                    }
                    return Err(format!("unexpected 302 query: {}", query));
                }
            }
        }
        StatusCode::BAD_REQUEST => {
            let body: OAuth2Error = runtime.block_on(async { test::read_body_json(resp).await });
            if body.error.as_str() != expect_error {
                return Err(format!("unexpected 400 error: {}", body.error.as_str()));
            }
            return Ok(());
        }
        _ => return Err(format!("unexpect status code: {}", resp.status())),
    }
}

fn test_get_login(
    runtime: &Runtime,
    state: &routes::State,
    params: Option<&request::GetAuthRequest>,
    params_raw: Option<&str>,
    expect_status: StatusCode,
    expect_error: &str,
) -> Result<(), String> {
    let mut app = runtime.block_on(async {
        test::init_service(
            App::new()
                .wrap(NormalizePath::trim())
                .service(routes::new_service(&state)),
        )
        .await
    });

    let uri = match params {
        None => match params_raw {
            None => "/auth/oauth2/login".to_string(),
            Some(raw) => format!("/auth/oauth2/login?state={}", raw),
        },
        Some(params) => {
            let qs = serde_urlencoded::to_string(&request::GetLoginRequest {
                state: serde_urlencoded::to_string(params).unwrap(),
            })
            .unwrap();
            format!("/auth/oauth2/login?{}", qs)
        }
    };
    let req = TestRequest::get().uri(uri.as_str()).to_request();
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    if resp.status() != expect_status {
        return Err(format!(
            "get login response not {}, {}",
            expect_status,
            resp.status()
        ));
    }
    match resp.status() {
        StatusCode::OK => return Ok(()),
        StatusCode::FOUND => {
            let location = read_location(&resp)?;
            match location.query() {
                None => return Err("302 with no query content".to_string()),
                Some(query) => {
                    if let Ok(resp) = serde_urlencoded::from_str::<OAuth2Error>(query) {
                        if !location.as_str().starts_with(crate::TEST_REDIRECT_URI) {
                            return Err(format!("redirect wrong URI: {}", location.as_str()));
                        } else if resp.error == expect_error {
                            return Ok(());
                        }
                    } else if let Ok(resp) =
                        serde_urlencoded::from_str::<request::GetLoginRequest>(query)
                    {
                        if let Ok(resp) = serde_urlencoded::from_str::<request::GetAuthRequest>(
                            resp.state.as_str(),
                        ) {
                            expect(resp.response_type.as_str())
                                .to_equal(params.unwrap().response_type.as_str())?;
                            expect(resp.client_id.as_str())
                                .to_equal(params.unwrap().client_id.as_str())?;
                            expect(resp.redirect_uri.as_str())
                                .to_equal(params.unwrap().redirect_uri.as_str())?;
                            expect(resp.scope.as_deref())
                                .to_equal(params.unwrap().scope.as_deref())?;
                            expect(resp.state.as_deref())
                                .to_equal(params.unwrap().state.as_deref())?;
                            return Ok(());
                        }
                    }
                    return Err(format!("unexpected 302 query: {}", query));
                }
            }
        }
        StatusCode::BAD_REQUEST => {
            let body: OAuth2Error = runtime.block_on(async { test::read_body_json(resp).await });
            if body.error.as_str() != expect_error {
                return Err(format!("unexpected 400 error: {}", body.error.as_str()));
            }
            return Ok(());
        }
        _ => return Err(format!("unexpect status code: {}", resp.status())),
    }
}

// Returns session ID if success.
fn test_post_login(
    runtime: &Runtime,
    state: &routes::State,
    params: Option<&request::GetAuthRequest>,
    params_raw: Option<&str>,
    auth: (&str, &str),
    expect_status: StatusCode,
    expect_error: &str,
) -> Result<Option<String>, String> {
    let mut app = runtime.block_on(async {
        test::init_service(
            App::new()
                .wrap(NormalizePath::trim())
                .service(routes::new_service(&state)),
        )
        .await
    });

    let uri = "/auth/oauth2/login";
    let req = match params {
        None => match params_raw {
            None => TestRequest::post().uri(uri).to_request(),
            Some(raw) => TestRequest::post()
                .uri(uri)
                .set_form(&request::PostLoginRequest {
                    account: auth.0.to_string(),
                    password: auth.1.to_string(),
                    state: raw.to_string(),
                })
                .to_request(),
        },
        Some(params) => TestRequest::post()
            .uri(uri)
            .set_form(&request::PostLoginRequest {
                account: auth.0.to_string(),
                password: auth.1.to_string(),
                state: serde_urlencoded::to_string(params).unwrap(),
            })
            .to_request(),
    };
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    if resp.status() != expect_status {
        return Err(format!(
            "post login response not {}, {}",
            expect_status,
            resp.status()
        ));
    }
    match resp.status() {
        StatusCode::FOUND => {
            let location = read_location(&resp)?;
            match location.query() {
                None => return Err("302 with no query content".to_string()),
                Some(query) => {
                    if let Ok(resp) = serde_urlencoded::from_str::<OAuth2Error>(query) {
                        if !location.as_str().starts_with(crate::TEST_REDIRECT_URI) {
                            return Err(format!("redirect wrong URI: {}", location.as_str()));
                        } else if resp.error == expect_error {
                            return Ok(None);
                        }
                    } else if let Ok(resp) =
                        serde_urlencoded::from_str::<request::GetAuthorizeRequest>(query)
                    {
                        expect(resp.response_type.as_str())
                            .to_equal(params.unwrap().response_type.as_str())?;
                        expect(resp.client_id.as_str())
                            .to_equal(params.unwrap().client_id.as_str())?;
                        expect(resp.redirect_uri.as_str())
                            .to_equal(params.unwrap().redirect_uri.as_str())?;
                        expect(resp.scope.as_deref()).to_equal(params.unwrap().scope.as_deref())?;
                        expect(resp.state.as_deref()).to_equal(params.unwrap().state.as_deref())?;
                        expect(resp.session_id.len()).to_not_equal(0)?;
                        return Ok(Some(resp.session_id));
                    }
                    return Err(format!("unexpected 302 query: {}", query));
                }
            }
        }
        StatusCode::BAD_REQUEST => {
            let body: OAuth2Error = runtime.block_on(async { test::read_body_json(resp).await });
            if body.error.as_str() != expect_error {
                return Err(format!("unexpected 400 error: {}", body.error.as_str()));
            }
            return Ok(None);
        }
        _ => return Err(format!("unexpect status code: {}", resp.status())),
    }
}

fn test_get_authorize(
    runtime: &Runtime,
    state: &routes::State,
    params: Option<&request::GetAuthorizeRequest>,
    expect_status: StatusCode,
    expect_error: &str,
) -> Result<(), String> {
    let mut app = runtime.block_on(async {
        test::init_service(
            App::new()
                .wrap(NormalizePath::trim())
                .service(routes::new_service(&state)),
        )
        .await
    });

    let uri = format!(
        "/auth/oauth2/authorize?{}",
        serde_urlencoded::to_string(params).unwrap()
    );
    let req = TestRequest::get().uri(uri.as_str()).to_request();
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    if resp.status() != expect_status {
        return Err(format!(
            "get authorize response not {}, {}",
            expect_status,
            resp.status()
        ));
    }
    match resp.status() {
        StatusCode::OK => return Ok(()),
        StatusCode::FOUND => {
            let location = read_location(&resp)?;
            match location.query() {
                None => return Err("302 with no query content".to_string()),
                Some(query) => {
                    if let Ok(resp) = serde_urlencoded::from_str::<OAuth2Error>(query) {
                        if !location.as_str().starts_with(crate::TEST_REDIRECT_URI) {
                            return Err(format!("redirect wrong URI: {}", location.as_str()));
                        } else if resp.error == expect_error {
                            return Ok(());
                        }
                    }
                    return Err(format!("unexpected 302 query: {}", query));
                }
            }
        }
        StatusCode::BAD_REQUEST => {
            let body: OAuth2Error = runtime.block_on(async { test::read_body_json(resp).await });
            if body.error.as_str() != expect_error {
                return Err(format!("unexpected 400 error: {}", body.error.as_str()));
            }
            return Ok(());
        }
        _ => return Err(format!("unexpect status code: {}", resp.status())),
    }
}

fn test_post_authorize(
    runtime: &Runtime,
    state: &routes::State,
    params: Option<&request::PostAuthorizeRequest>,
    expect_status: StatusCode,
    expect_error: &str,
) -> Result<Option<String>, String> {
    let mut app = runtime.block_on(async {
        test::init_service(
            App::new()
                .wrap(NormalizePath::trim())
                .service(routes::new_service(&state)),
        )
        .await
    });

    let req = match params {
        None => TestRequest::post()
            .uri("/auth/oauth2/authorize")
            .to_request(),
        Some(params) => TestRequest::post()
            .uri("/auth/oauth2/authorize")
            .set_form(params)
            .to_request(),
    };
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    if resp.status() != expect_status {
        return Err(format!(
            "post authorize response not {}, {}",
            expect_status,
            resp.status()
        ));
    }
    match resp.status() {
        StatusCode::OK => return Ok(None),
        StatusCode::FOUND => {
            let location = read_location(&resp)?;
            match location.query() {
                None => return Err("302 with no query content".to_string()),
                Some(query) => {
                    if let Ok(resp) = serde_urlencoded::from_str::<OAuth2Error>(query) {
                        if !location.as_str().starts_with(crate::TEST_REDIRECT_URI) {
                            return Err(format!("redirect wrong URI: {}", location.as_str()));
                        } else if resp.error == expect_error {
                            return Ok(None);
                        }
                    } else if let Ok(resp) =
                        serde_urlencoded::from_str::<response::PostAuthorizeLocation>(query)
                    {
                        if !location.as_str().starts_with(crate::TEST_REDIRECT_URI) {
                            return Err(format!("redirect wrong URI: {}", location.as_str()));
                        } else if resp.code.len() == 0 {
                            return Err("code length zero".to_string());
                        }
                        if let Some(state) = params.as_ref().unwrap().state.as_ref() {
                            if resp.state.is_none()
                                || state.as_str() != resp.state.as_ref().unwrap().as_str()
                            {
                                return Err(format!(
                                    "state error, org: {}, found: {}",
                                    state.as_str(),
                                    resp.state.as_ref().unwrap().as_str()
                                ));
                            }
                        }
                        return Ok(Some(resp.code));
                    }
                    return Err(format!("unexpected 302 query: {}", query));
                }
            }
        }
        StatusCode::BAD_REQUEST => {
            let body: OAuth2Error = runtime.block_on(async { test::read_body_json(resp).await });
            if body.error.as_str() != expect_error {
                return Err(format!("unexpected 400 error: {}", body.error.as_str()));
            }
            return Ok(None);
        }
        _ => return Err(format!("unexpect status code: {}", resp.status())),
    }
}

fn test_post_token(
    runtime: &Runtime,
    state: &routes::State,
    params: Option<&mut request::PostTokenRequest>,
    auth_header: Option<&str>,
    expect_status: StatusCode,
    expect_error: &str,
) -> Result<Option<response::AccessToken>, String> {
    let mut app = runtime.block_on(async {
        test::init_service(
            App::new()
                .wrap(NormalizePath::trim())
                .service(routes::new_service(&state)),
        )
        .await
    });

    let req = match params {
        None => TestRequest::post().uri("/auth/oauth2/token").to_request(),
        Some(params) => {
            if params.code.as_str() == "public" || params.code.as_str() == "private" {
                let auth_params = request::GetAuthRequest {
                    response_type: "code".to_string(),
                    redirect_uri: crate::TEST_REDIRECT_URI.to_string(),
                    client_id: "public".to_string(),
                    scope: None,
                    state: None,
                };
                let session_id = match test_post_login(
                    runtime,
                    state,
                    Some(&auth_params),
                    None,
                    ("user", "user"),
                    StatusCode::FOUND,
                    "",
                )? {
                    None => return Err("get no session ID".to_string()),
                    Some(session_id) => session_id,
                };

                let mut body = request::PostAuthorizeRequest {
                    response_type: "code".to_string(),
                    client_id: "public".to_string(),
                    redirect_uri: crate::TEST_REDIRECT_URI.to_string(),
                    scope: None,
                    state: None,
                    session_id: session_id.to_string(),
                    allow: Some(ALLOW_VALUE.to_string()),
                };
                if params.code.as_str() == "private" {
                    body.client_id = "private".to_string();
                    body.scope = Some("scope1".to_string());
                }
                params.code =
                    test_post_authorize(runtime, state, Some(&body), StatusCode::FOUND, "")?
                        .unwrap();
            }
            match auth_header {
                None => TestRequest::post()
                    .uri("/auth/oauth2/token")
                    .set_form(params)
                    .to_request(),
                Some(header) => TestRequest::post()
                    .uri("/auth/oauth2/token")
                    .insert_header((header::AUTHORIZATION, header))
                    .set_form(params)
                    .to_request(),
            }
        }
    };
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    if resp.status() != expect_status {
        return Err(format!(
            "post token response not {}, {}",
            expect_status,
            resp.status()
        ));
    }
    match resp.status() {
        StatusCode::OK => {
            let body: response::AccessToken =
                runtime.block_on(async { test::read_body_json(resp).await });
            if body.access_token.len() == 0 {
                return Err("access token empty".to_string());
            } else if body.refresh_token.len() == 0 {
                return Err("refresh token empty".to_string());
            } else if body.token_type.to_lowercase().as_str() != "bearer" {
                return Err(format!("wrong token type: {}", body.token_type.as_str()));
            } else if body.expires_in == 0 {
                return Err("expires_in zero".to_string());
            }
            return Ok(Some(body));
        }
        StatusCode::BAD_REQUEST => {
            let body: OAuth2Error = runtime.block_on(async { test::read_body_json(resp).await });
            if body.error.as_str() != expect_error {
                return Err(format!("unexpected 400 error: {}", body.error.as_str()));
            }
            return Ok(None);
        }
        StatusCode::UNAUTHORIZED => {
            let body: OAuth2Error = runtime.block_on(async { test::read_body_json(resp).await });
            if body.error.as_str() != expect_error {
                return Err(format!("unexpected 401 error: {}", body.error.as_str()));
            }
            return Ok(None);
        }
        _ => return Err(format!("unexpect status code: {}", resp.status())),
    }
}

fn test_post_token_client(
    runtime: &Runtime,
    state: &routes::State,
    scope: Option<&str>,
    auth_header: Option<&str>,
    expect_status: StatusCode,
    expect_error: &str,
) -> Result<Option<response::AccessToken>, String> {
    let mut app = runtime.block_on(async {
        test::init_service(
            App::new()
                .wrap(NormalizePath::trim())
                .service(routes::new_service(&state)),
        )
        .await
    });

    let params = request::PostTokenClientRequest {
        grant_type: "client_credentials".to_string(),
        scope: match scope {
            None => None,
            Some(scope) => Some(scope.to_string()),
        },
    };
    let req = match auth_header {
        None => TestRequest::post()
            .uri("/auth/oauth2/token")
            .set_form(params)
            .to_request(),
        Some(header) => TestRequest::post()
            .uri("/auth/oauth2/token")
            .insert_header((header::AUTHORIZATION, header))
            .set_form(params)
            .to_request(),
    };
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    if resp.status() != expect_status {
        return Err(format!(
            "post token response not {}, {}",
            expect_status,
            resp.status()
        ));
    }
    match resp.status() {
        StatusCode::OK => {
            let body: response::AccessToken =
                runtime.block_on(async { test::read_body_json(resp).await });
            if body.access_token.len() == 0 {
                return Err("access token empty".to_string());
            } else if body.refresh_token.len() == 0 {
                return Err("refresh token empty".to_string());
            } else if body.token_type.to_lowercase().as_str() != "bearer" {
                return Err(format!("wrong token type: {}", body.token_type.as_str()));
            } else if body.expires_in == 0 {
                return Err("expires_in zero".to_string());
            }
            return Ok(Some(body));
        }
        StatusCode::BAD_REQUEST => {
            let body: OAuth2Error = runtime.block_on(async { test::read_body_json(resp).await });
            if body.error.as_str() != expect_error {
                return Err(format!("unexpected 400 error: {}", body.error.as_str()));
            }
            return Ok(None);
        }
        StatusCode::UNAUTHORIZED => {
            let body: OAuth2Error = runtime.block_on(async { test::read_body_json(resp).await });
            if body.error.as_str() != expect_error {
                return Err(format!("unexpected 401 error: {}", body.error.as_str()));
            }
            return Ok(None);
        }
        _ => return Err(format!("unexpect status code: {}", resp.status())),
    }
}

fn test_post_refresh(
    runtime: &Runtime,
    state: &routes::State,
    params: Option<&mut request::PostRefreshRequest>,
    auth_header: Option<&str>,
    expect_status: StatusCode,
    expect_error: &str,
) -> Result<Option<response::AccessToken>, String> {
    let mut app = runtime.block_on(async {
        test::init_service(
            App::new()
                .wrap(NormalizePath::trim())
                .service(routes::new_service(&state)),
        )
        .await
    });

    let req = match params {
        None => TestRequest::post().uri("/auth/oauth2/refresh").to_request(),
        Some(params) => {
            if params.refresh_token.as_str() == "public"
                || params.refresh_token.as_str() == "private"
            {
                let mut body = request::PostTokenRequest {
                    grant_type: "authorization_code".to_string(),
                    code: params.refresh_token.clone(),
                    redirect_uri: crate::TEST_REDIRECT_URI.to_string(),
                    client_id: None,
                };
                if params.refresh_token.as_str() == "public" {
                    body.code = "public".to_string();
                    body.client_id = Some("public".to_string());
                    params.refresh_token =
                        test_post_token(runtime, state, Some(&mut body), None, StatusCode::OK, "")?
                            .unwrap()
                            .refresh_token;
                } else {
                    body.code = "private".to_string();
                    params.refresh_token = test_post_token(
                        runtime,
                        state,
                        Some(&mut body),
                        Some("Basic cHJpdmF0ZTpwcml2YXRl"),
                        StatusCode::OK,
                        "",
                    )?
                    .unwrap()
                    .refresh_token;
                }
            }
            match auth_header {
                None => TestRequest::post()
                    .uri("/auth/oauth2/refresh")
                    .set_form(params)
                    .to_request(),
                Some(header) => TestRequest::post()
                    .uri("/auth/oauth2/refresh")
                    .insert_header((header::AUTHORIZATION, header))
                    .set_form(params)
                    .to_request(),
            }
        }
    };
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    if resp.status() != expect_status {
        return Err(format!(
            "post refresh response not {}, {}",
            expect_status,
            resp.status()
        ));
    }
    match resp.status() {
        StatusCode::OK => {
            let body: response::AccessToken =
                runtime.block_on(async { test::read_body_json(resp).await });
            if body.access_token.len() == 0 {
                return Err("access token empty".to_string());
            } else if body.refresh_token.len() == 0 {
                return Err("refresh token empty".to_string());
            } else if body.token_type.to_lowercase().as_str() != "bearer" {
                return Err(format!("wrong token type: {}", body.token_type.as_str()));
            } else if body.expires_in == 0 {
                return Err("expires_in zero".to_string());
            }
            return Ok(Some(body));
        }
        StatusCode::BAD_REQUEST => {
            let body: OAuth2Error = runtime.block_on(async { test::read_body_json(resp).await });
            if body.error.as_str() != expect_error {
                return Err(format!("unexpected 400 error: {}", body.error.as_str()));
            }
            return Ok(None);
        }
        StatusCode::UNAUTHORIZED => {
            let body: OAuth2Error = runtime.block_on(async { test::read_body_json(resp).await });
            if body.error.as_str() != expect_error {
                return Err(format!("unexpected 401 error: {}", body.error.as_str()));
            }
            return Ok(None);
        }
        _ => return Err(format!("unexpect status code: {}", resp.status())),
    }
}

async fn dummy_handler() -> impl Responder {
    HttpResponse::NoContent().finish()
}
