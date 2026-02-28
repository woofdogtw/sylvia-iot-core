use axum::{
    Router,
    http::{HeaderValue, StatusCode, header},
};
use axum_test::TestServer;
use chrono::{DateTime, TimeDelta, Utc};
use laboratory::{SpecContext, expect};
use serde_json::{Map, Value};
use tokio::runtime::Runtime;

use sylvia_iot_corelib::err;
use sylvia_iot_data::{models::network_dldata::NetworkDlData, routes};

use super::{
    super::{
        super::libs::{ApiError, test_get_400, test_invalid_token},
        STATE, TOKEN_MANAGER, TOKEN_MEMBER, TOKEN_OWNER, TestState, UNIT_OWNER,
    },
    request, response,
};

pub fn get_count(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    let now = Utc::now();
    count_list_dataset(runtime, routes_state, now)?;

    test_get_count(runtime, routes_state, TOKEN_MANAGER, None, 5)?;

    let mut param = request::GetCount {
        device: Some("device_id3".to_string()),
        ..Default::default()
    };
    test_get_count(runtime, routes_state, TOKEN_MANAGER, Some(&param), 1)?;
    param.device = Some("device_id0".to_string());
    test_get_count(runtime, routes_state, TOKEN_MANAGER, Some(&param), 0)?;

    let mut param = request::GetCount {
        unit: Some("".to_string()),
        network: Some("network_code1_1".to_string()),
        ..Default::default()
    };
    test_get_count(runtime, routes_state, TOKEN_MANAGER, Some(&param), 3)?;
    param.addr = Some("network_addr1_1".to_string());
    test_get_count(runtime, routes_state, TOKEN_MANAGER, Some(&param), 2)?;
    param.addr = Some("network_addr2".to_string());
    test_get_count(runtime, routes_state, TOKEN_MANAGER, Some(&param), 0)?;

    let mut param = request::GetCount {
        profile: Some("profile1".to_string()),
        ..Default::default()
    };
    test_get_count(runtime, routes_state, TOKEN_MANAGER, Some(&param), 3)?;
    param.profile = Some("profile2".to_string());
    test_get_count(runtime, routes_state, TOKEN_MANAGER, Some(&param), 1)?;

    let mut param = request::GetCount {
        unit: Some(UNIT_OWNER.to_string()),
        ..Default::default()
    };
    test_get_count(runtime, routes_state, TOKEN_MANAGER, Some(&param), 4)?;
    test_get_count(runtime, routes_state, TOKEN_OWNER, Some(&param), 4)?;
    test_get_count(runtime, routes_state, TOKEN_MEMBER, Some(&param), 4)?;
    param.device = Some("device_id3".to_string());
    test_get_count(runtime, routes_state, TOKEN_MANAGER, Some(&param), 0)?;
    test_get_count(runtime, routes_state, TOKEN_OWNER, Some(&param), 0)?;
    test_get_count(runtime, routes_state, TOKEN_MEMBER, Some(&param), 0)?;
    param.device = None;
    param.tfield = Some("proc".to_string());
    test_get_count(runtime, routes_state, TOKEN_MANAGER, Some(&param), 4)?;
    test_get_count(runtime, routes_state, TOKEN_OWNER, Some(&param), 4)?;
    test_get_count(runtime, routes_state, TOKEN_MEMBER, Some(&param), 4)?;
    param.tstart = Some(now.timestamp_millis() + 1);
    test_get_count(runtime, routes_state, TOKEN_MANAGER, Some(&param), 3)?;
    test_get_count(runtime, routes_state, TOKEN_OWNER, Some(&param), 3)?;
    test_get_count(runtime, routes_state, TOKEN_MEMBER, Some(&param), 3)?;
    param.tend = Some(now.timestamp_millis() + 2);
    test_get_count(runtime, routes_state, TOKEN_MANAGER, Some(&param), 2)?;
    test_get_count(runtime, routes_state, TOKEN_OWNER, Some(&param), 2)?;
    test_get_count(runtime, routes_state, TOKEN_MEMBER, Some(&param), 2)?;

    let mut param = request::GetCount {
        unit: Some(UNIT_OWNER.to_string()),
        tfield: Some("pub".to_string()),
        tstart: Some(now.timestamp_millis() + 5),
        ..Default::default()
    };
    test_get_count(runtime, routes_state, TOKEN_MANAGER, Some(&param), 4)?;
    test_get_count(runtime, routes_state, TOKEN_OWNER, Some(&param), 4)?;
    test_get_count(runtime, routes_state, TOKEN_MEMBER, Some(&param), 4)?;
    param.tend = Some(now.timestamp_millis() + 8);
    test_get_count(runtime, routes_state, TOKEN_MANAGER, Some(&param), 3)?;
    test_get_count(runtime, routes_state, TOKEN_OWNER, Some(&param), 3)?;
    test_get_count(runtime, routes_state, TOKEN_MEMBER, Some(&param), 3)?;

    let mut param = request::GetCount {
        unit: Some(UNIT_OWNER.to_string()),
        tfield: Some("resp".to_string()),
        tstart: Some(now.timestamp_millis()),
        ..Default::default()
    };
    test_get_count(runtime, routes_state, TOKEN_MANAGER, Some(&param), 3)?;
    test_get_count(runtime, routes_state, TOKEN_OWNER, Some(&param), 3)?;
    test_get_count(runtime, routes_state, TOKEN_MEMBER, Some(&param), 3)?;
    param.tend = Some(now.timestamp_millis() + 1);
    test_get_count(runtime, routes_state, TOKEN_MANAGER, Some(&param), 0)?;
    test_get_count(runtime, routes_state, TOKEN_OWNER, Some(&param), 0)?;
    test_get_count(runtime, routes_state, TOKEN_MEMBER, Some(&param), 0)
}

