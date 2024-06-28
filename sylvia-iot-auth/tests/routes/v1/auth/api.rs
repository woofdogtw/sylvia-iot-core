use std::collections::HashMap;

use axum::{
    http::{header, HeaderValue, StatusCode},
    Router,
};
use axum_test::TestServer;
use chrono::{TimeDelta, TimeZone, Utc};
use laboratory::{expect, SpecContext};
use mongodb::bson::Document;
use sql_builder::SqlBuilder;
use sqlx;
use tokio::runtime::Runtime;

use sylvia_iot_auth::{
    models::{
        access_token::AccessToken,
        authorization_code::AuthorizationCode,
        client::{Client, QueryCond as ClientQueryCond},
        refresh_token::RefreshToken,
        user::{QueryCond as UserQueryCond, User},
    },
    routes,
};

use super::{
    super::{
        super::libs::{create_client, create_user},
        libs::get_token,
        TestState, STATE,
    },
    response,
};

pub fn before_all_fn(state: &mut HashMap<&'static str, TestState>) {
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.routes_state.as_ref().unwrap().model.as_ref();

    runtime.block_on(async {
        let now = Utc::now();
        let mut roles = HashMap::<String, bool>::new();
        roles.insert("admin".to_string(), true);
        if let Err(e) = model.user().add(&create_user("admin", now, roles)).await {
            println!("add user admin error: {}", e);
        }

        let now = now + TimeDelta::try_seconds(1).unwrap();
        let mut roles = HashMap::<String, bool>::new();
        roles.insert("dev".to_string(), true);
        if let Err(e) = model.user().add(&create_user("dev", now, roles)).await {
            println!("add user dev error: {}", e);
        }

        let now = now + TimeDelta::try_seconds(1).unwrap();
        let mut roles = HashMap::<String, bool>::new();
        roles.insert("manager".to_string(), true);
        if let Err(e) = model.user().add(&create_user("manager", now, roles)).await {
            println!("add user manager error: {}", e);
        }

        let now = now + TimeDelta::try_seconds(1).unwrap();
        let roles = HashMap::<String, bool>::new();
        if let Err(e) = model.user().add(&create_user("user", now, roles)).await {
            println!("add user user error: {}", e);
        }

        let client = create_client("public", "dev", None);
        if let Err(e) = model.client().add(&client).await {
            println!("add client public error: {}", e);
        }

        let mut client = create_client("private", "dev", Some("private".to_string()));
        client.scopes = vec!["scope1".to_string()];
        if let Err(e) = model.client().add(&client).await {
            println!("add client private error: {}", e);
        }
    });
}

