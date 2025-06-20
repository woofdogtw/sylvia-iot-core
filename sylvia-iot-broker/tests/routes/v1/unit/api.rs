use axum::{
    Router,
    http::{HeaderValue, Method, StatusCode, header},
};
use axum_test::TestServer;
use chrono::{DateTime, SubsecRound, TimeDelta, Utc};
use laboratory::{SpecContext, expect};
use serde_json::{Map, Value};
use tokio::runtime::Runtime;

use sylvia_iot_broker::{models::unit::QueryCond, routes};
use sylvia_iot_corelib::err;

use super::{
    super::{
        super::libs::{
            ApiError, add_delete_rsc, add_unit_model, create_unit, get_application_model,
            get_device_model, get_device_route_model, get_dldata_buffer_model, get_network_model,
            get_network_route_model, get_unit_model, test_get_400, test_invalid_perm,
            test_invalid_token,
        },
        STATE, TOKEN_MANAGER, TOKEN_MEMBER, TOKEN_OWNER, TestState, libs,
    },
    request, response,
};

pub fn post(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    let mut info = Map::<String, Value>::new();
    info.insert("info".to_string(), Value::String("value".to_string()));
    let param = request::PostUnit {
        data: request::PostUnitData {
            code: "manager-empty".to_string(),
            owner_id: None,
            name: Some("manager-empty-name".to_string()),
            info: Some(info),
        },
    };
    test_post(runtime, routes_state, TOKEN_MANAGER, &param, "", "manager")?;

    let param = request::PostUnit {
        data: request::PostUnitData {
            code: "manager-self".to_string(),
            owner_id: Some("manager".to_string()),
            name: None,
            info: None,
        },
    };
    test_post(runtime, routes_state, TOKEN_MANAGER, &param, "", "manager")?;

    let param = request::PostUnit {
        data: request::PostUnitData {
            code: "manager-member".to_string(),
            owner_id: Some("member".to_string()),
            name: None,
            info: None,
        },
    };
    test_post(runtime, routes_state, TOKEN_MANAGER, &param, "", "member")?;

    let param = request::PostUnit {
        data: request::PostUnitData {
            code: "owner-empty".to_string(),
            owner_id: None,
            name: None,
            info: None,
        },
    };
    test_post(runtime, routes_state, TOKEN_OWNER, &param, "", "owner")?;

    let param = request::PostUnit {
        data: request::PostUnitData {
            code: "owner-member".to_string(),
            owner_id: Some("member".to_string()),
            name: None,
            info: None,
        },
    };
    test_post(runtime, routes_state, TOKEN_OWNER, &param, "", "owner")?;

    Ok(())
}

pub fn post_dup(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    let param = request::PostUnit {
        data: request::PostUnitData {
            code: "manager".to_string(),
            owner_id: None,
            name: None,
            info: None,
        },
    };
    test_post(runtime, routes_state, TOKEN_MANAGER, &param, "", "manager")?;

    let param = request::PostUnit {
        data: request::PostUnitData {
            code: "manager".to_string(),
            owner_id: None,
            name: None,
            info: None,
        },
    };
    test_post(
        runtime,
        routes_state,
        TOKEN_MANAGER,
        &param,
        "err_broker_unit_exist",
        "",
    )
}

pub fn post_not_exist_user(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    let param = request::PostUnit {
        data: request::PostUnitData {
            code: "manager".to_string(),
            owner_id: Some("not-exist-user".to_string()),
            name: None,
            info: None,
        },
    };
    test_post(
        runtime,
        routes_state,
        TOKEN_MANAGER,
        &param,
        "err_broker_owner_not_exist",
        "",
    )
}

pub fn post_invalid_param(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    test_post_invalid_param(runtime, &routes_state, TOKEN_OWNER, None)?;

    let param = request::PostUnit {
        data: request::PostUnitData {
            code: "".to_string(),
            owner_id: None,
            name: None,
            info: None,
        },
    };
    test_post_invalid_param(runtime, &routes_state, TOKEN_MANAGER, Some(&param))?;

    let param = request::PostUnit {
        data: request::PostUnitData {
            code: "code".to_string(),
            owner_id: Some("".to_string()),
            name: None,
            info: None,
        },
    };
    test_post_invalid_param(runtime, &routes_state, TOKEN_MANAGER, Some(&param))?;

    let mut info = Map::<String, Value>::new();
    info.insert("".to_string(), Value::String("value".to_string()));
    let param = request::PostUnit {
        data: request::PostUnitData {
            code: "code".to_string(),
            owner_id: Some("owner".to_string()),
            name: None,
            info: Some(info),
        },
    };
    test_post_invalid_param(runtime, &routes_state, TOKEN_MANAGER, Some(&param))
}

pub fn post_invalid_token(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    test_invalid_token(runtime, &routes_state, Method::POST, "/broker/api/v1/unit")
}

pub fn get_count(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    let data_size = count_list_dataset(runtime, &routes_state)?;

    test_get_count(runtime, &routes_state, TOKEN_MANAGER, None, data_size.0)?;
    test_get_count(runtime, &routes_state, TOKEN_OWNER, None, data_size.1)?;
    test_get_count(runtime, &routes_state, TOKEN_MEMBER, None, data_size.2)?;

    let param = request::GetUnitCount {
        owner: Some("owner".to_string()),
        ..Default::default()
    };
    test_get_count(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        Some(&param),
        data_size.1,
    )?;
    test_get_count(
        runtime,
        &routes_state,
        TOKEN_OWNER,
        Some(&param),
        data_size.1,
    )?;
    test_get_count(
        runtime,
        &routes_state,
        TOKEN_MEMBER,
        Some(&param),
        data_size.2,
    )?;

    let param = request::GetUnitCount {
        member: Some("member".to_string()),
        ..Default::default()
    };
    test_get_count(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        Some(&param),
        data_size.2,
    )?;
    test_get_count(
        runtime,
        &routes_state,
        TOKEN_OWNER,
        Some(&param),
        data_size.1,
    )?;
    test_get_count(
        runtime,
        &routes_state,
        TOKEN_MEMBER,
        Some(&param),
        data_size.2,
    )?;

    let param = request::GetUnitCount {
        member: Some("not-exist".to_string()),
        ..Default::default()
    };
    test_get_count(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 0)?;
    test_get_count(
        runtime,
        &routes_state,
        TOKEN_OWNER,
        Some(&param),
        data_size.1,
    )?;
    test_get_count(
        runtime,
        &routes_state,
        TOKEN_MEMBER,
        Some(&param),
        data_size.2,
    )?;

    let param = request::GetUnitCount {
        contains: Some("M".to_string()),
        ..Default::default()
    };
    test_get_count(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 2)?;
    test_get_count(runtime, &routes_state, TOKEN_OWNER, Some(&param), 0)?;
    test_get_count(runtime, &routes_state, TOKEN_MEMBER, Some(&param), 0)
}

