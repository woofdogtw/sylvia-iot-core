use std::cmp::Ordering;

use actix_web::{
    http::{header, StatusCode},
    middleware::NormalizePath,
    test::{self, TestRequest},
    App,
};
use chrono::{DateTime, TimeDelta, TimeZone, Utc};
use laboratory::{expect, SpecContext};
use serde_json::{Map, Value};
use serde_urlencoded;
use tokio::runtime::Runtime;

use sylvia_iot_broker::routes;
use sylvia_iot_corelib::err;

use super::{
    super::{
        super::libs::{
            add_application_model, add_device_model, add_dldata_buffer_model, add_network_model,
            add_unit_model, create_dldata_buffer, get_dldata_buffer_model, test_get_400,
            test_invalid_token, ApiError,
        },
        TestState, STATE, TOKEN_MANAGER, TOKEN_MEMBER, TOKEN_OWNER,
    },
    request, response,
};

pub fn get_count(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    count_list_dataset(runtime, &routes_state)?;

    test_get_count(runtime, &routes_state, TOKEN_MANAGER, None, 6)?;

    let param = request::GetDlDataBufferCount {
        unit: Some("".to_string()),
        ..Default::default()
    };
    test_get_count(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 6)?;

    let param = request::GetDlDataBufferCount {
        unit: Some("".to_string()),
        application: Some("".to_string()),
        network: Some("".to_string()),
        device: Some("".to_string()),
        ..Default::default()
    };
    test_get_count(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 6)?;

    let param = request::GetDlDataBufferCount {
        unit: Some("manager".to_string()),
        ..Default::default()
    };
    test_get_count(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 2)?;
    test_get_count(runtime, &routes_state, TOKEN_MEMBER, Some(&param), 2)?;

    let param = request::GetDlDataBufferCount {
        unit: Some("owner1".to_string()),
        ..Default::default()
    };
    test_get_count(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 3)?;
    test_get_count(runtime, &routes_state, TOKEN_OWNER, Some(&param), 3)?;

    let param = request::GetDlDataBufferCount {
        unit: Some("owner2".to_string()),
        ..Default::default()
    };
    test_get_count(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 1)?;
    test_get_count(runtime, &routes_state, TOKEN_OWNER, Some(&param), 1)?;
    test_get_count(runtime, &routes_state, TOKEN_MEMBER, Some(&param), 1)?;

    let mut param = request::GetDlDataBufferCount {
        application: Some("manager".to_string()),
        ..Default::default()
    };
    test_get_count(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 2)?;
    param.unit = Some("manager".to_string());
    test_get_count(runtime, &routes_state, TOKEN_MEMBER, Some(&param), 2)?;

    let mut param = request::GetDlDataBufferCount {
        application: Some("owner1".to_string()),
        ..Default::default()
    };
    test_get_count(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 3)?;
    param.unit = Some("owner1".to_string());
    test_get_count(runtime, &routes_state, TOKEN_OWNER, Some(&param), 3)?;
    param.unit = Some("owner2".to_string());
    test_get_count(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 0)?;
    test_get_count(runtime, &routes_state, TOKEN_OWNER, Some(&param), 0)?;
    test_get_count(runtime, &routes_state, TOKEN_MEMBER, Some(&param), 0)?;

    let param = request::GetDlDataBufferCount {
        unit: Some("owner2".to_string()),
        application: Some("owner2".to_string()),
        ..Default::default()
    };
    test_get_count(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 1)?;
    test_get_count(runtime, &routes_state, TOKEN_OWNER, Some(&param), 1)?;
    test_get_count(runtime, &routes_state, TOKEN_MEMBER, Some(&param), 1)?;

    let mut param = request::GetDlDataBufferCount {
        network: Some("public".to_string()),
        ..Default::default()
    };
    test_get_count(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 2)?;
    param.unit = Some("manager".to_string());
    test_get_count(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 1)?;
    test_get_count(runtime, &routes_state, TOKEN_MEMBER, Some(&param), 1)?;
    param.unit = Some("owner1".to_string());
    test_get_count(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 1)?;
    test_get_count(runtime, &routes_state, TOKEN_OWNER, Some(&param), 1)?;

    let mut param = request::GetDlDataBufferCount {
        network: Some("manager".to_string()),
        ..Default::default()
    };
    test_get_count(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 1)?;
    param.unit = Some("manager".to_string());
    test_get_count(runtime, &routes_state, TOKEN_MEMBER, Some(&param), 1)?;

    let mut param = request::GetDlDataBufferCount {
        network: Some("owner1".to_string()),
        ..Default::default()
    };
    test_get_count(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 2)?;
    param.unit = Some("owner1".to_string());
    test_get_count(runtime, &routes_state, TOKEN_OWNER, Some(&param), 2)?;
    param.unit = Some("owner2".to_string());
    test_get_count(runtime, &routes_state, TOKEN_OWNER, Some(&param), 0)?;

    let param = request::GetDlDataBufferCount {
        unit: Some("manager".to_string()),
        application: Some("manager".to_string()),
        network: Some("manager".to_string()),
        ..Default::default()
    };
    test_get_count(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 1)?;
    test_get_count(runtime, &routes_state, TOKEN_MEMBER, Some(&param), 1)?;

    let param = request::GetDlDataBufferCount {
        unit: Some("owner1".to_string()),
        application: Some("owner1".to_string()),
        network: Some("public".to_string()),
        ..Default::default()
    };
    test_get_count(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 1)?;
    test_get_count(runtime, &routes_state, TOKEN_OWNER, Some(&param), 1)?;

    let mut param = request::GetDlDataBufferCount {
        device: Some("manager-public".to_string()),
        ..Default::default()
    };
    test_get_count(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 1)?;
    param.device = Some("owner1".to_string());
    test_get_count(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 2)?;

    let param = request::GetDlDataBufferCount {
        unit: Some("manager".to_string()),
        device: Some("manager-public".to_string()),
        ..Default::default()
    };
    test_get_count(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 1)?;
    test_get_count(runtime, &routes_state, TOKEN_MEMBER, Some(&param), 1)?;

    let param = request::GetDlDataBufferCount {
        unit: Some("owner1".to_string()),
        device: Some("owner1".to_string()),
        ..Default::default()
    };
    test_get_count(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 2)?;
    test_get_count(runtime, &routes_state, TOKEN_OWNER, Some(&param), 2)
}

