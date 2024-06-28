use std::{cmp::Ordering, collections::HashMap, error::Error as StdError};

use axum::{
    http::{header, HeaderValue, Method, StatusCode},
    response::IntoResponse,
    routing, Extension, Router,
};
use axum_test::TestServer;
use chrono::Utc;
use laboratory::{describe, expect, SpecContext, Suite};

use sylvia_iot_auth::models::Model;
use sylvia_iot_broker::routes::middleware::{AuthService, GetTokenInfoData, RoleScopeType};
use sylvia_iot_corelib::{err::ErrResp, role::Role};

use super::{
    clear_state,
    libs::{create_client, create_token, create_user, new_state},
    remove_sqlite, stop_auth_svc,
};
use crate::TestState;

pub const STATE: &'static str = "routes/middleware";

pub fn suite(db_engine: &'static str) -> Suite<TestState> {
    let suite_name = format!("routes.middleware - {}", db_engine);
    describe(suite_name, move |context| {
        context.it("200", test_200);
        context.it("400", test_400);
        context.it("401", test_401);
        context.it("403", test_403);
        context.it("503", test_503);

        context
            .before_all(move |state| {
                state.insert(STATE, new_state(Some(db_engine), None, None));
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

    stop_auth_svc(state);

    if let Some(model) = state.mongodb.as_ref() {
        runtime.block_on(async {
            if let Err(e) = model.get_connection().drop().await {
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

    let role_scopes_root: HashMap<Method, RoleScopeType> = HashMap::new();
    let app = Router::new()
        .route("/", routing::get(test_200_handler))
        .layer(AuthService::new(auth_uri.clone(), role_scopes_root));
    let server = match TestServer::new(app) {
        Err(e) => return Err(format!("new server error: {}", e)),
        Ok(server) => server,
    };

    let req = server.get("/").add_header(
        header::AUTHORIZATION,
        HeaderValue::from_str("Bearer token").unwrap(),
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

    let role_scopes_root: HashMap<Method, RoleScopeType> = HashMap::new();
    let app = Router::new()
        .route("/", routing::get(dummy_handler))
        .layer(AuthService::new(auth_uri.clone(), role_scopes_root));
    let server = match TestServer::new(app) {
        Err(e) => return Err(format!("new server error: {}", e)),
        Ok(server) => server,
    };

    let req = server.get("/");
    let resp = runtime.block_on(async { req.await });
    expect(resp.status_code()).to_equal(StatusCode::BAD_REQUEST)?;

    let req = server
        .get("/")
        .add_header(
            header::AUTHORIZATION,
            HeaderValue::from_str("Bearer test").unwrap(),
        )
        .add_header(
            header::AUTHORIZATION,
            HeaderValue::from_str("Bearer test").unwrap(),
        );
    let resp = runtime.block_on(async { req.await });
    expect(resp.status_code()).to_equal(StatusCode::BAD_REQUEST)
}

fn test_401(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let auth_uri = state.auth_uri.as_ref().unwrap();

    let role_scopes_root: HashMap<Method, RoleScopeType> = HashMap::new();
    let app = Router::new()
        .route("/", routing::get(dummy_handler))
        .layer(AuthService::new(auth_uri.clone(), role_scopes_root));
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

fn test_403(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let auth_db = state.auth_db.as_ref().unwrap();
    let auth_uri = state.auth_uri.as_ref().unwrap();

    let result: Result<(), Box<dyn StdError>> = runtime.block_on(async move {
        let now = Utc::now();
        let mut user = create_user("user1", now, HashMap::<String, bool>::new());
        user.roles.insert(Role::MANAGER.to_string(), true);
        auth_db.user().add(&user).await?;
        let user = create_user("user2", now, HashMap::<String, bool>::new());
        auth_db.user().add(&user).await?;
        let mut client = create_client("client1", "user1", None);
        client.scopes = vec!["scope1".to_string()];
        auth_db.client().add(&client).await?;
        let client = create_client("client2", "user2", None);
        auth_db.client().add(&client).await?;
        let token = create_token("token1", "user1", "client1");
        auth_db.access_token().add(&token).await?;
        let token = create_token("token2", "user2", "client2");
        auth_db.access_token().add(&token).await?;
        Ok(())
    });
    expect(result.is_ok()).to_equal(true)?;

    let mut role_scopes_root: HashMap<Method, RoleScopeType> = HashMap::new();
    role_scopes_root.insert(Method::GET, (vec![], vec![]));
    role_scopes_root.insert(Method::POST, (vec![Role::MANAGER], vec![]));
    role_scopes_root.insert(Method::PATCH, (vec![], vec!["scope1".to_string()]));
    let app = Router::new()
        .route(
            "/",
            routing::get(dummy_handler)
                .post(dummy_handler)
                .patch(dummy_handler),
        )
        .layer(AuthService::new(auth_uri.clone(), role_scopes_root));
    let server = match TestServer::new(app) {
        Err(e) => return Err(format!("new server error: {}", e)),
        Ok(server) => server,
    };

    let req = server.get("/").add_header(
        header::AUTHORIZATION,
        HeaderValue::from_str("Bearer token1").unwrap(),
    );
    let resp = runtime.block_on(async { req.await });
    let status = resp.status_code();
    if status != StatusCode::NO_CONTENT {
        let body = resp.text();
        return Err(format!("status1-1 {}, body: {:?}", status, body));
    }
    let req = server.get("/").add_header(
        header::AUTHORIZATION,
        HeaderValue::from_str("Bearer token2").unwrap(),
    );
    let resp = runtime.block_on(async { req.await });
    let status = resp.status_code();
    if status != StatusCode::NO_CONTENT {
        let body = resp.text();
        return Err(format!("status1-2 {}, body: {:?}", status, body));
    }

    let req = server.post("/").add_header(
        header::AUTHORIZATION,
        HeaderValue::from_str("Bearer token1").unwrap(),
    );
    let resp = runtime.block_on(async { req.await });
    let status = resp.status_code();
    if status != StatusCode::NO_CONTENT {
        let body = resp.text();
        return Err(format!("status2-1 {}, body: {:?}", status, body));
    }
    let req = server.post("/").add_header(
        header::AUTHORIZATION,
        HeaderValue::from_str("Bearer token2").unwrap(),
    );
    let resp = runtime.block_on(async { req.await });
    let status = resp.status_code();
    if status != StatusCode::FORBIDDEN {
        let body = resp.text();
        return Err(format!("status2-2 {}, body: {:?}", status, body));
    }

    let req = server.patch("/").add_header(
        header::AUTHORIZATION,
        HeaderValue::from_str("Bearer token1").unwrap(),
    );
    let resp = runtime.block_on(async { req.await });
    let status = resp.status_code();
    if status != StatusCode::NO_CONTENT {
        let body = resp.text();
        return Err(format!("status3-1 {}, body: {:?}", status, body));
    }
    let req = server.patch("/").add_header(
        header::AUTHORIZATION,
        HeaderValue::from_str("Bearer token2").unwrap(),
    );
    let resp = runtime.block_on(async { req.await });
    let status = resp.status_code();
    if status != StatusCode::FORBIDDEN {
        let body = resp.text();
        return Err(format!("status3-2 {}, body: {:?}", status, body));
    }
    Ok(())
}

fn test_503(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let auth_uri = "http://localhost:65535";

    let role_scopes_root: HashMap<Method, RoleScopeType> = HashMap::new();
    let app = Router::new()
        .route("/", routing::get(dummy_handler))
        .layer(AuthService::new(auth_uri.to_string(), role_scopes_root));
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
    let mut found = false;
    for (k, v) in token_info.roles.iter() {
        if k.as_str().cmp(Role::MANAGER) == Ordering::Equal && *v {
            found = true;
            break;
        }
    }
    if token_info.user_id.as_str() != "user2" && !found {
        return Err(ErrResp::Custom(453, "", None));
    }
    Ok(StatusCode::NO_CONTENT)
}

async fn test_200_handler(Extension(token_info): Extension<GetTokenInfoData>) -> impl IntoResponse {
    if token_info.user_id.as_str() != "user" {
        return Err(ErrResp::ErrUnknown(Some(format!(
            "wrong user: {}",
            token_info.user_id
        ))));
    } else if token_info.client_id.as_str() != "client" {
        return Err(ErrResp::ErrUnknown(Some(format!(
            "wrong client: {}",
            token_info.client_id
        ))));
    } else if token_info.token.as_str() != "token" {
        return Err(ErrResp::ErrUnknown(Some(format!(
            "wrong token: {}",
            token_info.token
        ))));
    }
    Ok(StatusCode::NO_CONTENT)
}
