use actix_web::{
    http::{header, StatusCode},
    middleware::NormalizePath,
    test::{self, TestRequest},
    App,
};
use chrono::{DateTime, Duration, SubsecRound, Utc};
use laboratory::{expect, SpecContext};
use serde_json::{Map, Value};
use serde_urlencoded;
use tokio::runtime::Runtime;

use sylvia_iot_broker::{models::application::QueryCond, routes};
use sylvia_iot_corelib::err;

use super::{
    super::{
        super::libs::{
            add_application_model, add_delete_rsc, add_unit_model, create_application,
            get_application_model, get_device_model, get_device_route_model,
            get_dldata_buffer_model, get_network_model, get_network_route_model, get_unit_model,
            test_get_400, test_invalid_token, ApiError,
        },
        libs, TestState, STATE, TOKEN_MANAGER, TOKEN_MEMBER, TOKEN_OWNER,
    },
    request, response,
};

pub fn post(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    add_unit_model(runtime, routes_state, "manager", vec![], "manager")?;
    add_unit_model(runtime, routes_state, "owner", vec![], "owner")?;

    let mut info = Map::<String, Value>::new();
    info.insert("info".to_string(), Value::String("value".to_string()));
    let param = request::PostApplication {
        data: request::PostApplicationData {
            code: "manager".to_string(),
            unit_id: "manager".to_string(),
            host_uri: "amqp://host".to_string(),
            name: Some("manager".to_string()),
            info: Some(info),
        },
    };
    test_post(runtime, routes_state, TOKEN_MANAGER, &param, "")?;

    let param = request::PostApplication {
        data: request::PostApplicationData {
            code: "owner".to_string(),
            unit_id: "owner".to_string(),
            host_uri: "amqp://host".to_string(),
            name: None,
            info: None,
        },
    };
    test_post(runtime, routes_state, TOKEN_OWNER, &param, "")
}

pub fn post_dup(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    add_unit_model(runtime, routes_state, "manager", vec![], "manager")?;

    let param = request::PostApplication {
        data: request::PostApplicationData {
            code: "manager".to_string(),
            unit_id: "manager".to_string(),
            host_uri: "amqp://host".to_string(),
            name: None,
            info: None,
        },
    };
    test_post(runtime, routes_state, TOKEN_MANAGER, &param, "")?;

    let param = request::PostApplication {
        data: request::PostApplicationData {
            code: "manager".to_string(),
            unit_id: "manager".to_string(),
            host_uri: "amqp://host".to_string(),
            name: None,
            info: None,
        },
    };
    test_post(
        runtime,
        routes_state,
        TOKEN_MANAGER,
        &param,
        "err_broker_application_exist",
    )
}

pub fn post_not_exist_unit(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    add_unit_model(runtime, routes_state, "manager", vec!["member"], "manager")?;

    let param = request::PostApplication {
        data: request::PostApplicationData {
            code: "manager".to_string(),
            unit_id: "not-exist".to_string(),
            host_uri: "amqp://host".to_string(),
            name: None,
            info: None,
        },
    };
    test_post(
        runtime,
        routes_state,
        TOKEN_MANAGER,
        &param,
        "err_broker_unit_not_exist",
    )?;

    let param = request::PostApplication {
        data: request::PostApplicationData {
            code: "manager".to_string(),
            unit_id: "manager".to_string(),
            host_uri: "amqp://host".to_string(),
            name: None,
            info: None,
        },
    };
    test_post(
        runtime,
        routes_state,
        TOKEN_MEMBER,
        &param,
        "err_broker_unit_not_exist",
    )
}