pub fn get_count_not_exist_unit(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    add_unit_model(runtime, routes_state, "manager", vec![], "manager")?;

    let uri = "/broker/api/v1/dldata-buffer/count";
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

    let uri = "/broker/api/v1/dldata-buffer/count";
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

    let req = TestRequest::get().uri("/broker/api/v1/dldata-buffer/count");
    test_invalid_token(runtime, &routes_state, req)
}

pub fn get_list(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    count_list_dataset(runtime, &routes_state)?;

    test_get_list(runtime, &routes_state, TOKEN_MANAGER, None, 6)?;

    let param = request::GetDlDataBufferList {
        unit: Some("".to_string()),
        ..Default::default()
    };
    test_get_list(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 6)?;

    let param = request::GetDlDataBufferList {
        unit: Some("".to_string()),
        application: Some("".to_string()),
        network: Some("".to_string()),
        device: Some("".to_string()),
        ..Default::default()
    };
    test_get_list(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 6)?;

    let param = request::GetDlDataBufferList {
        unit: Some("manager".to_string()),
        ..Default::default()
    };
    test_get_list(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 2)?;
    test_get_list(runtime, &routes_state, TOKEN_MEMBER, Some(&param), 2)?;

    let param = request::GetDlDataBufferList {
        unit: Some("owner1".to_string()),
        ..Default::default()
    };
    test_get_list(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 3)?;
    test_get_list(runtime, &routes_state, TOKEN_OWNER, Some(&param), 3)?;

    let param = request::GetDlDataBufferList {
        unit: Some("owner2".to_string()),
        ..Default::default()
    };
    test_get_list(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 1)?;
    test_get_list(runtime, &routes_state, TOKEN_OWNER, Some(&param), 1)?;
    test_get_list(runtime, &routes_state, TOKEN_MEMBER, Some(&param), 1)?;

    let mut param = request::GetDlDataBufferList {
        application: Some("manager".to_string()),
        ..Default::default()
    };
    test_get_list(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 2)?;
    param.unit = Some("manager".to_string());
    test_get_list(runtime, &routes_state, TOKEN_MEMBER, Some(&param), 2)?;

    let mut param = request::GetDlDataBufferList {
        application: Some("owner1".to_string()),
        ..Default::default()
    };
    test_get_list(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 3)?;
    param.unit = Some("owner1".to_string());
    test_get_list(runtime, &routes_state, TOKEN_OWNER, Some(&param), 3)?;
    param.unit = Some("owner2".to_string());
    test_get_list(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 0)?;
    test_get_list(runtime, &routes_state, TOKEN_OWNER, Some(&param), 0)?;
    test_get_list(runtime, &routes_state, TOKEN_MEMBER, Some(&param), 0)?;

    let param = request::GetDlDataBufferList {
        unit: Some("owner2".to_string()),
        application: Some("owner2".to_string()),
        ..Default::default()
    };
    test_get_list(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 1)?;
    test_get_list(runtime, &routes_state, TOKEN_OWNER, Some(&param), 1)?;
    test_get_list(runtime, &routes_state, TOKEN_MEMBER, Some(&param), 1)?;

    let mut param = request::GetDlDataBufferList {
        network: Some("public".to_string()),
        ..Default::default()
    };
    test_get_list(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 2)?;
    param.unit = Some("manager".to_string());
    test_get_list(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 1)?;
    test_get_list(runtime, &routes_state, TOKEN_MEMBER, Some(&param), 1)?;
    param.unit = Some("owner1".to_string());
    test_get_list(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 1)?;
    test_get_list(runtime, &routes_state, TOKEN_OWNER, Some(&param), 1)?;

    let mut param = request::GetDlDataBufferList {
        network: Some("manager".to_string()),
        ..Default::default()
    };
    test_get_list(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 1)?;
    param.unit = Some("manager".to_string());
    test_get_list(runtime, &routes_state, TOKEN_MEMBER, Some(&param), 1)?;

    let mut param = request::GetDlDataBufferList {
        network: Some("owner1".to_string()),
        ..Default::default()
    };
    test_get_list(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 2)?;
    param.unit = Some("owner1".to_string());
    test_get_list(runtime, &routes_state, TOKEN_OWNER, Some(&param), 2)?;
    param.unit = Some("owner2".to_string());
    test_get_list(runtime, &routes_state, TOKEN_OWNER, Some(&param), 0)?;

    let param = request::GetDlDataBufferList {
        unit: Some("manager".to_string()),
        application: Some("manager".to_string()),
        network: Some("manager".to_string()),
        ..Default::default()
    };
    test_get_list(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 1)?;
    test_get_list(runtime, &routes_state, TOKEN_MEMBER, Some(&param), 1)?;

    let param = request::GetDlDataBufferList {
        unit: Some("owner1".to_string()),
        application: Some("owner1".to_string()),
        network: Some("public".to_string()),
        ..Default::default()
    };
    test_get_list(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 1)?;
    test_get_list(runtime, &routes_state, TOKEN_OWNER, Some(&param), 1)?;

    let mut param = request::GetDlDataBufferList {
        device: Some("manager-public".to_string()),
        ..Default::default()
    };
    test_get_list(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 1)?;
    param.device = Some("owner1".to_string());
    test_get_list(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 2)?;

    let param = request::GetDlDataBufferList {
        unit: Some("manager".to_string()),
        device: Some("manager-public".to_string()),
        ..Default::default()
    };
    test_get_list(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 1)?;
    test_get_list(runtime, &routes_state, TOKEN_MEMBER, Some(&param), 1)?;

    let param = request::GetDlDataBufferList {
        unit: Some("owner1".to_string()),
        device: Some("owner1".to_string()),
        ..Default::default()
    };
    test_get_list(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 2)?;
    test_get_list(runtime, &routes_state, TOKEN_OWNER, Some(&param), 2)
}