pub fn get_count_not_exist_unit(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    let uri = "/data/api/v1/network-dldata/count";
    let code = "err_data_unit_not_exist";

    let mut query = Map::<String, Value>::new();
    query.insert("unit".to_string(), Value::String("unit_id2".to_string()));
    test_get_400(runtime, routes_state, TOKEN_OWNER, uri, &query, code)?;
    test_get_400(runtime, routes_state, TOKEN_MEMBER, uri, &query, code)
}

pub fn get_count_invalid_param(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    let uri = "/data/api/v1/network-dldata/count";
    let code = err::E_PARAM;

    let query = Map::<String, Value>::new();
    test_get_400(runtime, routes_state, TOKEN_OWNER, uri, &query, code)?;
    test_get_400(runtime, routes_state, TOKEN_MEMBER, uri, &query, code)?;

    let mut query = Map::<String, Value>::new();
    query.insert("unit".to_string(), Value::String(UNIT_OWNER.to_string()));
    query.insert("tstart".to_string(), Value::String("0".to_string()));
    test_get_400(runtime, routes_state, TOKEN_MANAGER, uri, &query, code)?;
    test_get_400(runtime, routes_state, TOKEN_OWNER, uri, &query, code)?;
    test_get_400(runtime, routes_state, TOKEN_MEMBER, uri, &query, code)?;

    let mut query = Map::<String, Value>::new();
    query.insert("unit".to_string(), Value::String(UNIT_OWNER.to_string()));
    query.insert("tend".to_string(), Value::String("0".to_string()));
    test_get_400(runtime, routes_state, TOKEN_MANAGER, uri, &query, code)?;
    test_get_400(runtime, routes_state, TOKEN_OWNER, uri, &query, code)?;
    test_get_400(runtime, routes_state, TOKEN_MEMBER, uri, &query, code)?;

    let mut query = Map::<String, Value>::new();
    query.insert("unit".to_string(), Value::String(UNIT_OWNER.to_string()));
    query.insert("tstart".to_string(), Value::String("-1".to_string()));
    test_get_400(runtime, routes_state, TOKEN_MANAGER, uri, &query, code)?;
    test_get_400(runtime, routes_state, TOKEN_OWNER, uri, &query, code)?;
    test_get_400(runtime, routes_state, TOKEN_MEMBER, uri, &query, code)?;

    let mut query = Map::<String, Value>::new();
    query.insert("unit".to_string(), Value::String(UNIT_OWNER.to_string()));
    query.insert("tend".to_string(), Value::String("1.1".to_string()));
    test_get_400(runtime, routes_state, TOKEN_MANAGER, uri, &query, code)?;
    test_get_400(runtime, routes_state, TOKEN_OWNER, uri, &query, code)?;
    test_get_400(runtime, routes_state, TOKEN_MEMBER, uri, &query, code)?;

    let mut query = Map::<String, Value>::new();
    query.insert("unit".to_string(), Value::String(UNIT_OWNER.to_string()));
    query.insert("tfield".to_string(), Value::String("invalid".to_string()));
    query.insert("tstart".to_string(), Value::String("0".to_string()));
    test_get_400(runtime, routes_state, TOKEN_MANAGER, uri, &query, code)?;
    test_get_400(runtime, routes_state, TOKEN_OWNER, uri, &query, code)?;
    test_get_400(runtime, routes_state, TOKEN_MEMBER, uri, &query, code)?;

    let mut query = Map::<String, Value>::new();
    query.insert("unit".to_string(), Value::String(UNIT_OWNER.to_string()));
    query.insert("tfield".to_string(), Value::String("invalid".to_string()));
    query.insert("tend".to_string(), Value::String("0".to_string()));
    test_get_400(runtime, routes_state, TOKEN_MANAGER, uri, &query, code)?;
    test_get_400(runtime, routes_state, TOKEN_OWNER, uri, &query, code)?;
    test_get_400(runtime, routes_state, TOKEN_MEMBER, uri, &query, code)
}

