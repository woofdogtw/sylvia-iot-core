use std::collections::HashMap;

use axum::{
    Router,
    http::{HeaderValue, Method, StatusCode, header},
};
use axum_test::TestServer;
use chrono::{DateTime, SecondsFormat, SubsecRound, TimeDelta, Utc};
use laboratory::{SpecContext, expect};
use mongodb::bson::Document;
use serde_json::{Map, Value};
use sql_builder::SqlBuilder;
use sqlx;
use tokio::runtime::Runtime;

use sylvia_iot_auth::{
    models::user::{QueryCond, User},
    routes,
};
use sylvia_iot_corelib::{
    err,
    role::Role,
    strings::{password_hash, time_str},
};

use super::{
    super::{
        super::libs::{create_client, create_user},
        STATE, TestState,
        libs::{
            ApiError, get_token, test_get_list_invalid_param, test_invalid_perm, test_invalid_token,
        },
    },
    request, response,
};

pub fn before_each_fn(state: &mut HashMap<&'static str, TestState>) {
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.routes_state.as_ref().unwrap().model.as_ref();

    runtime.block_on(async {
        let now = Utc::now();

        let mut roles = HashMap::<String, bool>::new();
        roles.insert(Role::DEV.to_string(), true);
        if let Err(e) = model.user().add(&create_user("public", now, roles)).await {
            println!("add user public error: {}", e);
        }

        let client = create_client("public", "public", None);
        if let Err(e) = model.client().add(&client).await {
            println!("add client public error: {}", e);
        }
    })
}

pub fn after_each_fn(state: &mut HashMap<&'static str, TestState>) -> () {
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

pub fn get(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    test_get(runtime, routes_state, "admin")?;
    test_get(runtime, routes_state, "manager")?;
    test_get(runtime, routes_state, "dev")?;
    test_get(runtime, routes_state, "user")
}

pub fn get_invalid_token(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    let app = Router::new().merge(routes::new_service(routes_state));
    let server = TestServer::new(app);

    let req = server.get("/auth/api/v1/user").add_header(
        header::AUTHORIZATION,
        HeaderValue::from_str("Bearer token").unwrap(),
    );
    let resp = runtime.block_on(async { req.await });
    expect(resp.status_code()).to_equal(StatusCode::UNAUTHORIZED)
}

pub fn patch(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    test_patch(runtime, routes_state, "admin", false)?;
    test_patch(runtime, routes_state, "manager", false)?;
    test_patch(runtime, routes_state, "dev", false)?;
    test_patch(runtime, routes_state, "user", false)
}

pub fn patch_password(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    test_patch(runtime, routes_state, "admin", true)?;
    test_patch(runtime, routes_state, "manager", true)?;
    test_patch(runtime, routes_state, "dev", true)?;
    test_patch(runtime, routes_state, "user", true)
}

pub fn patch_invalid_param(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    let user_id = "user";
    add_user_model(runtime, &routes_state, user_id)?;
    let token = get_token(runtime, routes_state, user_id)?;

    test_patch_invalid_param(runtime, &routes_state, token.as_str(), None)?;

    let body = Map::<String, Value>::new();
    test_patch_invalid_param(runtime, &routes_state, token.as_str(), Some(&body))?;

    let mut body = Map::<String, Value>::new();
    body.insert("password".to_string(), Value::String("".to_string()));
    test_patch_invalid_param(runtime, &routes_state, token.as_str(), Some(&body))?;

    let mut body = Map::<String, Value>::new();
    body.insert("password".to_string(), Value::Number(0.into()));
    test_patch_invalid_param(runtime, &routes_state, token.as_str(), Some(&body))?;

    let mut info = Map::<String, Value>::new();
    info.insert("".to_string(), Value::Null);
    let mut body = Map::<String, Value>::new();
    body.insert("info".to_string(), Value::Object(info));
    test_patch_invalid_param(runtime, &routes_state, token.as_str(), Some(&body))
}

pub fn patch_invalid_token(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    test_invalid_token(runtime, &routes_state, Method::PATCH, "/auth/api/v1/user")
}

pub fn post_admin(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    let user_id = "admin";
    add_user_model(runtime, &routes_state, user_id)?;
    let token = get_token(runtime, routes_state, user_id)?;

    let mut info = Map::<String, Value>::new();
    info.insert("info".to_string(), Value::String("value".to_string()));
    let param = request::PostAdminUser {
        data: request::PostAdminUserData {
            account: "account".to_string(),
            password: "password".to_string(),
            name: Some("name".to_string()),
            info: Some(info),
        },
        expired_at: None,
    };
    test_post_admin(runtime, &routes_state, token.as_str(), &param, "")?;

    let param = request::PostAdminUser {
        data: request::PostAdminUserData {
            account: "account2".to_string(),
            password: "password".to_string(),
            name: None,
            info: None,
        },
        expired_at: Some(Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)),
    };
    test_post_admin(runtime, &routes_state, token.as_str(), &param, "")
}

pub fn post_admin_dup(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    let user_id = "admin";
    add_user_model(runtime, &routes_state, user_id)?;
    let token = get_token(runtime, routes_state, user_id)?;

    let param = request::PostAdminUser {
        data: request::PostAdminUserData {
            account: "account".to_string(),
            password: "password".to_string(),
            name: Some("name".to_string()),
            info: None,
        },
        expired_at: None,
    };
    test_post_admin(runtime, &routes_state, token.as_str(), &param, "")?;
    test_post_admin(
        runtime,
        &routes_state,
        token.as_str(),
        &param,
        "err_auth_user_exist",
    )
}

pub fn post_admin_invalid_param(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    let user_id = "admin";
    add_user_model(runtime, &routes_state, user_id)?;
    let token = get_token(runtime, routes_state, user_id)?;

    test_post_admin_invalid_param(runtime, &routes_state, token.as_str(), None)?;

    let body = Map::<String, Value>::new();
    test_post_admin_invalid_param(runtime, &routes_state, token.as_str(), Some(&body))?;

    let mut body = Map::<String, Value>::new();
    body.insert("account".to_string(), Value::String("".to_string()));
    body.insert("password".to_string(), Value::String("pass".to_string()));
    test_post_admin_invalid_param(runtime, &routes_state, token.as_str(), Some(&body))?;

    let mut body = Map::<String, Value>::new();
    body.insert("account".to_string(), Value::String("account".to_string()));
    body.insert("password".to_string(), Value::String("".to_string()));
    test_post_admin_invalid_param(runtime, &routes_state, token.as_str(), Some(&body))?;

    let mut body = Map::<String, Value>::new();
    body.insert("account".to_string(), Value::String("account".to_string()));
    body.insert("password".to_string(), Value::Number(0.into()));
    test_post_admin_invalid_param(runtime, &routes_state, token.as_str(), Some(&body))?;

    let mut info = Map::<String, Value>::new();
    info.insert("".to_string(), Value::Null);
    let mut body = Map::<String, Value>::new();
    body.insert("account".to_string(), Value::String("account".to_string()));
    body.insert("password".to_string(), Value::String("pass".to_string()));
    body.insert("info".to_string(), Value::Object(info));
    test_post_admin_invalid_param(runtime, &routes_state, token.as_str(), Some(&body))
}

pub fn post_admin_invalid_token(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    test_invalid_token(runtime, &routes_state, Method::POST, "/auth/api/v1/user")
}

pub fn post_admin_invalid_perm(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    let method = Method::POST;
    let uri = "/auth/api/v1/user";

    let user_id = "manager";
    add_user_model(runtime, &routes_state, user_id)?;
    let token = get_token(runtime, routes_state, user_id)?;
    test_invalid_perm(runtime, &routes_state, token.as_str(), method.clone(), uri)?;

    let user_id = "service";
    add_user_model(runtime, &routes_state, user_id)?;
    let token = get_token(runtime, routes_state, user_id)?;
    test_invalid_perm(runtime, &routes_state, token.as_str(), method.clone(), uri)?;

    let user_id = "user";
    add_user_model(runtime, &routes_state, user_id)?;
    let token = get_token(runtime, routes_state, user_id)?;
    test_invalid_perm(runtime, &routes_state, token.as_str(), method, uri)
}