pub fn get_list_sort(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    count_list_dataset(runtime, &routes_state)?;

    let mut param = request::GetDlDataBufferList {
        ..Default::default()
    };
    test_get_list_sort(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        &mut param,
        &[
            "manager-manager",
            "manager-public-manager",
            "owner1-public-owner1",
            "owner1-owner1-2",
            "owner1-owner1-1",
            "owner2-owner2",
        ],
    )?;

    param.sort_vec = Some(vec![("application", true), ("created", false)]);
    test_get_list_sort(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        &mut param,
        &[
            "manager-manager",
            "manager-public-manager",
            "owner1-public-owner1",
            "owner1-owner1-2",
            "owner1-owner1-1",
            "owner2-owner2",
        ],
    )?;
    param.sort_vec = Some(vec![("application", true), ("created", true)]);
    test_get_list_sort(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        &mut param,
        &[
            "manager-public-manager",
            "manager-manager",
            "owner1-owner1-1",
            "owner1-owner1-2",
            "owner1-public-owner1",
            "owner2-owner2",
        ],
    )?;
    param.sort_vec = Some(vec![("application", false), ("created", false)]);
    test_get_list_sort(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        &mut param,
        &[
            "owner2-owner2",
            "owner1-public-owner1",
            "owner1-owner1-2",
            "owner1-owner1-1",
            "manager-manager",
            "manager-public-manager",
        ],
    )?;
    param.sort_vec = Some(vec![("application", false), ("created", true)]);
    test_get_list_sort(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        &mut param,
        &[
            "owner2-owner2",
            "owner1-owner1-1",
            "owner1-owner1-2",
            "owner1-public-owner1",
            "manager-public-manager",
            "manager-manager",
        ],
    )?;

    param.sort_vec = Some(vec![("application", true), ("expired", false)]);
    test_get_list_sort(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        &mut param,
        &[
            "manager-public-manager",
            "manager-manager",
            "owner1-owner1-1",
            "owner1-owner1-2",
            "owner1-public-owner1",
            "owner2-owner2",
        ],
    )?;
    param.sort_vec = Some(vec![("application", true), ("expired", true)]);
    test_get_list_sort(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        &mut param,
        &[
            "manager-manager",
            "manager-public-manager",
            "owner1-public-owner1",
            "owner1-owner1-2",
            "owner1-owner1-1",
            "owner2-owner2",
        ],
    )?;
    param.sort_vec = Some(vec![("application", false), ("expired", false)]);
    test_get_list_sort(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        &mut param,
        &[
            "owner2-owner2",
            "owner1-owner1-1",
            "owner1-owner1-2",
            "owner1-public-owner1",
            "manager-public-manager",
            "manager-manager",
        ],
    )?;
    param.sort_vec = Some(vec![("application", false), ("expired", true)]);
    test_get_list_sort(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        &mut param,
        &[
            "owner2-owner2",
            "owner1-public-owner1",
            "owner1-owner1-2",
            "owner1-owner1-1",
            "manager-manager",
            "manager-public-manager",
        ],
    )?;

    param.sort_vec = Some(vec![("expired", true)]);
    test_get_list_sort(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        &mut param,
        &[
            "owner2-owner2",
            "owner1-public-owner1",
            "owner1-owner1-2",
            "owner1-owner1-1",
            "manager-manager",
            "manager-public-manager",
        ],
    )?;
    param.sort_vec = Some(vec![("expired", false)]);
    test_get_list_sort(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        &mut param,
        &[
            "manager-public-manager",
            "manager-manager",
            "owner1-owner1-1",
            "owner1-owner1-2",
            "owner1-public-owner1",
            "owner2-owner2",
        ],
    )
}

