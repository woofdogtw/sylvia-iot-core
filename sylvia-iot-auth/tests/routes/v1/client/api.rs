use std::collections::HashMap;

use actix_web::{
    http::{header, StatusCode},
    middleware::NormalizePath,
    test::{self, TestRequest},
    App,
};
use chrono::{DateTime, SubsecRound, TimeDelta, Utc};
use laboratory::{expect, SpecContext};
use mongodb::bson::Document;
use serde_json::{Map, Value};
use serde_urlencoded;
use sql_builder::SqlBuilder;
use sqlx;
use tokio::runtime::Runtime;

use sylvia_iot_auth::{
    models::client::{Client, QueryCond},
    routes,
};
use sylvia_iot_corelib::{err, role::Role};

use super::{
    super::{
        super::libs::{create_client, create_user},
        libs::{
            get_token, test_get_list_invalid_param, test_invalid_perm, test_invalid_token, ApiError,
        },
        TestState, STATE,
    },
    request, response,
};

const NO_ID_ERR_STR: &'static str = "get no client with ID";

pub fn before_all_fn(state: &mut HashMap<&'static str, TestState>) {
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.routes_state.as_ref().unwrap().model.as_ref();

    runtime.block_on(async {
        let now = Utc::now();

        let mut roles = HashMap::<String, bool>::new();
        roles.insert(Role::ADMIN.to_string(), true);
        if let Err(e) = model.user().add(&create_user("admin", now, roles)).await {
            println!("add user admin error: {}", e);
        }

        let mut roles = HashMap::<String, bool>::new();
        roles.insert(Role::MANAGER.to_string(), true);
        if let Err(e) = model.user().add(&create_user("manager", now, roles)).await {
            println!("add user manager error: {}", e);
        }

        let mut roles = HashMap::<String, bool>::new();
        roles.insert(Role::SERVICE.to_string(), true);
        if let Err(e) = model.user().add(&create_user("service", now, roles)).await {
            println!("add user manager error: {}", e);
        }

        let mut roles = HashMap::<String, bool>::new();
        roles.insert(Role::DEV.to_string(), true);
        if let Err(e) = model.user().add(&create_user("public", now, roles)).await {
            println!("add user public error: {}", e);
        }

        let roles = HashMap::<String, bool>::new();
        if let Err(e) = model.user().add(&create_user("user", now, roles)).await {
            println!("add user user error: {}", e);
        }
    })
}

pub fn after_all_fn(state: &mut HashMap<&'static str, TestState>) -> () {
    const USER_NAME: &'static str = "user";

    let state = state.get_mut(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    if let Some(model) = state.mongodb.as_ref() {
        runtime.block_on(async {
            struct Doc;

            let conn = model.get_connection();
            let _ = conn
                .collection::<Doc>(USER_NAME)
                .delete_many(Document::new(), None)
                .await;
        });
    }
    if let Some(model) = state.sqlite.as_ref() {
        runtime.block_on(async {
            let conn = model.get_connection();
            let sql = SqlBuilder::delete_from(USER_NAME).sql().unwrap();
            let _ = sqlx::query(sql.as_str()).execute(conn).await;
        });
    }
}

pub fn before_each_fn(state: &mut HashMap<&'static str, TestState>) {
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.routes_state.as_ref().unwrap().model.as_ref();

    runtime.block_on(async {
        let client = create_client("public", "public", None);
        if let Err(e) = model.client().add(&client).await {
            println!("add client public error: {}", e);
        }
    })
}

pub fn after_each_fn(state: &mut HashMap<&'static str, TestState>) -> () {
    const CLIENT_NAME: &'static str = "client";

    let state = state.get_mut(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    if let Some(model) = state.mongodb.as_ref() {
        runtime.block_on(async {
            struct Doc;

            let conn = model.get_connection();
            let _ = conn
                .collection::<Doc>(CLIENT_NAME)
                .delete_many(Document::new(), None)
                .await;
        });
    }
    if let Some(model) = state.sqlite.as_ref() {
        runtime.block_on(async {
            let conn = model.get_connection();
            let sql = SqlBuilder::delete_from(CLIENT_NAME).sql().unwrap();
            let _ = sqlx::query(sql.as_str()).execute(conn).await;
        });
    }
}

pub fn post(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    let user_id = "admin";
    let token = get_token(runtime, routes_state, user_id)?;

    let param = request::PostClient {
        data: request::PostClientData {
            redirect_uris: vec![],
            scopes: vec![],
            user_id: None,
            name: "".to_string(),
            image: None,
        },
        credentials: None,
    };
    test_post(runtime, &routes_state, token.as_str(), user_id, &param, "")?;

    let param = request::PostClient {
        data: request::PostClientData {
            redirect_uris: vec![
                "http://uri".to_string(),
                crate::TEST_REDIRECT_URI.to_string(),
                crate::TEST_REDIRECT_URI.to_string(),
            ],
            scopes: vec![
                "scope2".to_string(),
                "scope1".to_string(),
                "scope1".to_string(),
            ],
            user_id: Some("user".to_string()),
            name: "name".to_string(),
            image: Some("image".to_string()),
        },
        credentials: Some(true),
    };
    test_post(runtime, &routes_state, token.as_str(), user_id, &param, "")?;

    let user_id = "public";
    let token = get_token(runtime, routes_state, user_id)?;

    let param = request::PostClient {
        data: request::PostClientData {
            redirect_uris: vec![],
            scopes: vec![],
            user_id: None,
            name: "".to_string(),
            image: None,
        },
        credentials: Some(false),
    };
    test_post(runtime, &routes_state, token.as_str(), user_id, &param, "")?;

    let param = request::PostClient {
        data: request::PostClientData {
            redirect_uris: vec![
                "http://uri".to_string(),
                crate::TEST_REDIRECT_URI.to_string(),
                crate::TEST_REDIRECT_URI.to_string(),
            ],
            scopes: vec![
                "scope2".to_string(),
                "scope1".to_string(),
                "scope1".to_string(),
            ],
            user_id: Some("user".to_string()),
            name: "name".to_string(),
            image: Some("image".to_string()),
        },
        credentials: Some(true),
    };
    test_post(runtime, &routes_state, token.as_str(), user_id, &param, "")
}

pub fn post_dup(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    let user_id = "admin";
    let token = get_token(runtime, routes_state, user_id)?;

    let mut app = runtime.block_on(async {
        test::init_service(
            App::new()
                .wrap(NormalizePath::trim())
                .service(routes::new_service(routes_state)),
        )
        .await
    });

    let param = request::PostClient {
        data: request::PostClientData {
            redirect_uris: vec![
                "http://r1".to_string(),
                "http://r3".to_string(),
                "http://r1".to_string(),
                "http://r2".to_string(),
            ],
            scopes: vec![
                "s1".to_string(),
                "s3".to_string(),
                "s1".to_string(),
                "s4".to_string(),
                "s2".to_string(),
            ],
            user_id: None,
            name: "".to_string(),
            image: None,
        },
        credentials: None,
    };

    let req = TestRequest::post()
        .uri("/auth/api/v1/client")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)))
        .set_json(param)
        .to_request();
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    expect(resp.status()).to_equal(StatusCode::OK)?;
    let body: response::PostClient = runtime.block_on(async { test::read_body_json(resp).await });
    expect(body.data.client_id.len() > 0).to_equal(true)?;

    let client_info = match runtime.block_on(async {
        let cond = QueryCond {
            client_id: Some(body.data.client_id.as_str()),
            ..Default::default()
        };
        routes_state.model.client().get(&cond).await
    }) {
        Err(e) => return Err(format!("get client model error: {}", e)),
        Ok(info) => match info {
            None => return Err("add client then get none client".to_string()),
            Some(info) => info,
        },
    };
    expect(client_info.redirect_uris.len()).to_equal(3)?;
    expect(client_info.redirect_uris[0].as_str()).to_equal("http://r1")?;
    expect(client_info.redirect_uris[1].as_str()).to_equal("http://r2")?;
    expect(client_info.redirect_uris[2].as_str()).to_equal("http://r3")?;
    expect(client_info.scopes.len()).to_equal(4)?;
    expect(client_info.scopes[0].as_str()).to_equal("s1")?;
    expect(client_info.scopes[1].as_str()).to_equal("s2")?;
    expect(client_info.scopes[2].as_str()).to_equal("s3")?;
    expect(client_info.scopes[3].as_str()).to_equal("s4")
}