pub fn get_admin_count(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    let data_size = count_list_dataset(runtime, &routes_state)?;

    let user_id = "admin";
    add_user_model(runtime, &routes_state, user_id)?;
    let admin_token = get_token(runtime, routes_state, user_id)?;
    let user_id = "manager";
    add_user_model(runtime, &routes_state, user_id)?;
    let manager_token = get_token(runtime, routes_state, user_id)?;

    test_get_admin_count(
        runtime,
        &routes_state,
        admin_token.as_str(),
        None,
        data_size.0 + 3,
    )?;
    test_get_admin_count(
        runtime,
        &routes_state,
        manager_token.as_str(),
        None,
        data_size.0 + 3,
    )?;

    let param = request::GetAdminUserCount {
        account: Some("".to_string()),
        ..Default::default()
    };
    test_get_admin_count(
        runtime,
        &routes_state,
        admin_token.as_str(),
        Some(&param),
        data_size.0 + 3,
    )?;
    test_get_admin_count(
        runtime,
        &routes_state,
        manager_token.as_str(),
        Some(&param),
        data_size.0 + 3,
    )?;

    let param = request::GetAdminUserCount {
        account: Some("account".to_string()),
        ..Default::default()
    };
    test_get_admin_count(
        runtime,
        &routes_state,
        admin_token.as_str(),
        Some(&param),
        0,
    )?;
    test_get_admin_count(
        runtime,
        &routes_state,
        manager_token.as_str(),
        Some(&param),
        0,
    )?;

    let param = request::GetAdminUserCount {
        account: Some("account1@EXAMPLE.com".to_string()),
        contains: Some("example".to_string()),
    };
    test_get_admin_count(
        runtime,
        &routes_state,
        admin_token.as_str(),
        Some(&param),
        1,
    )?;
    test_get_admin_count(
        runtime,
        &routes_state,
        manager_token.as_str(),
        Some(&param),
        1,
    )?;

    let param = request::GetAdminUserCount {
        account: Some("".to_string()),
        contains: Some("@EXAMPLE.com".to_string()),
    };
    test_get_admin_count(
        runtime,
        &routes_state,
        admin_token.as_str(),
        Some(&param),
        data_size.0,
    )?;
    test_get_admin_count(
        runtime,
        &routes_state,
        manager_token.as_str(),
        Some(&param),
        data_size.0,
    )?;

    let param = request::GetAdminUserCount {
        contains: Some("A".to_string()),
        ..Default::default()
    };
    test_get_admin_count(
        runtime,
        &routes_state,
        admin_token.as_str(),
        Some(&param),
        data_size.0 + 2,
    )?;
    test_get_admin_count(
        runtime,
        &routes_state,
        manager_token.as_str(),
        Some(&param),
        data_size.0 + 2,
    )
}

pub fn get_admin_count_invalid_token(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    test_invalid_token(
        runtime,
        &routes_state,
        Method::GET,
        "/auth/api/v1/user/count",
    )
}

pub fn get_admin_count_invalid_perm(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    let method = Method::GET;
    let uri = "/auth/api/v1/user/count";

    let user_id = "service";
    add_user_model(runtime, &routes_state, user_id)?;
    let token = get_token(runtime, routes_state, user_id)?;
    test_invalid_perm(runtime, &routes_state, token.as_str(), method.clone(), uri)?;

    let user_id = "user";
    add_user_model(runtime, &routes_state, user_id)?;
    let token = get_token(runtime, routes_state, user_id)?;
    test_invalid_perm(runtime, &routes_state, token.as_str(), method, uri)
}

pub fn get_admin_list(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    let data_size = count_list_dataset(runtime, &routes_state)?;

    let user_id = "admin";
    add_user_model(runtime, &routes_state, user_id)?;
    let admin_token = get_token(runtime, routes_state, user_id)?;
    let user_id = "manager";
    add_user_model(runtime, &routes_state, user_id)?;
    let manager_token = get_token(runtime, routes_state, user_id)?;

    let mut param = request::GetAdminUserList {
        ..Default::default()
    };
    test_get_admin_list(
        runtime,
        &routes_state,
        admin_token.as_str(),
        &mut param,
        data_size.0 + 3,
        0,
        0,
    )?;
    test_get_admin_list(
        runtime,
        &routes_state,
        manager_token.as_str(),
        &mut param,
        data_size.0 + 3,
        0,
        0,
    )?;

    let mut param = request::GetAdminUserList {
        fields_vec: Some(vec!["expired", "disabled"]),
        ..Default::default()
    };
    test_get_admin_list(
        runtime,
        &routes_state,
        admin_token.as_str(),
        &mut param,
        data_size.0 + 3,
        data_size.1,
        data_size.2,
    )?;
    test_get_admin_list(
        runtime,
        &routes_state,
        manager_token.as_str(),
        &mut param,
        data_size.0 + 3,
        data_size.1,
        data_size.2,
    )?;

    let mut param = request::GetAdminUserList {
        account: Some("".to_string()),
        ..Default::default()
    };
    test_get_admin_list(
        runtime,
        &routes_state,
        admin_token.as_str(),
        &mut param,
        data_size.0 + 3,
        0,
        0,
    )?;
    test_get_admin_list(
        runtime,
        &routes_state,
        manager_token.as_str(),
        &mut param,
        data_size.0 + 3,
        0,
        0,
    )?;

    let mut param = request::GetAdminUserList {
        account: Some("account".to_string()),
        ..Default::default()
    };
    test_get_admin_list(
        runtime,
        &routes_state,
        admin_token.as_str(),
        &mut param,
        0,
        0,
        0,
    )?;
    test_get_admin_list(
        runtime,
        &routes_state,
        manager_token.as_str(),
        &mut param,
        0,
        0,
        0,
    )?;

    let mut param = request::GetAdminUserList {
        account: Some("account1@EXAMPLE.com".to_string()),
        contains: Some("example".to_string()),
        fields_vec: Some(vec!["testfield"]),
        ..Default::default()
    };
    test_get_admin_list(
        runtime,
        &routes_state,
        admin_token.as_str(),
        &mut param,
        1,
        0,
        0,
    )?;
    test_get_admin_list(
        runtime,
        &routes_state,
        manager_token.as_str(),
        &mut param,
        1,
        0,
        0,
    )?;

    let mut param = request::GetAdminUserList {
        account: Some("".to_string()),
        contains: Some("@EXAMPLE.com".to_string()),
        fields_vec: Some(vec!["expired"]),
        ..Default::default()
    };
    test_get_admin_list(
        runtime,
        &routes_state,
        admin_token.as_str(),
        &mut param,
        data_size.0,
        data_size.1,
        0,
    )?;
    test_get_admin_list(
        runtime,
        &routes_state,
        manager_token.as_str(),
        &mut param,
        data_size.0,
        data_size.1,
        0,
    )?;

    let mut param = request::GetAdminUserList {
        contains: Some("A".to_string()),
        fields_vec: Some(vec!["disabled"]),
        ..Default::default()
    };
    test_get_admin_list(
        runtime,
        &routes_state,
        admin_token.as_str(),
        &mut param,
        data_size.0 + 2,
        0,
        data_size.2,
    )?;
    test_get_admin_list(
        runtime,
        &routes_state,
        manager_token.as_str(),
        &mut param,
        data_size.0 + 2,
        0,
        data_size.2,
    )
}