pub fn get_list_offset_limit(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    add_unit_model(runtime, routes_state, "manager", vec![], "manager")?;
    add_application_model(runtime, routes_state, "manager", "manager", "amqp://host")?;
    add_network_model(runtime, routes_state, "manager", "manager", "amqp://host")?;
    add_device_model(
        runtime,
        &routes_state,
        "manager",
        "manager",
        "manager",
        false,
        "",
    )?;

    let now = Utc::now();
    for i in 100..302 {
        let mut data = create_dldata_buffer(
            format!("data_{}", i).as_str(),
            "manager",
            "manager",
            "manager",
            "manager",
        );
        data.created_at = now - TimeDelta::try_milliseconds(i).unwrap();
        runtime.block_on(async {
            if let Err(e) = routes_state.model.dldata_buffer().add(&data).await {
                return Err(format!("add dldata buffer {} error: {}", data.data_id, e));
            }
            Ok(())
        })?;
    }

    let param = request::GetDlDataBufferList {
        ..Default::default()
    };
    test_get_list_offset_limit(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        &param,
        (100..200).collect(),
    )?;

    let mut param = request::GetDlDataBufferList {
        unit: Some("manager".to_string()),
        ..Default::default()
    };
    test_get_list_offset_limit(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        &param,
        (100..200).collect(),
    )?;

    param.application = Some("manager".to_string());
    test_get_list_offset_limit(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        &param,
        (100..200).collect(),
    )?;

    param.network = Some("manager".to_string());
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

    param.application = None;
    test_get_list_offset_limit(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        &param,
        (100..200).collect(),
    )?;

    param.network = None;
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

    param.unit = Some("".to_string());
    param.application = Some("".to_string());
    param.network = Some("".to_string());
    param.device = Some("".to_string());
    test_get_list_offset_limit(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        &param,
        (100..302).collect(),
    )?;
    param.unit = None;
    param.application = None;
    param.network = None;
    param.device = None;

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
    add_network_model(runtime, routes_state, "manager", "manager", "amqp://host")?;
    add_device_model(
        runtime,
        &routes_state,
        "manager",
        "manager",
        "manager",
        false,
        "",
    )?;

    for i in 100..302 {
        let application_id = format!("manager_{}", i);
        add_application_model(
            runtime,
            routes_state,
            "manager",
            application_id.as_str(),
            "amqp://host",
        )?;
        add_dldata_buffer_model(
            runtime,
            &routes_state,
            format!("data_{}", i).as_str(),
            "manager",
            application_id.as_str(),
            "manager",
            "manager",
        )?;
    }

    let mut param = request::GetDlDataBufferList {
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

    let uri = "/broker/api/v1/dldata-buffer/list";
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

    let uri = "/broker/api/v1/dldata-buffer/list";
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
        Value::String("created:asc,application:true".to_string()),
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

    let req = TestRequest::get().uri("/broker/api/v1/dldata-buffer/list");
    test_invalid_token(runtime, &routes_state, req)
}

pub fn delete(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    add_unit_model(runtime, routes_state, "manager", vec![], "manager")?;
    add_application_model(runtime, routes_state, "manager", "manager", "amqp://host")?;
    add_network_model(runtime, routes_state, "manager", "manager", "amqp://host")?;
    add_device_model(
        runtime,
        &routes_state,
        "manager",
        "manager",
        "addr",
        false,
        "",
    )?;
    add_dldata_buffer_model(
        runtime,
        &routes_state,
        "data1",
        "manager",
        "manager",
        "manager",
        "addr",
    )?;
    add_dldata_buffer_model(
        runtime,
        &routes_state,
        "data2",
        "manager",
        "manager",
        "manager",
        "addr",
    )?;

    let mut app = runtime.block_on(async {
        test::init_service(
            App::new()
                .wrap(NormalizePath::trim())
                .service(routes::new_service(&routes_state)),
        )
        .await
    });

    let req = TestRequest::delete()
        .uri("/broker/api/v1/dldata-buffer/id")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", TOKEN_MANAGER)))
        .to_request();
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    expect(resp.status()).to_equal(StatusCode::NO_CONTENT)?;
    let _ = get_dldata_buffer_model(runtime, &routes_state, "data1", true)?;
    let _ = get_dldata_buffer_model(runtime, &routes_state, "data2", true)?;

    let req = TestRequest::delete()
        .uri("/broker/api/v1/dldata-buffer/data1")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", TOKEN_OWNER)))
        .to_request();
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    expect(resp.status()).to_equal(StatusCode::NO_CONTENT)?;
    let _ = get_dldata_buffer_model(runtime, &routes_state, "data1", true)?;
    let _ = get_dldata_buffer_model(runtime, &routes_state, "data2", true)?;

    let req = TestRequest::delete()
        .uri("/broker/api/v1/dldata-buffer/data1")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", TOKEN_MANAGER)))
        .to_request();
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    expect(resp.status()).to_equal(StatusCode::NO_CONTENT)?;
    let _ = get_dldata_buffer_model(runtime, &routes_state, "data1", false)?;
    let _ = get_dldata_buffer_model(runtime, &routes_state, "data2", true)?;

    Ok(())
}

pub fn delete_invalid_token(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    let req = TestRequest::delete().uri("/broker/api/v1/dldata-buffer/id");
    test_invalid_token(runtime, &routes_state, req)
}

fn test_get_count(
    runtime: &Runtime,
    state: &routes::State,
    token: &str,
    param: Option<&request::GetDlDataBufferCount>,
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
        None => "/broker/api/v1/dldata-buffer/count".to_string(),
        Some(param) => format!(
            "/broker/api/v1/dldata-buffer/count?{}",
            serde_urlencoded::to_string(&param).unwrap()
        ),
    };
    let req = TestRequest::get()
        .uri(uri.as_str())
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)))
        .to_request();
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    expect(resp.status()).to_equal(StatusCode::OK)?;
    let body: response::GetDlDataBufferCount =
        runtime.block_on(async { test::read_body_json(resp).await });
    expect(body.data.count).to_equal(expect_count)
}