pub fn get_count_invalid_token(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    test_invalid_token(
        runtime,
        &routes_state,
        Method::GET,
        "/broker/api/v1/unit/count",
    )
}

pub fn get_list(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    let data_size = count_list_dataset(runtime, &routes_state)?;

    test_get_list(runtime, &routes_state, TOKEN_MANAGER, None, data_size.0)?;
    test_get_list(runtime, &routes_state, TOKEN_OWNER, None, data_size.1)?;
    test_get_list(runtime, &routes_state, TOKEN_MEMBER, None, data_size.2)?;

    let param = request::GetUnitList {
        owner: Some("owner".to_string()),
        ..Default::default()
    };
    test_get_list(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        Some(&param),
        data_size.1,
    )?;
    test_get_list(
        runtime,
        &routes_state,
        TOKEN_OWNER,
        Some(&param),
        data_size.1,
    )?;
    test_get_list(
        runtime,
        &routes_state,
        TOKEN_MEMBER,
        Some(&param),
        data_size.2,
    )?;

    let param = request::GetUnitList {
        member: Some("member".to_string()),
        ..Default::default()
    };
    test_get_list(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        Some(&param),
        data_size.2,
    )?;
    test_get_list(
        runtime,
        &routes_state,
        TOKEN_OWNER,
        Some(&param),
        data_size.1,
    )?;
    test_get_list(
        runtime,
        &routes_state,
        TOKEN_MEMBER,
        Some(&param),
        data_size.2,
    )?;

    let param = request::GetUnitList {
        member: Some("not-exist".to_string()),
        ..Default::default()
    };
    test_get_list(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 0)?;
    test_get_list(
        runtime,
        &routes_state,
        TOKEN_OWNER,
        Some(&param),
        data_size.1,
    )?;
    test_get_list(
        runtime,
        &routes_state,
        TOKEN_MEMBER,
        Some(&param),
        data_size.2,
    )?;

    let param = request::GetUnitList {
        contains: Some("M".to_string()),
        ..Default::default()
    };
    test_get_list(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 2)?;
    test_get_list(runtime, &routes_state, TOKEN_OWNER, Some(&param), 0)?;
    test_get_list(runtime, &routes_state, TOKEN_MEMBER, Some(&param), 0)
}

pub fn get_list_sort(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    count_list_dataset(runtime, &routes_state)?;

    let mut param = request::GetUnitList {
        ..Default::default()
    };
    test_get_list_sort(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        &mut param,
        &["manager1", "manager2", "owner1", "owner2", "test"],
    )?;

    param.sort_vec = Some(vec![("code", true)]);
    test_get_list_sort(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        &mut param,
        &["manager1", "manager2", "owner1", "owner2", "test"],
    )?;
    param.sort_vec = Some(vec![("code", false)]);
    test_get_list_sort(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        &mut param,
        &["test", "owner2", "owner1", "manager2", "manager1"],
    )?;

    param.sort_vec = Some(vec![("name", true)]);
    test_get_list_sort(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        &mut param,
        &["manager1", "manager2", "owner2", "owner1", "test"],
    )?;
    param.sort_vec = Some(vec![("name", false)]);
    test_get_list_sort(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        &mut param,
        &["test", "owner1", "owner2", "manager2", "manager1"],
    )?;

    param.sort_vec = Some(vec![("created", true)]);
    test_get_list_sort(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        &mut param,
        &["test", "manager1", "manager2", "owner1", "owner2"],
    )?;
    param.sort_vec = Some(vec![("created", false)]);
    test_get_list_sort(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        &mut param,
        &["owner2", "owner1", "manager2", "manager1", "test"],
    )?;

    param.sort_vec = Some(vec![("modified", true)]);
    test_get_list_sort(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        &mut param,
        &["test", "owner2", "owner1", "manager2", "manager1"],
    )?;
    param.sort_vec = Some(vec![("modified", false)]);
    test_get_list_sort(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        &mut param,
        &["manager1", "manager2", "owner1", "owner2", "test"],
    )
}

pub fn get_list_offset_limit(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    let user_id = "manager";

    for i in 100..302 {
        add_unit_model(
            runtime,
            &routes_state,
            format!("unit_{}", i).as_str(),
            vec![],
            user_id,
        )?;
    }

    let mut param = request::GetUnitList {
        owner: Some(user_id.to_string()),
        member: Some(user_id.to_string()),
        contains: Some("U".to_string()),
        ..Default::default()
    };
    test_get_list_offset_limit(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        &param,
        (100..200).collect(),
    )?;

    param.limit = Some(0);
    test_get_list_offset_limit(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        &param,
        (100..302).collect(),
    )?;

    param.offset = Some(0);
    param.limit = Some(5);
    test_get_list_offset_limit(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        &param,
        (100..105).collect(),
    )?;

    param.offset = Some(5);
    param.limit = Some(0);
    test_get_list_offset_limit(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        &param,
        (105..302).collect(),
    )?;

    param.offset = Some(198);
    param.limit = Some(50);
    test_get_list_offset_limit(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        &param,
        (298..302).collect(),
    )?;

    param.offset = Some(2);
    param.limit = Some(105);
    test_get_list_offset_limit(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        &param,
        (102..207).collect(),
    )?;

    param.offset = Some(2);
    param.limit = None;
    test_get_list_offset_limit(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        &param,
        (102..202).collect(),
    )
}

pub fn get_list_format_array(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    let user_id = "manager";

    for i in 100..302 {
        add_unit_model(
            runtime,
            &routes_state,
            format!("unit_{}", i).as_str(),
            vec![],
            user_id,
        )?;
    }

    let mut param = request::GetUnitList {
        limit: Some(5),
        format: Some("array".to_string()),
        ..Default::default()
    };
    test_get_list_format_array(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        &param,
        (100..105).collect(),
    )?;

    param.offset = Some(2);
    param.limit = Some(105);
    test_get_list_format_array(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        &param,
        (102..207).collect(),
    )
}