pub fn post_not_exist_user(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    let user_id = "admin";
    let token = get_token(runtime, routes_state, user_id)?;

    let param = request::PostClient {
        data: request::PostClientData {
            redirect_uris: vec![],
            scopes: vec![],
            user_id: Some("test".to_string()),
            name: "".to_string(),
            image: None,
        },
        credentials: None,
    };
    test_post(
        runtime,
        &routes_state,
        token.as_str(),
        user_id,
        &param,
        "err_auth_user_not_exist",
    )
}

pub fn post_invalid_param(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    let user_id = "admin";
    let token = get_token(runtime, routes_state, user_id)?;

    test_post_invalid_param(runtime, &routes_state, token.as_str(), None)?;

    let param = request::PostClient {
        data: request::PostClientData {
            redirect_uris: vec!["".to_string()],
            scopes: vec![],
            user_id: Some("admin".to_string()),
            name: "".to_string(),
            image: None,
        },
        credentials: None,
    };
    test_post_invalid_param(runtime, &routes_state, token.as_str(), Some(&param))?;

    let param = request::PostClient {
        data: request::PostClientData {
            redirect_uris: vec![":://".to_string()],
            scopes: vec![],
            user_id: Some("admin".to_string()),
            name: "".to_string(),
            image: None,
        },
        credentials: None,
    };
    test_post_invalid_param(runtime, &routes_state, token.as_str(), Some(&param))?;

    let param = request::PostClient {
        data: request::PostClientData {
            redirect_uris: vec![],
            scopes: vec!["".to_string()],
            user_id: Some("admin".to_string()),
            name: "".to_string(),
            image: None,
        },
        credentials: None,
    };
    test_post_invalid_param(runtime, &routes_state, token.as_str(), Some(&param))?;

    let param = request::PostClient {
        data: request::PostClientData {
            redirect_uris: vec![],
            scopes: vec!["scope..abc".to_string()],
            user_id: Some("admin".to_string()),
            name: "".to_string(),
            image: None,
        },
        credentials: None,
    };
    test_post_invalid_param(runtime, &routes_state, token.as_str(), Some(&param))?;

    let param = request::PostClient {
        data: request::PostClientData {
            redirect_uris: vec![],
            scopes: vec![],
            user_id: Some("".to_string()),
            name: "".to_string(),
            image: None,
        },
        credentials: None,
    };
    test_post_invalid_param(runtime, &routes_state, token.as_str(), Some(&param))
}

pub fn post_invalid_token(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    let req = TestRequest::post().uri("/auth/api/v1/client");
    test_invalid_token(runtime, &routes_state, req)
}

pub fn post_invalid_perm(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    let token = get_token(runtime, routes_state, "manager")?;
    let req = TestRequest::post().uri("/auth/api/v1/client");
    test_invalid_perm(runtime, &routes_state, token.as_str(), req)?;

    let token = get_token(runtime, routes_state, "service")?;
    let req = TestRequest::post().uri("/auth/api/v1/client");
    test_invalid_perm(runtime, &routes_state, token.as_str(), req)?;

    let token = get_token(runtime, routes_state, "user")?;
    let req = TestRequest::post().uri("/auth/api/v1/client");
    test_invalid_perm(runtime, &routes_state, token.as_str(), req)
}

pub fn get_count(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    let data_size = count_list_dataset(runtime, &routes_state)?;

    let admin_token = get_token(runtime, routes_state, "admin")?;
    let public_token = get_token(runtime, routes_state, "public")?;

    test_get_count(
        runtime,
        &routes_state,
        admin_token.as_str(),
        None,
        data_size.0,
    )?;
    test_get_count(runtime, &routes_state, public_token.as_str(), None, 1)?;

    let param = request::GetClientCount {
        user: Some("user".to_string()),
    };
    test_get_count(
        runtime,
        &routes_state,
        admin_token.as_str(),
        Some(&param),
        data_size.1,
    )?;
    test_get_count(runtime, &routes_state, public_token.as_str(), None, 1)
}

pub fn get_count_invalid_token(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    let req = TestRequest::get().uri("/auth/api/v1/client/count");
    test_invalid_token(runtime, &routes_state, req)
}

pub fn get_count_invalid_perm(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    let token = get_token(runtime, routes_state, "manager")?;
    let req = TestRequest::get().uri("/auth/api/v1/client/count");
    test_invalid_perm(runtime, &routes_state, token.as_str(), req)?;

    let token = get_token(runtime, routes_state, "service")?;
    let req = TestRequest::get().uri("/auth/api/v1/client/count");
    test_invalid_perm(runtime, &routes_state, token.as_str(), req)?;

    let token = get_token(runtime, routes_state, "user")?;
    let req = TestRequest::get().uri("/auth/api/v1/client/count");
    test_invalid_perm(runtime, &routes_state, token.as_str(), req)
}