fn test_get_list(
    runtime: &Runtime,
    state: &routes::State,
    token: &str,
    param: Option<&request::GetDlDataBufferList>,
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
        None => "/broker/api/v1/dldata-buffer/list".to_string(),
        Some(param) => format!(
            "/broker/api/v1/dldata-buffer/list?{}",
            serde_urlencoded::to_string(&param).unwrap()
        ),
    };
    let req = TestRequest::get()
        .uri(uri.as_str())
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)))
        .to_request();
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    expect(resp.status()).to_equal(StatusCode::OK)?;
    let body: response::GetDlDataBufferList =
        runtime.block_on(async { test::read_body_json(resp).await });
    expect(body.data.len()).to_equal(expect_count)?;

    let now = Utc::now();
    let mut application_min = "";
    let mut created_at = now;
    for info in body.data.iter() {
        if let Err(_) = expect(info.application_id.as_str().ge(application_min)).to_equal(true) {
            return Err(format!(
                "application order error: {} - {}/{}",
                application_min,
                info.application_id.as_str(),
                info.data_id.as_str()
            ));
        }
        let info_created_at = match DateTime::parse_from_rfc3339(info.created_at.as_str()) {
            Err(e) => return Err(format!("created_at not RFC3339: {}", e)),
            Ok(dt) => Utc.timestamp_nanos(match dt.timestamp_nanos_opt() {
                None => i64::MAX,
                Some(ts) => ts,
            }),
        };
        if let Err(_) = expect(info_created_at.le(&created_at)).to_equal(true) {
            return Err(format!(
                "created_at order error: {} - {}/{}",
                application_min,
                info.application_id.as_str(),
                info_created_at
            ));
        }
        if application_min.cmp(info.application_id.as_str()) != Ordering::Equal {
            created_at = now;
        }
        application_min = info.application_id.as_str();
    }
    Ok(())
}