pub fn get_admin_list_sort(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    count_list_dataset(runtime, &routes_state)?;

    let user_id = "admin";
    add_user_model(runtime, &routes_state, user_id)?;
    let admin_token = get_token(runtime, routes_state, user_id)?;

    let mut param = request::GetAdminUserList {
        contains: Some("@example.com".to_string()),
        ..Default::default()
    };
    test_get_admin_list_sort(
        runtime,
        &routes_state,
        admin_token.as_str(),
        &mut param,
        &[
            "account1@example.com",
            "account2@example.com",
            "account3@example.com",
            "account4@example.com",
        ],
    )?;

    param.sort_vec = Some(vec![("account", true)]);
    test_get_admin_list_sort(
        runtime,
        &routes_state,
        admin_token.as_str(),
        &mut param,
        &[
            "account1@example.com",
            "account2@example.com",
            "account3@example.com",
            "account4@example.com",
        ],
    )?;
    param.sort_vec = Some(vec![("account", false)]);
    test_get_admin_list_sort(
        runtime,
        &routes_state,
        admin_token.as_str(),
        &mut param,
        &[
            "account4@example.com",
            "account3@example.com",
            "account2@example.com",
            "account1@example.com",
        ],
    )?;

    param.sort_vec = Some(vec![("created", true)]);
    test_get_admin_list_sort(
        runtime,
        &routes_state,
        admin_token.as_str(),
        &mut param,
        &[
            "account1@example.com",
            "account2@example.com",
            "account3@example.com",
            "account4@example.com",
        ],
    )?;
    param.sort_vec = Some(vec![("created", false)]);
    test_get_admin_list_sort(
        runtime,
        &routes_state,
        admin_token.as_str(),
        &mut param,
        &[
            "account4@example.com",
            "account3@example.com",
            "account2@example.com",
            "account1@example.com",
        ],
    )?;

    param.sort_vec = Some(vec![("modified", true)]);
    test_get_admin_list_sort(
        runtime,
        &routes_state,
        admin_token.as_str(),
        &mut param,
        &[
            "account4@example.com",
            "account3@example.com",
            "account2@example.com",
            "account1@example.com",
        ],
    )?;
    param.sort_vec = Some(vec![("modified", false)]);
    test_get_admin_list_sort(
        runtime,
        &routes_state,
        admin_token.as_str(),
        &mut param,
        &[
            "account1@example.com",
            "account2@example.com",
            "account3@example.com",
            "account4@example.com",
        ],
    )?;

    param.sort_vec = Some(vec![("verified", true), ("account", true)]);
    test_get_admin_list_sort(
        runtime,
        &routes_state,
        admin_token.as_str(),
        &mut param,
        &[
            "account2@example.com",
            "account4@example.com",
            "account1@example.com",
            "account3@example.com",
        ],
    )?;
    param.sort_vec = Some(vec![("verified", false), ("account", true)]);
    test_get_admin_list_sort(
        runtime,
        &routes_state,
        admin_token.as_str(),
        &mut param,
        &[
            "account3@example.com",
            "account1@example.com",
            "account2@example.com",
            "account4@example.com",
        ],
    )?;

    param.sort_vec = Some(vec![("name", true), ("account", true)]);
    test_get_admin_list_sort(
        runtime,
        &routes_state,
        admin_token.as_str(),
        &mut param,
        &[
            "account1@example.com",
            "account4@example.com",
            "account3@example.com",
            "account2@example.com",
        ],
    )?;
    param.sort_vec = Some(vec![("name", false), ("account", true)]);
    test_get_admin_list_sort(
        runtime,
        &routes_state,
        admin_token.as_str(),
        &mut param,
        &[
            "account2@example.com",
            "account3@example.com",
            "account4@example.com",
            "account1@example.com",
        ],
    )
}

pub fn get_admin_list_offset_limit(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    for i in 100..302 {
        add_user_model(runtime, &routes_state, format!("{}@example", i).as_str())?;
    }

    let user_id = "admin";
    add_user_model(runtime, &routes_state, user_id)?;
    let admin_token = get_token(runtime, routes_state, user_id)?;

    let mut param = request::GetAdminUserList {
        contains: Some("@example".to_string()),
        ..Default::default()
    };
    test_get_admin_list_offset_limit(
        runtime,
        &routes_state,
        admin_token.as_str(),
        &param,
        (100..200).collect(),
    )?;

    param.limit = Some(0);
    test_get_admin_list_offset_limit(
        runtime,
        &routes_state,
        admin_token.as_str(),
        &param,
        (100..302).collect(),
    )?;

    param.offset = Some(0);
    param.limit = Some(5);
    test_get_admin_list_offset_limit(
        runtime,
        &routes_state,
        admin_token.as_str(),
        &param,
        (100..105).collect(),
    )?;

    param.offset = Some(5);
    param.limit = Some(0);
    test_get_admin_list_offset_limit(
        runtime,
        &routes_state,
        admin_token.as_str(),
        &param,
        (105..302).collect(),
    )?;

    param.offset = Some(198);
    param.limit = Some(50);
    test_get_admin_list_offset_limit(
        runtime,
        &routes_state,
        admin_token.as_str(),
        &param,
        (298..302).collect(),
    )?;

    param.offset = Some(2);
    param.limit = Some(105);
    test_get_admin_list_offset_limit(
        runtime,
        &routes_state,
        admin_token.as_str(),
        &param,
        (102..207).collect(),
    )?;

    param.offset = Some(2);
    param.limit = None;
    test_get_admin_list_offset_limit(
        runtime,
        &routes_state,
        admin_token.as_str(),
        &param,
        (102..202).collect(),
    )
}

pub fn get_admin_list_format_array(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    for i in 100..302 {
        add_user_model(runtime, &routes_state, format!("{}@example", i).as_str())?;
    }

    let user_id = "admin";
    add_user_model(runtime, &routes_state, user_id)?;
    let admin_token = get_token(runtime, routes_state, user_id)?;

    let mut param = request::GetAdminUserList {
        account: Some("100@ExAmPlE".to_string()),
        format: Some("array".to_string()),
        ..Default::default()
    };
    test_get_admin_list_format_array(
        runtime,
        &routes_state,
        admin_token.as_str(),
        &param,
        (100..101).collect(),
    )?;

    param.account = None;
    param.contains = Some("@example".to_string());
    param.offset = Some(5);
    param.limit = Some(0);
    test_get_admin_list_format_array(
        runtime,
        &routes_state,
        admin_token.as_str(),
        &param,
        (105..302).collect(),
    )
}

pub fn get_admin_list_invalid_param(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    let user_id = "admin";
    add_user_model(runtime, &routes_state, user_id)?;
    let token = get_token(runtime, routes_state, user_id)?;
    let uri = "/auth/api/v1/user/list";

    let mut query = Map::<String, Value>::new();
    query.insert("offset".to_string(), Value::Number((-1).into()));
    test_get_list_invalid_param(runtime, &routes_state, token.as_str(), uri, &query)?;

    let mut query = Map::<String, Value>::new();
    query.insert("sort".to_string(), Value::String("".to_string()));
    test_get_list_invalid_param(runtime, &routes_state, token.as_str(), uri, &query)?;

    let mut query = Map::<String, Value>::new();
    query.insert("sort".to_string(), Value::String("created".to_string()));
    test_get_list_invalid_param(runtime, &routes_state, token.as_str(), uri, &query)?;

    let mut query = Map::<String, Value>::new();
    query.insert(
        "sort".to_string(),
        Value::String("created:asc:c".to_string()),
    );
    test_get_list_invalid_param(runtime, &routes_state, token.as_str(), uri, &query)?;

    let mut query = Map::<String, Value>::new();
    query.insert(
        "sort".to_string(),
        Value::String("created:asc,name:true".to_string()),
    );
    test_get_list_invalid_param(runtime, &routes_state, token.as_str(), uri, &query)?;

    let mut query = Map::<String, Value>::new();
    query.insert("sort".to_string(), Value::String("a:asc".to_string()));
    test_get_list_invalid_param(runtime, &routes_state, token.as_str(), uri, &query)
}

pub fn get_admin_list_invalid_token(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    test_invalid_token(
        runtime,
        &routes_state,
        Method::GET,
        "/auth/api/v1/user/list",
    )
}

pub fn get_admin_list_invalid_perm(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    let method = Method::GET;
    let uri = "/auth/api/v1/user/list";

    let user_id = "service";
    add_user_model(runtime, &routes_state, user_id)?;
    let token = get_token(runtime, routes_state, user_id)?;
    test_invalid_perm(runtime, &routes_state, token.as_str(), method.clone(), uri)?;

    let user_id = "user";
    add_user_model(runtime, &routes_state, user_id)?;
    let token = get_token(runtime, routes_state, user_id)?;
    test_invalid_perm(runtime, &routes_state, token.as_str(), method, uri)
}

pub fn get_admin(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    let user_id = "admin";
    add_user_model(runtime, &routes_state, user_id)?;
    let admin_token = get_token(runtime, routes_state, user_id)?;
    let user_id = "manager";
    add_user_model(runtime, &routes_state, user_id)?;
    let manager_token = get_token(runtime, routes_state, user_id)?;

    test_get_admin(runtime, &routes_state, admin_token.as_str(), "admin")?;
    test_get_admin(runtime, &routes_state, manager_token.as_str(), "admin")
}

pub fn get_admin_wrong_id(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    let user_id = "admin";
    add_user_model(runtime, &routes_state, user_id)?;
    let admin_token = get_token(runtime, routes_state, user_id)?;

    let app = Router::new().merge(routes::new_service(routes_state));
    let server = TestServer::new(app);

    let req = server.get("/auth/api/v1/user/id").add_header(
        header::AUTHORIZATION,
        HeaderValue::from_str(format!("Bearer {}", admin_token).as_str()).unwrap(),
    );
    let resp = runtime.block_on(async { req.await });
    expect(resp.status_code()).to_equal(StatusCode::NOT_FOUND)?;
    let body: ApiError = resp.json();
    if body.code.as_str() != err::E_NOT_FOUND {
        return Err(format!("unexpected 404 error: {}", body.code.as_str()));
    }
    Ok(())
}

pub fn get_admin_invalid_token(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    test_invalid_token(runtime, &routes_state, Method::GET, "/auth/api/v1/user/id")
}

