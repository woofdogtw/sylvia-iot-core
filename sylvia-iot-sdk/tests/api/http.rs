use actix_web::web::Bytes;
use laboratory::{describe, expect, SpecContext, Suite};
use reqwest::{Method, StatusCode};

use sylvia_iot_auth::models::{self as sylvia_iot_auth_models, Model};
use sylvia_iot_sdk::api::http::{Client as SdkClient, ClientOptions};

use super::{after_all_fn, before_all_fn, CLIENT_ID, CLIENT_SECRET, STATE, USER_ID};
use crate::TestState;

pub fn suite() -> Suite<TestState> {
    describe("http", |context| {
        context.it("new()", test_new);
        context.it("request()", test_req);
        context.it("request() with refreshing token", test_req_refresh);
        context.it("request() with error", test_req_err);

        context.before_all(before_all_fn).after_all(after_all_fn);
    })
}

fn test_new(_: &mut SpecContext<TestState>) -> Result<(), String> {
    let opts = ClientOptions {
        auth_base: crate::TEST_AUTH_BASE.to_string(),
        coremgr_base: crate::TEST_COREMGR_BASE.to_string(),
        client_id: CLIENT_ID.to_string(),
        client_secret: CLIENT_SECRET.to_string(),
    };
    let _ = SdkClient::new(opts);

    Ok(())
}

fn test_req(context: &mut SpecContext<TestState>) -> Result<(), String> {
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

        let result = client.request(Method::GET, "/api/v1/user", None).await;
        expect(result.is_ok()).to_equal(true)?;

        // Request twice to use in memory token.
        let result = client
            .request(
                Method::GET,
                "/api/v1/user",
                Some(Bytes::copy_from_slice(b"test")),
            )
            .await;
        expect(result.is_ok()).to_equal(true)
    })?;

    Ok(())
}

fn test_req_refresh(context: &mut SpecContext<TestState>) -> Result<(), String> {
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

        let result = client.request(Method::GET, "/api/v1/user", None).await;
        expect(result.is_ok()).to_equal(true)?;

        let cond = sylvia_iot_auth_models::access_token::QueryCond {
            user_id: Some(USER_ID),
            ..Default::default()
        };
        if let Err(e) = auth_db.access_token().del(&cond).await {
            return Err(format!("remove access token error: {}", e));
        }
        let cond = sylvia_iot_auth_models::refresh_token::QueryCond {
            user_id: Some(USER_ID),
            ..Default::default()
        };
        if let Err(e) = auth_db.refresh_token().del(&cond).await {
            return Err(format!("remove refresh token error: {}", e));
        }

        // Request to cover refresh token.
        let result = client.request(Method::GET, "/api/v1/user", None).await;
        match result {
            Err(e) => return Err(format!("error: {:?}", e)),
            Ok(result) => expect(result.0).to_equal(StatusCode::OK),
        }
    })?;

    Ok(())
}

fn test_req_err(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();

    runtime.block_on(async move {
        let opts = ClientOptions {
            auth_base: "".to_string(),
            coremgr_base: crate::TEST_COREMGR_BASE.to_string(),
            client_id: CLIENT_ID.to_string(),
            client_secret: CLIENT_SECRET.to_string(),
        };
        let mut client = SdkClient::new(opts);
        let result = client.request(Method::GET, "/api/v1/user", None).await;
        expect(result.is_err()).to_equal(true)?;

        let opts = ClientOptions {
            auth_base: "http://localhost:1234".to_string(),
            coremgr_base: crate::TEST_COREMGR_BASE.to_string(),
            client_id: CLIENT_ID.to_string(),
            client_secret: CLIENT_SECRET.to_string(),
        };
        let mut client = SdkClient::new(opts);
        let result = client.request(Method::GET, "/api/v1/user", None).await;
        expect(result.is_err()).to_equal(true)?;

        let opts = ClientOptions {
            auth_base: crate::TEST_AUTH_BASE.to_string(),
            coremgr_base: crate::TEST_COREMGR_BASE.to_string(),
            client_id: "error".to_string(),
            client_secret: CLIENT_SECRET.to_string(),
        };
        let mut client = SdkClient::new(opts);
        let result = client.request(Method::GET, "/api/v1/user", None).await;
        expect(result.is_err()).to_equal(true)?;

        let opts = ClientOptions {
            auth_base: crate::TEST_AUTH_BASE.to_string(),
            coremgr_base: "".to_string(),
            client_id: CLIENT_ID.to_string(),
            client_secret: CLIENT_SECRET.to_string(),
        };
        let mut client = SdkClient::new(opts);
        let result = client.request(Method::GET, "/api/v1/user", None).await;
        expect(result.is_err()).to_equal(true)?;

        let opts = ClientOptions {
            auth_base: crate::TEST_AUTH_BASE.to_string(),
            coremgr_base: "http://localhost:1234".to_string(),
            client_id: CLIENT_ID.to_string(),
            client_secret: CLIENT_SECRET.to_string(),
        };
        let mut client = SdkClient::new(opts);
        let result = client.request(Method::GET, "/api/v1/user", None).await;
        expect(result.is_err()).to_equal(true)?;

        let opts = ClientOptions {
            auth_base: crate::TEST_AUTH_BASE.to_string(),
            coremgr_base: crate::TEST_COREMGR_BASE.to_string(),
            client_id: "error".to_string(),
            client_secret: CLIENT_SECRET.to_string(),
        };
        let mut client = SdkClient::new(opts);
        let result = client.request(Method::GET, "/api/v1/user", None).await;
        expect(result.is_err()).to_equal(true)
    })?;

    Ok(())
}
