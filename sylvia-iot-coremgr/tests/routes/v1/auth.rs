use axum::http::{HeaderValue, Method, StatusCode, header};
use laboratory::{SpecContext, expect};

use super::{
    super::libs::{TOKEN_MEMBER, new_test_server, test_invalid_token},
    STATE,
};
use crate::TestState;

pub fn tokeninfo(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    test_invalid_token(
        runtime,
        &routes_state,
        Method::GET,
        "/coremgr/api/v1/auth/tokeninfo",
    )
}

pub fn logout(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    test_invalid_token(
        runtime,
        &routes_state,
        Method::POST,
        "/coremgr/api/v1/auth/logout",
    )
}

/// Bridged APIs reject bodies larger than 2 MiB.
pub fn logout_large_body(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    let server = new_test_server(routes_state)?;
    let req = server
        .post("/coremgr/api/v1/auth/logout")
        .add_header(
            header::AUTHORIZATION,
            HeaderValue::from_str(format!("Bearer {}", TOKEN_MEMBER).as_str()).unwrap(),
        )
        .text("0".repeat(2 * 1024 * 1024 + 1));
    let resp = runtime.block_on(async { req.await });
    expect(resp.status_code()).to_equal(StatusCode::BAD_REQUEST)
}