pub fn get_list(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    let data_size = count_list_dataset(runtime, &routes_state)?;

    let admin_token = get_token(runtime, routes_state, "admin")?;
    let public_token = get_token(runtime, routes_state, "public")?;

    test_get_list(
        runtime,
        &routes_state,
        admin_token.as_str(),
        true,
        None,
        data_size.0,
    )?;
    test_get_list(
        runtime,
        &routes_state,
        public_token.as_str(),
        false,
        None,
        1,
    )?;

    let param = request::GetClientList {
        user: Some("user".to_string()),
        ..Default::default()
    };
    test_get_list(
        runtime,
        &routes_state,
        admin_token.as_str(),
        true,
        Some(&param),
        data_size.1,
    )?;
    test_get_list(
        runtime,
        &routes_state,
        public_token.as_str(),
        false,
        Some(&param),
        1,
    )
}

pub fn get_list_sort(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    count_list_dataset(runtime, &routes_state)?;

    let admin_token = get_token(runtime, routes_state, "admin")?;

    let mut param = request::GetClientList {
        ..Default::default()
    };
    test_get_list_sort(
        runtime,
        &routes_state,
        admin_token.as_str(),
        &mut param,
        &[
            "client_admin1",
            "client_admin2",
            "client_user2",
            "client_user1",
            "public",
        ],
    )?;

    param.sort_vec = Some(vec![("name", true)]);
    test_get_list_sort(
        runtime,
        &routes_state,
        admin_token.as_str(),
        &mut param,
        &[
            "client_admin1",
            "client_admin2",
            "client_user2",
            "client_user1",
            "public",
        ],
    )?;
    param.sort_vec = Some(vec![("name", false)]);
    test_get_list_sort(
        runtime,
        &routes_state,
        admin_token.as_str(),
        &mut param,
        &[
            "public",
            "client_user1",
            "client_user2",
            "client_admin2",
            "client_admin1",
        ],
    )?;

    param.sort_vec = Some(vec![("created", true)]);
    test_get_list_sort(
        runtime,
        &routes_state,
        admin_token.as_str(),
        &mut param,
        &[
            "public",
            "client_admin1",
            "client_admin2",
            "client_user1",
            "client_user2",
        ],
    )?;
    param.sort_vec = Some(vec![("created", false)]);
    test_get_list_sort(
        runtime,
        &routes_state,
        admin_token.as_str(),
        &mut param,
        &[
            "client_user2",
            "client_user1",
            "client_admin2",
            "client_admin1",
            "public",
        ],
    )?;

    param.sort_vec = Some(vec![("modified", true)]);
    test_get_list_sort(
        runtime,
        &routes_state,
        admin_token.as_str(),
        &mut param,
        &[
            "public",
            "client_user2",
            "client_user1",
            "client_admin2",
            "client_admin1",
        ],
    )?;
    param.sort_vec = Some(vec![("modified", false)]);
    test_get_list_sort(
        runtime,
        &routes_state,
        admin_token.as_str(),
        &mut param,
        &[
            "client_admin1",
            "client_admin2",
            "client_user1",
            "client_user2",
            "public",
        ],
    )
}

pub fn get_list_offset_limit(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    for i in 100..302 {
        add_client_model(
            runtime,
            &routes_state,
            format!("client_{}", i).as_str(),
            "admin",
            None,
            None,
        )?;
    }

    let user_id = "admin";
    let admin_token = get_token(runtime, routes_state, user_id)?;

    let mut param = request::GetClientList {
        user: Some(user_id.to_string()),
        ..Default::default()
    };
    test_get_list_offset_limit(
        runtime,
        &routes_state,
        admin_token.as_str(),
        &param,
        (100..200).collect(),
    )?;

    param.limit = Some(0);
    test_get_list_offset_limit(
        runtime,
        &routes_state,
        admin_token.as_str(),
        &param,
        (100..302).collect(),
    )?;

    param.offset = Some(0);
    param.limit = Some(5);
    test_get_list_offset_limit(
        runtime,
        &routes_state,
        admin_token.as_str(),
        &param,
        (100..105).collect(),
    )?;

    param.offset = Some(5);
    param.limit = Some(0);
    test_get_list_offset_limit(
        runtime,
        &routes_state,
        admin_token.as_str(),
        &param,
        (105..302).collect(),
    )?;

    param.offset = Some(198);
    param.limit = Some(50);
    test_get_list_offset_limit(
        runtime,
        &routes_state,
        admin_token.as_str(),
        &param,
        (298..302).collect(),
    )?;

    param.offset = Some(2);
    param.limit = Some(105);
    test_get_list_offset_limit(
        runtime,
        &routes_state,
        admin_token.as_str(),
        &param,
        (102..207).collect(),
    )?;

    param.offset = Some(2);
    param.limit = None;
    test_get_list_offset_limit(
        runtime,
        &routes_state,
        admin_token.as_str(),
        &param,
        (102..202).collect(),
    )
}

pub fn get_list_format_array(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    for i in 100..302 {
        add_client_model(
            runtime,
            &routes_state,
            format!("client_{}", i).as_str(),
            "admin",
            None,
            None,
        )?;
    }

    let user_id = "admin";
    let admin_token = get_token(runtime, routes_state, user_id)?;

    let mut param = request::GetClientList {
        user: Some(user_id.to_string()),
        limit: Some(5),
        format: Some("array".to_string()),
        ..Default::default()
    };
    test_get_list_format_array(
        runtime,
        &routes_state,
        admin_token.as_str(),
        &param,
        (100..105).collect(),
    )?;

    param.offset = Some(2);
    param.limit = Some(105);
    test_get_list_format_array(
        runtime,
        &routes_state,
        admin_token.as_str(),
        &param,
        (102..207).collect(),
    )
}