pub fn get_list_invalid_param(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    let uri = "/broker/api/v1/unit/list";
    let code = err::E_PARAM;

    let mut query = Map::<String, Value>::new();
    query.insert("offset".to_string(), Value::Number((-1).into()));
    test_get_400(runtime, &routes_state, TOKEN_MANAGER, uri, &query, code)?;

    let mut query = Map::<String, Value>::new();
    query.insert("sort".to_string(), Value::String("".to_string()));
    test_get_400(runtime, &routes_state, TOKEN_MANAGER, uri, &query, code)?;

    let mut query = Map::<String, Value>::new();
    query.insert("sort".to_string(), Value::String("created".to_string()));
    test_get_400(runtime, &routes_state, TOKEN_MANAGER, uri, &query, code)?;

    let mut query = Map::<String, Value>::new();
    query.insert(
        "sort".to_string(),
        Value::String("created:asc:c".to_string()),
    );
    test_get_400(runtime, &routes_state, TOKEN_MANAGER, uri, &query, code)?;

    let mut query = Map::<String, Value>::new();
    query.insert(
        "sort".to_string(),
        Value::String("created:asc,name:true".to_string()),
    );
    test_get_400(runtime, &routes_state, TOKEN_MANAGER, uri, &query, code)?;

    let mut query = Map::<String, Value>::new();
    query.insert("sort".to_string(), Value::String("a:asc".to_string()));
    test_get_400(runtime, &routes_state, TOKEN_MANAGER, uri, &query, code)
}

pub fn get_list_invalid_token(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    test_invalid_token(
        runtime,
        &routes_state,
        Method::GET,
        "/broker/api/v1/unit/list",
    )
}

pub fn get(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    add_unit_model(runtime, routes_state, "manager", vec![], "manager")?;
    add_unit_model(runtime, routes_state, "owner", vec![], "owner")?;

    test_get(runtime, routes_state, TOKEN_MANAGER, "manager")?;
    test_get(runtime, routes_state, TOKEN_MANAGER, "owner")?;
    test_get(runtime, routes_state, TOKEN_OWNER, "owner")
}

pub fn get_wrong_id(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    add_unit_model(runtime, routes_state, "manager", vec![], "manager")?;
    add_unit_model(runtime, routes_state, "owner", vec![], "owner")?;

    test_get_wrong_id(runtime, routes_state, TOKEN_MANAGER, "manager1")?;
    test_get_wrong_id(runtime, routes_state, TOKEN_OWNER, "manager")?;
    test_get_wrong_id(runtime, routes_state, TOKEN_MEMBER, "manager")?;
    test_get_wrong_id(runtime, routes_state, TOKEN_MEMBER, "owner")
}

pub fn get_invalid_token(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    test_invalid_token(
        runtime,
        &routes_state,
        Method::GET,
        "/broker/api/v1/unit/id",
    )
}

pub fn patch(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    test_patch(
        runtime,
        routes_state,
        TOKEN_MANAGER,
        "manager",
        "manager",
        "owner",
        vec!["owner".to_string(), "manager".to_string()],
        true,
    )?;

    test_patch(
        runtime,
        routes_state,
        TOKEN_OWNER,
        "owner",
        "owner",
        "manager",
        vec!["owner".to_string(), "manager".to_string()],
        false,
    )
}

pub fn patch_wrong_id(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    add_unit_model(runtime, routes_state, "manager", vec![], "manager")?;

    test_patch_wrong_id(runtime, routes_state, TOKEN_MANAGER, "manager1")?;
    test_patch_wrong_id(runtime, routes_state, TOKEN_OWNER, "manager")
}

pub fn patch_not_exist_user(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    add_unit_model(runtime, routes_state, "manager", vec![], "manager")?;

    let app = Router::new().merge(routes::new_service(routes_state));
    let server = match TestServer::new(app) {
        Err(e) => return Err(format!("new server error: {}", e)),
        Ok(server) => server,
    };

    let body = request::PatchUnit {
        data: request::PatchUnitData {
            owner_id: Some("not-exist".to_string()),
            ..Default::default()
        },
    };
    let req = server
        .patch(format!("/broker/api/v1/unit/manager").as_str())
        .add_header(
            header::AUTHORIZATION,
            HeaderValue::from_str(format!("Bearer {}", TOKEN_MANAGER).as_str()).unwrap(),
        )
        .json(&body);
    let resp = runtime.block_on(async { req.await });
    expect(resp.status_code()).to_equal(StatusCode::BAD_REQUEST)?;
    let body: ApiError = resp.json();
    expect(body.code.as_str()).to_equal("err_broker_owner_not_exist")?;

    let body = request::PatchUnit {
        data: request::PatchUnitData {
            member_ids: Some(vec!["not-exist".to_string()]),
            ..Default::default()
        },
    };
    let req = server
        .patch(format!("/broker/api/v1/unit/manager").as_str())
        .add_header(
            header::AUTHORIZATION,
            HeaderValue::from_str(format!("Bearer {}", TOKEN_MANAGER).as_str()).unwrap(),
        )
        .json(&body);
    let resp = runtime.block_on(async { req.await });
    expect(resp.status_code()).to_equal(StatusCode::BAD_REQUEST)?;
    let body: ApiError = resp.json();
    expect(body.code.as_str()).to_equal("err_broker_member_not_exist")
}

pub fn patch_invalid_param(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    add_unit_model(runtime, routes_state, "manager", vec![], "manager")?;

    test_patch_invalid_param(runtime, routes_state, TOKEN_MANAGER, "manager", None)?;

    let param = request::PatchUnit {
        ..Default::default()
    };
    test_patch_invalid_param(
        runtime,
        routes_state,
        TOKEN_MANAGER,
        "manager",
        Some(&param),
    )?;

    let param = request::PatchUnit {
        data: request::PatchUnitData {
            ..Default::default()
        },
    };
    test_patch_invalid_param(
        runtime,
        routes_state,
        TOKEN_MANAGER,
        "manager",
        Some(&param),
    )?;

    let param = request::PatchUnit {
        data: request::PatchUnitData {
            owner_id: Some("".to_string()),
            ..Default::default()
        },
    };
    test_patch_invalid_param(
        runtime,
        routes_state,
        TOKEN_MANAGER,
        "manager",
        Some(&param),
    )?;

    let param = request::PatchUnit {
        data: request::PatchUnitData {
            member_ids: Some(vec!["".to_string()]),
            ..Default::default()
        },
    };
    test_patch_invalid_param(
        runtime,
        routes_state,
        TOKEN_MANAGER,
        "manager",
        Some(&param),
    )?;

    let mut info = Map::<String, Value>::new();
    info.insert("".to_string(), Value::String("value".to_string()));
    let param = request::PatchUnit {
        data: request::PatchUnitData {
            info: Some(info),
            ..Default::default()
        },
    };
    test_patch_invalid_param(
        runtime,
        routes_state,
        TOKEN_MANAGER,
        "manager",
        Some(&param),
    )
}