pub fn get_count_invalid_token(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    test_invalid_token(runtime, &routes_state, "/data/api/v1/network-dldata/count")
}

pub fn get_list(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    let now = Utc::now();
    count_list_dataset(runtime, routes_state, now)?;
    let max_proc = now.timestamp_millis() + 4;

    test_get_list(runtime, routes_state, TOKEN_MANAGER, None, 5, max_proc)?;

    let mut param = request::GetList {
        device: Some("device_id3".to_string()),
        ..Default::default()
    };
    test_get_list(
        runtime,
        routes_state,
        TOKEN_MANAGER,
        Some(&param),
        1,
        max_proc,
    )?;
    param.device = Some("device_id0".to_string());
    test_get_list(
        runtime,
        routes_state,
        TOKEN_MANAGER,
        Some(&param),
        0,
        max_proc,
    )?;

    let mut param = request::GetList {
        unit: Some("".to_string()),
        network: Some("network_code1_1".to_string()),
        ..Default::default()
    };
    test_get_list(
        runtime,
        routes_state,
        TOKEN_MANAGER,
        Some(&param),
        3,
        max_proc,
    )?;
    param.addr = Some("network_addr1_1".to_string());
    test_get_list(
        runtime,
        routes_state,
        TOKEN_MANAGER,
        Some(&param),
        2,
        max_proc,
    )?;
    param.addr = Some("network_addr2".to_string());
    test_get_list(
        runtime,
        routes_state,
        TOKEN_MANAGER,
        Some(&param),
        0,
        max_proc,
    )?;

    let mut param = request::GetList {
        profile: Some("profile1".to_string()),
        ..Default::default()
    };
    test_get_list(
        runtime,
        routes_state,
        TOKEN_MANAGER,
        Some(&param),
        3,
        max_proc,
    )?;
    param.profile = Some("profile2".to_string());
    test_get_list(
        runtime,
        routes_state,
        TOKEN_MANAGER,
        Some(&param),
        1,
        max_proc,
    )?;

    let mut param = request::GetList {
        unit: Some(UNIT_OWNER.to_string()),
        ..Default::default()
    };
    test_get_list(
        runtime,
        routes_state,
        TOKEN_MANAGER,
        Some(&param),
        4,
        max_proc,
    )?;
    test_get_list(
        runtime,
        routes_state,
        TOKEN_OWNER,
        Some(&param),
        4,
        max_proc,
    )?;
    test_get_list(
        runtime,
        routes_state,
        TOKEN_MEMBER,
        Some(&param),
        4,
        max_proc,
    )?;
    param.device = Some("device_id3".to_string());
    test_get_list(
        runtime,
        routes_state,
        TOKEN_MANAGER,
        Some(&param),
        0,
        max_proc,
    )?;
    test_get_list(
        runtime,
        routes_state,
        TOKEN_OWNER,
        Some(&param),
        0,
        max_proc,
    )?;
    test_get_list(
        runtime,
        routes_state,
        TOKEN_MEMBER,
        Some(&param),
        0,
        max_proc,
    )?;
    param.device = None;
    param.tfield = Some("proc".to_string());
    test_get_list(
        runtime,
        routes_state,
        TOKEN_MANAGER,
        Some(&param),
        4,
        max_proc,
    )?;
    test_get_list(
        runtime,
        routes_state,
        TOKEN_OWNER,
        Some(&param),
        4,
        max_proc,
    )?;
    test_get_list(
        runtime,
        routes_state,
        TOKEN_MEMBER,
        Some(&param),
        4,
        max_proc,
    )?;
    param.tstart = Some(now.timestamp_millis() + 1);
    test_get_list(
        runtime,
        routes_state,
        TOKEN_MANAGER,
        Some(&param),
        3,
        max_proc,
    )?;
    test_get_list(
        runtime,
        routes_state,
        TOKEN_OWNER,
        Some(&param),
        3,
        max_proc,
    )?;
    test_get_list(
        runtime,
        routes_state,
        TOKEN_MEMBER,
        Some(&param),
        3,
        max_proc,
    )?;
    param.tend = Some(now.timestamp_millis() + 2);
    test_get_list(
        runtime,
        routes_state,
        TOKEN_MANAGER,
        Some(&param),
        2,
        max_proc,
    )?;
    test_get_list(
        runtime,
        routes_state,
        TOKEN_OWNER,
        Some(&param),
        2,
        max_proc,
    )?;
    test_get_list(
        runtime,
        routes_state,
        TOKEN_MEMBER,
        Some(&param),
        2,
        max_proc,
    )?;

    let mut param = request::GetList {
        unit: Some(UNIT_OWNER.to_string()),
        tfield: Some("pub".to_string()),
        tstart: Some(now.timestamp_millis() + 5),
        ..Default::default()
    };
    test_get_list(
        runtime,
        routes_state,
        TOKEN_MANAGER,
        Some(&param),
        4,
        max_proc,
    )?;
    test_get_list(
        runtime,
        routes_state,
        TOKEN_OWNER,
        Some(&param),
        4,
        max_proc,
    )?;
    test_get_list(
        runtime,
        routes_state,
        TOKEN_MEMBER,
        Some(&param),
        4,
        max_proc,
    )?;
    param.tend = Some(now.timestamp_millis() + 8);
    test_get_list(
        runtime,
        routes_state,
        TOKEN_MANAGER,
        Some(&param),
        3,
        max_proc,
    )?;
    test_get_list(
        runtime,
        routes_state,
        TOKEN_OWNER,
        Some(&param),
        3,
        max_proc,
    )?;
    test_get_list(
        runtime,
        routes_state,
        TOKEN_MEMBER,
        Some(&param),
        3,
        max_proc,
    )?;

    let mut param = request::GetList {
        unit: Some(UNIT_OWNER.to_string()),
        tfield: Some("resp".to_string()),
        tstart: Some(now.timestamp_millis()),
        ..Default::default()
    };
    test_get_list(
        runtime,
        routes_state,
        TOKEN_MANAGER,
        Some(&param),
        3,
        max_proc,
    )?;
    test_get_list(
        runtime,
        routes_state,
        TOKEN_OWNER,
        Some(&param),
        3,
        max_proc,
    )?;
    test_get_list(
        runtime,
        routes_state,
        TOKEN_MEMBER,
        Some(&param),
        3,
        max_proc,
    )?;
    param.tend = Some(now.timestamp_millis());
    test_get_list(
        runtime,
        routes_state,
        TOKEN_MANAGER,
        Some(&param),
        0,
        max_proc,
    )?;
    test_get_list(
        runtime,
        routes_state,
        TOKEN_OWNER,
        Some(&param),
        0,
        max_proc,
    )?;
    test_get_list(
        runtime,
        routes_state,
        TOKEN_MEMBER,
        Some(&param),
        0,
        max_proc,
    )
}