pub fn get_list_invalid_param(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    let token = get_token(runtime, routes_state, "admin")?;
    let uri = "/auth/api/v1/client/list";

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

pub fn get_list_invalid_token(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    let req = TestRequest::get().uri("/auth/api/v1/client/list");
    test_invalid_token(runtime, &routes_state, req)
}

pub fn get_list_invalid_perm(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    let token = get_token(runtime, routes_state, "manager")?;
    let req = TestRequest::get().uri("/auth/api/v1/client/list");
    test_invalid_perm(runtime, &routes_state, token.as_str(), req)?;

    let token = get_token(runtime, routes_state, "service")?;
    let req = TestRequest::get().uri("/auth/api/v1/client/list");
    test_invalid_perm(runtime, &routes_state, token.as_str(), req)?;

    let token = get_token(runtime, routes_state, "user")?;
    let req = TestRequest::get().uri("/auth/api/v1/client/list");
    test_invalid_perm(runtime, &routes_state, token.as_str(), req)
}

pub fn get(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    add_client_model(runtime, routes_state, "client_admin", "admin", None, None)?;
    add_client_model(
        runtime,
        routes_state,
        "client_user",
        "user",
        Some("secret".to_string()),
        Some("image".to_string()),
    )?;

    let admin_token = get_token(runtime, routes_state, "admin")?;
    let public_token = get_token(runtime, routes_state, "public")?;

    test_get(
        runtime,
        &routes_state,
        admin_token.as_str(),
        true,
        "client_admin",
    )?;
    test_get(
        runtime,
        &routes_state,
        admin_token.as_str(),
        true,
        "client_user",
    )?;
    test_get(
        runtime,
        &routes_state,
        public_token.as_str(),
        false,
        "public",
    )
}

pub fn get_wrong_id(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    let mut app = runtime.block_on(async {
        test::init_service(
            App::new()
                .wrap(NormalizePath::trim())
                .service(routes::new_service(&routes_state)),
        )
        .await
    });

    let admin_token = get_token(runtime, routes_state, "admin")?;
    let req = TestRequest::get()
        .uri("/auth/api/v1/client/id")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", admin_token)))
        .to_request();
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    expect(resp.status()).to_equal(StatusCode::NOT_FOUND)?;
    let body: ApiError = runtime.block_on(async { test::read_body_json(resp).await });
    if body.code.as_str() != err::E_NOT_FOUND {
        return Err(format!("unexpected 404 error: {}", body.code.as_str()));
    }

    let public_token = get_token(runtime, routes_state, "public")?;
    let req = TestRequest::get()
        .uri("/auth/api/v1/client/client_user1")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", public_token)))
        .to_request();
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    expect(resp.status()).to_equal(StatusCode::NOT_FOUND)?;
    let body: ApiError = runtime.block_on(async { test::read_body_json(resp).await });
    if body.code.as_str() != err::E_NOT_FOUND {
        return Err(format!("unexpected 404 error: {}", body.code.as_str()));
    }
    Ok(())
}

pub fn get_invalid_token(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    let req = TestRequest::get().uri("/auth/api/v1/client/id");
    test_invalid_token(runtime, &routes_state, req)
}

pub fn get_invalid_perm(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    let token = get_token(runtime, routes_state, "manager")?;
    let req = TestRequest::get().uri("/auth/api/v1/client/id");
    test_invalid_perm(runtime, &routes_state, token.as_str(), req)?;

    let token = get_token(runtime, routes_state, "service")?;
    let req = TestRequest::get().uri("/auth/api/v1/client/id");
    test_invalid_perm(runtime, &routes_state, token.as_str(), req)?;

    let token = get_token(runtime, routes_state, "user")?;
    let req = TestRequest::get().uri("/auth/api/v1/client/id");
    test_invalid_perm(runtime, &routes_state, token.as_str(), req)
}

pub fn patch(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    let admin_token = get_token(runtime, routes_state, "admin")?;
    let public_token = get_token(runtime, routes_state, "public")?;

    test_patch(
        runtime,
        &routes_state,
        admin_token.as_str(),
        "client_admin",
        "admin",
        false,
    )?;
    test_patch(
        runtime,
        &routes_state,
        admin_token.as_str(),
        "client_user",
        "user",
        true,
    )?;
    test_patch(
        runtime,
        &routes_state,
        public_token.as_str(),
        "client_public",
        "public",
        false,
    )
}

pub fn patch_wrong_id(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    add_client_model(runtime, &routes_state, "client_user", "user", None, None)?;

    let admin_token = get_token(runtime, routes_state, "admin")?;
    let public_token = get_token(runtime, routes_state, "public")?;

    let mut app = runtime.block_on(async {
        test::init_service(
            App::new()
                .wrap(NormalizePath::trim())
                .service(routes::new_service(&routes_state)),
        )
        .await
    });

    let param = request::PatchClient {
        data: Some(request::PatchClientData {
            name: Some("name".to_string()),
            ..Default::default()
        }),
        ..Default::default()
    };

    let req = TestRequest::patch()
        .uri("/auth/api/v1/client/id")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", admin_token)))
        .set_json(&param)
        .to_request();
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    expect(resp.status()).to_equal(StatusCode::NOT_FOUND)?;
    let body: ApiError = runtime.block_on(async { test::read_body_json(resp).await });
    if body.code.as_str() != err::E_NOT_FOUND {
        return Err(format!("unexpected 404 error: {}", body.code.as_str()));
    }

    let req = TestRequest::patch()
        .uri("/auth/api/v1/client/client_user")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", public_token)))
        .set_json(&param)
        .to_request();
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    expect(resp.status()).to_equal(StatusCode::NOT_FOUND)?;
    let body: ApiError = runtime.block_on(async { test::read_body_json(resp).await });
    if body.code.as_str() != err::E_NOT_FOUND {
        return Err(format!("unexpected 404 error: {}", body.code.as_str()));
    }
    Ok(())
}

pub fn patch_invalid_param(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    add_client_model(
        runtime,
        &routes_state,
        "client_public",
        "public",
        None,
        None,
    )?;

    let admin_token = get_token(runtime, routes_state, "admin")?;
    let public_token = get_token(runtime, routes_state, "public")?;

    test_patch_invalid_param(
        runtime,
        &routes_state,
        admin_token.as_str(),
        "client_public",
        None,
    )?;

    let param = request::PatchClient {
        ..Default::default()
    };
    test_patch_invalid_param(
        runtime,
        &routes_state,
        admin_token.as_str(),
        "client_public",
        Some(&param),
    )?;

    let param = request::PatchClient {
        data: Some(request::PatchClientData {
            redirect_uris: Some(vec!["http://uri1".to_string(), "".to_string()]),
            ..Default::default()
        }),
        ..Default::default()
    };
    test_patch_invalid_param(
        runtime,
        &routes_state,
        admin_token.as_str(),
        "client_public",
        Some(&param),
    )?;

    let param = request::PatchClient {
        data: Some(request::PatchClientData {
            redirect_uris: Some(vec!["http://uri1".to_string(), ":://".to_string()]),
            ..Default::default()
        }),
        ..Default::default()
    };
    test_patch_invalid_param(
        runtime,
        &routes_state,
        admin_token.as_str(),
        "client_public",
        Some(&param),
    )?;

    let param = request::PatchClient {
        data: Some(request::PatchClientData {
            scopes: Some(vec!["scope1".to_string(), "".to_string()]),
            ..Default::default()
        }),
        ..Default::default()
    };
    test_patch_invalid_param(
        runtime,
        &routes_state,
        admin_token.as_str(),
        "client_public",
        Some(&param),
    )?;

    let param = request::PatchClient {
        data: Some(request::PatchClientData {
            scopes: Some(vec!["scope1".to_string(), "scope..abc".to_string()]),
            ..Default::default()
        }),
        ..Default::default()
    };
    test_patch_invalid_param(
        runtime,
        &routes_state,
        admin_token.as_str(),
        "client_public",
        Some(&param),
    )?;

    let param = request::PatchClient {
        regen_secret: Some(true),
        ..Default::default()
    };
    test_patch_invalid_param(
        runtime,
        &routes_state,
        admin_token.as_str(),
        "client_public",
        Some(&param),
    )?;

    let param = request::PatchClient {
        regen_secret: Some(true),
        ..Default::default()
    };
    test_patch_invalid_param(
        runtime,
        &routes_state,
        public_token.as_str(),
        "client_public",
        Some(&param),
    )
}

pub fn patch_invalid_token(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    let req = TestRequest::patch().uri("/auth/api/v1/client/id");
    test_invalid_token(runtime, &routes_state, req)
}

pub fn patch_invalid_perm(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    let token = get_token(runtime, routes_state, "manager")?;
    let req = TestRequest::patch().uri("/auth/api/v1/client/id");
    test_invalid_perm(runtime, &routes_state, token.as_str(), req)?;

    let token = get_token(runtime, routes_state, "service")?;
    let req = TestRequest::patch().uri("/auth/api/v1/client/id");
    test_invalid_perm(runtime, &routes_state, token.as_str(), req)?;

    let token = get_token(runtime, routes_state, "user")?;
    let req = TestRequest::patch().uri("/auth/api/v1/client/id");
    test_invalid_perm(runtime, &routes_state, token.as_str(), req)
}

pub fn delete(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    add_client_model(
        runtime,
        &routes_state,
        "client_public",
        "public",
        None,
        None,
    )?;
    add_client_model(runtime, &routes_state, "client_user", "user", None, None)?;

    let admin_token = get_token(runtime, routes_state, "admin")?;
    let public_token = get_token(runtime, routes_state, "public")?;

    let mut app = runtime.block_on(async {
        test::init_service(
            App::new()
                .wrap(NormalizePath::trim())
                .service(routes::new_service(&routes_state)),
        )
        .await
    });

    let req = TestRequest::delete()
        .uri("/auth/api/v1/client/id")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", admin_token)))
        .to_request();
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    expect(resp.status()).to_equal(StatusCode::NO_CONTENT)?;
    let _ = get_client_model(runtime, &routes_state, "client_public")?;

    let req = TestRequest::delete()
        .uri("/auth/api/v1/client/client_public")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", public_token)))
        .to_request();
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    expect(resp.status()).to_equal(StatusCode::NO_CONTENT)?;
    match get_client_model(runtime, &routes_state, "client_public") {
        Err(e) => expect(e.contains(NO_ID_ERR_STR)).to_equal(true)?,
        Ok(_) => return Err("public cannot delete client_public".to_string()),
    }

    let req = TestRequest::delete()
        .uri("/auth/api/v1/client/client_user")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", public_token)))
        .to_request();
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    expect(resp.status()).to_equal(StatusCode::NO_CONTENT)?;
    get_client_model(runtime, &routes_state, "client_user")?;

    let req = TestRequest::delete()
        .uri("/auth/api/v1/client/client_user")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", admin_token)))
        .to_request();
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    expect(resp.status()).to_equal(StatusCode::NO_CONTENT)?;
    match get_client_model(runtime, &routes_state, "client_user") {
        Err(e) => expect(e.contains(NO_ID_ERR_STR)).to_equal(true)?,
        Ok(_) => return Err("admin cannot delete client_user".to_string()),
    }

    let req = TestRequest::delete()
        .uri("/auth/api/v1/client/public")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", public_token)))
        .to_request();
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    expect(resp.status()).to_equal(StatusCode::FORBIDDEN)?;
    let body: ApiError = runtime.block_on(async { test::read_body_json(resp).await });
    if body.code.as_str() != err::E_PERM {
        return Err(format!("unexpected 403 error: {}", body.code.as_str()));
    }

    Ok(())
}

pub fn delete_invalid_token(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    let req = TestRequest::delete().uri("/auth/api/v1/client/id");
    test_invalid_token(runtime, &routes_state, req)
}

pub fn delete_invalid_perm(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    let token = get_token(runtime, routes_state, "manager")?;
    let req = TestRequest::delete().uri("/auth/api/v1/client/id");
    test_invalid_perm(runtime, &routes_state, token.as_str(), req)?;

    let token = get_token(runtime, routes_state, "service")?;
    let req = TestRequest::delete().uri("/auth/api/v1/client/id");
    test_invalid_perm(runtime, &routes_state, token.as_str(), req)?;

    let token = get_token(runtime, routes_state, "user")?;
    let req = TestRequest::delete().uri("/auth/api/v1/client/id");
    test_invalid_perm(runtime, &routes_state, token.as_str(), req)
}

pub fn delete_user(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    add_client_model(
        runtime,
        &routes_state,
        "client_public",
        "public",
        None,
        None,
    )?;
    add_client_model(runtime, &routes_state, "client_user", "user", None, None)?;

    let token = get_token(runtime, routes_state, "admin")?;

    let mut app = runtime.block_on(async {
        test::init_service(
            App::new()
                .wrap(NormalizePath::trim())
                .service(routes::new_service(&routes_state)),
        )
        .await
    });

    let req = TestRequest::delete()
        .uri("/auth/api/v1/client/user/user")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)))
        .to_request();
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    expect(resp.status()).to_equal(StatusCode::NO_CONTENT)?;
    match get_client_model(runtime, &routes_state, "client_user") {
        Err(e) => expect(e.contains(NO_ID_ERR_STR)).to_equal(true)?,
        Ok(_) => return Err("admin cannot delete user's clients".to_string()),
    }
    Ok(())
}

