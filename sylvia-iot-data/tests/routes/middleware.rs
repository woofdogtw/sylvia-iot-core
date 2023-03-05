use std::{cmp::Ordering, collections::HashMap, error::Error as StdError};

use actix_web::{
    http::{header, StatusCode},
    middleware::NormalizePath,
    test::{self, TestRequest},
    web, App, HttpMessage, HttpRequest, HttpResponse, Responder,
};
use chrono::Utc;
use laboratory::{describe, expect, SpecContext, Suite};

use sylvia_iot_data::routes::middleware::{AuthService, FullTokenInfo};
use sylvia_iot_auth::models::Model;
use sylvia_iot_corelib::{err::ErrResp, role::Role};

use super::{
    clear_state,
    libs::{create_client, create_token, create_user, new_state},
    remove_sqlite, stop_auth_broker_svc,
};
use crate::TestState;

pub const STATE: &'static str = "routes/middleware";

pub fn suite(db_engine: &'static str) -> Suite<TestState> {
    let suite_name = format!("routes.middleware - {}", db_engine);
    describe(suite_name, move |context| {
        context.it("200", test_200);
        context.it("400", test_400);
        context.it("401", test_401);
        context.it("503", test_503);

        context
            .before_all(move |state| {
                state.insert(STATE, new_state(Some(db_engine)));
            })
            .after_all(after_all_fn);
    })
}

fn after_all_fn(state: &mut HashMap<&'static str, TestState>) -> () {
    let state = state.get_mut(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();

    if let Some(state) = state.routes_state.as_mut() {
        runtime.block_on(async {
            clear_state(state).await;
        });
    }

    stop_auth_broker_svc(state);

    if let Some(model) = state.mongodb.as_ref() {
        runtime.block_on(async {
            if let Err(e) = model.get_connection().drop(None).await {
                println!("remove mongodb database error: {}", e);
            }
        })
    }
    let mut path = std::env::temp_dir();
    path.push(crate::TEST_SQLITE_PATH);
    remove_sqlite(path.to_str().unwrap());
}

fn test_200(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let auth_db = state.auth_db.as_ref().unwrap();
    let auth_uri = state.auth_uri.as_ref().unwrap();

    let result: Result<(), Box<dyn StdError>> = runtime.block_on(async move {
        let now = Utc::now();
        let mut user = create_user("user", now, HashMap::<String, bool>::new());
        user.roles.insert(Role::MANAGER.to_string(), true);
        auth_db.user().add(&user).await?;
        let client = create_client("client", "user", None);
        auth_db.client().add(&client).await?;
        let token = create_token("token", "user", "client");
        auth_db.access_token().add(&token).await?;
        Ok(())
    });
    expect(result.is_ok()).to_equal(true)?;

    let mut app = runtime.block_on(async {
        test::init_service(
            App::new()
                .wrap(NormalizePath::trim())
                .wrap(AuthService::new(auth_uri.clone()))
                .route("/", web::get().to(dummy_handler)),
        )
        .await
    });

    let req = TestRequest::get()
        .uri("/")
        .insert_header((header::AUTHORIZATION, format!("Bearer token")))
        .to_request();
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    let status = resp.status();
    if status != StatusCode::NO_CONTENT {
        let body = runtime.block_on(async { test::read_body(resp).await });
        return Err(format!("status {}, body: {:?}", status, body));
    }
    Ok(())
}

fn test_400(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let auth_uri = state.auth_uri.as_ref().unwrap();

    let mut app = runtime.block_on(async {
        test::init_service(
            App::new()
                .wrap(NormalizePath::trim())
                .wrap(AuthService::new(auth_uri.clone()))
                .route("/", web::get().to(dummy_handler)),
        )
        .await
    });

    let req = TestRequest::get().uri("/").to_request();
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    expect(resp.status()).to_equal(StatusCode::BAD_REQUEST)
}

fn test_401(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let auth_uri = state.auth_uri.as_ref().unwrap();

    let mut app = runtime.block_on(async {
        test::init_service(
            App::new()
                .wrap(NormalizePath::trim())
                .wrap(AuthService::new(auth_uri.clone()))
                .route("/", web::get().to(dummy_handler)),
        )
        .await
    });

    let req = TestRequest::get()
        .uri("/")
        .insert_header((header::AUTHORIZATION, format!("Bearer test")))
        .to_request();
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    expect(resp.status()).to_equal(StatusCode::UNAUTHORIZED)
}

fn test_503(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let auth_uri = "http://localhost:65535";

    let mut app = runtime.block_on(async {
        test::init_service(
            App::new()
                .wrap(NormalizePath::trim())
                .wrap(AuthService::new(auth_uri.to_string()))
                .route("/", web::get().to(dummy_handler)),
        )
        .await
    });

    let req = TestRequest::get()
        .uri("/")
        .insert_header((header::AUTHORIZATION, format!("Bearer test")))
        .to_request();
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    expect(resp.status()).to_equal(StatusCode::SERVICE_UNAVAILABLE)
}

async fn dummy_handler(req: HttpRequest) -> impl Responder {
    match req.extensions_mut().get::<FullTokenInfo>() {
        None => {
            return Err(ErrResp::Custom(450, "", None));
        }
        Some(data) => {
            if data.info.user_id.as_str() != "user"
                && data.info.user_id.as_str() != "user1"
                && data.info.user_id.as_str() != "user2"
            {
                return Err(ErrResp::Custom(451, "", Some(data.info.user_id.clone())));
            } else if data.info.client_id.as_str() != "client"
                && data.info.client_id.as_str() != "client1"
                && data.info.client_id.as_str() != "client2"
            {
                return Err(ErrResp::Custom(452, "", Some(data.info.client_id.clone())));
            }
            let mut found = false;
            for (k, v) in data.info.roles.iter() {
                if k.as_str().cmp(Role::MANAGER) == Ordering::Equal && *v {
                    found = true;
                    break;
                }
            }
            if data.info.user_id.as_str() != "user2" && !found {
                return Err(ErrResp::Custom(453, "", None));
            }
        }
    }
    Ok(HttpResponse::NoContent().finish())
}