pub fn get_list_sort(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    let now = Utc::now();
    count_list_dataset(runtime, &routes_state, now)?;

    let mut param = request::GetList {
        ..Default::default()
    };
    test_get_list_sort(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        &mut param,
        &["data_id5", "data_id4", "data_id3", "data_id2", "data_id1"],
    )?;

    param.sort_vec = Some(vec![("proc", true)]);
    test_get_list_sort(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        &mut param,
        &["data_id1", "data_id2", "data_id3", "data_id4", "data_id5"],
    )?;
    param.sort_vec = Some(vec![("proc", false)]);
    test_get_list_sort(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        &mut param,
        &["data_id5", "data_id4", "data_id3", "data_id2", "data_id1"],
    )?;

    param.sort_vec = Some(vec![("pub", true)]);
    test_get_list_sort(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        &mut param,
        &["data_id1", "data_id2", "data_id3", "data_id4", "data_id5"],
    )?;
    param.sort_vec = Some(vec![("pub", false)]);
    test_get_list_sort(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        &mut param,
        &["data_id5", "data_id4", "data_id3", "data_id2", "data_id1"],
    )?;

    param.sort_vec = Some(vec![("resp", true)]);
    test_get_list_sort(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        &mut param,
        &["data_id1", "data_id5", "data_id4", "data_id3", "data_id2"],
    )?;
    param.sort_vec = Some(vec![("resp", false)]);
    test_get_list_sort(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        &mut param,
        &["data_id2", "data_id3", "data_id4", "data_id5", "data_id1"],
    )?;

    param.sort_vec = Some(vec![("network", true), ("addr", true), ("proc", true)]);
    test_get_list_sort(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        &mut param,
        &["data_id1", "data_id2", "data_id3", "data_id4", "data_id5"],
    )?;
    param.sort_vec = Some(vec![("network", true), ("addr", true), ("proc", false)]);
    test_get_list_sort(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        &mut param,
        &["data_id2", "data_id1", "data_id3", "data_id4", "data_id5"],
    )?;
    param.sort_vec = Some(vec![("network", false), ("addr", true), ("proc", true)]);
    test_get_list_sort(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        &mut param,
        &["data_id5", "data_id4", "data_id1", "data_id2", "data_id3"],
    )?;
    param.sort_vec = Some(vec![("network", false), ("addr", false), ("proc", true)]);
    test_get_list_sort(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        &mut param,
        &["data_id5", "data_id4", "data_id3", "data_id1", "data_id2"],
    )
}

