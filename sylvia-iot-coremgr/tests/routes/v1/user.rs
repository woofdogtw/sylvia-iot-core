use actix_web::{http::header, test::TestRequest};
use laboratory::SpecContext;

use super::{
    super::libs::{test_count, test_invalid_param, test_invalid_token, test_list},
    STATE, TOKEN_MANAGER,
};
use crate::TestState;

pub fn get(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    let req = TestRequest::get().uri("/coremgr/api/v1/user");
    test_invalid_token(runtime, &routes_state, req)
}

pub fn patch(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    let req = TestRequest::patch().uri("/coremgr/api/v1/user");
    test_invalid_token(runtime, &routes_state, req)
}

pub fn post_admin(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    let req = TestRequest::post().uri("/coremgr/api/v1/user");
    test_invalid_token(runtime, &routes_state, req)
}

pub fn get_admin_count(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    let req = TestRequest::get().uri("/coremgr/api/v1/user/count");
    test_invalid_token(runtime, &routes_state, req)?;

    let req = TestRequest::get().uri("/coremgr/api/v1/user/count");
    test_invalid_param(runtime, &routes_state, req, "err_param")?;

    let uri = "/coremgr/api/v1/user/count?contains=10";
    test_count(runtime, &routes_state, uri, TOKEN_MANAGER, 11)
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

    let req = TestRequest::get().uri("/coremgr/api/v1/user/list");
    test_invalid_param(runtime, &routes_state, req, "err_param")?;

    let uri = "/coremgr/api/v1/user/list?contains=10";
    test_count(runtime, &routes_state, uri, TOKEN_MANAGER, 11)
}

pub fn get_admin(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    let req = TestRequest::get().uri("/coremgr/api/v1/user/id");
    test_invalid_token(runtime, &routes_state, req)
}

pub fn patch_admin(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    let req = TestRequest::patch()
        .uri("/coremgr/api/v1/user/id")
        .insert_header((header::CONTENT_TYPE, "application/json"));
    test_invalid_token(runtime, &routes_state, req)
}

pub fn delete_admin(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    let req = TestRequest::delete().uri("/coremgr/api/v1/user/id");
    test_invalid_token(runtime, &routes_state, req)
}