pub fn after_all_fn(state: &mut HashMap<&'static str, TestState>) -> () {
    const USER_NAME: &'static str = "user";
    const CLIENT_NAME: &'static str = "client";

    let state = state.get_mut(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    if let Some(model) = state.mongodb.as_ref() {
        runtime.block_on(async {
            struct Doc;

            let conn = model.get_connection();
            let _ = conn
                .collection::<Doc>(USER_NAME)
                .delete_many(Document::new())
                .await;
            let _ = conn
                .collection::<Doc>(CLIENT_NAME)
                .delete_many(Document::new())
                .await;
        });
    }
    if let Some(model) = state.sqlite.as_ref() {
        runtime.block_on(async {
            let conn = model.get_connection();
            let sql = SqlBuilder::delete_from(USER_NAME).sql().unwrap();
            let _ = sqlx::query(sql.as_str()).execute(conn).await;
            let sql = SqlBuilder::delete_from(CLIENT_NAME).sql().unwrap();
            let _ = sqlx::query(sql.as_str()).execute(conn).await;
        });
    }
}

pub fn after_each_fn(state: &mut HashMap<&'static str, TestState>) -> () {
    let state = state.get_mut(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    if let Some(model) = state.mongodb.as_ref() {
        runtime.block_on(async {
            struct Doc;

            let conn = model.get_connection();
            let _ = conn
                .collection::<Doc>("accessToken")
                .delete_many(Document::new())
                .await;
            let _ = conn
                .collection::<Doc>("authorizationCode")
                .delete_many(Document::new())
                .await;
            let _ = conn
                .collection::<Doc>("refreshToken")
                .delete_many(Document::new())
                .await;
        });
    }
    if let Some(model) = state.sqlite.as_ref() {
        runtime.block_on(async {
            let conn = model.get_connection();
            let sql = SqlBuilder::delete_from("access_token").sql().unwrap();
            let _ = sqlx::query(sql.as_str()).execute(conn).await;
            let sql = SqlBuilder::delete_from("authorization_code").sql().unwrap();
            let _ = sqlx::query(sql.as_str()).execute(conn).await;
            let sql = SqlBuilder::delete_from("refresh_token").sql().unwrap();
            let _ = sqlx::query(sql.as_str()).execute(conn).await;
        });
    }
}

pub fn get_tokeninfo(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    test_get_tokeninfo(runtime, routes_state, "admin")?;
    test_get_tokeninfo(runtime, routes_state, "manager")?;
    test_get_tokeninfo(runtime, routes_state, "dev")?;
    test_get_tokeninfo(runtime, routes_state, "user")
}

pub fn post_logout(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    test_post_logout(runtime, routes_state)
}

fn test_get_tokeninfo(
    runtime: &Runtime,
    state: &routes::State,
    user_id: &str,
) -> Result<(), String> {
    let user_info = get_user_model(runtime, state, user_id)?;
    let client_info = get_client_model(runtime, state, "public")?;

    let app = Router::new().merge(routes::new_service(&state));
    let server = match TestServer::new(app) {
        Err(e) => return Err(format!("new server error: {}", e)),
        Ok(server) => server,
    };

    let token = get_token(runtime, state, user_id)?;
    let req = server.get("/auth/api/v1/auth/tokeninfo").add_header(
        header::AUTHORIZATION,
        HeaderValue::from_str(format!("Bearer {}", token).as_str()).unwrap(),
    );
    let resp = runtime.block_on(async { req.await });
    expect(resp.status_code()).to_equal(StatusCode::OK)?;
    let body: response::GetTokenInfo = resp.json();
    expect(body.data.user_id.as_str()).to_equal(user_info.user_id.as_str())?;
    expect(body.data.account.as_str()).to_equal(user_info.account.as_str())?;
    expect(body.data.name.as_str()).to_equal(user_info.name.as_str())?;
    expect(body.data.roles).to_equal(user_info.roles)?;
    expect(body.data.client_id.as_str()).to_equal(client_info.client_id.as_str())?;
    expect(body.data.scopes).to_equal(client_info.scopes)
}

fn test_post_logout(runtime: &Runtime, state: &routes::State) -> Result<(), String> {
    runtime.block_on(async {
        let mut token = AccessToken {
            access_token: "access-user1".to_string(),
            refresh_token: Some("access-user1".to_string()),
            expires_at: Utc.timestamp_nanos((Utc::now().timestamp() + 3600) * 1000000000),
            scope: None,
            client_id: "public".to_string(),
            redirect_uri: crate::TEST_REDIRECT_URI.to_string().to_string(),
            user_id: "user".to_string(),
        };
        if let Err(e) = state.model.access_token().add(&token).await {
            return Err(format!("add access token user 1 error: {}", e));
        }
        token.access_token = "access-user2".to_string();
        token.refresh_token = Some("access-user2".to_string());
        if let Err(e) = state.model.access_token().add(&token).await {
            return Err(format!("add access token user 2 error: {}", e));
        }
        token.access_token = "access-admin".to_string();
        token.refresh_token = Some("access-admin".to_string());
        token.user_id = "admin".to_string();
        if let Err(e) = state.model.access_token().add(&token).await {
            return Err(format!("add access token admin error: {}", e));
        }

        let mut token = RefreshToken {
            refresh_token: "refresh-user1".to_string(),
            expires_at: Utc.timestamp_nanos((Utc::now().timestamp() + 3600) * 1000000000),
            scope: None,
            client_id: "public".to_string(),
            redirect_uri: crate::TEST_REDIRECT_URI.to_string().to_string(),
            user_id: "user".to_string(),
        };
        if let Err(e) = state.model.refresh_token().add(&token).await {
            return Err(format!("add refresh token user 1 error: {}", e));
        }
        token.refresh_token = "refresh-user2".to_string();
        if let Err(e) = state.model.refresh_token().add(&token).await {
            return Err(format!("add refresh token user 2 error: {}", e));
        }
        token.refresh_token = "refresh-admin".to_string();
        token.user_id = "admin".to_string();
        if let Err(e) = state.model.refresh_token().add(&token).await {
            return Err(format!("add refresh token admin error: {}", e));
        }

        let mut code = AuthorizationCode {
            code: "code-user1".to_string(),
            expires_at: Utc.timestamp_nanos((Utc::now().timestamp() + 3600) * 1000000000),
            scope: None,
            client_id: "public".to_string(),
            redirect_uri: crate::TEST_REDIRECT_URI.to_string().to_string(),
            user_id: "user".to_string(),
        };
        if let Err(e) = state.model.authorization_code().add(&code).await {
            return Err(format!("add authorization code user 1 error: {}", e));
        }
        code.code = "code-user2".to_string();
        if let Err(e) = state.model.authorization_code().add(&code).await {
            return Err(format!("add authorization code user 2 error: {}", e));
        }
        code.code = "code-admin".to_string();
        code.user_id = "admin".to_string();
        if let Err(e) = state.model.authorization_code().add(&code).await {
            return Err(format!("add authorization code admin error: {}", e));
        }
        Ok(())
    })?;

    let app = Router::new().merge(routes::new_service(&state));
    let server = match TestServer::new(app) {
        Err(e) => return Err(format!("new server error: {}", e)),
        Ok(server) => server,
    };

    let req = server.post("/auth/api/v1/auth/logout").add_header(
        header::AUTHORIZATION,
        HeaderValue::from_str(format!("Bearer access-user2").as_str()).unwrap(),
    );
    let resp = runtime.block_on(async { req.await });
    expect(resp.status_code()).to_equal(StatusCode::NO_CONTENT)?;
    runtime.block_on(async {
        match state.model.authorization_code().get("code-user1").await {
            Err(e) => return Err(format!("get code-user1 error: {}", e)),
            Ok(token) => match token {
                None => return Err("should get code-user1".to_string()),
                Some(_) => (),
            },
        }
        match state.model.authorization_code().get("code-user2").await {
            Err(e) => return Err(format!("get code-user2 error: {}", e)),
            Ok(token) => match token {
                None => return Err("should get code-user2".to_string()),
                Some(_) => (),
            },
        }
        match state.model.authorization_code().get("code-admin").await {
            Err(e) => return Err(format!("get code-admin error: {}", e)),
            Ok(token) => match token {
                None => return Err("should get code-admin".to_string()),
                Some(_) => (),
            },
        }

        match state.model.access_token().get("access-user1").await {
            Err(e) => return Err(format!("get access-user1 error: {}", e)),
            Ok(token) => match token {
                None => return Err("should get access-user1".to_string()),
                Some(_) => (),
            },
        }
        match state.model.access_token().get("access-user2").await {
            Err(e) => return Err(format!("get access-user2 error: {}", e)),
            Ok(token) => match token {
                None => (),
                Some(_) => return Err("should not get access-user2".to_string()),
            },
        }
        match state.model.access_token().get("access-admin").await {
            Err(e) => return Err(format!("get access-admin error: {}", e)),
            Ok(token) => match token {
                None => return Err("should get access-admin".to_string()),
                Some(_) => (),
            },
        }

        match state.model.refresh_token().get("refresh-user1").await {
            Err(e) => return Err(format!("get refresh-user1 error: {}", e)),
            Ok(token) => match token {
                None => return Err("should get refresh-user1".to_string()),
                Some(_) => (),
            },
        }
        match state.model.refresh_token().get("refresh-user2").await {
            Err(e) => return Err(format!("get refresh-user2 error: {}", e)),
            Ok(token) => match token {
                None => return Err("should get refresh-user2".to_string()),
                Some(_) => (),
            },
        }
        match state.model.refresh_token().get("refresh-admin").await {
            Err(e) => return Err(format!("get refresh-admin error: {}", e)),
            Ok(token) => match token {
                None => return Err("should get refresh-admin".to_string()),
                Some(_) => (),
            },
        }

        Ok(())
    })
}

fn get_user_model(runtime: &Runtime, state: &routes::State, user_id: &str) -> Result<User, String> {
    match runtime.block_on(async {
        let cond = UserQueryCond {
            user_id: Some(user_id),
            ..Default::default()
        };
        state.model.user().get(&cond).await
    }) {
        Err(e) => return Err(format!("get user model info error: {}", e)),
        Ok(user) => match user {
            None => return Err(format!("get no user with ID {}", user_id)),
            Some(user) => return Ok(user),
        },
    }
}

fn get_client_model(
    runtime: &Runtime,
    state: &routes::State,
    client_id: &str,
) -> Result<Client, String> {
    match runtime.block_on(async {
        let cond = ClientQueryCond {
            client_id: Some(client_id),
            ..Default::default()
        };
        state.model.client().get(&cond).await
    }) {
        Err(e) => return Err(format!("get client model info error: {}", e)),
        Ok(client) => match client {
            None => return Err(format!("get no client with ID {}", client_id)),
            Some(client) => return Ok(client),
        },
    }
}
