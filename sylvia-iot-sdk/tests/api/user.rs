use laboratory::{SpecContext, Suite, describe, expect};

use sylvia_iot_auth::models::{Model, user::QueryCond};
use sylvia_iot_sdk::api::{
    http::{Client as SdkClient, ClientOptions},
    user as userapi,
};

use super::{CLIENT_ID, CLIENT_SECRET, STATE, USER_ID, after_all_fn, before_all_fn};
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
    let auth_db = state.auth_db.as_ref().unwrap();

    runtime.block_on(async move {
        let opts = ClientOptions {
            auth_base: crate::TEST_AUTH_BASE.to_string(),
            coremgr_base: crate::TEST_COREMGR_BASE.to_string(),
            client_id: CLIENT_ID.to_string(),
            client_secret: CLIENT_SECRET.to_string(),
        };
        let mut client = SdkClient::new(opts);

        let result = userapi::get(&mut client).await;
        expect(result.is_ok()).to_equal(true)?;
        let user_info = result.unwrap();
        let db_info = auth_db
            .user()
            .get(&QueryCond {
                account: Some(user_info.account.as_str()),
                ..Default::default()
            })
            .await;
        expect(db_info.is_ok()).to_equal(true)?;
        let db_info = db_info.unwrap();
        expect(db_info.is_some()).to_equal(true)?;
        let db_info = db_info.unwrap();
        expect(user_info.account.as_str()).to_equal(db_info.account.as_str())?;
        expect(user_info.created_at).to_equal(db_info.created_at)?;
        expect(user_info.modified_at).to_equal(db_info.modified_at)?;
        expect(user_info.verified_at).to_equal(db_info.verified_at)?;
        expect(user_info.name.as_str()).to_equal(db_info.name.as_str())
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