fn test_get_list_sort(
    runtime: &Runtime,
    state: &routes::State,
    token: &str,
    param: &mut request::GetDlDataBufferList,
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
        "/broker/api/v1/dldata-buffer/list?{}",
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
    let body: response::GetDlDataBufferList =
        runtime.block_on(async { test::read_body_json(resp).await });
    expect(body.data.len()).to_equal(expect_ids.len())?;

    let mut index = 0;
    for data in body.data.iter() {
        expect(data.data_id.as_str()).to_equal(expect_ids[index])?;
        index += 1;
    }
    Ok(())
}

fn test_get_list_offset_limit(
    runtime: &Runtime,
    state: &routes::State,
    token: &str,
    param: &request::GetDlDataBufferList,
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
        "/broker/api/v1/dldata-buffer/list?{}",
        serde_urlencoded::to_string(&param).unwrap()
    );
    let req = TestRequest::get()
        .uri(uri.as_str())
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)))
        .to_request();
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    expect(resp.status()).to_equal(StatusCode::OK)?;
    let body: response::GetDlDataBufferList =
        runtime.block_on(async { test::read_body_json(resp).await });
    expect(body.data.len()).to_equal(expect_ids.len())?;

    let mut index = 0;
    for data in body.data.iter() {
        expect(data.data_id.as_str()).to_equal(format!("data_{}", expect_ids[index]).as_str())?;
        index += 1;
    }
    Ok(())
}

fn test_get_list_format_array(
    runtime: &Runtime,
    state: &routes::State,
    token: &str,
    param: &request::GetDlDataBufferList,
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
        "/broker/api/v1/dldata-buffer/list?{}",
        serde_urlencoded::to_string(&param).unwrap()
    );
    let req = TestRequest::get()
        .uri(uri.as_str())
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)))
        .to_request();
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    expect(resp.status()).to_equal(StatusCode::OK)?;
    let body: Vec<response::GetDlDataBufferListData> =
        runtime.block_on(async { test::read_body_json(resp).await });
    expect(body.len()).to_equal(expect_ids.len())?;

    let mut index = 0;
    for data in body.iter() {
        expect(data.data_id.as_str()).to_equal(format!("data_{}", expect_ids[index]).as_str())?;
        index += 1;
    }
    Ok(())
}