pub fn post_invalid_param(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    add_unit_model(runtime, routes_state, "manager", vec![], "manager")?;

    test_post_invalid_param(runtime, &routes_state, TOKEN_MANAGER, None)?;

    let param = request::PostApplication {
        data: request::PostApplicationData {
            code: "".to_string(),
            unit_id: "manager".to_string(),
            host_uri: "amqp://host".to_string(),
            name: None,
            info: None,
        },
    };
    test_post_invalid_param(runtime, &routes_state, TOKEN_MANAGER, Some(&param))?;

    let param = request::PostApplication {
        data: request::PostApplicationData {
            code: "code".to_string(),
            unit_id: "".to_string(),
            host_uri: "amqp://host".to_string(),
            name: None,
            info: None,
        },
    };
    test_post_invalid_param(runtime, &routes_state, TOKEN_MANAGER, Some(&param))?;

    let param = request::PostApplication {
        data: request::PostApplicationData {
            code: "code".to_string(),
            unit_id: "manager".to_string(),
            host_uri: "".to_string(),
            name: None,
            info: None,
        },
    };
    test_post_invalid_param(runtime, &routes_state, TOKEN_MANAGER, Some(&param))?;

    let param = request::PostApplication {
        data: request::PostApplicationData {
            code: "code".to_string(),
            unit_id: "manager".to_string(),
            host_uri: "http://host".to_string(),
            name: None,
            info: None,
        },
    };
    test_post_invalid_param(runtime, &routes_state, TOKEN_MANAGER, Some(&param))?;

    let mut info = Map::<String, Value>::new();
    info.insert("".to_string(), Value::String("value".to_string()));
    let param = request::PostApplication {
        data: request::PostApplicationData {
            code: "code".to_string(),
            unit_id: "manager".to_string(),
            host_uri: "amqp://host".to_string(),
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

    let req = TestRequest::post().uri("/broker/api/v1/application");
    test_invalid_token(runtime, &routes_state, req)
}

pub fn get_count(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    let data_size = count_list_dataset(runtime, &routes_state)?;

    test_get_count(runtime, &routes_state, TOKEN_MANAGER, None, data_size.0)?;

    let param = request::GetApplicationCount {
        unit: Some("".to_string()),
        ..Default::default()
    };
    test_get_count(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        Some(&param),
        data_size.0,
    )?;

    let param = request::GetApplicationCount {
        unit: Some("manager".to_string()),
        ..Default::default()
    };
    test_get_count(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 1)?;
    test_get_count(runtime, &routes_state, TOKEN_MEMBER, Some(&param), 1)?;

    let param = request::GetApplicationCount {
        unit: Some("owner1".to_string()),
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

    let param = request::GetApplicationCount {
        unit: Some("owner2".to_string()),
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
        data_size.2,
    )?;
    test_get_count(
        runtime,
        &routes_state,
        TOKEN_MEMBER,
        Some(&param),
        data_size.2,
    )?;

    let param = request::GetApplicationCount {
        contains: Some("2".to_string()),
        ..Default::default()
    };
    test_get_count(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 3)?;

    let param = request::GetApplicationCount {
        unit: Some("owner1".to_string()),
        contains: Some("2".to_string()),
        ..Default::default()
    };
    test_get_count(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 1)?;
    test_get_count(runtime, &routes_state, TOKEN_OWNER, Some(&param), 1)
}

pub fn get_count_not_exist_unit(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    add_unit_model(runtime, routes_state, "manager", vec![], "manager")?;

    let uri = "/broker/api/v1/application/count";
    let code = "err_broker_unit_not_exist";

    let mut query = Map::<String, Value>::new();
    query.insert("unit".to_string(), Value::String("not-exist".to_string()));
    test_get_400(runtime, routes_state, TOKEN_MANAGER, uri, &query, code)?;

    let mut query = Map::<String, Value>::new();
    query.insert("unit".to_string(), Value::String("manager".to_string()));
    test_get_400(runtime, routes_state, TOKEN_OWNER, uri, &query, code)?;

    let mut query = Map::<String, Value>::new();
    query.insert("unit".to_string(), Value::String("owner1".to_string()));
    test_get_400(runtime, routes_state, TOKEN_MEMBER, uri, &query, code)
}

pub fn get_count_invalid_param(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    let uri = "/broker/api/v1/application/count";
    let code = err::E_PARAM;

    let query = Map::<String, Value>::new();
    test_get_400(runtime, routes_state, TOKEN_OWNER, uri, &query, code)?;

    let mut query = Map::<String, Value>::new();
    query.insert("unit".to_string(), Value::String("".to_string()));
    test_get_400(runtime, routes_state, TOKEN_OWNER, uri, &query, code)
}

pub fn get_count_invalid_token(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    let req = TestRequest::get().uri("/broker/api/v1/application/count");
    test_invalid_token(runtime, &routes_state, req)
}

pub fn get_list(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    let data_size = count_list_dataset(runtime, &routes_state)?;

    test_get_list(runtime, &routes_state, TOKEN_MANAGER, None, data_size.0)?;

    let param = request::GetApplicationList {
        unit: Some("".to_string()),
        ..Default::default()
    };
    test_get_list(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        Some(&param),
        data_size.0,
    )?;

    let param = request::GetApplicationList {
        unit: Some("manager".to_string()),
        ..Default::default()
    };
    test_get_list(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 1)?;
    test_get_list(runtime, &routes_state, TOKEN_MEMBER, Some(&param), 1)?;

    let param = request::GetApplicationList {
        unit: Some("owner1".to_string()),
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

    let param = request::GetApplicationList {
        unit: Some("owner2".to_string()),
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
        data_size.2,
    )?;
    test_get_list(
        runtime,
        &routes_state,
        TOKEN_MEMBER,
        Some(&param),
        data_size.2,
    )?;

    let param = request::GetApplicationList {
        contains: Some("2".to_string()),
        ..Default::default()
    };
    test_get_list(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 3)?;

    let param = request::GetApplicationList {
        unit: Some("owner1".to_string()),
        contains: Some("2".to_string()),
        ..Default::default()
    };
    test_get_list(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 1)?;
    test_get_list(runtime, &routes_state, TOKEN_OWNER, Some(&param), 1)
}

pub fn get_list_sort(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    count_list_dataset(runtime, &routes_state)?;

    let mut param = request::GetApplicationList {
        ..Default::default()
    };
    test_get_list_sort(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        &mut param,
        &[
            "manager", "owner1-1", "owner1-2", "owner1-3", "owner2-1", "owner2-2",
        ],
    )?;

    param.sort_vec = Some(vec![("code", true)]);
    test_get_list_sort(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        &mut param,
        &[
            "manager", "owner1-1", "owner1-2", "owner1-3", "owner2-1", "owner2-2",
        ],
    )?;
    param.sort_vec = Some(vec![("code", false)]);
    test_get_list_sort(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        &mut param,
        &[
            "owner2-2", "owner2-1", "owner1-3", "owner1-2", "owner1-1", "manager",
        ],
    )?;

    param.sort_vec = Some(vec![("name", true)]);
    test_get_list_sort(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        &mut param,
        &[
            "manager", "owner1-1", "owner1-2", "owner1-3", "owner2-2", "owner2-1",
        ],
    )?;
    param.sort_vec = Some(vec![("name", false)]);
    test_get_list_sort(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        &mut param,
        &[
            "owner2-1", "owner2-2", "owner1-3", "owner1-2", "owner1-1", "manager",
        ],
    )?;

    param.sort_vec = Some(vec![("created", true)]);
    test_get_list_sort(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        &mut param,
        &[
            "manager", "owner1-1", "owner1-2", "owner1-3", "owner2-1", "owner2-2",
        ],
    )?;
    param.sort_vec = Some(vec![("created", false)]);
    test_get_list_sort(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        &mut param,
        &[
            "owner2-2", "owner2-1", "owner1-3", "owner1-2", "owner1-1", "manager",
        ],
    )?;

    param.sort_vec = Some(vec![("modified", true)]);
    test_get_list_sort(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        &mut param,
        &[
            "manager", "owner2-2", "owner2-1", "owner1-3", "owner1-2", "owner1-1",
        ],
    )?;
    param.sort_vec = Some(vec![("modified", false)]);
    test_get_list_sort(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        &mut param,
        &[
            "owner1-1", "owner1-2", "owner1-3", "owner2-1", "owner2-2", "manager",
        ],
    )
}

pub fn get_list_offset_limit(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    add_unit_model(runtime, routes_state, "manager", vec![], "manager")?;

    let unit_id = "manager";

    for i in 100..302 {
        add_application_model(
            runtime,
            &routes_state,
            unit_id,
            format!("application_{}", i).as_str(),
            "amqp://host",
        )?;
    }

    let mut param = request::GetApplicationList {
        unit: Some(unit_id.to_string()),
        contains: Some("A".to_string()),
        ..Default::default()
    };
    test_get_list_offset_limit(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        &param,
        (100..200).collect(),
    )?;

    param.unit = Some("".to_string());
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

    add_unit_model(runtime, routes_state, "manager", vec![], "manager")?;

    let unit_id = "manager";

    for i in 100..302 {
        add_application_model(
            runtime,
            &routes_state,
            unit_id,
            format!("application_{}", i).as_str(),
            "amqp://host",
        )?;
    }

    let mut param = request::GetApplicationList {
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

pub fn get_list_not_exist_unit(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    add_unit_model(runtime, routes_state, "manager", vec![], "manager")?;

    let uri = "/broker/api/v1/application/list";
    let code = "err_broker_unit_not_exist";

    let mut query = Map::<String, Value>::new();
    query.insert("unit".to_string(), Value::String("not-exist".to_string()));
    test_get_400(runtime, routes_state, TOKEN_MANAGER, uri, &query, code)?;

    let mut query = Map::<String, Value>::new();
    query.insert("unit".to_string(), Value::String("manager".to_string()));
    test_get_400(runtime, routes_state, TOKEN_OWNER, uri, &query, code)?;

    let mut query = Map::<String, Value>::new();
    query.insert("unit".to_string(), Value::String("owner1".to_string()));
    test_get_400(runtime, routes_state, TOKEN_MEMBER, uri, &query, code)
}

pub fn get_list_invalid_param(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    let uri = "/broker/api/v1/application/list";
    let code = err::E_PARAM;

    let query = Map::<String, Value>::new();
    test_get_400(runtime, routes_state, TOKEN_OWNER, uri, &query, code)?;

    let mut query = Map::<String, Value>::new();
    query.insert("unit".to_string(), Value::String("".to_string()));
    test_get_400(runtime, routes_state, TOKEN_OWNER, uri, &query, code)?;

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

    let req = TestRequest::get().uri("/broker/api/v1/application/list");
    test_invalid_token(runtime, &routes_state, req)
}

pub fn get(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    add_unit_model(runtime, routes_state, "manager", vec!["member"], "manager")?;
    add_unit_model(runtime, routes_state, "owner1", vec![], "owner")?;
    add_unit_model(runtime, routes_state, "owner2", vec!["member"], "owner")?;
    add_application_model(runtime, routes_state, "manager", "manager", "amqp://host")?;
    add_application_model(runtime, routes_state, "owner1", "owner1", "amqp://host")?;
    add_application_model(runtime, routes_state, "owner2", "owner2", "amqp://host")?;

    test_get(runtime, routes_state, TOKEN_MANAGER, "manager")?;
    test_get(runtime, routes_state, TOKEN_MANAGER, "owner1")?;
    test_get(runtime, routes_state, TOKEN_OWNER, "owner1")?;
    test_get(runtime, routes_state, TOKEN_OWNER, "owner2")?;
    test_get(runtime, routes_state, TOKEN_MEMBER, "manager")?;
    test_get(runtime, routes_state, TOKEN_MEMBER, "owner2")
}

pub fn get_wrong_id(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    add_unit_model(runtime, routes_state, "manager", vec!["member"], "manager")?;
    add_unit_model(runtime, routes_state, "owner1", vec![], "owner")?;
    add_unit_model(runtime, routes_state, "owner2", vec!["member"], "owner")?;
    add_application_model(runtime, routes_state, "manager", "manager", "amqp://host")?;
    add_application_model(runtime, routes_state, "owner1", "owner1", "amqp://host")?;
    add_application_model(runtime, routes_state, "owner2", "owner2", "amqp://host")?;

    test_get_wrong_id(runtime, routes_state, TOKEN_MANAGER, "manager1")?;
    test_get_wrong_id(runtime, routes_state, TOKEN_OWNER, "manager")?;
    test_get_wrong_id(runtime, routes_state, TOKEN_MEMBER, "owner1")
}

pub fn get_invalid_token(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    let req = TestRequest::get().uri("/broker/api/v1/application/id");
    test_invalid_token(runtime, &routes_state, req)
}

pub fn patch(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    add_unit_model(runtime, routes_state, "manager", vec![], "manager")?;
    add_unit_model(runtime, routes_state, "owner", vec![], "owner")?;

    test_patch(runtime, routes_state, TOKEN_MANAGER, "manager", "manager")?;
    test_patch(runtime, routes_state, TOKEN_OWNER, "owner", "owner")
}

pub fn patch_wrong_id(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    add_unit_model(runtime, routes_state, "manager", vec![], "manager")?;

    test_patch_wrong_id(runtime, routes_state, TOKEN_MANAGER, "manager1")?;
    test_patch_wrong_id(runtime, routes_state, TOKEN_OWNER, "manager")?;
    test_patch_wrong_id(runtime, routes_state, TOKEN_MEMBER, "manager")
}

pub fn patch_invalid_param(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    add_unit_model(runtime, routes_state, "manager", vec![], "manager")?;
    add_application_model(runtime, routes_state, "manager", "manager", "amqp://host")?;

    test_patch_invalid_param(runtime, routes_state, TOKEN_MANAGER, "manager", None)?;

    let param = request::PatchApplication {
        ..Default::default()
    };
    test_patch_invalid_param(
        runtime,
        routes_state,
        TOKEN_MANAGER,
        "manager",
        Some(&param),
    )?;

    let param = request::PatchApplication {
        data: request::PatchApplicationData {
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

    let param = request::PatchApplication {
        data: request::PatchApplicationData {
            host_uri: Some("".to_string()),
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

    let param = request::PatchApplication {
        data: request::PatchApplicationData {
            host_uri: Some(":://".to_string()),
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

    let param = request::PatchApplication {
        data: request::PatchApplicationData {
            host_uri: Some("http://localhost".to_string()),
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
    let param = request::PatchApplication {
        data: request::PatchApplicationData {
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

    let req = TestRequest::patch().uri("/broker/api/v1/application/id");
    test_invalid_token(runtime, &routes_state, req)
}

pub fn delete(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    add_unit_model(runtime, routes_state, "manager", vec!["member"], "manager")?;
    add_unit_model(runtime, routes_state, "owner", vec!["member"], "owner")?;
    add_application_model(runtime, routes_state, "manager", "manager", "amqp://host")?;
    add_application_model(runtime, routes_state, "owner", "owner", "amqp://host")?;
    add_application_model(runtime, routes_state, "owner", "owner2", "amqp://host")?;

    let mut app = runtime.block_on(async {
        test::init_service(
            App::new()
                .wrap(NormalizePath::trim())
                .service(routes::new_service(&routes_state)),
        )
        .await
    });

    let req = TestRequest::delete()
        .uri("/broker/api/v1/application/id")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", TOKEN_MANAGER)))
        .to_request();
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    expect(resp.status()).to_equal(StatusCode::NO_CONTENT)?;
    let _ = get_application_model(runtime, &routes_state, "manager", true)?;

    let req = TestRequest::delete()
        .uri("/broker/api/v1/application/manager")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", TOKEN_OWNER)))
        .to_request();
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    expect(resp.status()).to_equal(StatusCode::NO_CONTENT)?;
    let _ = get_application_model(runtime, &routes_state, "manager", true)?;

    let req = TestRequest::delete()
        .uri("/broker/api/v1/application/manager")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", TOKEN_MEMBER)))
        .to_request();
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    expect(resp.status()).to_equal(StatusCode::NO_CONTENT)?;
    let _ = get_application_model(runtime, &routes_state, "manager", true)?;

    let req = TestRequest::delete()
        .uri("/broker/api/v1/application/owner")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", TOKEN_MEMBER)))
        .to_request();
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    expect(resp.status()).to_equal(StatusCode::NO_CONTENT)?;
    let _ = get_application_model(runtime, &routes_state, "owner", true)?;

    let req = TestRequest::delete()
        .uri("/broker/api/v1/application/manager")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", TOKEN_MANAGER)))
        .to_request();
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    expect(resp.status()).to_equal(StatusCode::NO_CONTENT)?;
    let _ = get_application_model(runtime, &routes_state, "manager", false)?;

    let req = TestRequest::delete()
        .uri("/broker/api/v1/application/owner")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", TOKEN_OWNER)))
        .to_request();
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    expect(resp.status()).to_equal(StatusCode::NO_CONTENT)?;
    let _ = get_application_model(runtime, &routes_state, "owner", false)?;

    let req = TestRequest::delete()
        .uri("/broker/api/v1/application/owner2")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", TOKEN_MANAGER)))
        .to_request();
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    expect(resp.status()).to_equal(StatusCode::NO_CONTENT)?;
    let _ = get_application_model(runtime, &routes_state, "owner2", false)?;

    libs::clear_all_data(runtime, state);
    add_delete_rsc(runtime, routes_state)?;
    let req = TestRequest::delete()
        .uri("/broker/api/v1/application/manager")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", TOKEN_MANAGER)))
        .to_request();
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    expect(resp.status()).to_equal(StatusCode::NO_CONTENT)?;
    let _ = get_unit_model(runtime, routes_state, "manager", true)?;
    let _ = get_unit_model(runtime, routes_state, "owner", true)?;
    let _ = get_application_model(runtime, routes_state, "manager", false)?;
    let _ = get_application_model(runtime, routes_state, "owner", true)?;
    let _ = get_network_model(runtime, routes_state, "public", true)?;
    let _ = get_network_model(runtime, routes_state, "manager", true)?;
    let _ = get_network_model(runtime, routes_state, "owner", true)?;
    let _ = get_device_model(runtime, routes_state, "manager-public", true)?;
    let _ = get_device_model(runtime, routes_state, "manager", true)?;
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
    let req = TestRequest::delete()
        .uri("/broker/api/v1/application/owner")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", TOKEN_OWNER)))
        .to_request();
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    expect(resp.status()).to_equal(StatusCode::NO_CONTENT)?;
    let _ = get_unit_model(runtime, routes_state, "manager", true)?;
    let _ = get_unit_model(runtime, routes_state, "owner", true)?;
    let _ = get_application_model(runtime, routes_state, "manager", true)?;
    let _ = get_application_model(runtime, routes_state, "owner", false)?;
    let _ = get_network_model(runtime, routes_state, "public", true)?;
    let _ = get_network_model(runtime, routes_state, "manager", true)?;
    let _ = get_network_model(runtime, routes_state, "owner", true)?;
    let _ = get_device_model(runtime, routes_state, "manager-public", true)?;
    let _ = get_device_model(runtime, routes_state, "manager", true)?;
    let _ = get_device_model(runtime, routes_state, "owner-public", true)?;
    let _ = get_device_model(runtime, routes_state, "owner1", true)?;
    let _ = get_device_model(runtime, routes_state, "owner2", true)?;
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

    let req = TestRequest::delete().uri("/broker/api/v1/application/id");
    test_invalid_token(runtime, &routes_state, req)
}

fn test_post(
    runtime: &Runtime,
    state: &routes::State,
    token: &str,
    param: &request::PostApplication,
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
        .uri("/broker/api/v1/application")
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
    let body: response::PostApplication =
        runtime.block_on(async { test::read_body_json(resp).await });
    expect(body.data.application_id.len() > 0).to_equal(true)?;

    let application_info = match runtime.block_on(async {
        let cond = QueryCond {
            application_id: Some(body.data.application_id.as_str()),
            ..Default::default()
        };
        state.model.application().get(&cond).await
    }) {
        Err(e) => return Err(format!("get application model error: {}", e)),
        Ok(info) => match info {
            None => return Err("add application then get none application".to_string()),
            Some(info) => info,
        },
    };
    expect(application_info.code.as_str()).to_equal(param.data.code.as_str())?;
    expect(application_info.unit_id.as_str()).to_equal(param.data.unit_id.as_str())?;
    expect(application_info.unit_code.as_str()).to_equal(param.data.unit_id.as_str())?;
    expect(application_info.created_at.ge(&time_before)).to_equal(true)?;
    expect(application_info.created_at.le(&time_after)).to_equal(true)?;
    expect(application_info.modified_at.ge(&time_before)).to_equal(true)?;
    expect(application_info.modified_at.le(&time_after)).to_equal(true)?;
    expect(application_info.host_uri.as_str()).to_equal(param.data.host_uri.as_str())?;
    match param.data.name.as_ref() {
        None => expect(application_info.name.len()).to_equal(0)?,
        Some(name) => expect(application_info.name.as_str()).to_equal(name.as_str())?,
    }
    match param.data.info.as_ref() {
        None => expect(application_info.info).to_equal(Map::<String, Value>::new()),
        Some(info) => expect(application_info.info).to_equal(info.clone()),
    }
}

fn test_post_invalid_param(
    runtime: &Runtime,
    state: &routes::State,
    token: &str,
    param: Option<&request::PostApplication>,
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
        .uri("/broker/api/v1/application")
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
    param: Option<&request::GetApplicationCount>,
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
        None => "/broker/api/v1/application/count".to_string(),
        Some(param) => format!(
            "/broker/api/v1/application/count?{}",
            serde_urlencoded::to_string(&param).unwrap()
        ),
    };
    let req = TestRequest::get()
        .uri(uri.as_str())
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)))
        .to_request();
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    expect(resp.status()).to_equal(StatusCode::OK)?;
    let body: response::GetApplicationCount =
        runtime.block_on(async { test::read_body_json(resp).await });
    expect(body.data.count).to_equal(expect_count)
}

fn test_get_list(
    runtime: &Runtime,
    state: &routes::State,
    token: &str,
    param: Option<&request::GetApplicationList>,
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
        None => "/broker/api/v1/application/list".to_string(),
        Some(param) => format!(
            "/broker/api/v1/application/list?{}",
            serde_urlencoded::to_string(&param).unwrap()
        ),
    };
    let req = TestRequest::get()
        .uri(uri.as_str())
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)))
        .to_request();
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    expect(resp.status()).to_equal(StatusCode::OK)?;
    let body: response::GetApplicationList =
        runtime.block_on(async { test::read_body_json(resp).await });
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
    param: &mut request::GetApplicationList,
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
        "/broker/api/v1/application/list?{}",
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
    let body: response::GetApplicationList =
        runtime.block_on(async { test::read_body_json(resp).await });
    expect(body.data.len()).to_equal(expect_ids.len())?;

    let mut index = 0;
    for data in body.data.iter() {
        expect(data.application_id.as_str()).to_equal(expect_ids[index])?;
        index += 1;
    }
    Ok(())
}

fn test_get_list_offset_limit(
    runtime: &Runtime,
    state: &routes::State,
    token: &str,
    param: &request::GetApplicationList,
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
        "/broker/api/v1/application/list?{}",
        serde_urlencoded::to_string(&param).unwrap()
    );
    let req = TestRequest::get()
        .uri(uri.as_str())
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)))
        .to_request();
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    expect(resp.status()).to_equal(StatusCode::OK)?;
    let body: response::GetApplicationList =
        runtime.block_on(async { test::read_body_json(resp).await });
    expect(body.data.len()).to_equal(expect_ids.len())?;

    let mut index = 0;
    for data in body.data.iter() {
        expect(data.application_id.as_str())
            .to_equal(format!("application_{}", expect_ids[index]).as_str())?;
        index += 1;
    }
    Ok(())
}

fn test_get_list_format_array(
    runtime: &Runtime,
    state: &routes::State,
    token: &str,
    param: &request::GetApplicationList,
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
        "/broker/api/v1/application/list?{}",
        serde_urlencoded::to_string(&param).unwrap()
    );
    let req = TestRequest::get()
        .uri(uri.as_str())
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)))
        .to_request();
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    expect(resp.status()).to_equal(StatusCode::OK)?;
    let body: Vec<response::GetApplicationListData> =
        runtime.block_on(async { test::read_body_json(resp).await });
    expect(body.len()).to_equal(expect_ids.len())?;

    let mut index = 0;
    for data in body.iter() {
        expect(data.application_id.as_str())
            .to_equal(format!("application_{}", expect_ids[index]).as_str())?;
        index += 1;
    }
    Ok(())
}