pub fn patch_invalid_token(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    test_invalid_token(
        runtime,
        &routes_state,
        Method::PATCH,
        "/broker/api/v1/unit/id",
    )
}

pub fn delete(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    add_unit_model(runtime, routes_state, "manager", vec![], "manager")?;
    add_unit_model(runtime, routes_state, "owner", vec![], "owner")?;
    add_unit_model(runtime, routes_state, "member", vec![], "member")?;

    let app = Router::new().merge(routes::new_service(routes_state));
    let server = match TestServer::new(app) {
        Err(e) => return Err(format!("new server error: {}", e)),
        Ok(server) => server,
    };

    let req = server.delete("/broker/api/v1/unit/id").add_header(
        header::AUTHORIZATION,
        HeaderValue::from_str(format!("Bearer {}", TOKEN_MANAGER).as_str()).unwrap(),
    );
    let resp = runtime.block_on(async { req.await });
    expect(resp.status_code()).to_equal(StatusCode::NO_CONTENT)?;
    let _ = get_unit_model(runtime, &routes_state, "manager", true)?;

    let req = server.delete("/broker/api/v1/unit/manager").add_header(
        header::AUTHORIZATION,
        HeaderValue::from_str(format!("Bearer {}", TOKEN_OWNER).as_str()).unwrap(),
    );
    let resp = runtime.block_on(async { req.await });
    expect(resp.status_code()).to_equal(StatusCode::NO_CONTENT)?;
    let _ = get_unit_model(runtime, &routes_state, "manager", true)?;

    let req = server.delete("/broker/api/v1/unit/owner").add_header(
        header::AUTHORIZATION,
        HeaderValue::from_str(format!("Bearer {}", TOKEN_MEMBER).as_str()).unwrap(),
    );
    let resp = runtime.block_on(async { req.await });
    expect(resp.status_code()).to_equal(StatusCode::NO_CONTENT)?;
    let _ = get_unit_model(runtime, &routes_state, "owner", true)?;

    let req = server.delete("/broker/api/v1/unit/manager").add_header(
        header::AUTHORIZATION,
        HeaderValue::from_str(format!("Bearer {}", TOKEN_MANAGER).as_str()).unwrap(),
    );
    let resp = runtime.block_on(async { req.await });
    expect(resp.status_code()).to_equal(StatusCode::NO_CONTENT)?;
    let _ = get_unit_model(runtime, &routes_state, "manager", false)?;

    let req = server.delete("/broker/api/v1/unit/owner").add_header(
        header::AUTHORIZATION,
        HeaderValue::from_str(format!("Bearer {}", TOKEN_OWNER).as_str()).unwrap(),
    );
    let resp = runtime.block_on(async { req.await });
    expect(resp.status_code()).to_equal(StatusCode::NO_CONTENT)?;
    let _ = get_unit_model(runtime, &routes_state, "owner", false)?;

    let req = server.delete("/broker/api/v1/unit/member").add_header(
        header::AUTHORIZATION,
        HeaderValue::from_str(format!("Bearer {}", TOKEN_MANAGER).as_str()).unwrap(),
    );
    let resp = runtime.block_on(async { req.await });
    expect(resp.status_code()).to_equal(StatusCode::NO_CONTENT)?;
    let _ = get_unit_model(runtime, &routes_state, "member", false)?;

    libs::clear_all_data(runtime, state);
    add_delete_rsc(runtime, routes_state)?;
    let req = server.delete("/broker/api/v1/unit/manager").add_header(
        header::AUTHORIZATION,
        HeaderValue::from_str(format!("Bearer {}", TOKEN_MANAGER).as_str()).unwrap(),
    );
    let resp = runtime.block_on(async { req.await });
    expect(resp.status_code()).to_equal(StatusCode::NO_CONTENT)?;
    let _ = get_unit_model(runtime, routes_state, "manager", false)?;
    let _ = get_unit_model(runtime, routes_state, "owner", true)?;
    let _ = get_application_model(runtime, routes_state, "manager", false)?;
    let _ = get_application_model(runtime, routes_state, "owner", true)?;
    let _ = get_network_model(runtime, routes_state, "public", true)?;
    let _ = get_network_model(runtime, routes_state, "manager", false)?;
    let _ = get_network_model(runtime, routes_state, "owner", true)?;
    let _ = get_device_model(runtime, routes_state, "manager-public", false)?;
    let _ = get_device_model(runtime, routes_state, "manager", false)?;
    let _ = get_device_model(runtime, routes_state, "owner-public", true)?;
    let _ = get_device_model(runtime, routes_state, "owner1", true)?;
    let _ = get_device_model(runtime, routes_state, "owner2", true)?;
    let _ = get_network_route_model(runtime, routes_state, "public-manager", false)?;
    let _ = get_network_route_model(runtime, routes_state, "manager-manager", false)?;
    let _ = get_network_route_model(runtime, routes_state, "owner-owner", true)?;
    let _ = get_device_route_model(runtime, routes_state, "manager-public-manager", false)?;
    let _ = get_device_route_model(runtime, routes_state, "manager-manager", false)?;
    let _ = get_device_route_model(runtime, routes_state, "owner-public-owner", true)?;
    let _ = get_device_route_model(runtime, routes_state, "owner1-owner", true)?;
    let _ = get_device_route_model(runtime, routes_state, "owner2-owner", true)?;
    let _ = get_dldata_buffer_model(runtime, routes_state, "manager-public-manager", false)?;
    let _ = get_dldata_buffer_model(runtime, routes_state, "manager-manager", false)?;
    let _ = get_dldata_buffer_model(runtime, routes_state, "owner-public-owner", true)?;
    let _ = get_dldata_buffer_model(runtime, routes_state, "owner1-owner", true)?;
    let _ = get_dldata_buffer_model(runtime, routes_state, "owner2-owner", true)?;

    libs::clear_all_data(runtime, state);
    add_delete_rsc(runtime, routes_state)?;
    let req = server.delete("/broker/api/v1/unit/owner").add_header(
        header::AUTHORIZATION,
        HeaderValue::from_str(format!("Bearer {}", TOKEN_OWNER).as_str()).unwrap(),
    );
    let resp = runtime.block_on(async { req.await });
    expect(resp.status_code()).to_equal(StatusCode::NO_CONTENT)?;
    let _ = get_unit_model(runtime, routes_state, "manager", true)?;
    let _ = get_unit_model(runtime, routes_state, "owner", false)?;
    let _ = get_application_model(runtime, routes_state, "manager", true)?;
    let _ = get_application_model(runtime, routes_state, "owner", false)?;
    let _ = get_network_model(runtime, routes_state, "public", true)?;
    let _ = get_network_model(runtime, routes_state, "manager", true)?;
    let _ = get_network_model(runtime, routes_state, "owner", false)?;
    let _ = get_device_model(runtime, routes_state, "manager-public", true)?;
    let _ = get_device_model(runtime, routes_state, "manager", true)?;
    let _ = get_device_model(runtime, routes_state, "owner-public", false)?;
    let _ = get_device_model(runtime, routes_state, "owner1", false)?;
    let _ = get_device_model(runtime, routes_state, "owner2", false)?;
    let _ = get_network_route_model(runtime, routes_state, "public-manager", true)?;
    let _ = get_network_route_model(runtime, routes_state, "manager-manager", true)?;
    let _ = get_network_route_model(runtime, routes_state, "owner-owner", false)?;
    let _ = get_device_route_model(runtime, routes_state, "manager-public-manager", true)?;
    let _ = get_device_route_model(runtime, routes_state, "manager-manager", true)?;
    let _ = get_device_route_model(runtime, routes_state, "owner-public-owner", false)?;
    let _ = get_device_route_model(runtime, routes_state, "owner1-owner", false)?;
    let _ = get_device_route_model(runtime, routes_state, "owner2-owner", false)?;
    let _ = get_dldata_buffer_model(runtime, routes_state, "manager-public-manager", true)?;
    let _ = get_dldata_buffer_model(runtime, routes_state, "manager-manager", true)?;
    let _ = get_dldata_buffer_model(runtime, routes_state, "owner-public-owner", false)?;
    let _ = get_dldata_buffer_model(runtime, routes_state, "owner1-owner", false)?;
    let _ = get_dldata_buffer_model(runtime, routes_state, "owner2-owner", false)?;

    Ok(())
}