fn count_list_dataset(runtime: &Runtime, state: &routes::State) -> Result<(), String> {
    add_unit_model(runtime, state, "manager", vec!["member"], "manager")?;
    add_unit_model(runtime, state, "owner1", vec![], "owner")?;
    add_unit_model(runtime, state, "owner2", vec!["member"], "owner")?;
    add_application_model(runtime, state, "manager", "manager", "amqp://host")?;
    add_application_model(runtime, state, "owner1", "owner1", "amqp://host")?;
    add_application_model(runtime, state, "owner2", "owner2", "amqp://host")?;
    add_network_model(runtime, state, "", "public", "amqp://host")?;
    add_network_model(runtime, state, "manager", "manager", "amqp://host")?;
    add_network_model(runtime, state, "owner1", "owner1", "amqp://host")?;
    add_network_model(runtime, state, "owner2", "owner2", "amqp://host")?;
    add_device_model(
        runtime,
        state,
        "manager",
        "public",
        "manager-public",
        true,
        "",
    )?;
    add_device_model(runtime, state, "manager", "manager", "manager", false, "")?;
    add_device_model(
        runtime,
        state,
        "owner1",
        "public",
        "owner1-public",
        true,
        "",
    )?;
    add_device_model(runtime, state, "owner1", "owner1", "owner1", false, "")?;
    add_device_model(runtime, state, "owner2", "owner2", "owner2", false, "")?;
    let now = Utc::now();

    let mut data = create_dldata_buffer(
        "manager-public-manager",
        "manager",
        "manager",
        "public",
        "manager-public",
    );
    data.created_at = now;
    data.expired_at = now + TimeDelta::try_seconds(105).unwrap();
    runtime.block_on(async {
        if let Err(e) = state.model.dldata_buffer().add(&data).await {
            return Err(format!("add dldata buffer {} error: {}", data.data_id, e));
        }
        Ok(())
    })?;

    let mut data = create_dldata_buffer(
        "manager-manager",
        "manager",
        "manager",
        "manager",
        "manager",
    );
    data.created_at = now + TimeDelta::try_milliseconds(1).unwrap();
    data.expired_at = now + TimeDelta::try_seconds(104).unwrap();
    runtime.block_on(async {
        if let Err(e) = state.model.dldata_buffer().add(&data).await {
            return Err(format!("add dldata buffer {} error: {}", data.data_id, e));
        }
        Ok(())
    })?;

    let mut data = create_dldata_buffer("owner1-owner1-1", "owner1", "owner1", "owner1", "owner1");
    data.created_at = now + TimeDelta::try_milliseconds(2).unwrap();
    data.expired_at = now + TimeDelta::try_seconds(103).unwrap();
    runtime.block_on(async {
        if let Err(e) = state.model.dldata_buffer().add(&data).await {
            return Err(format!("add dldata buffer {} error: {}", data.data_id, e));
        }
        Ok(())
    })?;

    let mut data = create_dldata_buffer("owner1-owner1-2", "owner1", "owner1", "owner1", "owner1");
    data.created_at = now + TimeDelta::try_milliseconds(3).unwrap();
    data.expired_at = now + TimeDelta::try_seconds(102).unwrap();
    runtime.block_on(async {
        if let Err(e) = state.model.dldata_buffer().add(&data).await {
            return Err(format!("add dldata buffer {} error: {}", data.data_id, e));
        }
        Ok(())
    })?;

    let mut data = create_dldata_buffer(
        "owner1-public-owner1",
        "owner1",
        "owner1",
        "public",
        "owner1-public",
    );
    data.created_at = now + TimeDelta::try_milliseconds(4).unwrap();
    data.expired_at = now + TimeDelta::try_seconds(101).unwrap();
    runtime.block_on(async {
        if let Err(e) = state.model.dldata_buffer().add(&data).await {
            return Err(format!("add dldata buffer {} error: {}", data.data_id, e));
        }
        Ok(())
    })?;

    let mut data = create_dldata_buffer("owner2-owner2", "owner2", "owner2", "owner2", "owner2");
    data.created_at = now + TimeDelta::try_milliseconds(5).unwrap();
    data.expired_at = now + TimeDelta::try_seconds(100).unwrap();
    runtime.block_on(async {
        if let Err(e) = state.model.dldata_buffer().add(&data).await {
            return Err(format!("add dldata buffer {} error: {}", data.data_id, e));
        }
        Ok(())
    })?;

    Ok(())
}