fn test_get(
    runtime: &Runtime,
    state: &routes::State,
    token: &str,
    application_id: &str,
) -> Result<(), String> {
    let mut app = runtime.block_on(async {
        test::init_service(
            App::new()
                .wrap(NormalizePath::trim())
                .service(routes::new_service(state)),
        )
        .await
    });

    let application_info = get_application_model(runtime, state, application_id, true)?.unwrap();

    let uri = format!("/broker/api/v1/application/{}", application_id);
    let req = TestRequest::get()
        .uri(uri.as_str())
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)))
        .to_request();
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    if let Err(_) = expect(resp.status()).to_equal(StatusCode::OK) {
        return Err(format!("token:{}, app: {}", token, application_id));
    }
    let body: response::GetApplication =
        runtime.block_on(async { test::read_body_json(resp).await });
    expect(body.data.application_id.as_str()).to_equal(application_info.application_id.as_str())?;
    expect(body.data.code.as_str()).to_equal(application_info.code.as_str())?;
    expect(body.data.unit_id.as_str()).to_equal(application_info.unit_id.as_str())?;
    expect(body.data.unit_code.as_str()).to_equal(application_info.unit_id.as_str())?;
    expect(
        DateTime::parse_from_rfc3339(body.data.created_at.as_str())
            .unwrap()
            .timestamp_millis(),
    )
    .to_equal(application_info.created_at.timestamp_millis())?;
    expect(
        DateTime::parse_from_rfc3339(body.data.modified_at.as_str())
            .unwrap()
            .timestamp_millis(),
    )
    .to_equal(application_info.modified_at.timestamp_millis())?;
    expect(body.data.host_uri.as_str()).to_equal(application_info.host_uri.as_str())?;
    expect(body.data.name.as_str()).to_equal(application_info.name.as_str())?;
    expect(body.data.info).to_equal(application_info.info)
}