pub fn delete_invalid_token(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    test_invalid_token(
        runtime,
        &routes_state,
        Method::DELETE,
        "/broker/api/v1/unit/id",
    )
}

pub fn delete_user(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let _state = context.state.borrow();
    let state = _state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    test_delete_user(runtime, routes_state, TOKEN_MANAGER, "id", true, true)?;
    libs::clear_all_data(runtime, state);
    test_delete_user(runtime, routes_state, TOKEN_MANAGER, "manager", false, true)?;
    libs::clear_all_data(runtime, state);
    test_delete_user(runtime, routes_state, TOKEN_MANAGER, "owner", true, false)?;

    Ok(())
}

pub fn delete_user_invalid_token(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    test_invalid_token(
        runtime,
        &routes_state,
        Method::DELETE,
        "/broker/api/v1/unit/user/id",
    )
}

pub fn delete_user_invalid_perm(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    test_invalid_perm(
        runtime,
        &routes_state,
        TOKEN_OWNER,
        Method::DELETE,
        "/broker/api/v1/unit/user/id",
    )?;
    test_invalid_perm(
        runtime,
        &routes_state,
        TOKEN_MEMBER,
        Method::DELETE,
        "/broker/api/v1/unit/user/id",
    )
}

fn test_post(
    runtime: &Runtime,
    state: &routes::State,
    token: &str,
    param: &request::PostUnit,
    expect_code: &str,
    expect_owner: &str,
) -> Result<(), String> {
    let app = Router::new().merge(routes::new_service(state));
    let server = match TestServer::new(app) {
        Err(e) => return Err(format!("new server error: {}", e)),
        Ok(server) => server,
    };

    let time_before = Utc::now().trunc_subsecs(3);
    let req = server
        .post("/broker/api/v1/unit")
        .add_header(
            header::AUTHORIZATION,
            HeaderValue::from_str(format!("Bearer {}", token).as_str()).unwrap(),
        )
        .json(param);
    let resp = runtime.block_on(async { req.await });
    let time_after = Utc::now().trunc_subsecs(3);
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
    let body: response::PostUnit = resp.json();
    expect(body.data.unit_id.len() > 0).to_equal(true)?;

    let unit_info = match runtime.block_on(async {
        let cond = QueryCond {
            unit_id: Some(body.data.unit_id.as_str()),
            ..Default::default()
        };
        state.model.unit().get(&cond).await
    }) {
        Err(e) => return Err(format!("get unit model error: {}", e)),
        Ok(info) => match info {
            None => return Err("add unit then get none unit".to_string()),
            Some(info) => info,
        },
    };
    expect(unit_info.code.as_str()).to_equal(param.data.code.as_str())?;
    expect(unit_info.created_at.ge(&time_before)).to_equal(true)?;
    expect(unit_info.created_at.le(&time_after)).to_equal(true)?;
    expect(unit_info.modified_at.ge(&time_before)).to_equal(true)?;
    expect(unit_info.modified_at.le(&time_after)).to_equal(true)?;
    expect(unit_info.owner_id.as_str()).to_equal(expect_owner)?;
    expect(unit_info.member_ids.len()).to_equal(1)?;
    expect(unit_info.member_ids[0].as_str()).to_equal(expect_owner)?;
    match param.data.name.as_ref() {
        None => expect(unit_info.name.len()).to_equal(0)?,
        Some(name) => expect(unit_info.name.as_str()).to_equal(name.as_str())?,
    }
    match param.data.info.as_ref() {
        None => expect(unit_info.info).to_equal(Map::<String, Value>::new()),
        Some(info) => expect(unit_info.info).to_equal(info.clone()),
    }
}

fn test_post_invalid_param(
    runtime: &Runtime,
    state: &routes::State,
    token: &str,
    param: Option<&request::PostUnit>,
) -> Result<(), String> {
    let app = Router::new().merge(routes::new_service(state));
    let server = match TestServer::new(app) {
        Err(e) => return Err(format!("new server error: {}", e)),
        Ok(server) => server,
    };

    let req = server
        .post("/broker/api/v1/unit")
        .add_header(
            header::AUTHORIZATION,
            HeaderValue::from_str(format!("Bearer {}", token).as_str()).unwrap(),
        )
        .json(&param);
    let resp = runtime.block_on(async { req.await });
    expect(resp.status_code()).to_equal(StatusCode::BAD_REQUEST)?;
    let body: ApiError = resp.json();
    if body.code.as_str() != err::E_PARAM {
        return Err(format!("unexpected 400 error: {}", body.code.as_str()));
    }
    Ok(())
}

