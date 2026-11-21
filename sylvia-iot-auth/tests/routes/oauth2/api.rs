use std::collections::HashMap;

use axum::{
    Router,
    http::{HeaderValue, Method, StatusCode, header},
    response::IntoResponse,
    routing,
};
use axum_test::TestServer;
use chrono::{TimeDelta, Utc};
use laboratory::{SpecContext, expect};
use tokio::runtime::Runtime;

use sylvia_iot_auth::{
    models::{
        access_token::AccessToken,
        login_session::LoginSession,
        refresh_token::RefreshToken,
        user::{QueryCond as UserQueryCond, Updates as UserUpdates},
    },
    routes::{
        self,
        oauth2::middleware::{AuthService, RoleScopeType},
    },
};
use sylvia_iot_corelib::strings;

use super::{
    super::{libs::create_user, read_location},
    STATE, TestState, request,
    response::{self, OAuth2Error},
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
    )?;

    // The page uses the validated state, so unknown parameters are dropped and the content is
    // encoded.
    let app = Router::new().merge(routes::new_service(routes_state));
    let server = TestServer::new(app);
    let raw_state = format!(
        "response_type=code&client_id=public&{}&state=\"><script>alert(1)</script>&allow=yes",
        serde_urlencoded::to_string([("redirect_uri", crate::TEST_REDIRECT_URI)]).unwrap()
    );
    let req = server
        .get("/auth/oauth2/login")
        .add_query_param("state", raw_state.as_str());
    let resp = runtime.block_on(async { req.await });
    expect(resp.status_code()).to_equal(StatusCode::OK)?;
    let body = resp.text();
    expect(body.contains("allow=yes")).to_equal(false)?;
    expect(body.contains("<script>alert(1)</script>")).to_equal(false)
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

    // The redirection uses the validated state, so unknown parameters are dropped.
    let app = Router::new().merge(routes::new_service(routes_state));
    let server = TestServer::new(app);
    let raw_state = format!(
        "response_type=code&client_id=public&{}&allow=yes",
        serde_urlencoded::to_string([("redirect_uri", crate::TEST_REDIRECT_URI)]).unwrap()
    );
    let req = server
        .post("/auth/oauth2/login")
        .form(&request::PostLoginRequest {
            account: "user".to_string(),
            password: "user".to_string(),
            state: raw_state,
        });
    let resp = runtime.block_on(async { req.await });
    expect(resp.status_code()).to_equal(StatusCode::FOUND)?;
    let location = read_location(&resp)?;
    expect(location.as_str().contains("allow=")).to_equal(false)?;
    expect(location.as_str().contains("session_id=")).to_equal(true)
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
    test_get_authorize(runtime, routes_state, Some(&params), StatusCode::OK, "")?;

    // GET cannot allow the authorization, and the content in the grant page is HTML-escaped.
    let params = request::GetAuthorizeRequest {
        response_type: "code".to_string(),
        redirect_uri: crate::TEST_REDIRECT_URI.to_string(),
        client_id: "public".to_string(),
        scope: None,
        state: Some("\"><script>alert(1)</script>".to_string()),
        session_id: session_id.clone(),
    };
    let app = Router::new().merge(routes::new_service(routes_state));
    let server = TestServer::new(app);
    let req = server
        .get("/auth/oauth2/authorize")
        .add_query_params(&params)
        .add_query_param("allow", ALLOW_VALUE);
    let resp = runtime.block_on(async { req.await });
    expect(resp.status_code()).to_equal(StatusCode::OK)?;
    let body = resp.text();
    expect(body.contains("<script>alert(1)</script>")).to_equal(false)?;
    expect(body.contains("&quot;&gt;&lt;script&gt;")).to_equal(true)
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
    // The session is removed after denying.
    test_post_authorize(
        runtime,
        routes_state,
        Some(&params),
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

    // The session of the public client cannot be used for the private client.
    let params = request::PostAuthorizeRequest {
        response_type: "code".to_string(),
        redirect_uri: crate::TEST_REDIRECT_URI.to_string(),
        client_id: "private".to_string(),
        scope: Some("scope1".to_string()),
        state: None,
        session_id: session_id.clone(),
        allow: Some(ALLOW_VALUE.to_string()),
    };
    test_post_authorize(
        runtime,
        routes_state,
        Some(&params),
        StatusCode::BAD_REQUEST,
        INVALID_AUTH,
    )?;

    // The session can be used only once.
    let login_params = request::GetAuthRequest {
        response_type: "code".to_string(),
        redirect_uri: crate::TEST_REDIRECT_URI.to_string(),
        client_id: "private".to_string(),
        scope: Some("scope1".to_string()),
        state: None,
    };
    let session_id = match test_post_login(
        runtime,
        routes_state,
        Some(&login_params),
        None,
        ("user", "user"),
        StatusCode::FOUND,
        "",
    )? {
        None => return Err("get no session ID".to_string()),
        Some(session_id) => session_id,
    };
    let params = request::PostAuthorizeRequest {
        session_id,
        ..params
    };
    test_post_authorize(runtime, routes_state, Some(&params), StatusCode::FOUND, "")?;
    test_post_authorize(
        runtime,
        routes_state,
        Some(&params),
        StatusCode::BAD_REQUEST,
        INVALID_AUTH,
    )?;

    // Expired sessions cannot be used.
    let session = LoginSession {
        session_id: "session_id_expired".to_string(),
        expires_at: Utc::now() - TimeDelta::try_minutes(1).unwrap(),
        user_id: "user".to_string(),
        client_id: "private".to_string(),
        redirect_uri: crate::TEST_REDIRECT_URI.to_string(),
    };
    if let Err(e) =
        runtime.block_on(async { routes_state.model.login_session().add(&session).await })
    {
        return Err(format!("add expired session error: {}", e));
    }
    let params = request::PostAuthorizeRequest {
        session_id: session.session_id.clone(),
        ..params
    };
    test_post_authorize(
        runtime,
        routes_state,
        Some(&params),
        StatusCode::BAD_REQUEST,
        INVALID_AUTH,
    )?;

    Ok(())
}