pub fn get_list_offset_limit(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    let now = Utc::now();

    for i in 100..201 {
        add_offset_limit_data(
            runtime,
            &routes_state,
            format!("data_id{}", i).as_str(),
            now + TimeDelta::try_milliseconds(i).unwrap(),
            false,
        )?;
    }
    for i in 201..302 {
        add_offset_limit_data(
            runtime,
            &routes_state,
            format!("data_id{}", i).as_str(),
            now + TimeDelta::try_milliseconds(i).unwrap(),
            true,
        )?;
    }

    let mut param = request::GetList {
        tfield: Some("proc".to_string()),
        tstart: Some(now.timestamp_millis()),
        sort: Some("proc:asc".to_string()),
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

pub fn get_list_format_array_csv(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    let now = Utc::now();

    for i in 100..201 {
        add_offset_limit_data(
            runtime,
            &routes_state,
            format!("data_id{}", i).as_str(),
            now + TimeDelta::try_milliseconds(i).unwrap(),
            false,
        )?;
    }
    for i in 201..302 {
        add_offset_limit_data(
            runtime,
            &routes_state,
            format!("data_id{}", i).as_str(),
            now + TimeDelta::try_milliseconds(i).unwrap(),
            true,
        )?;
    }

    let mut param = request::GetList {
        tfield: Some("proc".to_string()),
        tstart: Some(now.timestamp_millis()),
        sort: Some("proc:asc".to_string()),
        limit: Some(5),
        ..Default::default()
    };
    test_get_list_format_array(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        &mut param,
        (100..105).collect(),
    )?;
    test_get_list_format_csv(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        &mut param,
        (100..105).collect(),
    )?;

    param.offset = Some(2);
    param.limit = Some(105);
    test_get_list_format_array(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        &mut param,
        (102..207).collect(),
    )?;
    test_get_list_format_csv(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        &mut param,
        (102..207).collect(),
    )
}

pub fn get_list_not_exist_unit(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    let uri = "/data/api/v1/network-dldata/list";
    let code = "err_data_unit_not_exist";

    let mut query = Map::<String, Value>::new();
    query.insert("unit".to_string(), Value::String("unit_id2".to_string()));
    test_get_400(runtime, routes_state, TOKEN_OWNER, uri, &query, code)?;
    test_get_400(runtime, routes_state, TOKEN_MEMBER, uri, &query, code)
}

pub fn get_list_invalid_param(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    let uri = "/data/api/v1/network-dldata/list";
    let code = err::E_PARAM;

    let query = Map::<String, Value>::new();
    test_get_400(runtime, routes_state, TOKEN_OWNER, uri, &query, code)?;
    test_get_400(runtime, routes_state, TOKEN_MEMBER, uri, &query, code)?;

    let mut query = Map::<String, Value>::new();
    query.insert("unit".to_string(), Value::String(UNIT_OWNER.to_string()));
    query.insert("tstart".to_string(), Value::String("0".to_string()));
    test_get_400(runtime, routes_state, TOKEN_MANAGER, uri, &query, code)?;
    test_get_400(runtime, routes_state, TOKEN_OWNER, uri, &query, code)?;
    test_get_400(runtime, routes_state, TOKEN_MEMBER, uri, &query, code)?;

    let mut query = Map::<String, Value>::new();
    query.insert("unit".to_string(), Value::String(UNIT_OWNER.to_string()));
    query.insert("tend".to_string(), Value::String("0".to_string()));
    test_get_400(runtime, routes_state, TOKEN_MANAGER, uri, &query, code)?;
    test_get_400(runtime, routes_state, TOKEN_OWNER, uri, &query, code)?;
    test_get_400(runtime, routes_state, TOKEN_MEMBER, uri, &query, code)?;

    let mut query = Map::<String, Value>::new();
    query.insert("unit".to_string(), Value::String(UNIT_OWNER.to_string()));
    query.insert("tstart".to_string(), Value::String("-1".to_string()));
    test_get_400(runtime, routes_state, TOKEN_MANAGER, uri, &query, code)?;
    test_get_400(runtime, routes_state, TOKEN_OWNER, uri, &query, code)?;
    test_get_400(runtime, routes_state, TOKEN_MEMBER, uri, &query, code)?;

    let mut query = Map::<String, Value>::new();
    query.insert("unit".to_string(), Value::String(UNIT_OWNER.to_string()));
    query.insert("tend".to_string(), Value::String("1.1".to_string()));
    test_get_400(runtime, routes_state, TOKEN_MANAGER, uri, &query, code)?;
    test_get_400(runtime, routes_state, TOKEN_OWNER, uri, &query, code)?;
    test_get_400(runtime, routes_state, TOKEN_MEMBER, uri, &query, code)?;

    let mut query = Map::<String, Value>::new();
    query.insert("unit".to_string(), Value::String(UNIT_OWNER.to_string()));
    query.insert("tfield".to_string(), Value::String("invalid".to_string()));
    query.insert("tstart".to_string(), Value::String("0".to_string()));
    test_get_400(runtime, routes_state, TOKEN_MANAGER, uri, &query, code)?;
    test_get_400(runtime, routes_state, TOKEN_OWNER, uri, &query, code)?;
    test_get_400(runtime, routes_state, TOKEN_MEMBER, uri, &query, code)?;

    let mut query = Map::<String, Value>::new();
    query.insert("unit".to_string(), Value::String(UNIT_OWNER.to_string()));
    query.insert("tfield".to_string(), Value::String("invalid".to_string()));
    query.insert("tend".to_string(), Value::String("0".to_string()));
    test_get_400(runtime, routes_state, TOKEN_MANAGER, uri, &query, code)?;
    test_get_400(runtime, routes_state, TOKEN_OWNER, uri, &query, code)?;
    test_get_400(runtime, routes_state, TOKEN_MEMBER, uri, &query, code)?;

    let mut query = Map::<String, Value>::new();
    query.insert("sort".to_string(), Value::String("".to_string()));
    test_get_400(runtime, &routes_state, TOKEN_MANAGER, uri, &query, code)?;

    let mut query = Map::<String, Value>::new();
    query.insert("sort".to_string(), Value::String("proc".to_string()));
    test_get_400(runtime, &routes_state, TOKEN_MANAGER, uri, &query, code)?;

    let mut query = Map::<String, Value>::new();
    query.insert("sort".to_string(), Value::String("proc:asc:c".to_string()));
    test_get_400(runtime, &routes_state, TOKEN_MANAGER, uri, &query, code)?;

    let mut query = Map::<String, Value>::new();
    query.insert(
        "sort".to_string(),
        Value::String("proc:asc,resp:true".to_string()),
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

    test_invalid_token(runtime, &routes_state, "/data/api/v1/network-dldata/list")
}

fn test_get_count(
    runtime: &Runtime,
    state: &routes::State,
    token: &str,
    param: Option<&request::GetCount>,
    expect_count: usize,
) -> Result<(), String> {
    let app = Router::new().merge(routes::new_service(state));
    let server = TestServer::new(app);

    let req = server
        .get("/data/api/v1/network-dldata/count")
        .add_query_params(&param)
        .add_header(
            header::AUTHORIZATION,
            HeaderValue::from_str(format!("Bearer {}", token).as_str()).unwrap(),
        );
    let resp = runtime.block_on(async { req.await });
    expect(resp.status_code()).to_equal(StatusCode::OK)?;
    let body: response::GetCount = resp.json();
    expect(body.data.count).to_equal(expect_count)
}

fn test_get_list(
    runtime: &Runtime,
    state: &routes::State,
    token: &str,
    param: Option<&request::GetList>,
    expect_count: usize,
    max_proc: i64,
) -> Result<(), String> {
    let app = Router::new().merge(routes::new_service(state));
    let server = TestServer::new(app);

    let req = server
        .get("/data/api/v1/network-dldata/list")
        .add_query_params(&param)
        .add_header(
            header::AUTHORIZATION,
            HeaderValue::from_str(format!("Bearer {}", token).as_str()).unwrap(),
        );
    let resp = runtime.block_on(async { req.await });
    expect(resp.status_code()).to_equal(StatusCode::OK)?;
    let body: response::GetList = resp.json();
    expect(body.data.len()).to_equal(expect_count)?;

    let mut prev_proc = max_proc;
    for info in body.data.iter() {
        let proc = match DateTime::parse_from_rfc3339(info.proc.as_str()) {
            Err(_) => return Err(format!("proc {} format error", info.proc.as_str())),
            Ok(proc) => proc.timestamp_millis(),
        };
        if let Some(resp) = info.resp.as_ref() {
            if let Err(_) = DateTime::parse_from_rfc3339(resp.as_str()) {
                return Err(format!("resp {} format error", resp.as_str()));
            }
        }
        if proc > prev_proc {
            return Err(format!(
                "proc order error: {} - {}",
                prev_proc,
                info.proc.as_str()
            ));
        }
        prev_proc = proc;
    }
    Ok(())
}

fn test_get_list_sort(
    runtime: &Runtime,
    state: &routes::State,
    token: &str,
    param: &mut request::GetList,
    expect_ids: &[&str],
) -> Result<(), String> {
    let app = Router::new().merge(routes::new_service(state));
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
        .get("/data/api/v1/network-dldata/list")
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
            "response not 200: /data/api/v1/network-dldata/list, {}, {}",
            body.code, message
        ));
    }
    let body: response::GetList = resp.json();
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
    param: &request::GetList,
    expect_ids: Vec<i32>,
) -> Result<(), String> {
    let app = Router::new().merge(routes::new_service(state));
    let server = TestServer::new(app);

    let req = server
        .get("/data/api/v1/network-dldata/list")
        .add_query_params(&param)
        .add_header(
            header::AUTHORIZATION,
            HeaderValue::from_str(format!("Bearer {}", token).as_str()).unwrap(),
        );
    let resp = runtime.block_on(async { req.await });
    expect(resp.status_code()).to_equal(StatusCode::OK)?;
    let body: response::GetList = resp.json();
    expect(body.data.len()).to_equal(expect_ids.len())?;

    let mut index = 0;
    for data in body.data.iter() {
        expect(data.data_id.as_str()).to_equal(format!("data_id{}", expect_ids[index]).as_str())?;
        index += 1;
    }
    Ok(())
}