pub fn get_admin_invalid_perm(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    let method = Method::GET;
    let uri = "/auth/api/v1/user/id";

    let user_id = "service";
    add_user_model(runtime, &routes_state, user_id)?;
    let token = get_token(runtime, routes_state, user_id)?;
    test_invalid_perm(runtime, &routes_state, token.as_str(), method.clone(), uri)?;

    let user_id = "user";
    add_user_model(runtime, &routes_state, user_id)?;
    let token = get_token(runtime, routes_state, user_id)?;
    test_invalid_perm(runtime, &routes_state, token.as_str(), method, uri)
}

pub fn patch_admin(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    let user_id = "admin";
    add_user_model(runtime, &routes_state, user_id)?;
    let admin_token = get_token(runtime, routes_state, user_id)?;

    let user_id = "manager";
    add_user_model(runtime, &routes_state, user_id)?;
    let manager_token = get_token(runtime, routes_state, user_id)?;

    test_patch_admin_admin(runtime, &routes_state, admin_token.as_str(), false)?;
    test_patch_admin_manager(runtime, &routes_state, manager_token.as_str())
}

pub fn patch_admin_password(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    let user_id = "admin";
    add_user_model(runtime, &routes_state, user_id)?;
    let admin_token = get_token(runtime, routes_state, user_id)?;

    test_patch_admin_admin(runtime, &routes_state, admin_token.as_str(), true)
}

pub fn patch_admin_invalid_param(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    add_user_model(runtime, &routes_state, "user")?;
    let user_id = "admin";
    add_user_model(runtime, &routes_state, user_id)?;
    let token = get_token(runtime, routes_state, user_id)?;

    test_patch_admin_invalid_param(runtime, &routes_state, token.as_str(), None)?;

    let body = Map::<String, Value>::new();
    test_patch_admin_invalid_param(runtime, &routes_state, token.as_str(), Some(&body))?;

    let mut body = Map::<String, Value>::new();
    body.insert("verifiedAt".to_string(), Value::String("time".to_string()));
    test_patch_admin_invalid_param(runtime, &routes_state, token.as_str(), Some(&body))?;

    let mut body = Map::<String, Value>::new();
    body.insert("password".to_string(), Value::String("".to_string()));
    test_patch_admin_invalid_param(runtime, &routes_state, token.as_str(), Some(&body))?;

    let mut body = Map::<String, Value>::new();
    body.insert("password".to_string(), Value::Number(0.into()));
    test_patch_admin_invalid_param(runtime, &routes_state, token.as_str(), Some(&body))?;

    let mut info = Map::<String, Value>::new();
    info.insert("".to_string(), Value::Null);
    let mut body = Map::<String, Value>::new();
    body.insert("info".to_string(), Value::Object(info));
    test_patch_admin_invalid_param(runtime, &routes_state, token.as_str(), Some(&body))
}

pub fn patch_admin_wrong_id(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    let user_id = "admin";
    add_user_model(runtime, &routes_state, user_id)?;
    let admin_token = get_token(runtime, routes_state, user_id)?;

    let app = Router::new().merge(routes::new_service(routes_state));
    let server = TestServer::new(app);

    let param = request::PatchAdminUser {
        data: Some(request::PatchAdminUserData {
            name: Some("name".to_string()),
            ..Default::default()
        }),
        ..Default::default()
    };

    let req = server
        .patch("/auth/api/v1/user/id")
        .add_header(
            header::AUTHORIZATION,
            HeaderValue::from_str(format!("Bearer {}", admin_token).as_str()).unwrap(),
        )
        .json(&param);
    let resp = runtime.block_on(async { req.await });
    expect(resp.status_code()).to_equal(StatusCode::NOT_FOUND)?;
    let body: ApiError = resp.json();
    if body.code.as_str() != err::E_NOT_FOUND {
        return Err(format!("unexpected 404 error: {}", body.code.as_str()));
    }
    Ok(())
}

pub fn patch_admin_invalid_token(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    test_invalid_token(
        runtime,
        &routes_state,
        Method::PATCH,
        "/auth/api/v1/user/id",
    )
}

pub fn patch_admin_invalid_perm(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    let method = Method::PATCH;
    let uri = "/auth/api/v1/user/id";

    add_user_model(runtime, &routes_state, "admin")?;

    let user_id = "manager";
    add_user_model(runtime, &routes_state, user_id)?;
    let manager_token = get_token(runtime, routes_state, user_id)?;

    let user_id = "service";
    add_user_model(runtime, &routes_state, user_id)?;
    let token = get_token(runtime, routes_state, user_id)?;
    test_invalid_perm(runtime, &routes_state, token.as_str(), method.clone(), uri)?;

    let user_id = "user";
    add_user_model(runtime, &routes_state, user_id)?;
    let token = get_token(runtime, routes_state, user_id)?;
    test_invalid_perm(runtime, &routes_state, token.as_str(), method, uri)?;

    test_patch_admin_manager_invalid_perm(runtime, &routes_state, manager_token.as_str())
}

pub fn delete_admin(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    let user_id = "admin";
    add_user_model(runtime, &routes_state, user_id)?;
    let token = get_token(runtime, routes_state, user_id)?;

    let user_id = "user";
    add_user_model(runtime, &routes_state, user_id)?;

    let app = Router::new().merge(routes::new_service(routes_state));
    let server = TestServer::new(app);

    let req = server.delete("/auth/api/v1/user/id").add_header(
        header::AUTHORIZATION,
        HeaderValue::from_str(format!("Bearer {}", token).as_str()).unwrap(),
    );
    let resp = runtime.block_on(async { req.await });
    expect(resp.status_code()).to_equal(StatusCode::NO_CONTENT)?;

    let cond = QueryCond {
        user_id: Some("user"),
        ..Default::default()
    };
    match runtime.block_on(async { routes_state.model.user().get(&cond).await }) {
        Err(e) => return Err(format!("get user info error: {}", e)),
        Ok(user) => match user {
            None => return Err("delete wrong user".to_string()),
            Some(_) => (),
        },
    }

    let req = server.delete("/auth/api/v1/user/user").add_header(
        header::AUTHORIZATION,
        HeaderValue::from_str(format!("Bearer {}", token).as_str()).unwrap(),
    );
    let resp = runtime.block_on(async { req.await });
    expect(resp.status_code()).to_equal(StatusCode::NO_CONTENT)?;

    let cond = QueryCond {
        user_id: Some("user"),
        ..Default::default()
    };
    match runtime.block_on(async { routes_state.model.user().get(&cond).await }) {
        Err(e) => return Err(format!("get user info error: {}", e)),
        Ok(user) => match user {
            None => (),
            Some(_) => return Err("delete user fail".to_string()),
        },
    }

    let req = server.delete("/auth/api/v1/user/admin").add_header(
        header::AUTHORIZATION,
        HeaderValue::from_str(format!("Bearer {}", token).as_str()).unwrap(),
    );
    let resp = runtime.block_on(async { req.await });
    expect(resp.status_code()).to_equal(StatusCode::FORBIDDEN)?;
    let body: ApiError = resp.json();
    if body.code.as_str() != err::E_PERM {
        return Err(format!("unexpected 403 error: {}", body.code.as_str()));
    }
    Ok(())
}

pub fn delete_admin_invalid_token(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    test_invalid_token(
        runtime,
        &routes_state,
        Method::DELETE,
        "/auth/api/v1/user/id",
    )
}

pub fn delete_admin_invalid_perm(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    let method = Method::DELETE;
    let uri = "/auth/api/v1/user/id";

    let user_id = "manager";
    add_user_model(runtime, &routes_state, user_id)?;
    let token = get_token(runtime, routes_state, user_id)?;
    test_invalid_perm(runtime, &routes_state, token.as_str(), method.clone(), uri)?;

    let user_id = "service";
    add_user_model(runtime, &routes_state, user_id)?;
    let token = get_token(runtime, routes_state, user_id)?;
    test_invalid_perm(runtime, &routes_state, token.as_str(), method.clone(), uri)?;

    let user_id = "user";
    add_user_model(runtime, &routes_state, user_id)?;
    let token = get_token(runtime, routes_state, user_id)?;
    test_invalid_perm(runtime, &routes_state, token.as_str(), method, uri)
}