pub fn user_status(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();
    let model = routes_state.model.clone();

    // A user with the legacy password hash (10000 rounds) of `legacy`.
    let mut user = create_user("legacy", Utc::now(), HashMap::new());
    user.password = "83aa57aeac5fbf0d4e25fb944629c7bb915bd874c112e95bd3abe7b2075086d0".to_string();
    if let Err(e) = runtime.block_on(async { model.user().add(&user).await }) {
        return Err(format!("add user error: {}", e));
    }

    // The legacy hash is upgraded after logging in.
    let params = request::GetAuthRequest {
        response_type: "code".to_string(),
        redirect_uri: crate::TEST_REDIRECT_URI.to_string(),
        client_id: "public".to_string(),
        scope: None,
        state: None,
    };
    // The first login upgrades the hash with a new salt, and the second login keeps them.
    let mut upgraded: Option<(String, String)> = None;
    for _ in 0..2 {
        test_post_login(
            runtime,
            routes_state,
            Some(&params),
            None,
            ("legacy", "legacy"),
            StatusCode::FOUND,
            "",
        )?;
        let cond = UserQueryCond {
            user_id: Some("legacy"),
            account: None,
        };
        let user = match runtime.block_on(async { model.user().get(&cond).await }) {
            Err(e) => return Err(format!("get user error: {}", e)),
            Ok(None) => return Err("get no user".to_string()),
            Ok(Some(user)) => user,
        };
        expect(user.salt.as_str()).to_not_equal("legacy")?;
        expect(user.password.starts_with("$pbkdf2-sha256$i=600000$")).to_equal(true)?;
        expect(strings::password_verify(
            "legacy",
            user.salt.as_str(),
            user.password.as_str(),
        ))
        .to_equal(true)?;
        match upgraded.as_ref() {
            None => upgraded = Some((user.salt, user.password)),
            Some((salt, hash)) => {
                expect(user.salt.as_str()).to_equal(salt.as_str())?;
                expect(user.password.as_str()).to_equal(hash.as_str())?;
            }
        }
    }

    // Tokens of disabled or expired users are invalid.
    let now = Utc::now();
    let access = AccessToken {
        access_token: "access_token_status".to_string(),
        refresh_token: Some("refresh_token_status".to_string()),
        expires_at: now + TimeDelta::try_hours(1).unwrap(),
        scope: None,
        client_id: "public".to_string(),
        redirect_uri: crate::TEST_REDIRECT_URI.to_string(),
        user_id: "legacy".to_string(),
    };
    let refresh = RefreshToken {
        refresh_token: "refresh_token_status".to_string(),
        expires_at: now + TimeDelta::try_hours(1).unwrap(),
        scope: None,
        client_id: "public".to_string(),
        redirect_uri: crate::TEST_REDIRECT_URI.to_string(),
        user_id: "legacy".to_string(),
    };
    if let Err(e) = runtime.block_on(async {
        model.access_token().add(&access).await?;
        model.refresh_token().add(&refresh).await
    }) {
        return Err(format!("add token error: {}", e));
    }
    let app = Router::new().merge(routes::new_service(routes_state));
    let server = TestServer::new(app);
    let tokeninfo = |expect_status: StatusCode| -> Result<(), String> {
        let req = server.get("/auth/api/v1/auth/tokeninfo").add_header(
            header::AUTHORIZATION,
            HeaderValue::from_str("Bearer access_token_status").unwrap(),
        );
        let resp = runtime.block_on(async { req.await });
        expect(resp.status_code()).to_equal(expect_status)
    };
    let mut refresh_params = request::PostRefreshRequest {
        grant_type: "refresh_token".to_string(),
        refresh_token: "refresh_token_status".to_string(),
        scope: None,
        client_id: None,
    };
    tokeninfo(StatusCode::OK)?;

    let updates_dataset = [
        UserUpdates {
            disabled_at: Some(Some(now)),
            ..Default::default()
        },
        UserUpdates {
            disabled_at: Some(None),
            expired_at: Some(Some(now - TimeDelta::try_minutes(1).unwrap())),
            ..Default::default()
        },
    ];
    for updates in updates_dataset.iter() {
        if let Err(e) = runtime.block_on(async { model.user().update("legacy", updates).await }) {
            return Err(format!("update user error: {}", e));
        }
        tokeninfo(StatusCode::UNAUTHORIZED)?;
        test_post_refresh(
            runtime,
            routes_state,
            Some(&mut refresh_params),
            None,
            StatusCode::BAD_REQUEST,
            INVALID_GRANT,
        )?;
    }

    // The tokens are valid again after the user is enabled.
    let updates = UserUpdates {
        expired_at: Some(None),
        ..Default::default()
    };
    if let Err(e) = runtime.block_on(async { model.user().update("legacy", &updates).await }) {
        return Err(format!("update user error: {}", e));
    }
    tokeninfo(StatusCode::OK)?;
    test_post_refresh(
        runtime,
        routes_state,
        Some(&mut refresh_params),
        None,
        StatusCode::OK,
        "",
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
    let app = Router::new()
        .route("/", routing::get(dummy_handler).post(dummy_handler))
        .layer(AuthService::new(&model, role_scopes_root));
    let server = TestServer::new(app);

    let req = server.get("/");
    let resp = runtime.block_on(async { req.await });
    let status = resp.status_code();
    if status != StatusCode::BAD_REQUEST {
        let body = resp.text();
        return Err(format!("public case status {}, body: {:?}", status, body));
    }
    let req = server.get("/").add_header(
        header::AUTHORIZATION,
        HeaderValue::from_str(format!("Bearer {}", public_token).as_str()).unwrap(),
    );
    let resp = runtime.block_on(async { req.await });
    let status = resp.status_code();
    if status != StatusCode::NO_CONTENT {
        let body = resp.text();
        return Err(format!("public case status {}, body: {:?}", status, body));
    }
    let req = server.post("/").add_header(
        header::AUTHORIZATION,
        HeaderValue::from_str(format!("Bearer {}", public_token).as_str()).unwrap(),
    );
    let resp = runtime.block_on(async { req.await });
    let status = resp.status_code();
    if status != StatusCode::FORBIDDEN {
        let body = resp.text();
        return Err(format!("public case status {}, body: {:?}", status, body));
    }

    let req = server.get("/").add_header(
        header::AUTHORIZATION,
        HeaderValue::from_str(format!("Bearer {}", private_token).as_str()).unwrap(),
    );
    let resp = runtime.block_on(async { req.await });
    let status = resp.status_code();
    if status != StatusCode::NO_CONTENT {
        let body = resp.text();
        return Err(format!("private case status {}, body: {:?}", status, body));
    }
    let req = server.post("/").add_header(
        header::AUTHORIZATION,
        HeaderValue::from_str(format!("Bearer {}", private_token).as_str()).unwrap(),
    );
    let resp = runtime.block_on(async { req.await });
    let status = resp.status_code();
    if status != StatusCode::NO_CONTENT {
        let body = resp.text();
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
    let app = Router::new().merge(routes::new_service(&state));
    let server = TestServer::new(app);

    let req = server.get("/auth/oauth2/auth").add_query_params(&params);
    let resp = runtime.block_on(async { req.await });
    let status = resp.status_code();
    if status != expect_status {
        return Err(format!(
            "get auth response not {}, {}",
            expect_status, status
        ));
    }
    match status {
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
            let body: OAuth2Error = resp.json();
            if body.error.as_str() != expect_error {
                return Err(format!("unexpected 400 error: {}", body.error.as_str()));
            }
            return Ok(());
        }
        _ => return Err(format!("unexpect status code: {}", status)),
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
    let app = Router::new().merge(routes::new_service(&state));
    let server = TestServer::new(app);

    let mut req = server.get("/auth/oauth2/login");
    match params {
        None => {
            if let Some(raw) = params_raw {
                req = req.add_query_param("state", raw);
            }
        }
        Some(params) => {
            req = req.add_query_params(&request::GetLoginRequest {
                state: serde_urlencoded::to_string(params).unwrap(),
            });
        }
    };
    let resp = runtime.block_on(async { req.await });
    let status = resp.status_code();
    if status != expect_status {
        return Err(format!(
            "get login response not {}, {}",
            expect_status, status
        ));
    }
    match status {
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
            let body: OAuth2Error = resp.json();
            if body.error.as_str() != expect_error {
                return Err(format!("unexpected 400 error: {}", body.error.as_str()));
            }
            return Ok(());
        }
        _ => return Err(format!("unexpect status code: {}", status)),
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
    let app = Router::new().merge(routes::new_service(&state));
    let server = TestServer::new(app);

    let uri = "/auth/oauth2/login";
    let req = match params {
        None => match params_raw {
            None => server.post(uri),
            Some(raw) => server.post(uri).form(&request::PostLoginRequest {
                account: auth.0.to_string(),
                password: auth.1.to_string(),
                state: raw.to_string(),
            }),
        },
        Some(params) => server.post(uri).form(&request::PostLoginRequest {
            account: auth.0.to_string(),
            password: auth.1.to_string(),
            state: serde_urlencoded::to_string(params).unwrap(),
        }),
    };
    let resp = runtime.block_on(async { req.await });
    let status = resp.status_code();
    if status != expect_status {
        return Err(format!(
            "post login response not {}, {}",
            expect_status, status
        ));
    }
    match status {
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
            let body: OAuth2Error = resp.json();
            if body.error.as_str() != expect_error {
                return Err(format!("unexpected 400 error: {}", body.error.as_str()));
            }
            return Ok(None);
        }
        _ => return Err(format!("unexpect status code: {}", status)),
    }
}

fn test_get_authorize(
    runtime: &Runtime,
    state: &routes::State,
    params: Option<&request::GetAuthorizeRequest>,
    expect_status: StatusCode,
    expect_error: &str,
) -> Result<(), String> {
    let app = Router::new().merge(routes::new_service(&state));
    let server = TestServer::new(app);

    let req = server
        .get("/auth/oauth2/authorize")
        .add_query_params(&params);
    let resp = runtime.block_on(async { req.await });
    let status = resp.status_code();
    if status != expect_status {
        return Err(format!(
            "get authorize response not {}, {}",
            expect_status, status
        ));
    }
    match status {
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
            let body: OAuth2Error = resp.json();
            if body.error.as_str() != expect_error {
                return Err(format!("unexpected 400 error: {}", body.error.as_str()));
            }
            return Ok(());
        }
        _ => return Err(format!("unexpect status code: {}", status)),
    }
}