pub fn delete_user_invalid_token(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    let req = TestRequest::delete().uri("/auth/api/v1/client/user/id");
    test_invalid_token(runtime, &routes_state, req)
}

pub fn delete_user_invalid_perm(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    let token = get_token(runtime, routes_state, "manager")?;
    let req = TestRequest::delete().uri("/auth/api/v1/client/user/id");
    test_invalid_perm(runtime, &routes_state, token.as_str(), req)?;

    let token = get_token(runtime, routes_state, "public")?;
    let req = TestRequest::delete().uri("/auth/api/v1/client/user/id");
    test_invalid_perm(runtime, &routes_state, token.as_str(), req)?;

    let token = get_token(runtime, routes_state, "service")?;
    let req = TestRequest::delete().uri("/auth/api/v1/client/user/id");
    test_invalid_perm(runtime, &routes_state, token.as_str(), req)?;

    let token = get_token(runtime, routes_state, "user")?;
    let req = TestRequest::delete().uri("/auth/api/v1/client/user/id");
    test_invalid_perm(runtime, &routes_state, token.as_str(), req)
}

fn test_post(
    runtime: &Runtime,
    state: &routes::State,
    token: &str,
    user_id: &str,
    param: &request::PostClient,
    expect_code: &str,
) -> Result<(), String> {
    let mut app = runtime.block_on(async {
        test::init_service(
            App::new()
                .wrap(NormalizePath::trim())
                .service(routes::new_service(state)),
        )
        .await
    });

    let time_before = Utc::now().trunc_subsecs(3);
    let req = TestRequest::post()
        .uri("/auth/api/v1/client")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)))
        .set_json(param)
        .to_request();
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    let time_after = Utc::now().trunc_subsecs(3);
    if resp.status() != StatusCode::OK {
        let status = resp.status();
        let body: ApiError = runtime.block_on(async { test::read_body_json(resp).await });
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
    let body: response::PostClient = runtime.block_on(async { test::read_body_json(resp).await });
    expect(body.data.client_id.len() > 0).to_equal(true)?;

    let client_info = match runtime.block_on(async {
        let cond = QueryCond {
            client_id: Some(body.data.client_id.as_str()),
            ..Default::default()
        };
        state.model.client().get(&cond).await
    }) {
        Err(e) => return Err(format!("get client model error: {}", e)),
        Ok(info) => match info {
            None => return Err("add client then get none client".to_string()),
            Some(info) => info,
        },
    };
    expect(client_info.created_at.ge(&time_before)).to_equal(true)?;
    expect(client_info.created_at.le(&time_after)).to_equal(true)?;
    expect(client_info.modified_at.ge(&time_before)).to_equal(true)?;
    expect(client_info.modified_at.le(&time_after)).to_equal(true)?;
    match param.credentials {
        None => expect(client_info.client_secret.is_none()).to_equal(true)?,
        Some(false) => expect(client_info.client_secret.is_none()).to_equal(true)?,
        Some(true) => {
            expect(client_info.client_secret.is_some()).to_equal(true)?;
            expect(client_info.client_secret.as_ref().unwrap().len() > 0).to_equal(true)?;
        }
    }
    let mut redirect_uris = param.data.redirect_uris.clone();
    redirect_uris.sort();
    redirect_uris.dedup();
    expect(client_info.redirect_uris.clone()).to_equal(redirect_uris)?;
    let mut scopes = param.data.scopes.clone();
    scopes.sort();
    scopes.dedup();
    expect(client_info.scopes.clone()).to_equal(scopes)?;
    match param.data.user_id.as_ref() {
        None => expect(client_info.user_id.as_str()).to_equal(user_id)?,
        Some(user_id_param) => match user_id {
            "admin" => expect(client_info.user_id.as_str()).to_equal(user_id_param)?,
            _ => expect(client_info.user_id.as_str()).to_equal(user_id)?,
        },
    }
    expect(client_info.name.as_str()).to_equal(param.data.name.as_str())?;
    match param.data.image.as_ref() {
        None => expect(client_info.image_url.is_none()).to_equal(true)?,
        Some(image) => {
            expect(client_info.image_url.is_some()).to_equal(true)?;
            expect(client_info.image_url.as_ref().unwrap().as_str()).to_equal(image.as_str())?;
        }
    }
    Ok(())
}

