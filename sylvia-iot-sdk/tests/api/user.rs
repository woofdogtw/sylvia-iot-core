use laboratory::{describe, expect, SpecContext, Suite};

use sylvia_iot_sdk::api::{
    http::{Client as SdkClient, ClientOptions},
    user as userapi,
};

use super::{after_all_fn, before_all_fn, CLIENT_ID, CLIENT_SECRET, STATE, USER_ID};
use crate::TestState;

pub fn suite() -> Suite<TestState> {
    describe("user", |context| {
        context.it("get()", test_get);
        context.it("get() with error", test_get_err);
        context.it("update()", test_update);
        context.it("update() with error", test_update_err);

        context.before_all(before_all_fn).after_all(after_all_fn);
    })
}

fn test_get(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();

    runtime.block_on(async move {
        let opts = ClientOptions {
            auth_base: crate::TEST_AUTH_BASE.to_string(),
            coremgr_base: crate::TEST_COREMGR_BASE.to_string(),
            client_id: CLIENT_ID.to_string(),
            client_secret: CLIENT_SECRET.to_string(),
        };
        let mut client = SdkClient::new(opts);

        let result = userapi::get(&mut client).await;
        expect(result.is_ok()).to_equal(true)
    })?;
    Ok(())
}

fn test_get_err(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();

    runtime.block_on(async move {
        let opts = ClientOptions {
            auth_base: crate::TEST_AUTH_BASE.to_string(),
            coremgr_base: crate::TEST_COREMGR_BASE.to_string(),
            client_id: "error".to_string(),
            client_secret: CLIENT_SECRET.to_string(),
        };
        let mut client = SdkClient::new(opts);

        let result = userapi::get(&mut client).await;
        expect(result.is_err()).to_equal(true)
    })?;
    Ok(())
}

fn test_update(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();

    runtime.block_on(async move {
        let opts = ClientOptions {
            auth_base: crate::TEST_AUTH_BASE.to_string(),
            coremgr_base: crate::TEST_COREMGR_BASE.to_string(),
            client_id: CLIENT_ID.to_string(),
            client_secret: CLIENT_SECRET.to_string(),
        };
        let mut client = SdkClient::new(opts);

        let data = userapi::PatchReqData {
            name: Some(USER_ID.to_string()),
            ..Default::default()
        };
        let result = userapi::update(&mut client, data).await;
        expect(result.is_ok()).to_equal(true)
    })?;
    Ok(())
}

fn test_update_err(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();

    runtime.block_on(async move {
        let opts = ClientOptions {
            auth_base: crate::TEST_AUTH_BASE.to_string(),
            coremgr_base: crate::TEST_COREMGR_BASE.to_string(),
            client_id: CLIENT_ID.to_string(),
            client_secret: CLIENT_SECRET.to_string(),
        };
        let mut client = SdkClient::new(opts);

        let data = userapi::PatchReqData::default();
        let result = userapi::update(&mut client, data).await;
        expect(result.is_err()).to_equal(true)
    })?;
    Ok(())
}