fn test_get(runtime: &Runtime, state: &routes::State, user_id: &str) -> Result<(), String> {
    add_user_model(runtime, state, user_id)?;
    let user_info = get_user_model(runtime, state, user_id)?;

    let app = Router::new().merge(routes::new_service(&state));
    let server = TestServer::new(app);

    let token = get_token(runtime, state, user_id)?;
    let req = server.get("/auth/api/v1/user").add_header(
        header::AUTHORIZATION,
        HeaderValue::from_str(format!("Bearer {}", token).as_str()).unwrap(),
    );
    let resp = runtime.block_on(async { req.await });
    expect(resp.status_code()).to_equal(StatusCode::OK)?;
    let body: response::GetUser = resp.json();
    expect(body.data.account.as_str()).to_equal(user_info.account.as_str())?;
    expect(body.data.created_at.as_str()).to_equal(time_str(&user_info.created_at).as_str())?;
    expect(body.data.modified_at.as_str()).to_equal(time_str(&user_info.modified_at).as_str())?;
    match user_info.verified_at {
        None => expect(body.data.verified_at.is_none()).to_equal(true)?,
        Some(verified_at) => {
            expect(body.data.verified_at.is_some()).to_equal(true)?;
            expect(body.data.verified_at.as_ref().unwrap().as_str())
                .to_equal(time_str(&verified_at).as_str())?;
        }
    }
    expect(body.data.roles).to_equal(user_info.roles)?;
    expect(body.data.name.as_str()).to_equal(user_info.name.as_str())?;
    expect(body.data.info).to_equal(user_info.info)
}

fn test_patch(
    runtime: &Runtime,
    state: &routes::State,
    user_id: &str,
    patch_password: bool,
) -> Result<(), String> {
    add_user_model(runtime, state, user_id)?;
    let user_old = get_user_model(runtime, state, user_id)?;

    let app = Router::new().merge(routes::new_service(&state));
    let server = TestServer::new(app);

    let mut info = Map::<String, Value>::new();
    info.insert(
        "key_update".to_string(),
        Value::String("updated".to_string()),
    );
    let body = request::PatchUser {
        data: request::PatchUserData {
            password: match patch_password {
                false => None,
                true => Some("password_update".to_string()),
            },
            name: Some("name_update".to_string()),
            info: Some(info.clone()),
        },
    };
    let token = get_token(runtime, state, user_id)?;
    let req = server
        .patch("/auth/api/v1/user")
        .add_header(
            header::AUTHORIZATION,
            HeaderValue::from_str(format!("Bearer {}", token).as_str()).unwrap(),
        )
        .json(&body);
    let resp = runtime.block_on(async { req.await });
    expect(resp.status_code()).to_equal(StatusCode::NO_CONTENT)?;

    let user_info = get_user_model(runtime, state, user_id)?;
    expect(user_info.modified_at.ge(&user_old.modified_at)).to_equal(true)?;
    match patch_password {
        false => {
            expect(user_info.salt.as_str()).to_equal(user_old.salt.as_str())?;
            expect(user_info.password.as_str())
                .to_equal(password_hash(user_id, user_info.salt.as_str()).as_str())?;
        }
        true => {
            expect(user_info.salt.as_str()).to_not_equal(user_old.salt.as_str())?;
            expect(user_info.password.as_str())
                .to_equal(password_hash("password_update", user_info.salt.as_str()).as_str())?;
        }
    }
    expect(user_info.name.as_str()).to_equal("name_update")?;
    expect(user_info.info).to_equal(info)?;

    // The token is valid if password is not patched.
    let req = server
        .patch("/auth/api/v1/user")
        .add_header(
            header::AUTHORIZATION,
            HeaderValue::from_str(format!("Bearer {}", token).as_str()).unwrap(),
        )
        .json(&body);
    let resp = runtime.block_on(async { req.await });
    match patch_password {
        false => expect(resp.status_code()).to_equal(StatusCode::NO_CONTENT),
        true => expect(resp.status_code()).to_equal(StatusCode::UNAUTHORIZED),
    }
}

fn test_patch_invalid_param(
    runtime: &Runtime,
    state: &routes::State,
    token: &str,
    param: Option<&Map<String, Value>>,
) -> Result<(), String> {
    let app = Router::new().merge(routes::new_service(&state));
    let server = TestServer::new(app);

    let mut req = server.patch("/auth/api/v1/user").add_header(
        header::AUTHORIZATION,
        HeaderValue::from_str(format!("Bearer {}", token).as_str()).unwrap(),
    );
    if let Some(param) = param {
        let mut data = Map::<String, Value>::new();
        data.insert("data".to_string(), Value::Object(param.clone()));
        req = req.json(&data)
    }
    let resp = runtime.block_on(async { req.await });
    expect(resp.status_code()).to_equal(StatusCode::BAD_REQUEST)?;
    let body: ApiError = resp.json();
    if body.code.as_str() != err::E_PARAM {
        return Err(format!("unexpected 400 error: {}", body.code.as_str()));
    }
    Ok(())
}

fn test_post_admin(
    runtime: &Runtime,
    state: &routes::State,
    token: &str,
    param: &request::PostAdminUser,
    expect_code: &str,
) -> Result<(), String> {
    let app = Router::new().merge(routes::new_service(&state));
    let server = TestServer::new(app);

    let time_before = Utc::now();
    let req = server
        .post("/auth/api/v1/user")
        .add_header(
            header::AUTHORIZATION,
            HeaderValue::from_str(format!("Bearer {}", token).as_str()).unwrap(),
        )
        .json(param);
    let resp = runtime.block_on(async { req.await });
    let time_after = Utc::now();
    let status = resp.status_code();
    if status != StatusCode::OK {
        let body: ApiError = resp.json();
        let message = match body.message.as_ref() {
            None => "",
            Some(message) => message.as_str(),
        };
        if expect_code == body.code.as_str() {
            return Ok(());
        }
        return Err(format!(
            "API not 200, status: {}, code: {}, message: {}",
            status,
            body.code.as_str(),
            message
        ));
    }
    let body: response::PostAdminUser = resp.json();
    expect(body.data.user_id.len() > 0).to_equal(true)?;

    let user_info = match runtime.block_on(async {
        let cond = QueryCond {
            user_id: Some(body.data.user_id.as_str()),
            ..Default::default()
        };
        state.model.user().get(&cond).await
    }) {
        Err(e) => return Err(format!("get user model error: {}", e)),
        Ok(info) => match info {
            None => return Err("add user then get none user".to_string()),
            Some(info) => info,
        },
    };
    expect(user_info.account.as_str()).to_equal(param.data.account.as_str())?;
    expect(user_info.created_at.ge(&time_before)).to_equal(true)?;
    expect(user_info.created_at.le(&time_after)).to_equal(true)?;
    expect(user_info.modified_at.ge(&time_before)).to_equal(true)?;
    expect(user_info.modified_at.le(&time_after)).to_equal(true)?;
    match param.expired_at.as_ref() {
        None => {
            expect(user_info.verified_at.is_some()).to_equal(true)?;
            expect(user_info.verified_at.unwrap().ge(&time_before)).to_equal(true)?;
            expect(user_info.verified_at.unwrap().le(&time_after)).to_equal(true)?;
            expect(user_info.expired_at.is_none()).to_equal(true)?;
        }
        Some(expired_at) => {
            expect(user_info.verified_at.is_none()).to_equal(true)?;
            expect(user_info.expired_at.is_some()).to_equal(true)?;
            expect(user_info.expired_at.unwrap().timestamp_millis()).to_equal(
                DateTime::parse_from_rfc3339(expired_at.as_str())
                    .unwrap()
                    .timestamp_millis(),
            )?;
        }
    }
    expect(user_info.disabled_at.is_none()).to_equal(true)?;
    expect(user_info.password.as_str())
        .to_equal(password_hash(param.data.password.as_str(), user_info.salt.as_str()).as_str())?;
    match param.data.name.as_ref() {
        None => expect(user_info.name.len()).to_equal(0)?,
        Some(name) => expect(user_info.name.as_str()).to_equal(name.as_str())?,
    }
    match param.data.info.as_ref() {
        None => expect(user_info.info).to_equal(Map::<String, Value>::new()),
        Some(info) => expect(user_info.info).to_equal(info.clone()),
    }
}

fn test_post_admin_invalid_param(
    runtime: &Runtime,
    state: &routes::State,
    token: &str,
    param: Option<&Map<String, Value>>,
) -> Result<(), String> {
    let app = Router::new().merge(routes::new_service(&state));
    let server = TestServer::new(app);

    let mut req = server.post("/auth/api/v1/user").add_header(
        header::AUTHORIZATION,
        HeaderValue::from_str(format!("Bearer {}", token).as_str()).unwrap(),
    );
    if let Some(param) = param {
        let mut data = Map::<String, Value>::new();
        data.insert("data".to_string(), Value::Object(param.clone()));
        req = req.json(&data)
    }
    let resp = runtime.block_on(async { req.await });
    expect(resp.status_code()).to_equal(StatusCode::BAD_REQUEST)?;
    let body: ApiError = resp.json();
    if body.code.as_str() != err::E_PARAM {
        return Err(format!("unexpected 400 error: {}", body.code.as_str()));
    }
    Ok(())
}