fn test_get_list_format_array(
    runtime: &Runtime,
    state: &routes::State,
    token: &str,
    param: &mut request::GetList,
    expect_ids: Vec<i32>,
) -> Result<(), String> {
    let app = Router::new().merge(routes::new_service(state));
    let server = TestServer::new(app);

    param.format = Some("array".to_string());
    let req = server
        .get("/data/api/v1/network-dldata/list")
        .add_query_params(&param)
        .add_header(
            header::AUTHORIZATION,
            HeaderValue::from_str(format!("Bearer {}", token).as_str()).unwrap(),
        );
    let resp = runtime.block_on(async { req.await });
    expect(resp.status_code()).to_equal(StatusCode::OK)?;
    let body: Vec<response::GetListData> = resp.json();
    expect(body.len()).to_equal(expect_ids.len())?;

    let mut index = 0;
    for data in body.iter() {
        expect(data.data_id.as_str()).to_equal(format!("data_id{}", expect_ids[index]).as_str())?;
        index += 1;
    }
    Ok(())
}

fn test_get_list_format_csv(
    runtime: &Runtime,
    state: &routes::State,
    token: &str,
    param: &mut request::GetList,
    expect_ids: Vec<i32>,
) -> Result<(), String> {
    let app = Router::new().merge(routes::new_service(state));
    let server = TestServer::new(app);

    param.format = Some("csv".to_string());
    let req = server
        .get("/data/api/v1/network-dldata/list")
        .add_query_params(&param)
        .add_header(
            header::AUTHORIZATION,
            HeaderValue::from_str(format!("Bearer {}", token).as_str()).unwrap(),
        );
    let resp = runtime.block_on(async { req.await });
    expect(resp.status_code()).to_equal(StatusCode::OK)?;
    let body = resp.text();

    let fields =
        b"dataId,proc,pub,resp,status,unitId,deviceId,networkCode,networkAddr,profile,data,extension";
    let mut count = 0;
    for line in body.lines() {
        if count == 0 {
            let mut fields_line: Vec<u8> = vec![0xEF, 0xBB, 0xBF];
            fields_line.extend_from_slice(fields);
            expect(fields_line.as_slice()).to_equal(line.as_bytes())?;
        } else {
            expect(line.starts_with(format!("data_id{},", expect_ids[count - 1]).as_str()))
                .to_equal(true)?;
        }
        count += 1;
    }
    expect(expect_ids.len() + 1).to_equal(count)
}

