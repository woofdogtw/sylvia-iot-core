use actix_web::test::TestRequest;
use laboratory::SpecContext;

use super::{
    super::libs::{test_invalid_token, test_list, TOKEN_MANAGER},
    STATE,
};
use crate::TestState;

pub fn get_count(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    let req = TestRequest::get().uri("/coremgr/api/v1/dldata-buffer/count");
    test_invalid_token(runtime, &routes_state, req)
}

pub fn get_list(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    test_list(
        runtime,
        routes_state,
        "/coremgr/api/v1/dldata-buffer/list",
        TOKEN_MANAGER,
        "dataId,unitId,applicationId,applicationCode,deviceId,networkId,createdAt,expiredAt",
    )
}

pub fn delete(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    let req = TestRequest::delete().uri("/coremgr/api/v1/dldata-buffer/id");
    test_invalid_token(runtime, &routes_state, req)
}
