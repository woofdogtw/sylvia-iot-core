use actix_web::test::TestRequest;
use laboratory::SpecContext;

use super::{super::libs::test_invalid_token, STATE};
use crate::TestState;

pub fn tokeninfo(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    let req = TestRequest::get().uri("/coremgr/api/v1/auth/tokeninfo");
    test_invalid_token(runtime, &routes_state, req)
}

pub fn logout(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    let req = TestRequest::post().uri("/coremgr/api/v1/auth/logout");
    test_invalid_token(runtime, &routes_state, req)
}