fn test_get_count(
    runtime: &Runtime,
    state: &routes::State,
    token: &str,
    param: Option<&request::GetUnitCount>,
    expect_count: usize,
) -> Result<(), String> {
    let app = Router::new().merge(routes::new_service(state));
    let server = match TestServer::new(app) {
        Err(e) => return Err(format!("new server error: {}", e)),
        Ok(server) => server,
    };

    let req = server
        .get("/broker/api/v1/unit/count")
        .add_query_params(&param)
        .add_header(
            header::AUTHORIZATION,
            HeaderValue::from_str(format!("Bearer {}", token).as_str()).unwrap(),
        );
    let resp = runtime.block_on(async { req.await });
    expect(resp.status_code()).to_equal(StatusCode::OK)?;
    let body: response::GetUnitCount = resp.json();
    expect(body.data.count).to_equal(expect_count)
}

fn test_get_list(
    runtime: &Runtime,
    state: &routes::State,
    token: &str,
    param: Option<&request::GetUnitList>,
    expect_count: usize,
) -> Result<(), String> {
    let app = Router::new().merge(routes::new_service(state));
    let server = match TestServer::new(app) {
        Err(e) => return Err(format!("new server error: {}", e)),
        Ok(server) => server,
    };

    let req = server
        .get("/broker/api/v1/unit/list")
        .add_query_params(&param)
        .add_header(
            header::AUTHORIZATION,
            HeaderValue::from_str(format!("Bearer {}", token).as_str()).unwrap(),
        );
    let resp = runtime.block_on(async { req.await });
    expect(resp.status_code()).to_equal(StatusCode::OK)?;
    let body: response::GetUnitList = resp.json();
    expect(body.data.len()).to_equal(expect_count)?;

    let mut code_min = "";
    for info in body.data.iter() {
        if let Err(_) = expect(info.code.as_str().ge(code_min)).to_equal(true) {
            return Err(format!(
                "code order error: {} - {}",
                code_min,
                info.code.as_str()
            ));
        }
        code_min = info.code.as_str();
    }
    Ok(())
}

fn test_get_list_sort(
    runtime: &Runtime,
    state: &routes::State,
    token: &str,
    param: &mut request::GetUnitList,
    expect_ids: &[&str],
) -> Result<(), String> {
    let app = Router::new().merge(routes::new_service(state));
    let server = match TestServer::new(app) {
        Err(e) => return Err(format!("new server error: {}", e)),
        Ok(server) => server,
    };

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
        .get("/broker/api/v1/unit/list")
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
            "response not 200: /broker/api/v1/unit/list, {}, {}",
            body.code, message
        ));
    }
    let body: response::GetUnitList = resp.json();
    expect(body.data.len()).to_equal(expect_ids.len())?;

    let mut index = 0;
    for data in body.data.iter() {
        expect(data.unit_id.as_str()).to_equal(expect_ids[index])?;
        index += 1;
    }
    Ok(())
}

fn test_get_list_offset_limit(
    runtime: &Runtime,
    state: &routes::State,
    token: &str,
    param: &request::GetUnitList,
    expect_ids: Vec<i32>,
) -> Result<(), String> {
    let app = Router::new().merge(routes::new_service(state));
    let server = match TestServer::new(app) {
        Err(e) => return Err(format!("new server error: {}", e)),
        Ok(server) => server,
    };

    let req = server
        .get("/broker/api/v1/unit/list")
        .add_query_params(&param)
        .add_header(
            header::AUTHORIZATION,
            HeaderValue::from_str(format!("Bearer {}", token).as_str()).unwrap(),
        );
    let resp = runtime.block_on(async { req.await });
    expect(resp.status_code()).to_equal(StatusCode::OK)?;
    let body: response::GetUnitList = resp.json();
    expect(body.data.len()).to_equal(expect_ids.len())?;

    let mut index = 0;
    for data in body.data.iter() {
        expect(data.unit_id.as_str()).to_equal(format!("unit_{}", expect_ids[index]).as_str())?;
        index += 1;
    }
    Ok(())
}

fn test_get_list_format_array(
    runtime: &Runtime,
    state: &routes::State,
    token: &str,
    param: &request::GetUnitList,
    expect_ids: Vec<i32>,
) -> Result<(), String> {
    let app = Router::new().merge(routes::new_service(state));
    let server = match TestServer::new(app) {
        Err(e) => return Err(format!("new server error: {}", e)),
        Ok(server) => server,
    };

    let req = server
        .get("/broker/api/v1/unit/list")
        .add_query_params(param)
        .add_header(
            header::AUTHORIZATION,
            HeaderValue::from_str(format!("Bearer {}", token).as_str()).unwrap(),
        );
    let resp = runtime.block_on(async { req.await });
    expect(resp.status_code()).to_equal(StatusCode::OK)?;
    let body: Vec<response::GetUnitListData> = resp.json();
    expect(body.len()).to_equal(expect_ids.len())?;

    let mut index = 0;
    for data in body.iter() {
        expect(data.unit_id.as_str()).to_equal(format!("unit_{}", expect_ids[index]).as_str())?;
        index += 1;
    }
    Ok(())
}

fn test_get(
    runtime: &Runtime,
    state: &routes::State,
    token: &str,
    unit_id: &str,
) -> Result<(), String> {
    let app = Router::new().merge(routes::new_service(state));
    let server = match TestServer::new(app) {
        Err(e) => return Err(format!("new server error: {}", e)),
        Ok(server) => server,
    };

    let unit_info = get_unit_model(runtime, state, unit_id, true)?.unwrap();

    let req = server
        .get(format!("/broker/api/v1/unit/{}", unit_id).as_str())
        .add_header(
            header::AUTHORIZATION,
            HeaderValue::from_str(format!("Bearer {}", token).as_str()).unwrap(),
        );
    let resp = runtime.block_on(async { req.await });
    expect(resp.status_code()).to_equal(StatusCode::OK)?;
    let body: response::GetUnit = resp.json();
    expect(body.data.unit_id.as_str()).to_equal(unit_info.unit_id.as_str())?;
    expect(body.data.code.as_str()).to_equal(unit_info.code.as_str())?;
    expect(
        DateTime::parse_from_rfc3339(body.data.created_at.as_str())
            .unwrap()
            .timestamp_millis(),
    )
    .to_equal(unit_info.created_at.timestamp_millis())?;
    expect(
        DateTime::parse_from_rfc3339(body.data.modified_at.as_str())
            .unwrap()
            .timestamp_millis(),
    )
    .to_equal(unit_info.modified_at.timestamp_millis())?;
    expect(body.data.owner_id.as_str()).to_equal(unit_info.owner_id.as_str())?;
    expect(body.data.member_ids.as_slice()).to_equal(unit_info.member_ids.as_slice())?;
    expect(body.data.name.as_str()).to_equal(unit_info.name.as_str())?;
    expect(body.data.info).to_equal(unit_info.info)
}