fn test_post_invalid_param(
    runtime: &Runtime,
    state: &routes::State,
    token: &str,
    param: Option<&request::PostClient>,
) -> Result<(), String> {
    let mut app = runtime.block_on(async {
        test::init_service(
            App::new()
                .wrap(NormalizePath::trim())
                .service(routes::new_service(state)),
        )
        .await
    });

    let req = TestRequest::post()
        .uri("/auth/api/v1/client")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)));
    let req = match param {
        None => req.to_request(),
        Some(param) => req.set_json(&param).to_request(),
    };
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    expect(resp.status()).to_equal(StatusCode::BAD_REQUEST)?;
    let body: ApiError = runtime.block_on(async { test::read_body_json(resp).await });
    if body.code.as_str() != err::E_PARAM {
        return Err(format!("unexpected 400 error: {}", body.code.as_str()));
    }
    Ok(())
}

fn test_get_count(
    runtime: &Runtime,
    state: &routes::State,
    token: &str,
    param: Option<&request::GetClientCount>,
    expect_count: usize,
) -> Result<(), String> {
    let mut app = runtime.block_on(async {
        test::init_service(
            App::new()
                .wrap(NormalizePath::trim())
                .service(routes::new_service(state)),
        )
        .await
    });

    let uri = match param {
        None => "/auth/api/v1/client/count".to_string(),
        Some(param) => format!(
            "/auth/api/v1/client/count?{}",
            serde_urlencoded::to_string(&param).unwrap()
        ),
    };
    let req = TestRequest::get()
        .uri(uri.as_str())
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)))
        .to_request();
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    expect(resp.status()).to_equal(StatusCode::OK)?;
    let body: response::GetClientCount =
        runtime.block_on(async { test::read_body_json(resp).await });
    expect(body.data.count).to_equal(expect_count)
}

fn test_get_list(
    runtime: &Runtime,
    state: &routes::State,
    token: &str,
    is_admin: bool,
    param: Option<&request::GetClientList>,
    expect_count: usize,
) -> Result<(), String> {
    let mut app = runtime.block_on(async {
        test::init_service(
            App::new()
                .wrap(NormalizePath::trim())
                .service(routes::new_service(state)),
        )
        .await
    });

    let uri = match param {
        None => "/auth/api/v1/client/list".to_string(),
        Some(param) => format!(
            "/auth/api/v1/client/list?{}",
            serde_urlencoded::to_string(&param).unwrap()
        ),
    };
    let req = TestRequest::get()
        .uri(uri.as_str())
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)))
        .to_request();
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    expect(resp.status()).to_equal(StatusCode::OK)?;
    let body: response::GetClientList =
        runtime.block_on(async { test::read_body_json(resp).await });
    expect(body.data.len()).to_equal(expect_count)?;

    let mut name_min = "";
    for info in body.data.iter() {
        if let Err(_) = expect(info.name.as_str().ge(name_min)).to_equal(true) {
            return Err(format!(
                "name order error: {} - {}",
                name_min,
                info.name.as_str()
            ));
        }
        name_min = info.name.as_str();
        expect(info.user_id.is_some()).to_equal(is_admin)?;
    }
    Ok(())
}