fn count_list_dataset(
    runtime: &Runtime,
    state: &routes::State,
    now: DateTime<Utc>,
) -> Result<(), String> {
    let mut data = NetworkDlData {
        data_id: "data_id1".to_string(),
        proc: now,
        publish: now + TimeDelta::try_milliseconds(6).unwrap(),
        resp: None,
        status: -3,
        unit_id: UNIT_OWNER.to_string(),
        device_id: "device_id1_1".to_string(),
        network_code: "network_code1_1".to_string(),
        network_addr: "network_addr1_1".to_string(),
        profile: "profile1".to_string(),
        data: "data".to_string(),
        extension: None,
    };
    if let Err(e) = runtime.block_on(async {
        state.model.network_dldata().add(&data).await?;
        data.data_id = "data_id2".to_string();
        data.proc = now + TimeDelta::try_milliseconds(1).unwrap();
        data.publish = now + TimeDelta::try_milliseconds(7).unwrap();
        data.resp = Some(now + TimeDelta::try_milliseconds(4).unwrap());
        state.model.network_dldata().add(&data).await?;
        data.data_id = "data_id3".to_string();
        data.proc = now + TimeDelta::try_milliseconds(2).unwrap();
        data.publish = now + TimeDelta::try_milliseconds(8).unwrap();
        data.resp = Some(now + TimeDelta::try_milliseconds(3).unwrap());
        data.device_id = "device_id1_2".to_string();
        data.network_addr = "network_addr1_2".to_string();
        state.model.network_dldata().add(&data).await?;
        data.data_id = "data_id4".to_string();
        data.proc = now + TimeDelta::try_milliseconds(3).unwrap();
        data.publish = now + TimeDelta::try_milliseconds(9).unwrap();
        data.resp = Some(now + TimeDelta::try_milliseconds(2).unwrap());
        data.device_id = "device_id2".to_string();
        data.network_code = "network_code2".to_string();
        data.network_addr = "network_addr2".to_string();
        data.profile = "profile2".to_string();
        state.model.network_dldata().add(&data).await?;
        data.data_id = "data_id5".to_string();
        data.unit_id = "unit_id2".to_string();
        data.proc = now + TimeDelta::try_milliseconds(4).unwrap();
        data.publish = now + TimeDelta::try_milliseconds(10).unwrap();
        data.resp = Some(now + TimeDelta::try_milliseconds(1).unwrap());
        data.device_id = "device_id3".to_string();
        data.network_code = "network_code3".to_string();
        data.network_addr = "network_addr3".to_string();
        data.profile = "profile3".to_string();
        data.extension = Some(Map::new());
        state.model.network_dldata().add(&data).await
    }) {
        return Err(format!("model.add() error: {}", e));
    }

    Ok(())
}

fn add_offset_limit_data(
    runtime: &Runtime,
    state: &routes::State,
    data_id: &str,
    proc: DateTime<Utc>,
    use_some: bool,
) -> Result<(), String> {
    match runtime.block_on(async {
        let data = NetworkDlData {
            data_id: data_id.to_string(),
            proc,
            publish: proc,
            resp: match use_some {
                false => None,
                true => Some(proc),
            },
            status: 0,
            unit_id: UNIT_OWNER.to_string(),
            device_id: "device_id".to_string(),
            network_code: "network_code".to_string(),
            network_addr: "network_addr".to_string(),
            profile: "profile".to_string(),
            data: "00".to_string(),
            extension: match use_some {
                false => None,
                true => Some(Map::new()),
            },
        };
        state.model.network_dldata().add(&data).await
    }) {
        Err(e) => Err(format!("add data model error: {}", e)),
        Ok(_) => Ok(()),
    }
}