fn test_get_wrong_id(
    runtime: &Runtime,
    state: &routes::State,
    token: &str,
    unit_id: &str,
) -> Result<(), String> {
    let app = Router::new().merge(routes::new_service(state));
    let server = match TestServer::new(app) {
        Err(e) => return Err(format!("new server error: {}", e)),
        Ok(server) => server,
    };

    let req = server
        .get(format!("/broker/api/v1/unit/{}", unit_id).as_str())
        .add_header(
            header::AUTHORIZATION,
            HeaderValue::from_str(format!("Bearer {}", token).as_str()).unwrap(),
        );
    let resp = runtime.block_on(async { req.await });
    expect(resp.status_code()).to_equal(StatusCode::NOT_FOUND)?;
    let body: ApiError = resp.json();
    expect(body.code.as_str()).to_equal(err::E_NOT_FOUND)
}

fn test_patch(
    runtime: &Runtime,
    state: &routes::State,
    token: &str,
    unit_id: &str,
    user_id: &str,
    new_owner_id: &str,
    mut new_member_ids: Vec<String>,
    expect_owner_member_changed: bool,
) -> Result<(), String> {
    add_unit_model(runtime, state, unit_id, vec![], user_id)?;

    let app = Router::new().merge(routes::new_service(state));
    let server = match TestServer::new(app) {
        Err(e) => return Err(format!("new server error: {}", e)),
        Ok(server) => server,
    };

    let time_before = Utc::now().trunc_subsecs(3);
    let mut info = Map::<String, Value>::new();
    info.insert(
        "key_update".to_string(),
        Value::String("updated".to_string()),
    );
    let body = request::PatchUnit {
        data: request::PatchUnitData {
            owner_id: Some(new_owner_id.to_string()),
            member_ids: Some(new_member_ids.clone()),
            name: Some("name changes".to_string()),
            info: Some(info.clone()),
        },
    };
    let req = server
        .patch(format!("/broker/api/v1/unit/{}", unit_id).as_str())
        .add_header(
            header::AUTHORIZATION,
            HeaderValue::from_str(format!("Bearer {}", token).as_str()).unwrap(),
        )
        .json(&body);
    let resp = runtime.block_on(async { req.await });
    expect(resp.status_code()).to_equal(StatusCode::NO_CONTENT)?;

    let time_after = Utc::now().trunc_subsecs(3);
    let unit_info = get_unit_model(runtime, state, unit_id, true)?.unwrap();
    expect(unit_info.modified_at.ge(&time_before)).to_equal(true)?;
    expect(unit_info.modified_at.le(&time_after)).to_equal(true)?;
    if expect_owner_member_changed {
        expect(unit_info.owner_id.as_str()).to_equal(new_owner_id)?;
        new_member_ids.sort();
        new_member_ids.dedup();
        expect(unit_info.member_ids.clone()).to_equal(new_member_ids)?;
    } else {
        expect(unit_info.owner_id.as_str()).to_equal(user_id)?;
        expect(unit_info.member_ids.clone()).to_equal(vec![user_id.to_string()])?;
    }
    expect(unit_info.name.as_str()).to_equal("name changes")?;
    expect(unit_info.info).to_equal(info)?;

    let body = request::PatchUnit {
        data: request::PatchUnitData {
            owner_id: Some(user_id.to_string()),
            member_ids: Some(vec![]),
            name: Some("".to_string()),
            info: Some(Map::<String, Value>::new()),
        },
    };
    let req = server
        .patch(format!("/broker/api/v1/unit/{}", unit_id).as_str())
        .add_header(
            header::AUTHORIZATION,
            HeaderValue::from_str(format!("Bearer {}", token).as_str()).unwrap(),
        )
        .json(&body);
    let resp = runtime.block_on(async { req.await });
    expect(resp.status_code()).to_equal(StatusCode::NO_CONTENT)?;

    let unit_info = get_unit_model(runtime, state, user_id, true)?.unwrap();
    if expect_owner_member_changed {
        expect(unit_info.owner_id.as_str()).to_equal(user_id)?;
        expect(unit_info.member_ids.clone()).to_equal(vec![user_id.to_string()])?;
    } else {
        expect(unit_info.owner_id.as_str()).to_equal(user_id)?;
        expect(unit_info.member_ids.clone()).to_equal(vec![user_id.to_string()])?;
    }
    expect(unit_info.name.as_str()).to_equal("")?;
    expect(unit_info.info).to_equal(Map::<String, Value>::new())?;

    if token != TOKEN_MANAGER {
        return Ok(());
    }
    let body = request::PatchUnit {
        data: request::PatchUnitData {
            owner_id: Some(new_owner_id.to_string()),
            member_ids: None,
            name: None,
            info: None,
        },
    };
    let req = server
        .patch(format!("/broker/api/v1/unit/{}", unit_id).as_str())
        .add_header(
            header::AUTHORIZATION,
            HeaderValue::from_str(format!("Bearer {}", token).as_str()).unwrap(),
        )
        .json(&body);
    let resp = runtime.block_on(async { req.await });
    expect(resp.status_code()).to_equal(StatusCode::NO_CONTENT)?;

    let unit_info = get_unit_model(runtime, state, user_id, true)?.unwrap();
    expect(unit_info.owner_id.as_str()).to_equal(new_owner_id)?;
    expect(unit_info.member_ids.contains(&new_owner_id.to_string())).to_equal(true)?;
    expect(unit_info.member_ids.contains(&user_id.to_string())).to_equal(true)
}

fn test_patch_wrong_id(
    runtime: &Runtime,
    state: &routes::State,
    token: &str,
    unit_id: &str,
) -> Result<(), String> {
    let app = Router::new().merge(routes::new_service(state));
    let server = match TestServer::new(app) {
        Err(e) => return Err(format!("new server error: {}", e)),
        Ok(server) => server,
    };

    let body = request::PatchUnit {
        data: request::PatchUnitData {
            name: Some("".to_string()),
            info: Some(Map::<String, Value>::new()),
            ..Default::default()
        },
    };
    let req = server
        .patch(format!("/broker/api/v1/unit/{}", unit_id).as_str())
        .add_header(
            header::AUTHORIZATION,
            HeaderValue::from_str(format!("Bearer {}", token).as_str()).unwrap(),
        )
        .json(&body);
    let resp = runtime.block_on(async { req.await });
    expect(resp.status_code()).to_equal(StatusCode::NOT_FOUND)?;
    let body: ApiError = resp.json();
    expect(body.code.as_str()).to_equal(err::E_NOT_FOUND)
}