fn test_get_admin_count(
    runtime: &Runtime,
    state: &routes::State,
    token: &str,
    param: Option<&request::GetAdminUserCount>,
    expect_count: usize,
) -> Result<(), String> {
    let app = Router::new().merge(routes::new_service(&state));
    let server = TestServer::new(app);

    let req = server
        .get("/auth/api/v1/user/count")
        .add_query_params(&param)
        .add_header(
            header::AUTHORIZATION,
            HeaderValue::from_str(format!("Bearer {}", token).as_str()).unwrap(),
        );
    let resp = runtime.block_on(async { req.await });
    expect(resp.status_code()).to_equal(StatusCode::OK)?;
    let body: response::GetAdminUserCount = resp.json();
    expect(body.data.count).to_equal(expect_count)
}

fn test_get_admin_list(
    runtime: &Runtime,
    state: &routes::State,
    token: &str,
    param: &mut request::GetAdminUserList,
    expect_count: usize,
    expect_expired: usize,
    expect_disabled: usize,
) -> Result<(), String> {
    let app = Router::new().merge(routes::new_service(&state));
    let server = TestServer::new(app);

    if let Some(fields) = param.fields_vec.as_ref() {
        if fields.len() > 0 {
            param.fields = Some(fields.join(","));
        }
    }

    let fields_vec = param.fields_vec.clone();
    let req = server
        .get("/auth/api/v1/user/list")
        .add_query_params(param)
        .add_header(
            header::AUTHORIZATION,
            HeaderValue::from_str(format!("Bearer {}", token).as_str()).unwrap(),
        );
    let resp = runtime.block_on(async { req.await });
    expect(resp.status_code()).to_equal(StatusCode::OK)?;
    let body: response::GetAdminUserList = resp.json();
    expect(body.data.len()).to_equal(expect_count)?;

    let mut account_min = "";
    let mut expired_count = 0;
    let mut disabled_count = 0;
    for info in body.data.iter() {
        if let Err(_) = expect(info.account.as_str().ge(account_min)).to_equal(true) {
            return Err(format!(
                "account order error: {} - {}",
                account_min,
                info.account.as_str()
            ));
        }
        account_min = info.account.as_str();
        if let Some(fields) = fields_vec.as_ref() {
            if fields.contains(&"expired") {
                expect(info.expired_at.is_some()).to_equal(true)?;
                if info.expired_at.as_ref().unwrap().is_some() {
                    expired_count += 1;
                }
            } else {
                expect(info.expired_at.is_none()).to_equal(true)?;
            }
            if fields.contains(&"disabled") {
                expect(info.disabled_at.is_some()).to_equal(true)?;
                if info.disabled_at.as_ref().unwrap().is_some() {
                    disabled_count += 1;
                }
            } else {
                expect(info.disabled_at.is_none()).to_equal(true)?;
            }
        } else {
            expect(info.expired_at.is_none()).to_equal(true)?;
            expect(info.disabled_at.is_none()).to_equal(true)?;
        }
    }
    expect(expired_count).to_equal(expect_expired)?;
    expect(disabled_count).to_equal(expect_disabled)
}

fn test_get_admin_list_sort(
    runtime: &Runtime,
    state: &routes::State,
    token: &str,
    param: &mut request::GetAdminUserList,
    expect_accounts: &[&str],
) -> Result<(), String> {
    let app = Router::new().merge(routes::new_service(&state));
    let server = TestServer::new(app);

    if let Some(sorts) = param.sort_vec.as_ref() {
        let sorts: Vec<String> = sorts
            .iter()
            .map(|(k, v)| {
                format!(
                    "{}:{}",
                    k,
                    match v {
                        false => "desc",
                        true => "asc",
                    }
                )
            })
            .collect();
        if sorts.len() > 0 {
            param.sort = Some(sorts.join(","));
        }
    }

    let req = server
        .get("/auth/api/v1/user/list")
        .add_query_params(&param)
        .add_header(
            header::AUTHORIZATION,
            HeaderValue::from_str(format!("Bearer {}", token).as_str()).unwrap(),
        );
    let resp = runtime.block_on(async { req.await });
    if let Err(_) = expect(resp.status_code()).to_equal(StatusCode::OK) {
        let body: ApiError = resp.json();
        let message = match body.message.as_ref() {
            None => "",
            Some(message) => message.as_str(),
        };
        return Err(format!(
            "response not 200: /auth/api/v1/user/list, {}, {}",
            body.code, message
        ));
    }
    let body: response::GetAdminUserList = resp.json();
    expect(body.data.len()).to_equal(expect_accounts.len())?;

    let mut index = 0;
    for data in body.data.iter() {
        expect(data.account.as_str()).to_equal(expect_accounts[index])?;
        index += 1;
    }
    Ok(())
}

fn test_get_admin_list_offset_limit(
    runtime: &Runtime,
    state: &routes::State,
    token: &str,
    param: &request::GetAdminUserList,
    expect_ids: Vec<i32>,
) -> Result<(), String> {
    let app = Router::new().merge(routes::new_service(&state));
    let server = TestServer::new(app);

    let req = server
        .get("/auth/api/v1/user/list")
        .add_query_params(param)
        .add_header(
            header::AUTHORIZATION,
            HeaderValue::from_str(format!("Bearer {}", token).as_str()).unwrap(),
        );
    let resp = runtime.block_on(async { req.await });
    expect(resp.status_code()).to_equal(StatusCode::OK)?;
    let body: response::GetAdminUserList = resp.json();
    expect(body.data.len()).to_equal(expect_ids.len())?;

    let mut index = 0;
    for data in body.data.iter() {
        expect(data.user_id.as_str())
            .to_equal(format!("{}@example", expect_ids[index]).as_str())?;
        index += 1;
    }
    Ok(())
}

fn test_get_admin_list_format_array(
    runtime: &Runtime,
    state: &routes::State,
    token: &str,
    param: &request::GetAdminUserList,
    expect_ids: Vec<i32>,
) -> Result<(), String> {
    let app = Router::new().merge(routes::new_service(&state));
    let server = TestServer::new(app);

    let req = server
        .get("/auth/api/v1/user/list")
        .add_query_params(param)
        .add_header(
            header::AUTHORIZATION,
            HeaderValue::from_str(format!("Bearer {}", token).as_str()).unwrap(),
        );
    let resp = runtime.block_on(async { req.await });
    expect(resp.status_code()).to_equal(StatusCode::OK)?;
    let body: Vec<response::GetAdminUserListData> = resp.json();
    expect(body.len()).to_equal(expect_ids.len())?;

    let mut index = 0;
    for data in body.iter() {
        expect(data.user_id.as_str())
            .to_equal(format!("{}@example", expect_ids[index]).as_str())?;
        index += 1;
    }
    Ok(())
}

fn test_get_admin(
    runtime: &Runtime,
    state: &routes::State,
    token: &str,
    user_id: &str,
) -> Result<(), String> {
    let app = Router::new().merge(routes::new_service(&state));
    let server = TestServer::new(app);

    let user_info = get_user_model(runtime, state, user_id)?;

    let uri = format!("/auth/api/v1/user/{}", user_id);
    let req = server.get(uri.as_str()).add_header(
        header::AUTHORIZATION,
        HeaderValue::from_str(format!("Bearer {}", token).as_str()).unwrap(),
    );
    let resp = runtime.block_on(async { req.await });
    expect(resp.status_code()).to_equal(StatusCode::OK)?;
    let body: response::GetAdminUser = resp.json();
    expect(body.data.user_id.as_str()).to_equal(user_info.user_id.as_str())?;
    expect(body.data.account.as_str()).to_equal(user_info.account.as_str())?;
    expect(
        DateTime::parse_from_rfc3339(body.data.created_at.as_str())
            .unwrap()
            .timestamp_millis(),
    )
    .to_equal(user_info.created_at.timestamp_millis())?;
    expect(
        DateTime::parse_from_rfc3339(body.data.modified_at.as_str())
            .unwrap()
            .timestamp_millis(),
    )
    .to_equal(user_info.modified_at.timestamp_millis())?;
    match body.data.verified_at.as_ref() {
        None => expect(user_info.verified_at.is_none()).to_equal(true)?,
        Some(verified_at) => {
            expect(user_info.verified_at.is_some()).to_equal(true)?;
            expect(
                DateTime::parse_from_rfc3339(verified_at.as_str())
                    .unwrap()
                    .timestamp_millis(),
            )
            .to_equal(user_info.verified_at.as_ref().unwrap().timestamp_millis())?;
        }
    }
    expect(body.data.expired_at.is_none()).to_equal(true)?;
    expect(body.data.disabled_at.is_none()).to_equal(true)?;
    expect(body.data.name.as_str()).to_equal(user_info.name.as_str())?;
    expect(body.data.info).to_equal(user_info.info)
}

