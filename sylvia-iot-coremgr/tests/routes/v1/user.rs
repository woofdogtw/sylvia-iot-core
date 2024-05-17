use axum::http::Method;
use laboratory::SpecContext;

use super::{
    super::libs::{new_test_server, test_count, test_invalid_param, test_invalid_token, test_list},
    STATE, TOKEN_MANAGER,
};
use crate::TestState;

pub fn get(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    test_invalid_token(runtime, &routes_state, Method::GET, "/coremgr/api/v1/user")
}

pub fn patch(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    test_invalid_token(
        runtime,
        &routes_state,
        Method::PATCH,
        "/coremgr/api/v1/user",
    )
}

pub fn post_admin(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    test_invalid_token(runtime, &routes_state, Method::POST, "/coremgr/api/v1/user")
}

pub fn get_admin_count(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    test_invalid_token(
        runtime,
        &routes_state,
        Method::GET,
        "/coremgr/api/v1/user/count",
    )?;

    let server = new_test_server(routes_state)?;
    let req = server.get("/coremgr/api/v1/user/count");
    test_invalid_param(runtime, req, "err_param")?;

    test_count(
        runtime,
        &routes_state,
        "/coremgr/api/v1/user/count",
        &[("contains", "10")],
        TOKEN_MANAGER,
        11,
    )
}

pub fn get_admin_list(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    test_list(
        runtime,
        routes_state,
        "/coremgr/api/v1/user/list",
        TOKEN_MANAGER,
        "account,createdAt,modifiedAt,verifiedAt,roles,name,info",
    )?;

    let server = new_test_server(routes_state)?;
    let req = server.get("/coremgr/api/v1/user/list");
    test_invalid_param(runtime, req, "err_param")?;

    test_count(
        runtime,
        &routes_state,
        "/coremgr/api/v1/user/list",
        &[("contains", "10")],
        TOKEN_MANAGER,
        11,
    )
}

pub fn get_admin(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    test_invalid_token(
        runtime,
        &routes_state,
        Method::GET,
        "/coremgr/api/v1/user/id",
    )
}

pub fn patch_admin(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    test_invalid_token(
        runtime,
        &routes_state,
        Method::PATCH,
        "/coremgr/api/v1/user/id",
    )
}

pub fn delete_admin(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    test_invalid_token(
        runtime,
        &routes_state,
        Method::DELETE,
        "/coremgr/api/v1/user/id",
    )
}
