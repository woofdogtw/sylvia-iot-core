use std::{collections::HashMap, error::Error as StdError, sync::mpsc, thread, time::Duration};

use actix_http::KeepAlive;
use actix_web::{
    http::{header, StatusCode},
    middleware::NormalizePath,
    test::{self, TestRequest},
    web, App, HttpMessage, HttpRequest, HttpResponse, HttpServer, Responder,
};
use chrono::{DateTime, TimeZone, Utc};
use laboratory::{describe, expect, SpecContext, Suite};
use serde_json::{Map, Value};
use tokio::{runtime::Runtime, time};

use sylvia_iot_auth::{
    libs::config as sylvia_iot_auth_config,
    models::{
        self as sylvia_iot_auth_models, access_token::AccessToken, client::Client, user::User,
        Model,
    },
    routes as sylvia_iot_auth_routes,
};

use sylvia_iot_sdk::{
    middlewares::auth::{AuthService, FullTokenInfo},
    util::err::ErrResp,
};

use crate::{TestState, WAIT_COUNT, WAIT_TICK};

pub const STATE: &'static str = "middlewares/auth";

pub fn suite() -> Suite<TestState> {
    describe("auth", |context| {
        context.it("200", test_200);
        context.it("400", test_400);
        context.it("401", test_401);
        context.it("503", test_503);

        context.before_all(before_all_fn).after_all(after_all_fn);
    })
}

fn test_200(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let auth_db = state.auth_db.as_ref().unwrap();
    let auth_uri = state.auth_uri.as_ref().unwrap();

    let result: Result<(), Box<dyn StdError>> = runtime.block_on(async move {
        let now = Utc::now();
        let user = create_user("user", now, HashMap::<String, bool>::new());
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
        }
    }
    Ok(HttpResponse::NoContent().finish())
}

fn before_all_fn(state: &mut HashMap<&'static str, TestState>) -> () {
    let runtime = match Runtime::new() {
        Err(e) => panic!("create runtime error: {}", e),
        Ok(runtime) => runtime,
    };

    let (tx, rx) = mpsc::channel();
    {
        let state = match runtime.block_on(async {
            let mut path = std::env::temp_dir();
            path.push(sylvia_iot_auth_config::DEF_SQLITE_PATH);
            let conf = sylvia_iot_auth_config::Config {
                db: Some(sylvia_iot_auth_config::Db {
                    engine: Some("sqlite".to_string()),
                    sqlite: Some(sylvia_iot_auth_config::Sqlite {
                        path: Some(path.to_str().unwrap().to_string()),
                    }),
                    ..Default::default()
                }),
                ..Default::default()
            };
            sylvia_iot_auth_routes::new_state("auth", &conf).await
        }) {
            Err(e) => panic!("create auth state error: {}", e),
            Ok(state) => state,
        };

        thread::spawn(move || {
            let srv = match HttpServer::new(move || {
                App::new()
                    .wrap(NormalizePath::trim())
                    .service(sylvia_iot_auth_routes::new_service(&state))
            })
            .keep_alive(KeepAlive::Timeout(Duration::from_secs(60)))
            .shutdown_timeout(1)
            .bind("0.0.0.0:1080")
            {
                Err(e) => panic!("bind auth server error: {}", e),
                Ok(server) => server,
            }
            .run();

            let _ = tx.send(srv.handle());
            let runtime = match Runtime::new() {
                Err(e) => panic!("create auth server runtime error: {}", e),
                Ok(runtime) => runtime,
            };
            runtime.block_on(async { srv.await })
        });
    };
    let auth_svc = rx.recv().unwrap();

    if let Err(e) = runtime.block_on(async {
        for _ in 0..WAIT_COUNT {
            if reqwest::get("http://localhost:1080").await.is_ok() {
                return Ok(());
            }
            time::sleep(Duration::from_millis(WAIT_TICK)).await;
        }
        Err("timeout")
    }) {
        panic!("create auth server error: {}", e);
    }

    let auth_uri = Some(format!("{}/api/v1/auth/tokeninfo", crate::TEST_AUTH_BASE));

    let auth_db = match runtime.block_on(async {
        let mut path = std::env::temp_dir();
        path.push(sylvia_iot_auth_config::DEF_SQLITE_PATH);
        sylvia_iot_auth_models::SqliteModel::new(&sylvia_iot_auth_models::SqliteOptions {
            path: path.to_str().unwrap().to_string(),
        })
        .await
    }) {
        Err(e) => panic!("create auth DB model error: {}", e),
        Ok(model) => Some(model),
    };

    state.insert(
        STATE,
        TestState {
            runtime: Some(runtime),
            auth_db,
            auth_svc: Some(auth_svc),
            auth_uri,
            ..Default::default()
        },
    );
}

fn after_all_fn(state: &mut HashMap<&'static str, TestState>) -> () {
    let state = state.get_mut(STATE).unwrap();

    stop_auth_svc(state);
}

fn remove_sqlite(path: &str) {
    if let Err(e) = std::fs::remove_file(path) {
        println!("remove file {} error: {}", path, e);
    }
    let file = format!("{}-shm", path);
    if let Err(e) = std::fs::remove_file(file.as_str()) {
        println!("remove file {} error: {}", file.as_str(), e);
    }
    let file = format!("{}-wal", path);
    if let Err(e) = std::fs::remove_file(file.as_str()) {
        println!("remove file {} error: {}", file.as_str(), e);
    }
}

fn stop_auth_svc(state: &TestState) {
    let runtime = state.runtime.as_ref().unwrap();
    if let Some(svc) = state.auth_svc.as_ref() {
        runtime.block_on(async { svc.stop(false).await });
    }
    let mut path = std::env::temp_dir();
    path.push(sylvia_iot_auth_config::DEF_SQLITE_PATH);
    remove_sqlite(path.to_str().unwrap());
}

pub fn create_user(name: &str, time: DateTime<Utc>, roles: HashMap<String, bool>) -> User {
    User {
        user_id: name.to_string(),
        account: name.to_string(),
        created_at: time,
        modified_at: time,
        verified_at: Some(time),
        expired_at: None,
        disabled_at: None,
        roles,
        password: "password".to_string(),
        salt: name.to_string(),
        name: name.to_string(),
        info: Map::<String, Value>::new(),
    }
}

pub fn create_client(name: &str, user_id: &str, secret: Option<String>) -> Client {
    let now = Utc::now();
    Client {
        client_id: name.to_string(),
        created_at: now,
        modified_at: now,
        client_secret: secret,
        redirect_uris: vec![crate::TEST_REDIRECT_URI.to_string()],
        scopes: vec![],
        user_id: user_id.to_string(),
        name: name.to_string(),
        image_url: None,
    }
}

pub fn create_token(token: &str, user_id: &str, client_id: &str) -> AccessToken {
    let expires_at = Utc.timestamp_nanos((Utc::now().timestamp() + 3600) * 1000000000);
    AccessToken {
        access_token: token.to_string(),
        refresh_token: None,
        expires_at,
        scope: None,
        client_id: client_id.to_string(),
        redirect_uri: "http://localhost".to_string(),
        user_id: user_id.to_string(),
    }
}