fn test_get_wrong_id(
    runtime: &Runtime,
    state: &routes::State,
    token: &str,
    application_id: &str,
) -> Result<(), String> {
    let mut app = runtime.block_on(async {
        test::init_service(
            App::new()
                .wrap(NormalizePath::trim())
                .service(routes::new_service(state)),
        )
        .await
    });

    let req = TestRequest::get()
        .uri(format!("/broker/api/v1/application/{}", application_id).as_str())
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)))
        .to_request();
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    expect(resp.status()).to_equal(StatusCode::NOT_FOUND)?;
    let body: ApiError = runtime.block_on(async { test::read_body_json(resp).await });
    expect(body.code.as_str()).to_equal(err::E_NOT_FOUND)
}

fn test_patch(
    runtime: &Runtime,
    state: &routes::State,
    token: &str,
    unit_id: &str,
    application_id: &str,
) -> Result<(), String> {
    add_application_model(runtime, state, unit_id, application_id, "amqp://host")?;

    let mut app = runtime.block_on(async {
        test::init_service(
            App::new()
                .wrap(NormalizePath::trim())
                .service(routes::new_service(state)),
        )
        .await
    });

    let time_before = Utc::now().trunc_subsecs(3);
    let mut info = Map::<String, Value>::new();
    info.insert(
        "key_update".to_string(),
        Value::String("updated".to_string()),
    );
    let body = request::PatchApplication {
        data: request::PatchApplicationData {
            host_uri: Some("amqp://newhost".to_string()),
            name: Some("name changes".to_string()),
            info: Some(info.clone()),
        },
    };
    let req = TestRequest::patch()
        .uri(format!("/broker/api/v1/application/{}", application_id).as_str())
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)))
        .set_json(&body)
        .to_request();
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    expect(resp.status()).to_equal(StatusCode::NO_CONTENT)?;

    let time_after = Utc::now().trunc_subsecs(3);
    let application_info = get_application_model(runtime, state, application_id, true)?.unwrap();
    expect(application_info.modified_at.ge(&time_before)).to_equal(true)?;
    expect(application_info.modified_at.le(&time_after)).to_equal(true)?;
    expect(application_info.host_uri.as_str()).to_equal("amqp://newhost")?;
    expect(application_info.name.as_str()).to_equal("name changes")?;
    expect(application_info.info).to_equal(info)?;

    let body = request::PatchApplication {
        data: request::PatchApplicationData {
            name: Some("".to_string()),
            info: Some(Map::<String, Value>::new()),
            ..Default::default()
        },
    };
    let req = TestRequest::patch()
        .uri(format!("/broker/api/v1/application/{}", application_id).as_str())
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)))
        .set_json(&body)
        .to_request();
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    expect(resp.status()).to_equal(StatusCode::NO_CONTENT)?;

    let application_info = get_application_model(runtime, state, application_id, true)?.unwrap();
    expect(application_info.host_uri.as_str()).to_equal("amqp://newhost")?;
    expect(application_info.name.as_str()).to_equal("")?;
    expect(application_info.info).to_equal(Map::<String, Value>::new())
}