fn test_get_list_sort(
    runtime: &Runtime,
    state: &routes::State,
    token: &str,
    param: &mut request::GetClientList,
    expect_ids: &[&str],
) -> Result<(), String> {
    let mut app = runtime.block_on(async {
        test::init_service(
            App::new()
                .wrap(NormalizePath::trim())
                .service(routes::new_service(state)),
        )
        .await
    });

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

    let uri = format!(
        "/auth/api/v1/client/list?{}",
        serde_urlencoded::to_string(&param).unwrap()
    );
    let req = TestRequest::get()
        .uri(uri.as_str())
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)))
        .to_request();
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    if let Err(_) = expect(resp.status()).to_equal(StatusCode::OK) {
        let body: ApiError = runtime.block_on(async { test::read_body_json(resp).await });
        let message = match body.message.as_ref() {
            None => "",
            Some(message) => message.as_str(),
        };
        return Err(format!(
            "response not 200: {}, {}, {}",
            uri.as_str(),
            body.code,
            message
        ));
    }
    let body: response::GetClientList =
        runtime.block_on(async { test::read_body_json(resp).await });
    expect(body.data.len()).to_equal(expect_ids.len())?;

    let mut index = 0;
    for data in body.data.iter() {
        expect(data.client_id.as_str()).to_equal(expect_ids[index])?;
        index += 1;
    }
    Ok(())
}

fn test_get_list_offset_limit(
    runtime: &Runtime,
    state: &routes::State,
    token: &str,
    param: &request::GetClientList,
    expect_ids: Vec<i32>,
) -> Result<(), String> {
    let mut app = runtime.block_on(async {
        test::init_service(
            App::new()
                .wrap(NormalizePath::trim())
                .service(routes::new_service(state)),
        )
        .await
    });

    let uri = format!(
        "/auth/api/v1/client/list?{}",
        serde_urlencoded::to_string(&param).unwrap()
    );
    let req = TestRequest::get()
        .uri(uri.as_str())
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)))
        .to_request();
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    expect(resp.status()).to_equal(StatusCode::OK)?;
    let body: response::GetClientList =
        runtime.block_on(async { test::read_body_json(resp).await });
    expect(body.data.len()).to_equal(expect_ids.len())?;

    let mut index = 0;
    for data in body.data.iter() {
        expect(data.client_id.as_str())
            .to_equal(format!("client_{}", expect_ids[index]).as_str())?;
        index += 1;
    }
    Ok(())
}

fn test_get_list_format_array(
    runtime: &Runtime,
    state: &routes::State,
    token: &str,
    param: &request::GetClientList,
    expect_ids: Vec<i32>,
) -> Result<(), String> {
    let mut app = runtime.block_on(async {
        test::init_service(
            App::new()
                .wrap(NormalizePath::trim())
                .service(routes::new_service(state)),
        )
        .await
    });

    let uri = format!(
        "/auth/api/v1/client/list?{}",
        serde_urlencoded::to_string(&param).unwrap()
    );
    let req = TestRequest::get()
        .uri(uri.as_str())
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)))
        .to_request();
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    expect(resp.status()).to_equal(StatusCode::OK)?;
    let body: Vec<response::GetClientListData> =
        runtime.block_on(async { test::read_body_json(resp).await });
    expect(body.len()).to_equal(expect_ids.len())?;

    let mut index = 0;
    for data in body.iter() {
        expect(data.client_id.as_str())
            .to_equal(format!("client_{}", expect_ids[index]).as_str())?;
        index += 1;
    }
    Ok(())
}

fn test_get(
    runtime: &Runtime,
    state: &routes::State,
    token: &str,
    is_admin: bool,
    client_id: &str,
) -> Result<(), String> {
    let mut app = runtime.block_on(async {
        test::init_service(
            App::new()
                .wrap(NormalizePath::trim())
                .service(routes::new_service(state)),
        )
        .await
    });

    let client_info = get_client_model(runtime, state, client_id)?;

    let uri = format!("/auth/api/v1/client/{}", client_id);
    let req = TestRequest::get()
        .uri(uri.as_str())
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)))
        .to_request();
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    expect(resp.status()).to_equal(StatusCode::OK)?;
    let body: response::GetClient = runtime.block_on(async { test::read_body_json(resp).await });
    expect(body.data.client_id.as_str()).to_equal(client_info.client_id.as_str())?;
    expect(
        DateTime::parse_from_rfc3339(body.data.created_at.as_str())
            .unwrap()
            .timestamp_millis(),
    )
    .to_equal(client_info.created_at.timestamp_millis())?;
    expect(
        DateTime::parse_from_rfc3339(body.data.modified_at.as_str())
            .unwrap()
            .timestamp_millis(),
    )
    .to_equal(client_info.modified_at.timestamp_millis())?;
    match client_info.client_secret.as_ref() {
        None => expect(body.data.client_secret.is_none()).to_equal(true)?,
        Some(secret) => {
            expect(body.data.client_secret.is_some()).to_equal(true)?;
            expect(body.data.client_secret.as_ref().unwrap().as_str()).to_equal(secret.as_str())?;
        }
    }
    expect(body.data.redirect_uris.clone()).to_equal(client_info.redirect_uris.clone())?;
    expect(body.data.scopes.clone()).to_equal(client_info.scopes.clone())?;
    match is_admin {
        false => expect(body.data.user_id.is_none()).to_equal(true)?,
        true => {
            expect(body.data.user_id.is_some()).to_equal(true)?;
            expect(body.data.user_id.as_ref().unwrap().as_str())
                .to_equal(client_info.user_id.as_str())?;
        }
    }
    expect(body.data.name.as_str()).to_equal(client_info.name.as_str())?;
    match client_info.image_url.as_ref() {
        None => expect(body.data.image.is_none()).to_equal(true)?,
        Some(image) => {
            expect(body.data.image.is_some()).to_equal(true)?;
            expect(body.data.image.as_ref().unwrap().as_str()).to_equal(image.as_str())?;
        }
    }
    Ok(())
}

