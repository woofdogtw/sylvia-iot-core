use std::{collections::HashMap, error::Error as StdError, net::SocketAddr, time::Duration};

use axum::{
    http::{header, HeaderValue, StatusCode},
    response::IntoResponse,
    routing, Extension, Router,
};
use axum_test::TestServer;
use chrono::{DateTime, TimeZone, Utc};
use laboratory::{describe, expect, SpecContext, Suite};
use serde_json::{Map, Value};
use tokio::{net::TcpListener, runtime::Runtime, time};

use sylvia_iot_auth::{
    libs::config as sylvia_iot_auth_config,
    models::{
        self as sylvia_iot_auth_models, access_token::AccessToken, client::Client, user::User,
        Model,
    },
    routes as sylvia_iot_auth_routes,
};

use sylvia_iot_sdk::{
    middlewares::auth::{AuthService, GetTokenInfoData},
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
        let mut roles = HashMap::<String, bool>::new();
        roles.insert("user".to_string(), true);
        let user = create_user("user", now, roles);
        auth_db.user().add(&user).await?;
        let client = create_client("client", "user", None);
        auth_db.client().add(&client).await?;
        let token = create_token("token", "user", "client");
        auth_db.access_token().add(&token).await?;
        Ok(())
    });
    expect(result.is_ok()).to_equal(true)?;

    let app = Router::new()
        .route("/", routing::get(test_200_handler))
        .layer(AuthService::new(auth_uri.clone()));
    let server = match TestServer::new(app) {
        Err(e) => return Err(format!("new server error: {}", e)),
        Ok(server) => server,
    };

    let req = server.get("/").add_header(
        header::AUTHORIZATION,
        HeaderValue::from_str("  bearer token  ").unwrap(),
    );
    let resp = runtime.block_on(async { req.await });
    let status = resp.status_code();
    if status != StatusCode::NO_CONTENT {
        let body = resp.text();
        return Err(format!("status {}, body: {:?}", status, body));
    }
    Ok(())
}

fn test_400(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let auth_uri = state.auth_uri.as_ref().unwrap();

    let app = Router::new()
        .route("/", routing::get(dummy_handler))
        .layer(AuthService::new(auth_uri.clone()));
    let server = match TestServer::new(app) {
        Err(e) => return Err(format!("new server error: {}", e)),
        Ok(server) => server,
    };

    let req = server.get("/");
    let resp = runtime.block_on(async { req.await });
    expect(resp.status_code()).to_equal(StatusCode::BAD_REQUEST)
}

fn test_401(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let auth_uri = state.auth_uri.as_ref().unwrap();

    let app = Router::new()
        .route("/", routing::get(dummy_handler))
        .layer(AuthService::new(auth_uri.clone()));
    let server = match TestServer::new(app) {
        Err(e) => return Err(format!("new server error: {}", e)),
        Ok(server) => server,
    };

    let req = server.get("/").add_header(
        header::AUTHORIZATION,
        HeaderValue::from_str("Bearer test").unwrap(),
    );
    let resp = runtime.block_on(async { req.await });
    expect(resp.status_code()).to_equal(StatusCode::UNAUTHORIZED)
}

fn test_503(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let auth_uri = "http://localhost:65535";

    let app = Router::new()
        .route("/", routing::get(dummy_handler))
        .layer(AuthService::new(auth_uri.to_string()));
    let server = match TestServer::new(app) {
        Err(e) => return Err(format!("new server error: {}", e)),
        Ok(server) => server,
    };

    let req = server.get("/").add_header(
        header::AUTHORIZATION,
        HeaderValue::from_str("Bearer test").unwrap(),
    );
    let resp = runtime.block_on(async { req.await });
    expect(resp.status_code()).to_equal(StatusCode::SERVICE_UNAVAILABLE)
}

async fn test_200_handler(Extension(token_info): Extension<GetTokenInfoData>) -> impl IntoResponse {
    if token_info.token.ne("token") {
        return StatusCode::BAD_REQUEST;
    } else if token_info.user_id.ne("user") {
        return StatusCode::BAD_REQUEST;
    } else if token_info.account.ne("user") {
        return StatusCode::BAD_REQUEST;
    } else if token_info.name.ne("user") {
        return StatusCode::BAD_REQUEST;
    } else if token_info.client_id.ne("client") {
        return StatusCode::BAD_REQUEST;
    } else if token_info.scopes.len() > 0 {
        return StatusCode::BAD_REQUEST;
    } else if token_info.roles.keys().len() != 1 {
        return StatusCode::BAD_REQUEST;
    }
    match token_info.roles.get("user") {
        None => return StatusCode::BAD_REQUEST,
        Some(enabled) => match *enabled {
            false => return StatusCode::BAD_REQUEST,
            true => (),
        },
    }
    StatusCode::NO_CONTENT
}

async fn dummy_handler(Extension(token_info): Extension<GetTokenInfoData>) -> impl IntoResponse {
    if token_info.user_id.as_str() != "user"
        && token_info.user_id.as_str() != "user1"
        && token_info.user_id.as_str() != "user2"
    {
        return Err(ErrResp::Custom(451, "", Some(token_info.user_id.clone())));
    } else if token_info.client_id.as_str() != "client"
        && token_info.client_id.as_str() != "client1"
        && token_info.client_id.as_str() != "client2"
    {
        return Err(ErrResp::Custom(452, "", Some(token_info.client_id.clone())));
    }
    Ok(StatusCode::NO_CONTENT)
}

fn before_all_fn(state: &mut HashMap<&'static str, TestState>) -> () {
    let runtime = match Runtime::new() {
        Err(e) => panic!("create runtime error: {}", e),
        Ok(runtime) => runtime,
    };

    let auth_state = match runtime.block_on(async {
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
        sylvia_iot_auth_routes::new_state("/auth", &conf).await
    }) {
        Err(e) => panic!("create auth state error: {}", e),
        Ok(state) => state,
    };

    let core_svc = runtime.spawn(async move {
        let app = Router::new().merge(sylvia_iot_auth_routes::new_service(&auth_state));
        let listener = match TcpListener::bind("0.0.0.0:1080").await {
            Err(e) => panic!("bind auth/broker server error: {}", e),
            Ok(listener) => listener,
        };
        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .await
        .unwrap()
    });

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
            core_svc: Some(core_svc),
            auth_uri,
            ..Default::default()
        },
    );
}

fn after_all_fn(state: &mut HashMap<&'static str, TestState>) -> () {
    let state = state.get_mut(STATE).unwrap();

    stop_core_svc(state);
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

fn stop_core_svc(state: &TestState) {
    if let Some(svc) = state.core_svc.as_ref() {
        svc.abort();
    }
    let mut path = std::env::temp_dir();
    path.push(sylvia_iot_auth_config::DEF_SQLITE_PATH);
    remove_sqlite(path.to_str().unwrap());
}

fn create_user(name: &str, time: DateTime<Utc>, roles: HashMap<String, bool>) -> User {
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

fn create_client(name: &str, user_id: &str, secret: Option<String>) -> Client {
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

fn create_token(token: &str, user_id: &str, client_id: &str) -> AccessToken {
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