fn test_patch_wrong_id(
    runtime: &Runtime,
    state: &routes::State,
    token: &str,
    application_id: &str,
) -> Result<(), String> {
    let mut app = runtime.block_on(async {
        test::init_service(
            App::new()
                .wrap(NormalizePath::trim())
                .service(routes::new_service(state)),
        )
        .await
    });

    let body = request::PatchApplication {
        data: request::PatchApplicationData {
            name: Some("".to_string()),
            info: Some(Map::<String, Value>::new()),
            ..Default::default()
        },
    };
    let req = TestRequest::patch()
        .uri(format!("/broker/api/v1/application/{}", application_id).as_str())
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)))
        .set_json(&body)
        .to_request();
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    expect(resp.status()).to_equal(StatusCode::NOT_FOUND)?;
    let body: ApiError = runtime.block_on(async { test::read_body_json(resp).await });
    expect(body.code.as_str()).to_equal(err::E_NOT_FOUND)
}

fn test_patch_invalid_param(
    runtime: &Runtime,
    state: &routes::State,
    token: &str,
    application_id: &str,
    param: Option<&request::PatchApplication>,
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
        .uri(format!("/broker/api/v1/application/{}", application_id).as_str())
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

/// Returns (total, owner1, owner2, member) tuple.
fn count_list_dataset(
    runtime: &Runtime,
    state: &routes::State,
) -> Result<(usize, usize, usize, usize), String> {
    add_unit_model(runtime, state, "manager", vec!["member"], "manager")?;
    add_unit_model(runtime, state, "owner1", vec![], "owner")?;
    add_unit_model(runtime, state, "owner2", vec!["member"], "owner")?;
    let now = Utc::now();

    let mut application = create_application("manager", "amqp://host", "manager");
    application.created_at = now;
    application.modified_at = now;
    runtime.block_on(async {
        if let Err(e) = state.model.application().add(&application).await {
            return Err(format!(
                "add application {} error: {}",
                application.application_id, e
            ));
        }
        Ok(())
    })?;

    let mut application = create_application("owner1-1", "amqp://host", "owner1");
    application.created_at = now + Duration::milliseconds(1);
    application.modified_at = now + Duration::milliseconds(5);
    runtime.block_on(async {
        if let Err(e) = state.model.application().add(&application).await {
            return Err(format!(
                "add application {} error: {}",
                application.application_id, e
            ));
        }
        Ok(())
    })?;

    let mut application = create_application("owner1-2", "amqp://host", "owner1");
    application.created_at = now + Duration::milliseconds(2);
    application.modified_at = now + Duration::milliseconds(4);
    runtime.block_on(async {
        if let Err(e) = state.model.application().add(&application).await {
            return Err(format!(
                "add application {} error: {}",
                application.application_id, e
            ));
        }
        Ok(())
    })?;

    let mut application = create_application("owner1-3", "amqp://host", "owner1");
    application.created_at = now + Duration::milliseconds(3);
    application.modified_at = now + Duration::milliseconds(3);
    runtime.block_on(async {
        if let Err(e) = state.model.application().add(&application).await {
            return Err(format!(
                "add application {} error: {}",
                application.application_id, e
            ));
        }
        Ok(())
    })?;

    let mut application = create_application("owner2-1", "amqp://host", "owner2");
    application.created_at = now + Duration::milliseconds(4);
    application.modified_at = now + Duration::milliseconds(2);
    application.name = "owner2-2".to_string();
    runtime.block_on(async {
        if let Err(e) = state.model.application().add(&application).await {
            return Err(format!(
                "add application {} error: {}",
                application.application_id, e
            ));
        }
        Ok(())
    })?;

    let mut application = create_application("owner2-2", "amqp://host", "owner2");
    application.created_at = now + Duration::milliseconds(5);
    application.modified_at = now + Duration::milliseconds(1);
    application.name = "owner2-1".to_string();
    runtime.block_on(async {
        if let Err(e) = state.model.application().add(&application).await {
            return Err(format!(
                "add application {} error: {}",
                application.application_id, e
            ));
        }
        Ok(())
    })?;

    Ok((6, 3, 2, 3))
}