fn test_patch_admin_admin(
    runtime: &Runtime,
    state: &routes::State,
    token: &str,
    patch_password: bool,
) -> Result<(), String> {
    let user_id = "user_patch_admin";
    let mut user = create_user(user_id, Utc::now(), HashMap::<String, bool>::new());
    user.verified_at = None;
    user.expired_at = Some(Utc::now() + TimeDelta::try_seconds(120).unwrap());
    if let Err(e) = runtime.block_on(async { state.model.user().add(&user).await }) {
        return Err(format!("add user {} error: {}", user_id, e));
    }
    let user_old = get_user_model(runtime, state, user_id)?;
    let user_token = get_token(runtime, state, user_id)?;

    let app = Router::new().merge(routes::new_service(&state));
    let server = TestServer::new(app);

    let time_before = Utc::now().trunc_subsecs(3);
    let mut info = Map::<String, Value>::new();
    info.insert(
        "key_update".to_string(),
        Value::String("updated".to_string()),
    );
    let mut roles = HashMap::<String, bool>::new();
    roles.insert(Role::ADMIN.to_string(), true);
    roles.insert(Role::DEV.to_string(), true);
    roles.insert(Role::MANAGER.to_string(), true);
    roles.insert(Role::SERVICE.to_string(), true);
    let body = request::PatchAdminUser {
        data: Some(request::PatchAdminUserData {
            verified_at: Some(time_before.to_rfc3339_opts(SecondsFormat::Millis, true)),
            roles: Some(roles),
            password: match patch_password {
                false => None,
                true => Some("password_update".to_string()),
            },
            name: Some("name_update".to_string()),
            info: Some(info.clone()),
        }),
        disable: Some(true),
    };
    let req = server
        .patch(format!("/auth/api/v1/user/{}", user_id).as_str())
        .add_header(
            header::AUTHORIZATION,
            HeaderValue::from_str(format!("Bearer {}", token).as_str()).unwrap(),
        )
        .json(&body);
    let resp = runtime.block_on(async { req.await });
    expect(resp.status_code()).to_equal(StatusCode::NO_CONTENT)?;

    let time_after = Utc::now().trunc_subsecs(3);
    let user_info = get_user_model(runtime, state, user_id)?;
    expect(user_info.modified_at.ge(&time_before)).to_equal(true)?;
    expect(user_info.modified_at.le(&time_after)).to_equal(true)?;
    expect(user_info.verified_at.is_some()).to_equal(true)?;
    expect(user_info.verified_at.as_ref().unwrap().timestamp_millis())
        .to_equal(time_before.timestamp_millis())?;
    expect(user_info.expired_at.is_none()).to_equal(true)?;
    expect(user_info.disabled_at.is_some()).to_equal(true)?;
    expect(user_info.disabled_at.as_ref().unwrap().ge(&time_before)).to_equal(true)?;
    expect(user_info.disabled_at.as_ref().unwrap().le(&time_after)).to_equal(true)?;
    match patch_password {
        false => {
            expect(user_info.salt.as_str()).to_equal(user_old.salt.as_str())?;
            expect(user_info.password.as_str())
                .to_equal(password_hash(user_id, user_info.salt.as_str()).as_str())?;
        }
        true => {
            expect(user_info.salt.as_str()).to_not_equal(user_old.salt.as_str())?;
            expect(user_info.password.as_str())
                .to_equal(password_hash("password_update", user_info.salt.as_str()).as_str())?;
        }
    }
    expect(user_info.name.as_str()).to_equal("name_update")?;
    expect(user_info.info).to_equal(info)?;

    let req = server.get("/auth/api/v1/user").add_header(
        header::AUTHORIZATION,
        HeaderValue::from_str(format!("Bearer {}", user_token).as_str()).unwrap(),
    );
    let resp = runtime.block_on(async { req.await });
    match patch_password {
        false => expect(resp.status_code()).to_equal(StatusCode::OK)?,
        true => expect(resp.status_code()).to_equal(StatusCode::UNAUTHORIZED)?,
    }

    let body = request::PatchAdminUser {
        disable: Some(false),
        ..Default::default()
    };
    let req = server
        .patch(format!("/auth/api/v1/user/{}", user_id).as_str())
        .add_header(
            header::AUTHORIZATION,
            HeaderValue::from_str(format!("Bearer {}", token).as_str()).unwrap(),
        )
        .json(&body);
    let resp = runtime.block_on(async { req.await });
    expect(resp.status_code()).to_equal(StatusCode::NO_CONTENT)?;

    let user_info = get_user_model(runtime, state, user_id)?;
    expect(user_info.disabled_at.is_none()).to_equal(true)
}

fn test_patch_admin_manager(
    runtime: &Runtime,
    state: &routes::State,
    token: &str,
) -> Result<(), String> {
    let user_id = "user_patch_manager";
    add_user_model(runtime, state, user_id)?;

    let app = Router::new().merge(routes::new_service(&state));
    let server = TestServer::new(app);

    let time_before = Utc::now().trunc_subsecs(3);
    let mut roles = HashMap::<String, bool>::new();
    roles.insert(Role::DEV.to_string(), true);
    roles.insert(Role::MANAGER.to_string(), true);
    let body = request::PatchAdminUser {
        data: Some(request::PatchAdminUserData {
            roles: Some(roles),
            ..Default::default()
        }),
        disable: Some(true),
    };
    let req = server
        .patch(format!("/auth/api/v1/user/{}", user_id).as_str())
        .add_header(
            header::AUTHORIZATION,
            HeaderValue::from_str(format!("Bearer {}", token).as_str()).unwrap(),
        )
        .json(&body);
    let resp = runtime.block_on(async { req.await });
    expect(resp.status_code()).to_equal(StatusCode::NO_CONTENT)?;

    let time_after = Utc::now().trunc_subsecs(3);
    let user_info = get_user_model(runtime, state, user_id)?;
    expect(user_info.modified_at.ge(&time_before)).to_equal(true)?;
    expect(user_info.modified_at.le(&time_after)).to_equal(true)?;
    expect(user_info.disabled_at.is_some()).to_equal(true)?;
    expect(user_info.disabled_at.as_ref().unwrap().ge(&time_before)).to_equal(true)?;
    expect(user_info.disabled_at.as_ref().unwrap().le(&time_after)).to_equal(true)?;

    let body = request::PatchAdminUser {
        data: Some(request::PatchAdminUserData {
            roles: Some(HashMap::<String, bool>::new()),
            ..Default::default()
        }),
        ..Default::default()
    };
    let req = server
        .patch(format!("/auth/api/v1/user/{}", user_id).as_str())
        .add_header(
            header::AUTHORIZATION,
            HeaderValue::from_str(format!("Bearer {}", token).as_str()).unwrap(),
        )
        .json(&body);
    let resp = runtime.block_on(async { req.await });
    expect(resp.status_code()).to_equal(StatusCode::NO_CONTENT)?;

    let body = request::PatchAdminUser {
        disable: Some(false),
        ..Default::default()
    };
    let req = server
        .patch(format!("/auth/api/v1/user/{}", user_id).as_str())
        .add_header(
            header::AUTHORIZATION,
            HeaderValue::from_str(format!("Bearer {}", token).as_str()).unwrap(),
        )
        .json(&body);
    let resp = runtime.block_on(async { req.await });
    expect(resp.status_code()).to_equal(StatusCode::NO_CONTENT)?;

    let user_info = get_user_model(runtime, state, user_id)?;
    expect(user_info.disabled_at.is_none()).to_equal(true)
}

fn test_patch_admin_invalid_param(
    runtime: &Runtime,
    state: &routes::State,
    token: &str,
    param: Option<&Map<String, Value>>,
) -> Result<(), String> {
    let app = Router::new().merge(routes::new_service(&state));
    let server = TestServer::new(app);

    let mut req = server.patch("/auth/api/v1/user/user").add_header(
        header::AUTHORIZATION,
        HeaderValue::from_str(format!("Bearer {}", token).as_str()).unwrap(),
    );
    if let Some(param) = param {
        let mut data = Map::<String, Value>::new();
        data.insert("data".to_string(), Value::Object(param.clone()));
        req = req.json(&data)
    }
    let resp = runtime.block_on(async { req.await });
    expect(resp.status_code()).to_equal(StatusCode::BAD_REQUEST)?;
    let body: ApiError = resp.json();
    if body.code.as_str() != err::E_PARAM {
        return Err(format!("unexpected 400 error: {}", body.code.as_str()));
    }
    Ok(())
}