fn test_patch(
    runtime: &Runtime,
    state: &routes::State,
    token: &str,
    client_id: &str,
    user_id: &str,
    use_secret: bool,
) -> Result<(), String> {
    let secret = match use_secret {
        false => None,
        true => Some("secret".to_string()),
    };
    add_client_model(runtime, state, client_id, user_id, secret, None)?;

    let mut app = runtime.block_on(async {
        test::init_service(
            App::new()
                .wrap(NormalizePath::trim())
                .service(routes::new_service(state)),
        )
        .await
    });

    let time_before = Utc::now().trunc_subsecs(3);
    let mut redirect_uris = vec![
        "http://uri2".to_string(),
        "http://uri1".to_string(),
        "http://uri2".to_string(),
    ];
    let mut scopes = vec![
        "scope2".to_string(),
        "scope1".to_string(),
        "scope2".to_string(),
    ];
    let body = request::PatchClient {
        data: Some(request::PatchClientData {
            redirect_uris: Some(redirect_uris.clone()),
            scopes: Some(scopes.clone()),
            name: Some("name changes".to_string()),
            image: Some(Some("image url".to_string())),
        }),
        regen_secret: match use_secret {
            false => None,
            true => Some(true),
        },
    };
    let req = TestRequest::patch()
        .uri(format!("/auth/api/v1/client/{}", client_id).as_str())
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)))
        .set_json(&body)
        .to_request();
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    expect(resp.status()).to_equal(StatusCode::NO_CONTENT)?;

    let time_after = Utc::now().trunc_subsecs(3);
    let client_info = get_client_model(runtime, state, client_id)?;
    expect(client_info.modified_at.ge(&time_before)).to_equal(true)?;
    expect(client_info.modified_at.le(&time_after)).to_equal(true)?;
    redirect_uris.sort();
    redirect_uris.dedup();
    expect(client_info.redirect_uris.clone()).to_equal(redirect_uris)?;
    scopes.sort();
    scopes.dedup();
    expect(client_info.scopes.clone()).to_equal(scopes)?;
    expect(client_info.name.as_str()).to_equal("name changes")?;
    expect(client_info.image_url.is_some()).to_equal(true)?;
    expect(client_info.image_url.as_ref().unwrap().as_str()).to_equal("image url")?;
    if use_secret {
        expect(client_info.client_secret.is_some()).to_equal(true)?;
        expect(client_info.client_secret.as_ref().unwrap().as_str()).to_not_equal("secret")?;
    }

    let body = request::PatchClient {
        data: Some(request::PatchClientData {
            redirect_uris: Some(vec![]),
            scopes: Some(vec![]),
            ..Default::default()
        }),
        ..Default::default()
    };
    let req = TestRequest::patch()
        .uri(format!("/auth/api/v1/client/{}", client_id).as_str())
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)))
        .set_json(&body)
        .to_request();
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    expect(resp.status()).to_equal(StatusCode::NO_CONTENT)?;

    let client_info = get_client_model(runtime, state, client_id)?;
    expect(client_info.redirect_uris.len()).to_equal(0)?;
    expect(client_info.scopes.len()).to_equal(0)?;
    expect(client_info.image_url.is_some()).to_equal(true)?;
    expect(client_info.image_url.as_ref().unwrap().as_str()).to_equal("image url")?;

    let body = request::PatchClient {
        data: Some(request::PatchClientData {
            image: Some(None),
            ..Default::default()
        }),
        ..Default::default()
    };
    let req = TestRequest::patch()
        .uri(format!("/auth/api/v1/client/{}", client_id).as_str())
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)))
        .set_json(&body)
        .to_request();
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    expect(resp.status()).to_equal(StatusCode::NO_CONTENT)?;

    let client_info = get_client_model(runtime, state, client_id)?;
    if use_secret {
        expect(client_info.client_secret.is_some()).to_equal(true)?;
    }
    expect(client_info.image_url.is_none()).to_equal(true)
}

fn test_patch_invalid_param(
    runtime: &Runtime,
    state: &routes::State,
    token: &str,
    client_id: &str,
    param: Option<&request::PatchClient>,
) -> Result<(), String> {
    let mut app = runtime.block_on(async {
        test::init_service(
            App::new()
                .wrap(NormalizePath::trim())
                .service(routes::new_service(state)),
        )
        .await
    });

    let req = TestRequest::patch()
        .uri(format!("/auth/api/v1/client/{}", client_id).as_str())
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)));
    let req = match param {
        None => req.to_request(),
        Some(param) => req.set_json(&param).to_request(),
    };
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    expect(resp.status()).to_equal(StatusCode::BAD_REQUEST)?;
    let body: ApiError = runtime.block_on(async { test::read_body_json(resp).await });
    if body.code.as_str() != err::E_PARAM {
        return Err(format!("unexpected 400 error: {}", body.code.as_str()));
    }
    Ok(())
}

fn get_client_model(
    runtime: &Runtime,
    state: &routes::State,
    client_id: &str,
) -> Result<Client, String> {
    match runtime.block_on(async {
        let cond = QueryCond {
            client_id: Some(client_id),
            ..Default::default()
        };
        state.model.client().get(&cond).await
    }) {
        Err(e) => return Err(format!("get client model info error: {}", e)),
        Ok(client) => match client {
            None => return Err(format!("{} {}", NO_ID_ERR_STR, client_id)),
            Some(client) => return Ok(client),
        },
    }
}

fn add_client_model(
    runtime: &Runtime,
    state: &routes::State,
    client_id: &str,
    user_id: &str,
    secret: Option<String>,
    image: Option<String>,
) -> Result<(), String> {
    match runtime.block_on(async {
        let mut client = create_client(client_id, user_id, secret);
        if let Some(image) = image.as_ref() {
            client.image_url = Some(image.to_string());
        }
        state.model.client().add(&client).await
    }) {
        Err(e) => Err(format!("add client model info error: {}", e)),
        Ok(_) => Ok(()),
    }
}

/// Returns (total_clients, user_client) tuple.
fn count_list_dataset(runtime: &Runtime, state: &routes::State) -> Result<(usize, usize), String> {
    let now = Utc::now();

    let mut client = create_client("client_admin1", "admin", None);
    client.created_at = now;
    client.modified_at = now + TimeDelta::try_milliseconds(3).unwrap();
    runtime.block_on(async {
        if let Err(e) = state.model.client().add(&client).await {
            return Err(format!("add client {} error: {}", client.client_id, e));
        }
        Ok(())
    })?;

    client.client_id = "client_admin2".to_string();
    client.created_at = now + TimeDelta::try_milliseconds(1).unwrap();
    client.modified_at = now + TimeDelta::try_milliseconds(2).unwrap();
    client.name = "client_admin2".to_string();
    runtime.block_on(async {
        if let Err(e) = state.model.client().add(&client).await {
            return Err(format!("add client {} error: {}", client.client_id, e));
        }
        Ok(())
    })?;

    client.client_id = "client_user1".to_string();
    client.created_at = now + TimeDelta::try_milliseconds(2).unwrap();
    client.modified_at = now + TimeDelta::try_milliseconds(1).unwrap();
    client.user_id = "user".to_string();
    client.name = "client_user2".to_string();
    runtime.block_on(async {
        if let Err(e) = state.model.client().add(&client).await {
            return Err(format!("add client {} error: {}", client.client_id, e));
        }
        Ok(())
    })?;

    client.client_id = "client_user2".to_string();
    client.created_at = now + TimeDelta::try_milliseconds(3).unwrap();
    client.modified_at = now;
    client.client_secret = Some("secret".to_string());
    client.name = "client_user1".to_string();
    runtime.block_on(async {
        if let Err(e) = state.model.client().add(&client).await {
            return Err(format!("add client {} error: {}", client.client_id, e));
        }
        Ok(())
    })?;

    Ok((5, 2))
}