fn test_patch_invalid_param(
    runtime: &Runtime,
    state: &routes::State,
    token: &str,
    unit_id: &str,
    param: Option<&request::PatchUnit>,
) -> Result<(), String> {
    let app = Router::new().merge(routes::new_service(state));
    let server = match TestServer::new(app) {
        Err(e) => return Err(format!("new server error: {}", e)),
        Ok(server) => server,
    };

    let req = server
        .patch(format!("/broker/api/v1/unit/{}", unit_id).as_str())
        .add_header(
            header::AUTHORIZATION,
            HeaderValue::from_str(format!("Bearer {}", token).as_str()).unwrap(),
        )
        .json(&param);
    let resp = runtime.block_on(async { req.await });
    expect(resp.status_code()).to_equal(StatusCode::BAD_REQUEST)?;
    let body: ApiError = resp.json();
    if body.code.as_str() != err::E_PARAM {
        return Err(format!("unexpected 400 error: {}", body.code.as_str()));
    }
    Ok(())
}

fn test_delete_user(
    runtime: &Runtime,
    state: &routes::State,
    token: &str,
    user_id: &str,
    expect_manager: bool,
    expect_owner: bool,
) -> Result<(), String> {
    add_delete_rsc(runtime, state)?;

    let app = Router::new().merge(routes::new_service(state));
    let server = match TestServer::new(app) {
        Err(e) => return Err(format!("new server error: {}", e)),
        Ok(server) => server,
    };

    let req = server
        .delete(format!("/broker/api/v1/unit/user/{}", user_id).as_str())
        .add_header(
            header::AUTHORIZATION,
            HeaderValue::from_str(format!("Bearer {}", token).as_str()).unwrap(),
        );
    let resp = runtime.block_on(async { req.await });
    expect(resp.status_code()).to_equal(StatusCode::NO_CONTENT)?;

    let _ = get_unit_model(runtime, state, "manager", expect_manager)?;
    let _ = get_unit_model(runtime, state, "owner", expect_owner)?;
    let _ = get_application_model(runtime, state, "manager", expect_manager)?;
    let _ = get_application_model(runtime, state, "owner", expect_owner)?;
    let _ = get_network_model(runtime, state, "public", true)?;
    let _ = get_network_model(runtime, state, "manager", expect_manager)?;
    let _ = get_network_model(runtime, state, "owner", expect_owner)?;
    let _ = get_device_model(runtime, state, "manager-public", expect_manager)?;
    let _ = get_device_model(runtime, state, "manager", expect_manager)?;
    let _ = get_device_model(runtime, state, "owner-public", expect_owner)?;
    let _ = get_device_model(runtime, state, "owner1", expect_owner)?;
    let _ = get_device_model(runtime, state, "owner2", expect_owner)?;
    let _ = get_network_route_model(runtime, state, "public-manager", expect_manager)?;
    let _ = get_network_route_model(runtime, state, "manager-manager", expect_manager)?;
    let _ = get_network_route_model(runtime, state, "owner-owner", expect_owner)?;
    let _ = get_device_route_model(runtime, state, "manager-public-manager", expect_manager)?;
    let _ = get_device_route_model(runtime, state, "manager-manager", expect_manager)?;
    let _ = get_device_route_model(runtime, state, "owner-public-owner", expect_owner)?;
    let _ = get_device_route_model(runtime, state, "owner1-owner", expect_owner)?;
    let _ = get_device_route_model(runtime, state, "owner2-owner", expect_owner)?;
    let _ = get_dldata_buffer_model(runtime, state, "manager-public-manager", expect_manager)?;
    let _ = get_dldata_buffer_model(runtime, state, "manager-manager", expect_manager)?;
    let _ = get_dldata_buffer_model(runtime, state, "owner-public-owner", expect_owner)?;
    let _ = get_dldata_buffer_model(runtime, state, "owner1-owner", expect_owner)?;
    let _ = get_dldata_buffer_model(runtime, state, "owner2-owner", expect_owner)?;

    Ok(())
}

/// Returns (total, owner, member) tuple.
fn count_list_dataset(
    runtime: &Runtime,
    state: &routes::State,
) -> Result<(usize, usize, usize), String> {
    let now = Utc::now();

    let mut unit = create_unit("test", "manager");
    unit.created_at = now;
    unit.modified_at = now;
    runtime.block_on(async {
        if let Err(e) = state.model.unit().add(&unit).await {
            return Err(format!("add unit {} error: {}", unit.unit_id, e));
        }
        Ok(())
    })?;

    let mut unit = create_unit("manager1", "manager");
    unit.created_at = now + TimeDelta::try_milliseconds(1).unwrap();
    unit.modified_at = now + TimeDelta::try_milliseconds(4).unwrap();
    runtime.block_on(async {
        if let Err(e) = state.model.unit().add(&unit).await {
            return Err(format!("add unit {} error: {}", unit.unit_id, e));
        }
        Ok(())
    })?;

    let mut unit = create_unit("manager2", "manager");
    unit.created_at = now + TimeDelta::try_milliseconds(2).unwrap();
    unit.modified_at = now + TimeDelta::try_milliseconds(3).unwrap();
    runtime.block_on(async {
        if let Err(e) = state.model.unit().add(&unit).await {
            return Err(format!("add unit {} error: {}", unit.unit_id, e));
        }
        Ok(())
    })?;

    let mut unit = create_unit("owner1", "owner");
    unit.created_at = now + TimeDelta::try_milliseconds(3).unwrap();
    unit.modified_at = now + TimeDelta::try_milliseconds(2).unwrap();
    unit.name = "owner2".to_string();
    runtime.block_on(async {
        if let Err(e) = state.model.unit().add(&unit).await {
            return Err(format!("add unit {} error: {}", unit.unit_id, e));
        }
        Ok(())
    })?;

    let mut unit = create_unit("owner2", "owner");
    unit.created_at = now + TimeDelta::try_milliseconds(4).unwrap();
    unit.modified_at = now + TimeDelta::try_milliseconds(1).unwrap();
    unit.member_ids.push("member".to_string());
    unit.name = "owner1".to_string();
    runtime.block_on(async {
        if let Err(e) = state.model.unit().add(&unit).await {
            return Err(format!("add unit {} error: {}", unit.unit_id, e));
        }
        Ok(())
    })?;

    Ok((5, 2, 1))
}