fn test_patch_admin_manager_invalid_perm(
    runtime: &Runtime,
    state: &routes::State,
    token: &str,
) -> Result<(), String> {
    let app = Router::new().merge(routes::new_service(&state));
    let server = TestServer::new(app);

    let body = request::PatchAdminUser {
        ..Default::default()
    };
    let req = server
        .patch("/auth/api/v1/user/admin")
        .add_header(
            header::AUTHORIZATION,
            HeaderValue::from_str(format!("Bearer {}", token).as_str()).unwrap(),
        )
        .json(&body);
    let resp = runtime.block_on(async { req.await });
    if let Err(e) = expect(resp.status_code()).to_equal(StatusCode::FORBIDDEN) {
        let body: ApiError = resp.json();
        let message = match body.message.as_ref() {
            None => "",
            Some(message) => message.as_str(),
        };
        return Err(format!("1 {}, {}, {}", e, body.code.as_str(), message));
    }
    let body: ApiError = resp.json();
    if body.code.as_str() != err::E_PERM {
        return Err(format!("unexpected 403 error: {}", body.code.as_str()));
    }

    let mut roles = HashMap::<String, bool>::new();
    roles.insert(Role::ADMIN.to_string(), true);
    let body = request::PatchAdminUser {
        data: Some(request::PatchAdminUserData {
            roles: Some(roles),
            ..Default::default()
        }),
        ..Default::default()
    };
    let req = server
        .patch("/auth/api/v1/user/user")
        .add_header(
            header::AUTHORIZATION,
            HeaderValue::from_str(format!("Bearer {}", token).as_str()).unwrap(),
        )
        .json(&body);
    let resp = runtime.block_on(async { req.await });
    if let Err(e) = expect(resp.status_code()).to_equal(StatusCode::FORBIDDEN) {
        let body: ApiError = resp.json();
        let message = match body.message.as_ref() {
            None => "",
            Some(message) => message.as_str(),
        };
        return Err(format!("2 {}, {}, {}", e, body.code.as_str(), message));
    }
    let body: ApiError = resp.json();
    if body.code.as_str() != err::E_PERM {
        return Err(format!("unexpected 403 error: {}", body.code.as_str()));
    }

    let mut roles = HashMap::<String, bool>::new();
    roles.insert(Role::SERVICE.to_string(), true);
    let body = request::PatchAdminUser {
        data: Some(request::PatchAdminUserData {
            roles: Some(roles),
            ..Default::default()
        }),
        ..Default::default()
    };
    let req = server
        .patch("/auth/api/v1/user/user")
        .add_header(
            header::AUTHORIZATION,
            HeaderValue::from_str(format!("Bearer {}", token).as_str()).unwrap(),
        )
        .json(&body);
    let resp = runtime.block_on(async { req.await });
    if let Err(e) = expect(resp.status_code()).to_equal(StatusCode::FORBIDDEN) {
        let body: ApiError = resp.json();
        let message = match body.message.as_ref() {
            None => "",
            Some(message) => message.as_str(),
        };
        return Err(format!("3 {}, {}, {}", e, body.code.as_str(), message));
    }
    let body: ApiError = resp.json();
    if body.code.as_str() != err::E_PERM {
        return Err(format!("unexpected 403 error: {}", body.code.as_str()));
    }

    let body = request::PatchAdminUser {
        disable: Some(true),
        ..Default::default()
    };
    let req = server
        .patch("/auth/api/v1/user/admin")
        .add_header(
            header::AUTHORIZATION,
            HeaderValue::from_str(format!("Bearer {}", token).as_str()).unwrap(),
        )
        .json(&body);
    let resp = runtime.block_on(async { req.await });
    if let Err(e) = expect(resp.status_code()).to_equal(StatusCode::FORBIDDEN) {
        let body: ApiError = resp.json();
        let message = match body.message.as_ref() {
            None => "",
            Some(message) => message.as_str(),
        };
        return Err(format!("4 {}, {}, {}", e, body.code.as_str(), message));
    }
    let body: ApiError = resp.json();
    if body.code.as_str() != err::E_PERM {
        return Err(format!("unexpected 403 error: {}", body.code.as_str()));
    }

    let body = request::PatchAdminUser {
        disable: Some(true),
        ..Default::default()
    };
    let req = server
        .patch("/auth/api/v1/user/manager")
        .add_header(
            header::AUTHORIZATION,
            HeaderValue::from_str(format!("Bearer {}", token).as_str()).unwrap(),
        )
        .json(&body);
    let resp = runtime.block_on(async { req.await });
    if let Err(e) = expect(resp.status_code()).to_equal(StatusCode::FORBIDDEN) {
        let body: ApiError = resp.json();
        let message = match body.message.as_ref() {
            None => "",
            Some(message) => message.as_str(),
        };
        return Err(format!("5 {}, {}, {}", e, body.code.as_str(), message));
    }
    let body: ApiError = resp.json();
    if body.code.as_str() != err::E_PERM {
        return Err(format!("unexpected 403 error: {}", body.code.as_str()));
    }

    Ok(())
}

fn get_user_model(runtime: &Runtime, state: &routes::State, user_id: &str) -> Result<User, String> {
    match runtime.block_on(async {
        let cond = QueryCond {
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

fn add_user_model(runtime: &Runtime, state: &routes::State, user_id: &str) -> Result<(), String> {
    match runtime.block_on(async {
        let mut roles = HashMap::<String, bool>::new();
        if user_id == "admin" || user_id == "manager" || user_id == "dev" {
            roles.insert(user_id.to_string(), true);
        }
        let mut user = create_user(user_id, Utc::now(), roles);
        user.info
            .insert("name".to_string(), Value::String(user_id.to_string()));
        state.model.user().add(&user).await
    }) {
        Err(e) => Err(format!("add user model info error: {}", e)),
        Ok(_) => Ok(()),
    }
}

/// Returns (total_users, expired_users, disabled_users) tuple.
fn count_list_dataset(
    runtime: &Runtime,
    state: &routes::State,
) -> Result<(usize, usize, usize), String> {
    let now = Utc::now();

    let mut user = create_user("account1@example.com", now, HashMap::<String, bool>::new());
    user.created_at = now;
    user.modified_at = now + TimeDelta::try_milliseconds(4).unwrap();
    user.verified_at = Some(now);
    user.expired_at = None;
    user.name = "name1@user.com".to_string();
    runtime.block_on(async {
        if let Err(e) = state.model.user().add(&user).await {
            return Err(format!("add user {} error: {}", user.account, e));
        }
        Ok(())
    })?;

    user.user_id = "account2@example.com".to_string();
    user.account = user.user_id.clone();
    user.created_at = now + TimeDelta::try_milliseconds(1).unwrap();
    user.modified_at = now + TimeDelta::try_milliseconds(3).unwrap();
    user.verified_at = None;
    user.expired_at = Some(now);
    user.name = "name4@user.com".to_string();
    runtime.block_on(async {
        if let Err(e) = state.model.user().add(&user).await {
            return Err(format!("add user {} error: {}", user.account, e));
        }
        Ok(())
    })?;

    user.user_id = "account3@example.com".to_string();
    user.account = user.user_id.clone();
    user.created_at = now + TimeDelta::try_milliseconds(2).unwrap();
    user.modified_at = now + TimeDelta::try_milliseconds(2).unwrap();
    user.verified_at = Some(now + TimeDelta::try_milliseconds(2).unwrap());
    user.expired_at = None;
    user.name = "name3@user.com".to_string();
    runtime.block_on(async {
        if let Err(e) = state.model.user().add(&user).await {
            return Err(format!("add user {} error: {}", user.account, e));
        }
        Ok(())
    })?;

    user.user_id = "account4@example.com".to_string();
    user.account = user.user_id.clone();
    user.created_at = now + TimeDelta::try_milliseconds(3).unwrap();
    user.modified_at = now + TimeDelta::try_milliseconds(1).unwrap();
    user.verified_at = None;
    user.expired_at = Some(now);
    user.disabled_at = Some(now);
    user.name = "name2@user.com".to_string();
    runtime.block_on(async {
        if let Err(e) = state.model.user().add(&user).await {
            return Err(format!("add user {} error: {}", user.account, e));
        }
        Ok(())
    })?;

    Ok((4, 2, 1))
}
