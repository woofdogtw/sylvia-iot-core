use axum::{
    Router,
    http::{HeaderValue, Method, StatusCode, header},
};
use axum_test::TestServer;
use chrono::{DateTime, SubsecRound, TimeDelta, Utc};
use laboratory::{SpecContext, expect};
use serde_json::{Map, Value};
use tokio::runtime::Runtime;

use sylvia_iot_broker::{models::network::QueryCond, routes};
use sylvia_iot_corelib::err;

use super::{
    super::{
        super::libs::{
            ApiError, add_delete_rsc, add_network_model, add_unit_model, create_network,
            get_application_model, get_device_model, get_device_route_model,
            get_dldata_buffer_model, get_network_model, get_network_route_model, get_unit_model,
            test_get_400, test_invalid_token,
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

    add_unit_model(runtime, routes_state, "manager", vec![], "manager")?;
    add_unit_model(runtime, routes_state, "owner", vec![], "owner")?;

    let mut info = Map::<String, Value>::new();
    info.insert("info".to_string(), Value::String("value".to_string()));
    let param = request::PostNetwork {
        data: request::PostNetworkData {
            code: "manager".to_string(),
            unit_id: Some("manager".to_string()),
            host_uri: "amqp://host".to_string(),
            name: Some("manager".to_string()),
            info: Some(info),
        },
    };
    test_post(runtime, routes_state, TOKEN_MANAGER, &param, "")?;

    let param = request::PostNetwork {
        data: request::PostNetworkData {
            code: "public".to_string(),
            unit_id: None,
            host_uri: "amqp://host".to_string(),
            name: None,
            info: Some(Map::<String, Value>::new()),
        },
    };
    test_post(runtime, routes_state, TOKEN_MANAGER, &param, "")?;

    let param = request::PostNetwork {
        data: request::PostNetworkData {
            code: "owner".to_string(),
            unit_id: Some("owner".to_string()),
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

    let param = request::PostNetwork {
        data: request::PostNetworkData {
            code: "public".to_string(),
            unit_id: None,
            host_uri: "amqp://host".to_string(),
            name: None,
            info: None,
        },
    };
    test_post(runtime, routes_state, TOKEN_MANAGER, &param, "")?;

    let param = request::PostNetwork {
        data: request::PostNetworkData {
            code: "public".to_string(),
            unit_id: None,
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
        "err_broker_network_exist",
    )?;

    let param = request::PostNetwork {
        data: request::PostNetworkData {
            code: "manager".to_string(),
            unit_id: Some("manager".to_string()),
            host_uri: "amqp://host".to_string(),
            name: None,
            info: None,
        },
    };
    test_post(runtime, routes_state, TOKEN_MANAGER, &param, "")?;

    let param = request::PostNetwork {
        data: request::PostNetworkData {
            code: "manager".to_string(),
            unit_id: Some("manager".to_string()),
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
        "err_broker_network_exist",
    )
}

pub fn post_not_exist_unit(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    add_unit_model(runtime, routes_state, "manager", vec!["member"], "manager")?;

    let param = request::PostNetwork {
        data: request::PostNetworkData {
            code: "manager".to_string(),
            unit_id: Some("not-exist".to_string()),
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

    let param = request::PostNetwork {
        data: request::PostNetworkData {
            code: "manager".to_string(),
            unit_id: Some("manager".to_string()),
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
    add_unit_model(runtime, routes_state, "owner", vec![], "owner")?;

    test_post_invalid_param(runtime, &routes_state, TOKEN_MANAGER, None)?;

    let param = request::PostNetwork {
        data: request::PostNetworkData {
            code: "".to_string(),
            unit_id: Some("manager".to_string()),
            host_uri: "amqp://host".to_string(),
            name: None,
            info: None,
        },
    };
    test_post_invalid_param(runtime, &routes_state, TOKEN_MANAGER, Some(&param))?;

    let param = request::PostNetwork {
        data: request::PostNetworkData {
            code: "code".to_string(),
            unit_id: Some("".to_string()),
            host_uri: "amqp://host".to_string(),
            name: None,
            info: None,
        },
    };
    test_post_invalid_param(runtime, &routes_state, TOKEN_MANAGER, Some(&param))?;

    let param = request::PostNetwork {
        data: request::PostNetworkData {
            code: "code".to_string(),
            unit_id: None,
            host_uri: "amqp://host".to_string(),
            name: None,
            info: None,
        },
    };
    test_post_invalid_param(runtime, &routes_state, TOKEN_OWNER, Some(&param))?;

    let param = request::PostNetwork {
        data: request::PostNetworkData {
            code: "code".to_string(),
            unit_id: Some("manager".to_string()),
            host_uri: "".to_string(),
            name: None,
            info: None,
        },
    };
    test_post_invalid_param(runtime, &routes_state, TOKEN_MANAGER, Some(&param))?;

    let param = request::PostNetwork {
        data: request::PostNetworkData {
            code: "code".to_string(),
            unit_id: Some("manager".to_string()),
            host_uri: "http://host".to_string(),
            name: None,
            info: None,
        },
    };
    test_post_invalid_param(runtime, &routes_state, TOKEN_MANAGER, Some(&param))?;

    let mut info = Map::<String, Value>::new();
    info.insert("".to_string(), Value::String("value".to_string()));
    let param = request::PostNetwork {
        data: request::PostNetworkData {
            code: "code".to_string(),
            unit_id: Some("manager".to_string()),
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

    test_invalid_token(
        runtime,
        &routes_state,
        Method::POST,
        "/broker/api/v1/network",
    )
}

pub fn get_count(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    let data_size = count_list_dataset(runtime, &routes_state)?;

    test_get_count(runtime, &routes_state, TOKEN_MANAGER, None, data_size.0)?;

    let param = request::GetNetworkCount {
        unit: Some("".to_string()),
        ..Default::default()
    };
    test_get_count(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 1)?;

    let param = request::GetNetworkCount {
        unit: Some("manager".to_string()),
        ..Default::default()
    };
    test_get_count(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 1)?;
    test_get_count(runtime, &routes_state, TOKEN_MEMBER, Some(&param), 1)?;

    let param = request::GetNetworkCount {
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

    let param = request::GetNetworkCount {
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

    let param = request::GetNetworkCount {
        code: Some("owner2-1".to_string()),
        ..Default::default()
    };
    test_get_count(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 1)?;

    let param = request::GetNetworkCount {
        unit: Some("owner1".to_string()),
        code: Some("owner2-1".to_string()),
        ..Default::default()
    };
    test_get_count(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 0)?;
    test_get_count(runtime, &routes_state, TOKEN_OWNER, Some(&param), 0)?;

    let param = request::GetNetworkCount {
        unit: Some("owner2".to_string()),
        code: Some("owner2-1".to_string()),
        ..Default::default()
    };
    test_get_count(runtime, &routes_state, TOKEN_OWNER, Some(&param), 1)?;

    let param = request::GetNetworkCount {
        contains: Some("2".to_string()),
        ..Default::default()
    };
    test_get_count(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 3)?;

    let param = request::GetNetworkCount {
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
    add_network_model(runtime, routes_state, "", "public", "amqp://host")?;

    let uri = "/broker/api/v1/network/count";
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

    let uri = "/broker/api/v1/network/count";
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

    test_invalid_token(
        runtime,
        &routes_state,
        Method::GET,
        "/broker/api/v1/network/count",
    )
}

pub fn get_list(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    let data_size = count_list_dataset(runtime, &routes_state)?;

    test_get_list(runtime, &routes_state, TOKEN_MANAGER, None, data_size.0)?;

    let param = request::GetNetworkList {
        unit: Some("".to_string()),
        ..Default::default()
    };
    test_get_list(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 1)?;

    let param = request::GetNetworkList {
        unit: Some("manager".to_string()),
        ..Default::default()
    };
    test_get_list(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 1)?;
    test_get_list(runtime, &routes_state, TOKEN_MEMBER, Some(&param), 1)?;

    let param = request::GetNetworkList {
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

    let param = request::GetNetworkList {
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

    let param = request::GetNetworkList {
        code: Some("owner2-1".to_string()),
        ..Default::default()
    };
    test_get_list(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 1)?;

    let param = request::GetNetworkList {
        unit: Some("owner1".to_string()),
        code: Some("owner2-1".to_string()),
        ..Default::default()
    };
    test_get_list(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 0)?;
    test_get_list(runtime, &routes_state, TOKEN_OWNER, Some(&param), 0)?;

    let param = request::GetNetworkList {
        unit: Some("owner2".to_string()),
        code: Some("owner2-1".to_string()),
        ..Default::default()
    };
    test_get_list(runtime, &routes_state, TOKEN_OWNER, Some(&param), 1)?;

    let param = request::GetNetworkList {
        contains: Some("2".to_string()),
        ..Default::default()
    };
    test_get_list(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 3)?;

    let param = request::GetNetworkList {
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

    let mut param = request::GetNetworkList {
        ..Default::default()
    };
    test_get_list_sort(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        &mut param,
        &[
            "manager", "owner1-1", "owner1-2", "owner1-3", "owner2-1", "owner2-2", "public",
        ],
    )?;

    param.sort_vec = Some(vec![("code", true)]);
    test_get_list_sort(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        &mut param,
        &[
            "manager", "owner1-1", "owner1-2", "owner1-3", "owner2-1", "owner2-2", "public",
        ],
    )?;
    param.sort_vec = Some(vec![("code", false)]);
    test_get_list_sort(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        &mut param,
        &[
            "public", "owner2-2", "owner2-1", "owner1-3", "owner1-2", "owner1-1", "manager",
        ],
    )?;

    param.sort_vec = Some(vec![("name", true)]);
    test_get_list_sort(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        &mut param,
        &[
            "manager", "owner1-1", "owner1-2", "owner1-3", "owner2-2", "owner2-1", "public",
        ],
    )?;
    param.sort_vec = Some(vec![("name", false)]);
    test_get_list_sort(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        &mut param,
        &[
            "public", "owner2-1", "owner2-2", "owner1-3", "owner1-2", "owner1-1", "manager",
        ],
    )?;

    param.sort_vec = Some(vec![("created", true)]);
    test_get_list_sort(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        &mut param,
        &[
            "public", "manager", "owner1-1", "owner1-2", "owner1-3", "owner2-1", "owner2-2",
        ],
    )?;
    param.sort_vec = Some(vec![("created", false)]);
    test_get_list_sort(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        &mut param,
        &[
            "owner2-2", "owner2-1", "owner1-3", "owner1-2", "owner1-1", "manager", "public",
        ],
    )?;

    param.sort_vec = Some(vec![("modified", true)]);
    test_get_list_sort(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        &mut param,
        &[
            "public", "manager", "owner2-2", "owner2-1", "owner1-3", "owner1-2", "owner1-1",
        ],
    )?;
    param.sort_vec = Some(vec![("modified", false)]);
    test_get_list_sort(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        &mut param,
        &[
            "owner1-1", "owner1-2", "owner1-3", "owner2-1", "owner2-2", "manager", "public",
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
        add_network_model(
            runtime,
            &routes_state,
            unit_id,
            format!("network_{}", i).as_str(),
            "amqp://host",
        )?;
    }

    let mut param = request::GetNetworkList {
        unit: Some(unit_id.to_string()),
        contains: Some("N".to_string()),
        ..Default::default()
    };
    test_get_list_offset_limit(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        &param,
        (100..200).collect(),
    )?;

    param.unit = None;
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
    )?;

    for i in 400..602 {
        add_network_model(
            runtime,
            &routes_state,
            "",
            format!("network_{}", i).as_str(),
            "amqp://host",
        )?;
    }

    let mut param = request::GetNetworkList {
        unit: Some("".to_string()),
        contains: Some("N".to_string()),
        ..Default::default()
    };
    test_get_list_offset_limit(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        &param,
        (400..500).collect(),
    )?;

    param.limit = Some(0);
    test_get_list_offset_limit(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        &param,
        (400..602).collect(),
    )?;

    param.unit = None;
    let mut vec1: Vec<i32> = (100..302).collect();
    let vec2: Vec<i32> = (400..602).collect();
    vec1.extend(&vec2);
    test_get_list_offset_limit(runtime, &routes_state, TOKEN_MANAGER, &param, vec1)
}

pub fn get_list_format_array(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    add_unit_model(runtime, routes_state, "manager", vec![], "manager")?;

    let unit_id = "manager";

    for i in 100..302 {
        add_network_model(
            runtime,
            &routes_state,
            unit_id,
            format!("network_{}", i).as_str(),
            "amqp://host",
        )?;
    }

    let mut param = request::GetNetworkList {
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
    add_network_model(runtime, routes_state, "", "public", "amqp://host")?;

    let uri = "/broker/api/v1/network/list";
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

    let uri = "/broker/api/v1/network/list";
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

    test_invalid_token(
        runtime,
        &routes_state,
        Method::GET,
        "/broker/api/v1/network/list",
    )
}

pub fn get(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    add_unit_model(runtime, routes_state, "manager", vec!["member"], "manager")?;
    add_unit_model(runtime, routes_state, "owner1", vec![], "owner")?;
    add_unit_model(runtime, routes_state, "owner2", vec!["member"], "owner")?;
    add_network_model(runtime, routes_state, "manager", "manager", "amqp://host")?;
    add_network_model(runtime, routes_state, "", "public", "amqp://host")?;
    add_network_model(runtime, routes_state, "owner1", "owner1", "amqp://host")?;
    add_network_model(runtime, routes_state, "owner2", "owner2", "amqp://host")?;

    test_get(runtime, routes_state, TOKEN_MANAGER, "manager")?;
    test_get(runtime, routes_state, TOKEN_MANAGER, "public")?;
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
    add_network_model(runtime, routes_state, "manager", "manager", "amqp://host")?;
    add_network_model(runtime, routes_state, "", "public", "amqp://host")?;
    add_network_model(runtime, routes_state, "owner1", "owner1", "amqp://host")?;
    add_network_model(runtime, routes_state, "owner2", "owner2", "amqp://host")?;

    test_get_wrong_id(runtime, routes_state, TOKEN_MANAGER, "manager1")?;
    test_get_wrong_id(runtime, routes_state, TOKEN_OWNER, "public")?;
    test_get_wrong_id(runtime, routes_state, TOKEN_OWNER, "manager")?;
    test_get_wrong_id(runtime, routes_state, TOKEN_MEMBER, "public")?;
    test_get_wrong_id(runtime, routes_state, TOKEN_MEMBER, "owner1")
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
        "/broker/api/v1/network/id",
    )
}

pub fn patch(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    add_unit_model(runtime, routes_state, "manager", vec![], "manager")?;
    add_unit_model(runtime, routes_state, "owner", vec![], "owner")?;

    test_patch(runtime, routes_state, TOKEN_MANAGER, "manager", "manager")?;
    test_patch(runtime, routes_state, TOKEN_MANAGER, "", "public")?;
    test_patch(runtime, routes_state, TOKEN_OWNER, "owner", "owner")
}

pub fn patch_wrong_id(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    add_unit_model(runtime, routes_state, "manager", vec!["member"], "manager")?;
    add_unit_model(runtime, routes_state, "", vec!["member"], "public")?;

    test_patch_wrong_id(runtime, routes_state, TOKEN_MANAGER, "manager1")?;
    test_patch_wrong_id(runtime, routes_state, TOKEN_OWNER, "manager")?;
    test_patch_wrong_id(runtime, routes_state, TOKEN_OWNER, "public")?;
    test_patch_wrong_id(runtime, routes_state, TOKEN_MEMBER, "manager")?;
    test_patch_wrong_id(runtime, routes_state, TOKEN_MEMBER, "public")
}

pub fn patch_invalid_param(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    add_unit_model(runtime, routes_state, "manager", vec![], "manager")?;
    add_network_model(runtime, routes_state, "manager", "manager", "amqp://host")?;

    test_patch_invalid_param(runtime, routes_state, TOKEN_MANAGER, "manager", None)?;

    let param = request::PatchNetwork {
        ..Default::default()
    };
    test_patch_invalid_param(
        runtime,
        routes_state,
        TOKEN_MANAGER,
        "manager",
        Some(&param),
    )?;

    let param = request::PatchNetwork {
        data: request::PatchNetworkData {
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

    let param = request::PatchNetwork {
        data: request::PatchNetworkData {
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

    let param = request::PatchNetwork {
        data: request::PatchNetworkData {
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

    let param = request::PatchNetwork {
        data: request::PatchNetworkData {
            host_uri: Some("http://host".to_string()),
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
    let param = request::PatchNetwork {
        data: request::PatchNetworkData {
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
        "/broker/api/v1/network/id",
    )
}

pub fn delete(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    add_unit_model(runtime, routes_state, "manager", vec!["member"], "manager")?;
    add_unit_model(runtime, routes_state, "owner", vec!["member"], "owner")?;
    add_network_model(runtime, routes_state, "manager", "manager", "amqp://host")?;
    add_network_model(runtime, routes_state, "", "public", "amqp://host")?;
    add_network_model(runtime, routes_state, "owner", "owner", "amqp://host")?;
    add_network_model(runtime, routes_state, "owner", "owner2", "amqp://host")?;

    let app = Router::new().merge(routes::new_service(routes_state));
    let server = match TestServer::new(app) {
        Err(e) => return Err(format!("new server error: {}", e)),
        Ok(server) => server,
    };

    let req = server.delete("/broker/api/v1/network/id").add_header(
        header::AUTHORIZATION,
        HeaderValue::from_str(format!("Bearer {}", TOKEN_MANAGER).as_str()).unwrap(),
    );
    let resp = runtime.block_on(async { req.await });
    expect(resp.status_code()).to_equal(StatusCode::NO_CONTENT)?;
    let _ = get_network_model(runtime, &routes_state, "manager", true)?;

    let req = server.delete("/broker/api/v1/network/manager").add_header(
        header::AUTHORIZATION,
        HeaderValue::from_str(format!("Bearer {}", TOKEN_OWNER).as_str()).unwrap(),
    );
    let resp = runtime.block_on(async { req.await });
    expect(resp.status_code()).to_equal(StatusCode::NO_CONTENT)?;
    let _ = get_network_model(runtime, &routes_state, "manager", true)?;

    let req = server.delete("/broker/api/v1/network/manager").add_header(
        header::AUTHORIZATION,
        HeaderValue::from_str(format!("Bearer {}", TOKEN_MEMBER).as_str()).unwrap(),
    );
    let resp = runtime.block_on(async { req.await });
    expect(resp.status_code()).to_equal(StatusCode::NO_CONTENT)?;
    let _ = get_network_model(runtime, &routes_state, "manager", true)?;

    let req = server.delete("/broker/api/v1/network/public").add_header(
        header::AUTHORIZATION,
        HeaderValue::from_str(format!("Bearer {}", TOKEN_MEMBER).as_str()).unwrap(),
    );
    let resp = runtime.block_on(async { req.await });
    expect(resp.status_code()).to_equal(StatusCode::NO_CONTENT)?;
    let _ = get_network_model(runtime, &routes_state, "public", true)?;

    let req = server.delete("/broker/api/v1/network/owner").add_header(
        header::AUTHORIZATION,
        HeaderValue::from_str(format!("Bearer {}", TOKEN_MEMBER).as_str()).unwrap(),
    );
    let resp = runtime.block_on(async { req.await });
    expect(resp.status_code()).to_equal(StatusCode::NO_CONTENT)?;
    let _ = get_network_model(runtime, &routes_state, "owner", true)?;

    let req = server.delete("/broker/api/v1/network/manager").add_header(
        header::AUTHORIZATION,
        HeaderValue::from_str(format!("Bearer {}", TOKEN_MANAGER).as_str()).unwrap(),
    );
    let resp = runtime.block_on(async { req.await });
    expect(resp.status_code()).to_equal(StatusCode::NO_CONTENT)?;
    let _ = get_network_model(runtime, &routes_state, "manager", false)?;

    let req = server.delete("/broker/api/v1/network/owner").add_header(
        header::AUTHORIZATION,
        HeaderValue::from_str(format!("Bearer {}", TOKEN_OWNER).as_str()).unwrap(),
    );
    let resp = runtime.block_on(async { req.await });
    expect(resp.status_code()).to_equal(StatusCode::NO_CONTENT)?;
    let _ = get_network_model(runtime, &routes_state, "owner", false)?;

    let req = server.delete("/broker/api/v1/network/owner2").add_header(
        header::AUTHORIZATION,
        HeaderValue::from_str(format!("Bearer {}", TOKEN_MANAGER).as_str()).unwrap(),
    );
    let resp = runtime.block_on(async { req.await });
    expect(resp.status_code()).to_equal(StatusCode::NO_CONTENT)?;
    let _ = get_network_model(runtime, &routes_state, "owner2", false)?;

    libs::clear_all_data(runtime, state);
    add_delete_rsc(runtime, routes_state)?;
    let req = server.delete("/broker/api/v1/network/public").add_header(
        header::AUTHORIZATION,
        HeaderValue::from_str(format!("Bearer {}", TOKEN_MANAGER).as_str()).unwrap(),
    );
    let resp = runtime.block_on(async { req.await });
    expect(resp.status_code()).to_equal(StatusCode::NO_CONTENT)?;
    let _ = get_unit_model(runtime, routes_state, "manager", true)?;
    let _ = get_unit_model(runtime, routes_state, "owner", true)?;
    let _ = get_application_model(runtime, routes_state, "manager", true)?;
    let _ = get_application_model(runtime, routes_state, "owner", true)?;
    let _ = get_network_model(runtime, routes_state, "public", false)?;
    let _ = get_network_model(runtime, routes_state, "manager", true)?;
    let _ = get_network_model(runtime, routes_state, "owner", true)?;
    let _ = get_device_model(runtime, routes_state, "manager-public", false)?;
    let _ = get_device_model(runtime, routes_state, "manager", true)?;
    let _ = get_device_model(runtime, routes_state, "owner-public", false)?;
    let _ = get_device_model(runtime, routes_state, "owner1", true)?;
    let _ = get_device_model(runtime, routes_state, "owner2", true)?;
    let _ = get_network_route_model(runtime, routes_state, "public-manager", false)?;
    let _ = get_network_route_model(runtime, routes_state, "manager-manager", true)?;
    let _ = get_network_route_model(runtime, routes_state, "owner-owner", true)?;
    let _ = get_device_route_model(runtime, routes_state, "manager-public-manager", false)?;
    let _ = get_device_route_model(runtime, routes_state, "manager-manager", true)?;
    let _ = get_device_route_model(runtime, routes_state, "owner-public-owner", false)?;
    let _ = get_device_route_model(runtime, routes_state, "owner1-owner", true)?;
    let _ = get_device_route_model(runtime, routes_state, "owner2-owner", true)?;
    let _ = get_dldata_buffer_model(runtime, routes_state, "manager-public-manager", false)?;
    let _ = get_dldata_buffer_model(runtime, routes_state, "manager-manager", true)?;
    let _ = get_dldata_buffer_model(runtime, routes_state, "owner-public-owner", false)?;
    let _ = get_dldata_buffer_model(runtime, routes_state, "owner1-owner", true)?;
    let _ = get_dldata_buffer_model(runtime, routes_state, "owner2-owner", true)?;

    libs::clear_all_data(runtime, state);
    add_delete_rsc(runtime, routes_state)?;
    let req = server.delete("/broker/api/v1/network/manager").add_header(
        header::AUTHORIZATION,
        HeaderValue::from_str(format!("Bearer {}", TOKEN_MANAGER).as_str()).unwrap(),
    );
    let resp = runtime.block_on(async { req.await });
    expect(resp.status_code()).to_equal(StatusCode::NO_CONTENT)?;
    let _ = get_unit_model(runtime, routes_state, "manager", true)?;
    let _ = get_unit_model(runtime, routes_state, "owner", true)?;
    let _ = get_application_model(runtime, routes_state, "manager", true)?;
    let _ = get_application_model(runtime, routes_state, "owner", true)?;
    let _ = get_network_model(runtime, routes_state, "public", true)?;
    let _ = get_network_model(runtime, routes_state, "manager", false)?;
    let _ = get_network_model(runtime, routes_state, "owner", true)?;
    let _ = get_device_model(runtime, routes_state, "manager-public", true)?;
    let _ = get_device_model(runtime, routes_state, "manager", false)?;
    let _ = get_device_model(runtime, routes_state, "owner-public", true)?;
    let _ = get_device_model(runtime, routes_state, "owner1", true)?;
    let _ = get_device_model(runtime, routes_state, "owner2", true)?;
    let _ = get_network_route_model(runtime, routes_state, "public-manager", true)?;
    let _ = get_network_route_model(runtime, routes_state, "manager-manager", false)?;
    let _ = get_network_route_model(runtime, routes_state, "owner-owner", true)?;
    let _ = get_device_route_model(runtime, routes_state, "manager-public-manager", true)?;
    let _ = get_device_route_model(runtime, routes_state, "manager-manager", false)?;
    let _ = get_device_route_model(runtime, routes_state, "owner-public-owner", true)?;
    let _ = get_device_route_model(runtime, routes_state, "owner1-owner", true)?;
    let _ = get_device_route_model(runtime, routes_state, "owner2-owner", true)?;
    let _ = get_dldata_buffer_model(runtime, routes_state, "manager-public-manager", true)?;
    let _ = get_dldata_buffer_model(runtime, routes_state, "manager-manager", false)?;
    let _ = get_dldata_buffer_model(runtime, routes_state, "owner-public-owner", true)?;
    let _ = get_dldata_buffer_model(runtime, routes_state, "owner1-owner", true)?;
    let _ = get_dldata_buffer_model(runtime, routes_state, "owner2-owner", true)?;

    libs::clear_all_data(runtime, state);
    add_delete_rsc(runtime, routes_state)?;
    let req = server.delete("/broker/api/v1/network/owner").add_header(
        header::AUTHORIZATION,
        HeaderValue::from_str(format!("Bearer {}", TOKEN_OWNER).as_str()).unwrap(),
    );
    let resp = runtime.block_on(async { req.await });
    expect(resp.status_code()).to_equal(StatusCode::NO_CONTENT)?;
    let _ = get_unit_model(runtime, routes_state, "manager", true)?;
    let _ = get_unit_model(runtime, routes_state, "owner", true)?;
    let _ = get_application_model(runtime, routes_state, "manager", true)?;
    let _ = get_application_model(runtime, routes_state, "owner", true)?;
    let _ = get_network_model(runtime, routes_state, "public", true)?;
    let _ = get_network_model(runtime, routes_state, "manager", true)?;
    let _ = get_network_model(runtime, routes_state, "owner", false)?;
    let _ = get_device_model(runtime, routes_state, "manager-public", true)?;
    let _ = get_device_model(runtime, routes_state, "manager", true)?;
    let _ = get_device_model(runtime, routes_state, "owner-public", true)?;
    let _ = get_device_model(runtime, routes_state, "owner1", false)?;
    let _ = get_device_model(runtime, routes_state, "owner2", false)?;
    let _ = get_network_route_model(runtime, routes_state, "public-manager", true)?;
    let _ = get_network_route_model(runtime, routes_state, "manager-manager", true)?;
    let _ = get_network_route_model(runtime, routes_state, "owner-owner", false)?;
    let _ = get_device_route_model(runtime, routes_state, "manager-public-manager", true)?;
    let _ = get_device_route_model(runtime, routes_state, "manager-manager", true)?;
    let _ = get_device_route_model(runtime, routes_state, "owner-public-owner", true)?;
    let _ = get_device_route_model(runtime, routes_state, "owner1-owner", false)?;
    let _ = get_device_route_model(runtime, routes_state, "owner2-owner", false)?;
    let _ = get_dldata_buffer_model(runtime, routes_state, "manager-public-manager", true)?;
    let _ = get_dldata_buffer_model(runtime, routes_state, "manager-manager", true)?;
    let _ = get_dldata_buffer_model(runtime, routes_state, "owner-public-owner", true)?;
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
        "/broker/api/v1/network/id",
    )
}

fn test_post(
    runtime: &Runtime,
    state: &routes::State,
    token: &str,
    param: &request::PostNetwork,
    expect_code: &str,
) -> Result<(), String> {
    let app = Router::new().merge(routes::new_service(state));
    let server = match TestServer::new(app) {
        Err(e) => return Err(format!("new server error: {}", e)),
        Ok(server) => server,
    };

    let time_before = Utc::now().trunc_subsecs(3);
    let req = server
        .post("/broker/api/v1/network")
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
    let body: response::PostNetwork = resp.json();
    expect(body.data.network_id.len() > 0).to_equal(true)?;

    let network_info = match runtime.block_on(async {
        let cond = QueryCond {
            network_id: Some(body.data.network_id.as_str()),
            ..Default::default()
        };
        state.model.network().get(&cond).await
    }) {
        Err(e) => return Err(format!("get network model error: {}", e)),
        Ok(info) => match info {
            None => return Err("add network then get none network".to_string()),
            Some(info) => info,
        },
    };
    expect(network_info.code.as_str()).to_equal(param.data.code.as_str())?;
    expect(network_info.unit_id.as_ref()).to_equal(param.data.unit_id.as_ref())?;
    expect(network_info.unit_code.as_ref()).to_equal(param.data.unit_id.as_ref())?;
    expect(network_info.created_at.ge(&time_before)).to_equal(true)?;
    expect(network_info.created_at.le(&time_after)).to_equal(true)?;
    expect(network_info.modified_at.ge(&time_before)).to_equal(true)?;
    expect(network_info.modified_at.le(&time_after)).to_equal(true)?;
    expect(network_info.host_uri.as_str()).to_equal(param.data.host_uri.as_str())?;
    match param.data.name.as_ref() {
        None => expect(network_info.name.len()).to_equal(0)?,
        Some(name) => expect(network_info.name.as_str()).to_equal(name.as_str())?,
    }
    match param.data.info.as_ref() {
        None => expect(network_info.info).to_equal(Map::<String, Value>::new()),
        Some(info) => expect(network_info.info).to_equal(info.clone()),
    }
}

fn test_post_invalid_param(
    runtime: &Runtime,
    state: &routes::State,
    token: &str,
    param: Option<&request::PostNetwork>,
) -> Result<(), String> {
    let app = Router::new().merge(routes::new_service(state));
    let server = match TestServer::new(app) {
        Err(e) => return Err(format!("new server error: {}", e)),
        Ok(server) => server,
    };

    let req = server
        .post("/broker/api/v1/network")
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
    param: Option<&request::GetNetworkCount>,
    expect_count: usize,
) -> Result<(), String> {
    let app = Router::new().merge(routes::new_service(state));
    let server = match TestServer::new(app) {
        Err(e) => return Err(format!("new server error: {}", e)),
        Ok(server) => server,
    };

    let req = server
        .get("/broker/api/v1/network/count")
        .add_query_params(&param)
        .add_header(
            header::AUTHORIZATION,
            HeaderValue::from_str(format!("Bearer {}", token).as_str()).unwrap(),
        );
    let resp = runtime.block_on(async { req.await });
    expect(resp.status_code()).to_equal(StatusCode::OK)?;
    let body: response::GetNetworkCount = resp.json();
    expect(body.data.count).to_equal(expect_count)
}

fn test_get_list(
    runtime: &Runtime,
    state: &routes::State,
    token: &str,
    param: Option<&request::GetNetworkList>,
    expect_count: usize,
) -> Result<(), String> {
    let app = Router::new().merge(routes::new_service(state));
    let server = match TestServer::new(app) {
        Err(e) => return Err(format!("new server error: {}", e)),
        Ok(server) => server,
    };

    let req = server
        .get("/broker/api/v1/network/list")
        .add_query_params(&param)
        .add_header(
            header::AUTHORIZATION,
            HeaderValue::from_str(format!("Bearer {}", token).as_str()).unwrap(),
        );
    let resp = runtime.block_on(async { req.await });
    expect(resp.status_code()).to_equal(StatusCode::OK)?;
    let body: response::GetNetworkList = resp.json();
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
    param: &mut request::GetNetworkList,
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
        .get("/broker/api/v1/network/list")
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
            "response not 200: /broker/api/v1/network/list, {}, {}",
            body.code, message
        ));
    }
    let body: response::GetNetworkList = resp.json();
    expect(body.data.len()).to_equal(expect_ids.len())?;

    let mut index = 0;
    for data in body.data.iter() {
        expect(data.network_id.as_str()).to_equal(expect_ids[index])?;
        index += 1;
    }
    Ok(())
}

fn test_get_list_offset_limit(
    runtime: &Runtime,
    state: &routes::State,
    token: &str,
    param: &request::GetNetworkList,
    expect_ids: Vec<i32>,
) -> Result<(), String> {
    let app = Router::new().merge(routes::new_service(state));
    let server = match TestServer::new(app) {
        Err(e) => return Err(format!("new server error: {}", e)),
        Ok(server) => server,
    };

    let req = server
        .get("/broker/api/v1/network/list")
        .add_query_params(&param)
        .add_header(
            header::AUTHORIZATION,
            HeaderValue::from_str(format!("Bearer {}", token).as_str()).unwrap(),
        );
    let resp = runtime.block_on(async { req.await });
    expect(resp.status_code()).to_equal(StatusCode::OK)?;
    let body: response::GetNetworkList = resp.json();
    expect(body.data.len()).to_equal(expect_ids.len())?;

    let mut index = 0;
    for data in body.data.iter() {
        expect(data.network_id.as_str())
            .to_equal(format!("network_{}", expect_ids[index]).as_str())?;
        index += 1;
    }
    Ok(())
}

fn test_get_list_format_array(
    runtime: &Runtime,
    state: &routes::State,
    token: &str,
    param: &request::GetNetworkList,
    expect_ids: Vec<i32>,
) -> Result<(), String> {
    let app = Router::new().merge(routes::new_service(state));
    let server = match TestServer::new(app) {
        Err(e) => return Err(format!("new server error: {}", e)),
        Ok(server) => server,
    };

    let req = server
        .get("/broker/api/v1/network/list")
        .add_query_params(&param)
        .add_header(
            header::AUTHORIZATION,
            HeaderValue::from_str(format!("Bearer {}", token).as_str()).unwrap(),
        );
    let resp = runtime.block_on(async { req.await });
    expect(resp.status_code()).to_equal(StatusCode::OK)?;
    let body: Vec<response::GetNetworkListData> = resp.json();
    expect(body.len()).to_equal(expect_ids.len())?;

    let mut index = 0;
    for data in body.iter() {
        expect(data.network_id.as_str())
            .to_equal(format!("network_{}", expect_ids[index]).as_str())?;
        index += 1;
    }
    Ok(())
}

fn test_get(
    runtime: &Runtime,
    state: &routes::State,
    token: &str,
    network_id: &str,
) -> Result<(), String> {
    let app = Router::new().merge(routes::new_service(state));
    let server = match TestServer::new(app) {
        Err(e) => return Err(format!("new server error: {}", e)),
        Ok(server) => server,
    };

    let network_info = get_network_model(runtime, state, network_id, true)?.unwrap();

    let req = server
        .get(format!("/broker/api/v1/network/{}", network_id).as_str())
        .add_header(
            header::AUTHORIZATION,
            HeaderValue::from_str(format!("Bearer {}", token).as_str()).unwrap(),
        );
    let resp = runtime.block_on(async { req.await });
    if let Err(_) = expect(resp.status_code()).to_equal(StatusCode::OK) {
        return Err(format!("token:{}, app: {}", token, network_id));
    }
    let body: response::GetNetwork = resp.json();
    expect(body.data.network_id.as_str()).to_equal(network_info.network_id.as_str())?;
    expect(body.data.code.as_str()).to_equal(network_info.code.as_str())?;
    expect(body.data.unit_id.as_ref()).to_equal(network_info.unit_id.as_ref())?;
    expect(body.data.unit_code.as_ref()).to_equal(network_info.unit_id.as_ref())?;
    expect(
        DateTime::parse_from_rfc3339(body.data.created_at.as_str())
            .unwrap()
            .timestamp_millis(),
    )
    .to_equal(network_info.created_at.timestamp_millis())?;
    expect(
        DateTime::parse_from_rfc3339(body.data.modified_at.as_str())
            .unwrap()
            .timestamp_millis(),
    )
    .to_equal(network_info.modified_at.timestamp_millis())?;
    expect(body.data.host_uri.as_str()).to_equal(network_info.host_uri.as_str())?;
    expect(body.data.name.as_str()).to_equal(network_info.name.as_str())?;
    expect(body.data.info).to_equal(network_info.info)
}

fn test_get_wrong_id(
    runtime: &Runtime,
    state: &routes::State,
    token: &str,
    network_id: &str,
) -> Result<(), String> {
    let app = Router::new().merge(routes::new_service(state));
    let server = match TestServer::new(app) {
        Err(e) => return Err(format!("new server error: {}", e)),
        Ok(server) => server,
    };

    let req = server
        .get(format!("/broker/api/v1/network/{}", network_id).as_str())
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
    network_id: &str,
) -> Result<(), String> {
    add_network_model(runtime, state, unit_id, network_id, "amqp://host")?;

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
    let body = request::PatchNetwork {
        data: request::PatchNetworkData {
            host_uri: Some("amqp://newhost".to_string()),
            name: Some("name changes".to_string()),
            info: Some(info.clone()),
        },
    };
    let req = server
        .patch(format!("/broker/api/v1/network/{}", network_id).as_str())
        .add_header(
            header::AUTHORIZATION,
            HeaderValue::from_str(format!("Bearer {}", token).as_str()).unwrap(),
        )
        .json(&body);
    let resp = runtime.block_on(async { req.await });
    expect(resp.status_code()).to_equal(StatusCode::NO_CONTENT)?;

    let time_after = Utc::now().trunc_subsecs(3);
    let network_info = get_network_model(runtime, state, network_id, true)?.unwrap();
    expect(network_info.modified_at.ge(&time_before)).to_equal(true)?;
    expect(network_info.modified_at.le(&time_after)).to_equal(true)?;
    expect(network_info.host_uri.as_str()).to_equal("amqp://newhost")?;
    expect(network_info.name.as_str()).to_equal("name changes")?;
    expect(network_info.info).to_equal(info)?;

    let body = request::PatchNetwork {
        data: request::PatchNetworkData {
            name: Some("".to_string()),
            info: Some(Map::<String, Value>::new()),
            ..Default::default()
        },
    };
    let req = server
        .patch(format!("/broker/api/v1/network/{}", network_id).as_str())
        .add_header(
            header::AUTHORIZATION,
            HeaderValue::from_str(format!("Bearer {}", token).as_str()).unwrap(),
        )
        .json(&body);
    let resp = runtime.block_on(async { req.await });
    expect(resp.status_code()).to_equal(StatusCode::NO_CONTENT)?;

    let network_info = get_network_model(runtime, state, network_id, true)?.unwrap();
    expect(network_info.host_uri.as_str()).to_equal("amqp://newhost")?;
    expect(network_info.name.as_str()).to_equal("")?;
    expect(network_info.info).to_equal(Map::<String, Value>::new())
}

fn test_patch_wrong_id(
    runtime: &Runtime,
    state: &routes::State,
    token: &str,
    network_id: &str,
) -> Result<(), String> {
    let app = Router::new().merge(routes::new_service(state));
    let server = match TestServer::new(app) {
        Err(e) => return Err(format!("new server error: {}", e)),
        Ok(server) => server,
    };

    let body = request::PatchNetwork {
        data: request::PatchNetworkData {
            name: Some("".to_string()),
            info: Some(Map::<String, Value>::new()),
            ..Default::default()
        },
    };
    let req = server
        .patch(format!("/broker/api/v1/network/{}", network_id).as_str())
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
    network_id: &str,
    param: Option<&request::PatchNetwork>,
) -> Result<(), String> {
    let app = Router::new().merge(routes::new_service(state));
    let server = match TestServer::new(app) {
        Err(e) => return Err(format!("new server error: {}", e)),
        Ok(server) => server,
    };

    let req = server
        .patch(format!("/broker/api/v1/network/{}", network_id).as_str())
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

/// Returns (total, owner1, owner2, member) tuple.
fn count_list_dataset(
    runtime: &Runtime,
    state: &routes::State,
) -> Result<(usize, usize, usize, usize), String> {
    add_unit_model(runtime, state, "manager", vec!["member"], "manager")?;
    add_unit_model(runtime, state, "owner1", vec![], "owner")?;
    add_unit_model(runtime, state, "owner2", vec!["member"], "owner")?;
    let now = Utc::now();

    let mut network = create_network("public", "amqp://host", "");
    network.created_at = now - TimeDelta::try_milliseconds(1).unwrap();
    network.modified_at = now - TimeDelta::try_milliseconds(1).unwrap();
    runtime.block_on(async {
        if let Err(e) = state.model.network().add(&network).await {
            return Err(format!("add network {} error: {}", network.network_id, e));
        }
        Ok(())
    })?;

    let mut network = create_network("manager", "amqp://host", "manager");
    network.created_at = now;
    network.modified_at = now;
    runtime.block_on(async {
        if let Err(e) = state.model.network().add(&network).await {
            return Err(format!("add network {} error: {}", network.network_id, e));
        }
        Ok(())
    })?;

    let mut network = create_network("owner1-1", "amqp://host", "owner1");
    network.created_at = now + TimeDelta::try_milliseconds(1).unwrap();
    network.modified_at = now + TimeDelta::try_milliseconds(5).unwrap();
    runtime.block_on(async {
        if let Err(e) = state.model.network().add(&network).await {
            return Err(format!("add network {} error: {}", network.network_id, e));
        }
        Ok(())
    })?;

    let mut network = create_network("owner1-2", "amqp://host", "owner1");
    network.created_at = now + TimeDelta::try_milliseconds(2).unwrap();
    network.modified_at = now + TimeDelta::try_milliseconds(4).unwrap();
    runtime.block_on(async {
        if let Err(e) = state.model.network().add(&network).await {
            return Err(format!("add network {} error: {}", network.network_id, e));
        }
        Ok(())
    })?;

    let mut network = create_network("owner1-3", "amqp://host", "owner1");
    network.created_at = now + TimeDelta::try_milliseconds(3).unwrap();
    network.modified_at = now + TimeDelta::try_milliseconds(3).unwrap();
    runtime.block_on(async {
        if let Err(e) = state.model.network().add(&network).await {
            return Err(format!("add network {} error: {}", network.network_id, e));
        }
        Ok(())
    })?;

    let mut network = create_network("owner2-1", "amqp://host", "owner2");
    network.created_at = now + TimeDelta::try_milliseconds(4).unwrap();
    network.modified_at = now + TimeDelta::try_milliseconds(2).unwrap();
    network.name = "owner2-2".to_string();
    runtime.block_on(async {
        if let Err(e) = state.model.network().add(&network).await {
            return Err(format!("add network {} error: {}", network.network_id, e));
        }
        Ok(())
    })?;

    let mut network = create_network("owner2-2", "amqp://host", "owner2");
    network.created_at = now + TimeDelta::try_milliseconds(5).unwrap();
    network.modified_at = now + TimeDelta::try_milliseconds(1).unwrap();
    network.name = "owner2-1".to_string();
    runtime.block_on(async {
        if let Err(e) = state.model.network().add(&network).await {
            return Err(format!("add network {} error: {}", network.network_id, e));
        }
        Ok(())
    })?;

    Ok((7, 3, 2, 3))
}