fn test_post_authorize(
    runtime: &Runtime,
    state: &routes::State,
    params: Option<&request::PostAuthorizeRequest>,
    expect_status: StatusCode,
    expect_error: &str,
) -> Result<Option<String>, String> {
    let app = Router::new().merge(routes::new_service(&state));
    let server = TestServer::new(app);

    let req = server.post("/auth/oauth2/authorize").form(&params);
    let resp = runtime.block_on(async { req.await });
    let status = resp.status_code();
    if status != expect_status {
        return Err(format!(
            "post authorize response not {}, {}",
            expect_status, status
        ));
    }
    match status {
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
            let body: OAuth2Error = resp.json();
            if body.error.as_str() != expect_error {
                return Err(format!("unexpected 400 error: {}", body.error.as_str()));
            }
            return Ok(None);
        }
        _ => return Err(format!("unexpect status code: {}", status)),
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
    let app = Router::new().merge(routes::new_service(&state));
    let server = TestServer::new(app);

    let req = match params {
        None => server.post("/auth/oauth2/token"),
        Some(params) => {
            if params.code.as_str() == "public" || params.code.as_str() == "private" {
                // The login session is bound to the client, so log in with the same client.
                let mut auth_params = request::GetAuthRequest {
                    response_type: "code".to_string(),
                    redirect_uri: crate::TEST_REDIRECT_URI.to_string(),
                    client_id: "public".to_string(),
                    scope: None,
                    state: None,
                };
                if params.code.as_str() == "private" {
                    auth_params.client_id = "private".to_string();
                    auth_params.scope = Some("scope1".to_string());
                }
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
                None => server.post("/auth/oauth2/token").form(params),
                Some(header) => server
                    .post("/auth/oauth2/token")
                    .add_header(
                        header::AUTHORIZATION,
                        HeaderValue::from_str(header).unwrap(),
                    )
                    .form(params),
            }
        }
    };
    let resp = runtime.block_on(async { req.await });
    let status = resp.status_code();
    if status != expect_status {
        return Err(format!(
            "post token response not {}, {}",
            expect_status, status
        ));
    }
    match status {
        StatusCode::OK => {
            let body: response::AccessToken = resp.json();
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
            let body: OAuth2Error = resp.json();
            if body.error.as_str() != expect_error {
                return Err(format!("unexpected 400 error: {}", body.error.as_str()));
            }
            return Ok(None);
        }
        StatusCode::UNAUTHORIZED => {
            let body: OAuth2Error = resp.json();
            if body.error.as_str() != expect_error {
                return Err(format!("unexpected 401 error: {}", body.error.as_str()));
            }
            return Ok(None);
        }
        _ => return Err(format!("unexpect status code: {}", status)),
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
    let app = Router::new().merge(routes::new_service(&state));
    let server = TestServer::new(app);

    let params = request::PostTokenClientRequest {
        grant_type: "client_credentials".to_string(),
        scope: match scope {
            None => None,
            Some(scope) => Some(scope.to_string()),
        },
    };
    let req = match auth_header {
        None => server.post("/auth/oauth2/token").form(&params),
        Some(header) => server
            .post("/auth/oauth2/token")
            .add_header(
                header::AUTHORIZATION,
                HeaderValue::from_str(header).unwrap(),
            )
            .form(&params),
    };
    let resp = runtime.block_on(async { req.await });
    let status = resp.status_code();
    if status != expect_status {
        return Err(format!(
            "post token response not {}, {}",
            expect_status, status
        ));
    }
    match status {
        StatusCode::OK => {
            let body: response::AccessToken = resp.json();
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
            let body: OAuth2Error = resp.json();
            if body.error.as_str() != expect_error {
                return Err(format!("unexpected 400 error: {}", body.error.as_str()));
            }
            return Ok(None);
        }
        StatusCode::UNAUTHORIZED => {
            let body: OAuth2Error = resp.json();
            if body.error.as_str() != expect_error {
                return Err(format!("unexpected 401 error: {}", body.error.as_str()));
            }
            return Ok(None);
        }
        _ => return Err(format!("unexpect status code: {}", status)),
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
    let app = Router::new().merge(routes::new_service(&state));
    let server = TestServer::new(app);

    let req = match params {
        None => server.post("/auth/oauth2/refresh"),
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
                None => server.post("/auth/oauth2/refresh").form(params),
                Some(header) => server
                    .post("/auth/oauth2/refresh")
                    .add_header(
                        header::AUTHORIZATION,
                        HeaderValue::from_str(header).unwrap(),
                    )
                    .form(params),
            }
        }
    };
    let resp = runtime.block_on(async { req.await });
    let status = resp.status_code();
    if status != expect_status {
        return Err(format!(
            "post refresh response not {}, {}",
            expect_status, status
        ));
    }
    match status {
        StatusCode::OK => {
            let body: response::AccessToken = resp.json();
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
            let body: OAuth2Error = resp.json();
            if body.error.as_str() != expect_error {
                return Err(format!("unexpected 400 error: {}", body.error.as_str()));
            }
            return Ok(None);
        }
        StatusCode::UNAUTHORIZED => {
            let body: OAuth2Error = resp.json();
            if body.error.as_str() != expect_error {
                return Err(format!("unexpected 401 error: {}", body.error.as_str()));
            }
            return Ok(None);
        }
        _ => return Err(format!("unexpect status code: {}", status)),
    }
}

async fn dummy_handler() -> impl IntoResponse {
    StatusCode::NO_CONTENT
}
