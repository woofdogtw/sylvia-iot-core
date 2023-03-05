use std::cmp::Ordering;

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

use sylvia_iot_broker::{
    models::device::{ListOptions, ListQueryCond, QueryCond, SortCond, SortKey},
    routes,
};
use sylvia_iot_corelib::{err, strings};

use super::{
    super::{
        super::libs::{
            add_delete_rsc, add_device_model, add_network_model, add_unit_model, create_device,
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
    add_network_model(runtime, routes_state, "", "public", "amqp://host")?;
    add_network_model(runtime, routes_state, "manager", "manager", "amqp://host")?;
    add_network_model(runtime, routes_state, "owner", "owner", "amqp://host")?;

    let mut info = Map::<String, Value>::new();
    info.insert("info".to_string(), Value::String("value".to_string()));
    let param = request::PostDevice {
        data: request::PostDeviceData {
            unit_id: "manager".to_string(),
            network_id: "public".to_string(),
            network_addr: "addr".to_string(),
            name: Some("manager".to_string()),
            info: Some(info),
        },
    };
    test_post(
        runtime,
        routes_state,
        TOKEN_MANAGER,
        &param,
        "",
        None,
        "public",
    )?;

    let param = request::PostDevice {
        data: request::PostDeviceData {
            unit_id: "manager".to_string(),
            network_id: "manager".to_string(),
            network_addr: "addr".to_string(),
            name: None,
            info: None,
        },
    };
    test_post(
        runtime,
        routes_state,
        TOKEN_MANAGER,
        &param,
        "",
        Some("manager"),
        "manager",
    )?;

    let param = request::PostDevice {
        data: request::PostDeviceData {
            unit_id: "owner".to_string(),
            network_id: "public".to_string(),
            network_addr: "owner-addr".to_string(),
            name: None,
            info: None,
        },
    };
    test_post(
        runtime,
        routes_state,
        TOKEN_MANAGER,
        &param,
        "",
        None,
        "public",
    )?;

    let param = request::PostDevice {
        data: request::PostDeviceData {
            unit_id: "owner".to_string(),
            network_id: "owner".to_string(),
            network_addr: "addr-Owner".to_string(),
            name: None,
            info: None,
        },
    };
    test_post(
        runtime,
        routes_state,
        TOKEN_MANAGER,
        &param,
        "",
        Some("owner"),
        "owner",
    )?;

    let param = request::PostDevice {
        data: request::PostDeviceData {
            unit_id: "owner".to_string(),
            network_id: "owner".to_string(),
            network_addr: "addr".to_string(),
            name: None,
            info: None,
        },
    };
    test_post(
        runtime,
        routes_state,
        TOKEN_OWNER,
        &param,
        "",
        Some("owner"),
        "owner",
    )
}

pub fn post_dup(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    add_unit_model(runtime, routes_state, "manager", vec![], "manager")?;
    add_network_model(runtime, routes_state, "", "public", "amqp://host")?;
    add_network_model(runtime, routes_state, "manager", "manager", "amqp://host")?;

    let param = request::PostDevice {
        data: request::PostDeviceData {
            unit_id: "manager".to_string(),
            network_id: "public".to_string(),
            network_addr: "addr".to_string(),
            name: None,
            info: None,
        },
    };
    test_post(
        runtime,
        routes_state,
        TOKEN_MANAGER,
        &param,
        "",
        None,
        "public",
    )?;

    let param = request::PostDevice {
        data: request::PostDeviceData {
            unit_id: "manager".to_string(),
            network_id: "public".to_string(),
            network_addr: "addr".to_string(),
            name: None,
            info: None,
        },
    };
    test_post(
        runtime,
        routes_state,
        TOKEN_MANAGER,
        &param,
        "err_broker_network_addr_exist",
        None,
        "",
    )?;

    let param = request::PostDevice {
        data: request::PostDeviceData {
            unit_id: "manager".to_string(),
            network_id: "manager".to_string(),
            network_addr: "addr".to_string(),
            name: None,
            info: None,
        },
    };
    test_post(
        runtime,
        routes_state,
        TOKEN_MANAGER,
        &param,
        "",
        Some("manager"),
        "manager",
    )?;

    let param = request::PostDevice {
        data: request::PostDeviceData {
            unit_id: "manager".to_string(),
            network_id: "manager".to_string(),
            network_addr: "addr".to_string(),
            name: None,
            info: None,
        },
    };
    test_post(
        runtime,
        routes_state,
        TOKEN_MANAGER,
        &param,
        "err_broker_network_addr_exist",
        None,
        "",
    )
}

pub fn post_not_exist(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    add_unit_model(runtime, routes_state, "manager", vec!["member"], "manager")?;
    add_unit_model(runtime, routes_state, "owner", vec![], "owner")?;
    add_network_model(runtime, routes_state, "", "public", "amqp://host")?;
    add_network_model(runtime, routes_state, "manager", "manager", "amqp://host")?;
    add_network_model(runtime, routes_state, "owner", "owner", "amqp://host")?;

    let param = request::PostDevice {
        data: request::PostDeviceData {
            unit_id: "id".to_string(),
            network_id: "public".to_string(),
            network_addr: "addr".to_string(),
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
        None,
        "public",
    )?;

    let param = request::PostDevice {
        data: request::PostDeviceData {
            unit_id: "manager".to_string(),
            network_id: "manager".to_string(),
            network_addr: "addr".to_string(),
            name: None,
            info: None,
        },
    };
    test_post(
        runtime,
        routes_state,
        TOKEN_OWNER,
        &param,
        "err_broker_unit_not_exist",
        Some("manager"),
        "manager",
    )?;

    let param = request::PostDevice {
        data: request::PostDeviceData {
            unit_id: "manager".to_string(),
            network_id: "id".to_string(),
            network_addr: "addr".to_string(),
            name: None,
            info: None,
        },
    };
    test_post(
        runtime,
        routes_state,
        TOKEN_MANAGER,
        &param,
        "err_broker_network_not_exist",
        None,
        "public",
    )?;

    let param = request::PostDevice {
        data: request::PostDeviceData {
            unit_id: "owner".to_string(),
            network_id: "public".to_string(),
            network_addr: "addr".to_string(),
            name: None,
            info: None,
        },
    };
    test_post(
        runtime,
        routes_state,
        TOKEN_OWNER,
        &param,
        "err_broker_network_not_exist",
        None,
        "public",
    )?;

    let param = request::PostDevice {
        data: request::PostDeviceData {
            unit_id: "owner".to_string(),
            network_id: "manager".to_string(),
            network_addr: "addr".to_string(),
            name: None,
            info: None,
        },
    };
    test_post(
        runtime,
        routes_state,
        TOKEN_OWNER,
        &param,
        "err_broker_network_not_exist",
        None,
        "public",
    )
}

pub fn post_invalid_param(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    add_unit_model(runtime, routes_state, "manager", vec![], "manager")?;

    test_post_invalid_param(runtime, &routes_state, TOKEN_MANAGER, None)?;

    let param = request::PostDevice {
        data: request::PostDeviceData {
            unit_id: "".to_string(),
            network_id: "manager".to_string(),
            network_addr: "addr".to_string(),
            name: None,
            info: None,
        },
    };
    test_post_invalid_param(runtime, &routes_state, TOKEN_MANAGER, Some(&param))?;

    let param = request::PostDevice {
        data: request::PostDeviceData {
            unit_id: "manager".to_string(),
            network_id: "".to_string(),
            network_addr: "addr".to_string(),
            name: None,
            info: None,
        },
    };
    test_post_invalid_param(runtime, &routes_state, TOKEN_MANAGER, Some(&param))?;

    let param = request::PostDevice {
        data: request::PostDeviceData {
            unit_id: "manager".to_string(),
            network_id: "manager".to_string(),
            network_addr: "".to_string(),
            name: None,
            info: None,
        },
    };
    test_post_invalid_param(runtime, &routes_state, TOKEN_MANAGER, Some(&param))
}

pub fn post_invalid_token(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    let req = TestRequest::post().uri("/broker/api/v1/device");
    test_invalid_token(runtime, &routes_state, req)
}

pub fn post_bulk(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    add_unit_model(runtime, routes_state, "manager", vec![], "manager")?;
    add_unit_model(runtime, routes_state, "owner", vec![], "owner")?;
    add_network_model(runtime, routes_state, "", "public", "amqp://host")?;
    add_network_model(runtime, routes_state, "owner", "owner", "amqp://host")?;

    let mut network_addrs1 = vec![];
    for i in 0..1024 {
        network_addrs1.push(strings::u128_to_addr(i, 32));
    }
    let mut param = request::PostDeviceBulk {
        data: request::PostDeviceBulkData {
            unit_id: "manager".to_string(),
            network_id: "public".to_string(),
            network_addrs: network_addrs1.clone(),
        },
    };
    test_post_bulk(
        runtime,
        routes_state,
        TOKEN_MANAGER,
        true,
        &param,
        "",
        None,
        "public",
        &param.data.network_addrs,
    )?;

    let mut network_addrs2 = vec![];
    for i in 1..1025 {
        network_addrs2.push(strings::u128_to_addr(i, 32));
    }
    network_addrs1.push(strings::u128_to_addr(1024, 32));
    param.data.network_addrs = network_addrs2.clone();
    test_post_bulk(
        runtime,
        routes_state,
        TOKEN_MANAGER,
        true,
        &param,
        "",
        None,
        "public",
        &network_addrs1,
    )?;

    network_addrs1.pop();
    param.data.unit_id = "owner".to_string();
    param.data.network_id = "owner".to_string();
    param.data.network_addrs = network_addrs1.clone();
    test_post_bulk(
        runtime,
        routes_state,
        TOKEN_OWNER,
        true,
        &param,
        "",
        Some("owner"),
        "owner",
        &network_addrs1,
    )?;

    param.data.network_addrs = network_addrs2;
    network_addrs1.push(strings::u128_to_addr(1024, 32));
    test_post_bulk(
        runtime,
        routes_state,
        TOKEN_OWNER,
        true,
        &param,
        "",
        Some("owner"),
        "owner",
        &network_addrs1,
    )
}

pub fn post_bulk_not_exist(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    add_unit_model(runtime, routes_state, "manager", vec!["member"], "manager")?;
    add_unit_model(runtime, routes_state, "owner", vec![], "owner")?;
    add_network_model(runtime, routes_state, "", "public", "amqp://host")?;
    add_network_model(runtime, routes_state, "manager", "manager", "amqp://host")?;
    add_network_model(runtime, routes_state, "owner", "owner", "amqp://host")?;

    let param = request::PostDeviceBulk {
        data: request::PostDeviceBulkData {
            unit_id: "id".to_string(),
            network_id: "public".to_string(),
            network_addrs: vec!["0000".to_string()],
        },
    };
    test_post_bulk(
        runtime,
        routes_state,
        TOKEN_MANAGER,
        true,
        &param,
        "err_broker_unit_not_exist",
        None,
        "public",
        &vec!["0000".to_string()],
    )?;

    let param = request::PostDeviceBulk {
        data: request::PostDeviceBulkData {
            unit_id: "manager".to_string(),
            network_id: "manager".to_string(),
            network_addrs: vec!["0000".to_string()],
        },
    };
    test_post_bulk(
        runtime,
        routes_state,
        TOKEN_OWNER,
        true,
        &param,
        "err_broker_unit_not_exist",
        Some("manager"),
        "manager",
        &vec!["0000".to_string()],
    )?;

    let param = request::PostDeviceBulk {
        data: request::PostDeviceBulkData {
            unit_id: "manager".to_string(),
            network_id: "id".to_string(),
            network_addrs: vec!["0000".to_string()],
        },
    };
    test_post_bulk(
        runtime,
        routes_state,
        TOKEN_MANAGER,
        true,
        &param,
        "err_broker_network_not_exist",
        None,
        "public",
        &vec!["0000".to_string()],
    )?;

    let param = request::PostDeviceBulk {
        data: request::PostDeviceBulkData {
            unit_id: "owner".to_string(),
            network_id: "public".to_string(),
            network_addrs: vec!["0000".to_string()],
        },
    };
    test_post_bulk(
        runtime,
        routes_state,
        TOKEN_OWNER,
        true,
        &param,
        "err_broker_network_not_exist",
        None,
        "public",
        &vec!["0000".to_string()],
    )?;

    let param = request::PostDeviceBulk {
        data: request::PostDeviceBulkData {
            unit_id: "owner".to_string(),
            network_id: "manager".to_string(),
            network_addrs: vec!["0000".to_string()],
        },
    };
    test_post_bulk(
        runtime,
        routes_state,
        TOKEN_OWNER,
        true,
        &param,
        "err_broker_network_not_exist",
        None,
        "public",
        &vec!["0000".to_string()],
    )
}

pub fn post_bulk_invalid_param(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    add_unit_model(runtime, routes_state, "manager", vec![], "manager")?;

    test_post_bulk_invalid_param(runtime, &routes_state, TOKEN_MANAGER, true, None)?;

    let param = request::PostDeviceBulk {
        data: request::PostDeviceBulkData {
            unit_id: "".to_string(),
            network_id: "manager".to_string(),
            network_addrs: vec!["0000".to_string()],
        },
    };
    test_post_bulk_invalid_param(runtime, &routes_state, TOKEN_MANAGER, true, Some(&param))?;

    let param = request::PostDeviceBulk {
        data: request::PostDeviceBulkData {
            unit_id: "manager".to_string(),
            network_id: "".to_string(),
            network_addrs: vec!["0000".to_string()],
        },
    };
    test_post_bulk_invalid_param(runtime, &routes_state, TOKEN_MANAGER, true, Some(&param))?;

    let param = request::PostDeviceBulk {
        data: request::PostDeviceBulkData {
            unit_id: "manager".to_string(),
            network_id: "manager".to_string(),
            network_addrs: vec![],
        },
    };
    test_post_bulk_invalid_param(runtime, &routes_state, TOKEN_MANAGER, true, Some(&param))?;

    let mut network_addrs = vec![];
    for i in 0..1025 {
        network_addrs.push(format!("{:#04}", i));
    }
    let param = request::PostDeviceBulk {
        data: request::PostDeviceBulkData {
            unit_id: "manager".to_string(),
            network_id: "manager".to_string(),
            network_addrs,
        },
    };
    test_post_bulk_invalid_param(runtime, &routes_state, TOKEN_MANAGER, true, Some(&param))?;

    let param = request::PostDeviceBulk {
        data: request::PostDeviceBulkData {
            unit_id: "manager".to_string(),
            network_id: "manager".to_string(),
            network_addrs: vec!["".to_string()],
        },
    };
    test_post_bulk_invalid_param(runtime, &routes_state, TOKEN_MANAGER, true, Some(&param))
}

pub fn post_bulk_invalid_token(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    let req = TestRequest::post().uri("/broker/api/v1/device/bulk");
    test_invalid_token(runtime, &routes_state, req)
}

pub fn post_bulk_del(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    add_unit_model(runtime, routes_state, "manager", vec![], "manager")?;
    add_unit_model(runtime, routes_state, "owner", vec![], "owner")?;
    add_network_model(runtime, routes_state, "", "public", "amqp://host")?;
    add_network_model(runtime, routes_state, "owner", "owner", "amqp://host")?;

    let mut network_addrs1 = vec![];
    for i in 0..1024 {
        network_addrs1.push(strings::u128_to_addr(i, 32));
    }
    let mut param = request::PostDeviceBulk {
        data: request::PostDeviceBulkData {
            unit_id: "manager".to_string(),
            network_id: "public".to_string(),
            network_addrs: network_addrs1.clone(),
        },
    };
    test_post_bulk(
        runtime,
        routes_state,
        TOKEN_MANAGER,
        true,
        &param,
        "",
        None,
        "public",
        &network_addrs1,
    )?;
    param.data.network_addrs = vec![strings::u128_to_addr(1024, 32)];
    network_addrs1.push(strings::u128_to_addr(1024, 32));
    test_post_bulk(
        runtime,
        routes_state,
        TOKEN_MANAGER,
        true,
        &param,
        "",
        None,
        "public",
        &network_addrs1,
    )?;
    network_addrs1.pop();
    param.data.unit_id = "owner".to_string();
    param.data.network_id = "owner".to_string();
    param.data.network_addrs = network_addrs1.clone();
    test_post_bulk(
        runtime,
        routes_state,
        TOKEN_OWNER,
        true,
        &param,
        "",
        Some("owner"),
        "owner",
        &network_addrs1,
    )?;
    param.data.network_addrs = vec![strings::u128_to_addr(1024, 32)];
    network_addrs1.push(strings::u128_to_addr(1024, 32));
    test_post_bulk(
        runtime,
        routes_state,
        TOKEN_OWNER,
        true,
        &param,
        "",
        Some("owner"),
        "owner",
        &network_addrs1,
    )?;

    let mut network_addrs1 = vec![];
    for i in 0..1024 {
        network_addrs1.push(strings::u128_to_addr(i, 32));
    }
    let mut param = request::PostDeviceBulk {
        data: request::PostDeviceBulkData {
            unit_id: "manager".to_string(),
            network_id: "public".to_string(),
            network_addrs: network_addrs1.clone(),
        },
    };
    test_post_bulk(
        runtime,
        routes_state,
        TOKEN_MANAGER,
        false,
        &param,
        "",
        None,
        "public",
        &vec![strings::u128_to_addr(1024, 32)],
    )?;

    param.data.unit_id = "owner".to_string();
    param.data.network_id = "owner".to_string();
    test_post_bulk(
        runtime,
        routes_state,
        TOKEN_OWNER,
        false,
        &param,
        "",
        Some("owner"),
        "owner",
        &vec![strings::u128_to_addr(1024, 32)],
    )
}

pub fn post_bulk_del_not_exist(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    add_unit_model(runtime, routes_state, "manager", vec!["member"], "manager")?;
    add_unit_model(runtime, routes_state, "owner", vec![], "owner")?;
    add_network_model(runtime, routes_state, "", "public", "amqp://host")?;
    add_network_model(runtime, routes_state, "manager", "manager", "amqp://host")?;
    add_network_model(runtime, routes_state, "owner", "owner", "amqp://host")?;

    let param = request::PostDeviceBulk {
        data: request::PostDeviceBulkData {
            unit_id: "id".to_string(),
            network_id: "public".to_string(),
            network_addrs: vec!["0000".to_string()],
        },
    };
    test_post_bulk(
        runtime,
        routes_state,
        TOKEN_MANAGER,
        false,
        &param,
        "err_broker_unit_not_exist",
        None,
        "public",
        &vec!["0000".to_string()],
    )?;

    let param = request::PostDeviceBulk {
        data: request::PostDeviceBulkData {
            unit_id: "manager".to_string(),
            network_id: "manager".to_string(),
            network_addrs: vec!["0000".to_string()],
        },
    };
    test_post_bulk(
        runtime,
        routes_state,
        TOKEN_OWNER,
        false,
        &param,
        "err_broker_unit_not_exist",
        Some("manager"),
        "manager",
        &vec!["0000".to_string()],
    )?;

    let param = request::PostDeviceBulk {
        data: request::PostDeviceBulkData {
            unit_id: "manager".to_string(),
            network_id: "id".to_string(),
            network_addrs: vec!["0000".to_string()],
        },
    };
    test_post_bulk(
        runtime,
        routes_state,
        TOKEN_MANAGER,
        false,
        &param,
        "err_broker_network_not_exist",
        None,
        "public",
        &vec!["0000".to_string()],
    )?;

    let param = request::PostDeviceBulk {
        data: request::PostDeviceBulkData {
            unit_id: "owner".to_string(),
            network_id: "public".to_string(),
            network_addrs: vec!["0000".to_string()],
        },
    };
    test_post_bulk(
        runtime,
        routes_state,
        TOKEN_OWNER,
        false,
        &param,
        "err_broker_network_not_exist",
        None,
        "public",
        &vec!["0000".to_string()],
    )?;

    let param = request::PostDeviceBulk {
        data: request::PostDeviceBulkData {
            unit_id: "owner".to_string(),
            network_id: "manager".to_string(),
            network_addrs: vec!["0000".to_string()],
        },
    };
    test_post_bulk(
        runtime,
        routes_state,
        TOKEN_OWNER,
        false,
        &param,
        "err_broker_network_not_exist",
        None,
        "public",
        &vec!["0000".to_string()],
    )
}

pub fn post_bulk_del_invalid_param(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    add_unit_model(runtime, routes_state, "manager", vec![], "manager")?;

    test_post_bulk_invalid_param(runtime, &routes_state, TOKEN_MANAGER, false, None)?;

    let param = request::PostDeviceBulk {
        data: request::PostDeviceBulkData {
            unit_id: "".to_string(),
            network_id: "manager".to_string(),
            network_addrs: vec!["0000".to_string()],
        },
    };
    test_post_bulk_invalid_param(runtime, &routes_state, TOKEN_MANAGER, false, Some(&param))?;

    let param = request::PostDeviceBulk {
        data: request::PostDeviceBulkData {
            unit_id: "manager".to_string(),
            network_id: "".to_string(),
            network_addrs: vec!["0000".to_string()],
        },
    };
    test_post_bulk_invalid_param(runtime, &routes_state, TOKEN_MANAGER, false, Some(&param))?;

    let param = request::PostDeviceBulk {
        data: request::PostDeviceBulkData {
            unit_id: "manager".to_string(),
            network_id: "manager".to_string(),
            network_addrs: vec![],
        },
    };
    test_post_bulk_invalid_param(runtime, &routes_state, TOKEN_MANAGER, false, Some(&param))?;

    let mut network_addrs = vec![];
    for i in 0..1025 {
        network_addrs.push(format!("{:#04}", i));
    }
    let param = request::PostDeviceBulk {
        data: request::PostDeviceBulkData {
            unit_id: "manager".to_string(),
            network_id: "manager".to_string(),
            network_addrs,
        },
    };
    test_post_bulk_invalid_param(runtime, &routes_state, TOKEN_MANAGER, false, Some(&param))?;

    let param = request::PostDeviceBulk {
        data: request::PostDeviceBulkData {
            unit_id: "manager".to_string(),
            network_id: "manager".to_string(),
            network_addrs: vec!["".to_string()],
        },
    };
    test_post_bulk_invalid_param(runtime, &routes_state, TOKEN_MANAGER, false, Some(&param))
}

pub fn post_bulk_del_invalid_token(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    let req = TestRequest::post().uri("/broker/api/v1/device/bulk-delete");
    test_invalid_token(runtime, &routes_state, req)
}

pub fn post_range(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    add_unit_model(runtime, routes_state, "manager", vec![], "manager")?;
    add_unit_model(runtime, routes_state, "owner", vec![], "owner")?;
    add_network_model(runtime, routes_state, "", "public", "amqp://host")?;
    add_network_model(runtime, routes_state, "owner", "owner", "amqp://host")?;

    let mut network_addrs1 = vec![];
    for i in 0..1024 {
        network_addrs1.push(strings::u128_to_addr(i, 32));
    }
    let mut param = request::PostDeviceRange {
        data: request::PostDeviceRangeData {
            unit_id: "manager".to_string(),
            network_id: "public".to_string(),
            start_addr: strings::u128_to_addr(0, 32),
            end_addr: strings::u128_to_addr(1023, 32),
        },
    };
    test_post_range(
        runtime,
        routes_state,
        TOKEN_MANAGER,
        true,
        &param,
        "",
        None,
        "public",
        &network_addrs1,
    )?;

    network_addrs1.push(strings::u128_to_addr(1024, 32));
    param.data.start_addr = strings::u128_to_addr(1, 32);
    param.data.end_addr = strings::u128_to_addr(1024, 32);
    test_post_range(
        runtime,
        routes_state,
        TOKEN_MANAGER,
        true,
        &param,
        "",
        None,
        "public",
        &network_addrs1,
    )?;

    network_addrs1.pop();
    param.data.unit_id = "owner".to_string();
    param.data.network_id = "owner".to_string();
    param.data.start_addr = strings::u128_to_addr(0, 32);
    param.data.end_addr = strings::u128_to_addr(1023, 32);
    test_post_range(
        runtime,
        routes_state,
        TOKEN_OWNER,
        true,
        &param,
        "",
        Some("owner"),
        "owner",
        &network_addrs1,
    )?;

    param.data.start_addr = strings::u128_to_addr(1, 32);
    param.data.end_addr = strings::u128_to_addr(1024, 32);
    network_addrs1.push(strings::u128_to_addr(1024, 32));
    test_post_range(
        runtime,
        routes_state,
        TOKEN_OWNER,
        true,
        &param,
        "",
        Some("owner"),
        "owner",
        &network_addrs1,
    )
}

pub fn post_range_not_exist(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    add_unit_model(runtime, routes_state, "manager", vec!["member"], "manager")?;
    add_unit_model(runtime, routes_state, "owner", vec![], "owner")?;
    add_network_model(runtime, routes_state, "", "public", "amqp://host")?;
    add_network_model(runtime, routes_state, "manager", "manager", "amqp://host")?;
    add_network_model(runtime, routes_state, "owner", "owner", "amqp://host")?;

    let param = request::PostDeviceRange {
        data: request::PostDeviceRangeData {
            unit_id: "id".to_string(),
            network_id: "public".to_string(),
            start_addr: "0000".to_string(),
            end_addr: "0000".to_string(),
        },
    };
    test_post_range(
        runtime,
        routes_state,
        TOKEN_MANAGER,
        true,
        &param,
        "err_broker_unit_not_exist",
        None,
        "public",
        &vec!["0000".to_string()],
    )?;

    let param = request::PostDeviceRange {
        data: request::PostDeviceRangeData {
            unit_id: "manager".to_string(),
            network_id: "manager".to_string(),
            start_addr: "0000".to_string(),
            end_addr: "0000".to_string(),
        },
    };
    test_post_range(
        runtime,
        routes_state,
        TOKEN_OWNER,
        true,
        &param,
        "err_broker_unit_not_exist",
        Some("manager"),
        "manager",
        &vec!["0000".to_string()],
    )?;

    let param = request::PostDeviceRange {
        data: request::PostDeviceRangeData {
            unit_id: "manager".to_string(),
            network_id: "id".to_string(),
            start_addr: "0000".to_string(),
            end_addr: "0000".to_string(),
        },
    };
    test_post_range(
        runtime,
        routes_state,
        TOKEN_MANAGER,
        true,
        &param,
        "err_broker_network_not_exist",
        None,
        "public",
        &vec!["0000".to_string()],
    )?;

    let param = request::PostDeviceRange {
        data: request::PostDeviceRangeData {
            unit_id: "owner".to_string(),
            network_id: "public".to_string(),
            start_addr: "0000".to_string(),
            end_addr: "0000".to_string(),
        },
    };
    test_post_range(
        runtime,
        routes_state,
        TOKEN_OWNER,
        true,
        &param,
        "err_broker_network_not_exist",
        None,
        "public",
        &vec!["0000".to_string()],
    )?;

    let param = request::PostDeviceRange {
        data: request::PostDeviceRangeData {
            unit_id: "owner".to_string(),
            network_id: "manager".to_string(),
            start_addr: "0000".to_string(),
            end_addr: "0000".to_string(),
        },
    };
    test_post_range(
        runtime,
        routes_state,
        TOKEN_OWNER,
        true,
        &param,
        "err_broker_network_not_exist",
        None,
        "public",
        &vec!["0000".to_string()],
    )
}

pub fn post_range_invalid_param(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    add_unit_model(runtime, routes_state, "manager", vec![], "manager")?;

    test_post_range_invalid_param(runtime, &routes_state, TOKEN_MANAGER, true, None)?;

    let param = request::PostDeviceRange {
        data: request::PostDeviceRangeData {
            unit_id: "".to_string(),
            network_id: "manager".to_string(),
            start_addr: "0000".to_string(),
            end_addr: "0000".to_string(),
        },
    };
    test_post_range_invalid_param(runtime, &routes_state, TOKEN_MANAGER, true, Some(&param))?;

    let param = request::PostDeviceRange {
        data: request::PostDeviceRangeData {
            unit_id: "manager".to_string(),
            network_id: "".to_string(),
            start_addr: "0000".to_string(),
            end_addr: "0000".to_string(),
        },
    };
    test_post_range_invalid_param(runtime, &routes_state, TOKEN_MANAGER, true, Some(&param))?;

    let param = request::PostDeviceRange {
        data: request::PostDeviceRangeData {
            unit_id: "manager".to_string(),
            network_id: "manager".to_string(),
            start_addr: "".to_string(),
            end_addr: "0000".to_string(),
        },
    };
    test_post_range_invalid_param(runtime, &routes_state, TOKEN_MANAGER, true, Some(&param))?;

    let param = request::PostDeviceRange {
        data: request::PostDeviceRangeData {
            unit_id: "manager".to_string(),
            network_id: "manager".to_string(),
            start_addr: "0000".to_string(),
            end_addr: "".to_string(),
        },
    };
    test_post_range_invalid_param(runtime, &routes_state, TOKEN_MANAGER, true, Some(&param))?;

    let param = request::PostDeviceRange {
        data: request::PostDeviceRangeData {
            unit_id: "manager".to_string(),
            network_id: "manager".to_string(),
            start_addr: "000g".to_string(),
            end_addr: "0000".to_string(),
        },
    };
    test_post_range_invalid_param(runtime, &routes_state, TOKEN_MANAGER, true, Some(&param))?;

    let param = request::PostDeviceRange {
        data: request::PostDeviceRangeData {
            unit_id: "manager".to_string(),
            network_id: "manager".to_string(),
            start_addr: "0000".to_string(),
            end_addr: "000g".to_string(),
        },
    };
    test_post_range_invalid_param(runtime, &routes_state, TOKEN_MANAGER, true, Some(&param))?;

    let param = request::PostDeviceRange {
        data: request::PostDeviceRangeData {
            unit_id: "manager".to_string(),
            network_id: "manager".to_string(),
            start_addr: "0000".to_string(),
            end_addr: "000000".to_string(),
        },
    };
    test_post_range_invalid_param(runtime, &routes_state, TOKEN_MANAGER, true, Some(&param))?;

    let param = request::PostDeviceRange {
        data: request::PostDeviceRangeData {
            unit_id: "manager".to_string(),
            network_id: "manager".to_string(),
            start_addr: "0000".to_string(),
            end_addr: "0400".to_string(),
        },
    };
    test_post_range_invalid_param(runtime, &routes_state, TOKEN_MANAGER, true, Some(&param))?;

    let param = request::PostDeviceRange {
        data: request::PostDeviceRangeData {
            unit_id: "manager".to_string(),
            network_id: "manager".to_string(),
            start_addr: "0001".to_string(),
            end_addr: "0000".to_string(),
        },
    };
    test_post_range_invalid_param(runtime, &routes_state, TOKEN_MANAGER, true, Some(&param))?;

    let param = request::PostDeviceRange {
        data: request::PostDeviceRangeData {
            unit_id: "manager".to_string(),
            network_id: "manager".to_string(),
            start_addr: "0000000000000000000000000000000000".to_string(),
            end_addr: "0000000000000000000000000000000000".to_string(),
        },
    };
    test_post_range_invalid_param(runtime, &routes_state, TOKEN_MANAGER, true, Some(&param))
}

pub fn post_range_invalid_token(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    let req = TestRequest::post().uri("/broker/api/v1/device/range");
    test_invalid_token(runtime, &routes_state, req)
}

pub fn post_range_del(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    add_unit_model(runtime, routes_state, "manager", vec![], "manager")?;
    add_unit_model(runtime, routes_state, "owner", vec![], "owner")?;
    add_network_model(runtime, routes_state, "", "public", "amqp://host")?;
    add_network_model(runtime, routes_state, "owner", "owner", "amqp://host")?;

    let mut network_addrs1 = vec![];
    for i in 0..1024 {
        network_addrs1.push(strings::u128_to_addr(i, 32));
    }
    let mut param = request::PostDeviceRange {
        data: request::PostDeviceRangeData {
            unit_id: "manager".to_string(),
            network_id: "public".to_string(),
            start_addr: strings::u128_to_addr(0, 32),
            end_addr: strings::u128_to_addr(1023, 32),
        },
    };
    test_post_range(
        runtime,
        routes_state,
        TOKEN_MANAGER,
        true,
        &param,
        "",
        None,
        "public",
        &network_addrs1,
    )?;
    network_addrs1.push(strings::u128_to_addr(1024, 32));
    param.data.start_addr = strings::u128_to_addr(1024, 32);
    param.data.end_addr = strings::u128_to_addr(1024, 32);
    test_post_range(
        runtime,
        routes_state,
        TOKEN_MANAGER,
        true,
        &param,
        "",
        None,
        "public",
        &network_addrs1,
    )?;
    network_addrs1.pop();
    param.data.unit_id = "owner".to_string();
    param.data.network_id = "owner".to_string();
    param.data.start_addr = strings::u128_to_addr(0, 32);
    param.data.end_addr = strings::u128_to_addr(1023, 32);
    test_post_range(
        runtime,
        routes_state,
        TOKEN_OWNER,
        true,
        &param,
        "",
        Some("owner"),
        "owner",
        &network_addrs1,
    )?;
    param.data.start_addr = strings::u128_to_addr(1024, 32);
    param.data.end_addr = strings::u128_to_addr(1024, 32);
    network_addrs1.push(strings::u128_to_addr(1024, 32));
    test_post_range(
        runtime,
        routes_state,
        TOKEN_OWNER,
        true,
        &param,
        "",
        Some("owner"),
        "owner",
        &network_addrs1,
    )?;

    let mut param = request::PostDeviceRange {
        data: request::PostDeviceRangeData {
            unit_id: "manager".to_string(),
            network_id: "public".to_string(),
            start_addr: strings::u128_to_addr(0, 32),
            end_addr: strings::u128_to_addr(1023, 32),
        },
    };
    test_post_range(
        runtime,
        routes_state,
        TOKEN_MANAGER,
        false,
        &param,
        "",
        None,
        "public",
        &vec![strings::u128_to_addr(1024, 32)],
    )?;

    param.data.unit_id = "owner".to_string();
    param.data.network_id = "owner".to_string();
    test_post_range(
        runtime,
        routes_state,
        TOKEN_OWNER,
        false,
        &param,
        "",
        Some("owner"),
        "owner",
        &vec![strings::u128_to_addr(1024, 32)],
    )
}

pub fn post_range_del_not_exist(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    add_unit_model(runtime, routes_state, "manager", vec!["member"], "manager")?;
    add_unit_model(runtime, routes_state, "owner", vec![], "owner")?;
    add_network_model(runtime, routes_state, "", "public", "amqp://host")?;
    add_network_model(runtime, routes_state, "manager", "manager", "amqp://host")?;
    add_network_model(runtime, routes_state, "owner", "owner", "amqp://host")?;

    let param = request::PostDeviceRange {
        data: request::PostDeviceRangeData {
            unit_id: "id".to_string(),
            network_id: "public".to_string(),
            start_addr: "0000".to_string(),
            end_addr: "0000".to_string(),
        },
    };
    test_post_range(
        runtime,
        routes_state,
        TOKEN_MANAGER,
        false,
        &param,
        "err_broker_unit_not_exist",
        None,
        "public",
        &vec!["0000".to_string()],
    )?;

    let param = request::PostDeviceRange {
        data: request::PostDeviceRangeData {
            unit_id: "manager".to_string(),
            network_id: "manager".to_string(),
            start_addr: "0000".to_string(),
            end_addr: "0000".to_string(),
        },
    };
    test_post_range(
        runtime,
        routes_state,
        TOKEN_OWNER,
        false,
        &param,
        "err_broker_unit_not_exist",
        Some("manager"),
        "manager",
        &vec!["0000".to_string()],
    )?;

    let param = request::PostDeviceRange {
        data: request::PostDeviceRangeData {
            unit_id: "manager".to_string(),
            network_id: "id".to_string(),
            start_addr: "0000".to_string(),
            end_addr: "0000".to_string(),
        },
    };
    test_post_range(
        runtime,
        routes_state,
        TOKEN_MANAGER,
        false,
        &param,
        "err_broker_network_not_exist",
        None,
        "public",
        &vec!["0000".to_string()],
    )?;

    let param = request::PostDeviceRange {
        data: request::PostDeviceRangeData {
            unit_id: "owner".to_string(),
            network_id: "public".to_string(),
            start_addr: "0000".to_string(),
            end_addr: "0000".to_string(),
        },
    };
    test_post_range(
        runtime,
        routes_state,
        TOKEN_OWNER,
        false,
        &param,
        "err_broker_network_not_exist",
        None,
        "public",
        &vec!["0000".to_string()],
    )?;

    let param = request::PostDeviceRange {
        data: request::PostDeviceRangeData {
            unit_id: "owner".to_string(),
            network_id: "manager".to_string(),
            start_addr: "0000".to_string(),
            end_addr: "0000".to_string(),
        },
    };
    test_post_range(
        runtime,
        routes_state,
        TOKEN_OWNER,
        false,
        &param,
        "err_broker_network_not_exist",
        None,
        "public",
        &vec!["0000".to_string()],
    )
}

pub fn post_range_del_invalid_param(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    add_unit_model(runtime, routes_state, "manager", vec![], "manager")?;

    test_post_range_invalid_param(runtime, &routes_state, TOKEN_MANAGER, false, None)?;

    let param = request::PostDeviceRange {
        data: request::PostDeviceRangeData {
            unit_id: "".to_string(),
            network_id: "manager".to_string(),
            start_addr: "0000".to_string(),
            end_addr: "0000".to_string(),
        },
    };
    test_post_range_invalid_param(runtime, &routes_state, TOKEN_MANAGER, false, Some(&param))?;

    let param = request::PostDeviceRange {
        data: request::PostDeviceRangeData {
            unit_id: "manager".to_string(),
            network_id: "".to_string(),
            start_addr: "0000".to_string(),
            end_addr: "0000".to_string(),
        },
    };
    test_post_range_invalid_param(runtime, &routes_state, TOKEN_MANAGER, false, Some(&param))?;

    let param = request::PostDeviceRange {
        data: request::PostDeviceRangeData {
            unit_id: "manager".to_string(),
            network_id: "manager".to_string(),
            start_addr: "".to_string(),
            end_addr: "0000".to_string(),
        },
    };
    test_post_range_invalid_param(runtime, &routes_state, TOKEN_MANAGER, false, Some(&param))?;

    let param = request::PostDeviceRange {
        data: request::PostDeviceRangeData {
            unit_id: "manager".to_string(),
            network_id: "manager".to_string(),
            start_addr: "0000".to_string(),
            end_addr: "".to_string(),
        },
    };
    test_post_range_invalid_param(runtime, &routes_state, TOKEN_MANAGER, false, Some(&param))?;

    let param = request::PostDeviceRange {
        data: request::PostDeviceRangeData {
            unit_id: "manager".to_string(),
            network_id: "manager".to_string(),
            start_addr: "000g".to_string(),
            end_addr: "0000".to_string(),
        },
    };
    test_post_range_invalid_param(runtime, &routes_state, TOKEN_MANAGER, false, Some(&param))?;

    let param = request::PostDeviceRange {
        data: request::PostDeviceRangeData {
            unit_id: "manager".to_string(),
            network_id: "manager".to_string(),
            start_addr: "0000".to_string(),
            end_addr: "000g".to_string(),
        },
    };
    test_post_range_invalid_param(runtime, &routes_state, TOKEN_MANAGER, false, Some(&param))?;

    let param = request::PostDeviceRange {
        data: request::PostDeviceRangeData {
            unit_id: "manager".to_string(),
            network_id: "manager".to_string(),
            start_addr: "0000".to_string(),
            end_addr: "000000".to_string(),
        },
    };
    test_post_range_invalid_param(runtime, &routes_state, TOKEN_MANAGER, false, Some(&param))?;

    let param = request::PostDeviceRange {
        data: request::PostDeviceRangeData {
            unit_id: "manager".to_string(),
            network_id: "manager".to_string(),
            start_addr: "0000".to_string(),
            end_addr: "0400".to_string(),
        },
    };
    test_post_range_invalid_param(runtime, &routes_state, TOKEN_MANAGER, false, Some(&param))?;

    let param = request::PostDeviceRange {
        data: request::PostDeviceRangeData {
            unit_id: "manager".to_string(),
            network_id: "manager".to_string(),
            start_addr: "0001".to_string(),
            end_addr: "0000".to_string(),
        },
    };
    test_post_range_invalid_param(runtime, &routes_state, TOKEN_MANAGER, false, Some(&param))?;

    let param = request::PostDeviceRange {
        data: request::PostDeviceRangeData {
            unit_id: "manager".to_string(),
            network_id: "manager".to_string(),
            start_addr: "0000000000000000000000000000000000".to_string(),
            end_addr: "0000000000000000000000000000000000".to_string(),
        },
    };
    test_post_range_invalid_param(runtime, &routes_state, TOKEN_MANAGER, false, Some(&param))
}

pub fn post_range_del_invalid_token(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    let req = TestRequest::post().uri("/broker/api/v1/device/range-delete");
    test_invalid_token(runtime, &routes_state, req)
}

pub fn get_count(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    let data_size = count_list_dataset(runtime, &routes_state)?;

    test_get_count(runtime, &routes_state, TOKEN_MANAGER, None, data_size.0)?;

    let param = request::GetDeviceCount {
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

    let param = request::GetDeviceCount {
        unit: Some("".to_string()),
        network: Some("".to_string()),
        addr: Some("".to_string()),
        ..Default::default()
    };
    test_get_count(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        Some(&param),
        data_size.0,
    )?;

    let param = request::GetDeviceCount {
        unit: Some("manager".to_string()),
        ..Default::default()
    };
    test_get_count(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 2)?;
    test_get_count(runtime, &routes_state, TOKEN_MEMBER, Some(&param), 2)?;

    let param = request::GetDeviceCount {
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

    let param = request::GetDeviceCount {
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

    let param = request::GetDeviceCount {
        contains: Some("2".to_string()),
        ..Default::default()
    };
    test_get_count(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 3)?;

    let param = request::GetDeviceCount {
        unit: Some("owner1".to_string()),
        contains: Some("2".to_string()),
        ..Default::default()
    };
    test_get_count(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 1)?;
    test_get_count(runtime, &routes_state, TOKEN_OWNER, Some(&param), 1)?;

    libs::clear_all_data(runtime, state);
    let data_size = count_list_extra_dataset(runtime, &routes_state)?;

    let param = request::GetDeviceCount {
        network: Some("public".to_string()),
        ..Default::default()
    };
    test_get_count(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        Some(&param),
        data_size.0,
    )?;
    let param = request::GetDeviceCount {
        unit: Some("owner".to_string()),
        network: Some("public".to_string()),
        ..Default::default()
    };
    test_get_count(
        runtime,
        &routes_state,
        TOKEN_OWNER,
        Some(&param),
        data_size.3,
    )?;
    let param = request::GetDeviceCount {
        unit: Some("manager".to_string()),
        network: Some("public".to_string()),
        ..Default::default()
    };
    test_get_count(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 1)?;
    test_get_count(runtime, &routes_state, TOKEN_MEMBER, Some(&param), 1)?;

    let param = request::GetDeviceCount {
        unit: Some("manager".to_string()),
        ..Default::default()
    };
    test_get_count(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        Some(&param),
        data_size.1,
    )?;
    let param = request::GetDeviceCount {
        unit: Some("owner".to_string()),
        ..Default::default()
    };
    test_get_count(
        runtime,
        &routes_state,
        TOKEN_OWNER,
        Some(&param),
        data_size.4,
    )?;

    let param = request::GetDeviceCount {
        addr: Some("same-addr".to_string()),
        ..Default::default()
    };
    test_get_count(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        Some(&param),
        data_size.2,
    )?;
    let param = request::GetDeviceCount {
        unit: Some("owner".to_string()),
        addr: Some("same-addr".to_string()),
        ..Default::default()
    };
    test_get_count(
        runtime,
        &routes_state,
        TOKEN_OWNER,
        Some(&param),
        data_size.5,
    )?;

    let param = request::GetDeviceCount {
        unit: Some("manager".to_string()),
        addr: Some("same-addr".to_string()),
        ..Default::default()
    };
    test_get_count(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 1)?;
    let param = request::GetDeviceCount {
        network: Some("public".to_string()),
        addr: Some("same-addr".to_string()),
        ..Default::default()
    };
    test_get_count(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 0)?;
    let param = request::GetDeviceCount {
        unit: Some("owner".to_string()),
        addr: Some("same-addr".to_string()),
        ..Default::default()
    };
    test_get_count(runtime, &routes_state, TOKEN_OWNER, Some(&param), 1)?;
    let param = request::GetDeviceCount {
        unit: Some("owner".to_string()),
        network: Some("public".to_string()),
        addr: Some("same-addr".to_string()),
        ..Default::default()
    };
    test_get_count(runtime, &routes_state, TOKEN_OWNER, Some(&param), 0)?;
    let param = request::GetDeviceCount {
        unit: Some("manager".to_string()),
        addr: Some("same-addr".to_string()),
        ..Default::default()
    };
    test_get_count(runtime, &routes_state, TOKEN_MEMBER, Some(&param), 1)?;
    let param = request::GetDeviceCount {
        unit: Some("manager".to_string()),
        network: Some("public".to_string()),
        addr: Some("same-addr".to_string()),
        ..Default::default()
    };
    test_get_count(runtime, &routes_state, TOKEN_MEMBER, Some(&param), 0)
}

pub fn get_count_not_exist_unit(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    add_unit_model(runtime, routes_state, "manager", vec![], "manager")?;

    let uri = "/broker/api/v1/device/count";
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

    let uri = "/broker/api/v1/device/count";
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

    let req = TestRequest::get().uri("/broker/api/v1/device/count");
    test_invalid_token(runtime, &routes_state, req)
}

pub fn get_list(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    let data_size = count_list_dataset(runtime, &routes_state)?;

    test_get_list(runtime, &routes_state, TOKEN_MANAGER, None, data_size.0)?;

    let param = request::GetDeviceList {
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

    let param = request::GetDeviceList {
        unit: Some("".to_string()),
        network: Some("".to_string()),
        addr: Some("".to_string()),
        ..Default::default()
    };
    test_get_list(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        Some(&param),
        data_size.0,
    )?;

    let param = request::GetDeviceList {
        unit: Some("manager".to_string()),
        ..Default::default()
    };
    test_get_list(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 2)?;
    test_get_list(runtime, &routes_state, TOKEN_MEMBER, Some(&param), 2)?;

    let param = request::GetDeviceList {
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

    let param = request::GetDeviceList {
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

    let param = request::GetDeviceList {
        contains: Some("2".to_string()),
        ..Default::default()
    };
    test_get_list(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 3)?;

    let param = request::GetDeviceList {
        unit: Some("owner1".to_string()),
        contains: Some("2".to_string()),
        ..Default::default()
    };
    test_get_list(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 1)?;
    test_get_list(runtime, &routes_state, TOKEN_OWNER, Some(&param), 1)?;

    libs::clear_all_data(runtime, state);
    let data_size = count_list_extra_dataset(runtime, &routes_state)?;

    let param = request::GetDeviceList {
        network: Some("public".to_string()),
        ..Default::default()
    };
    test_get_list(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        Some(&param),
        data_size.0,
    )?;
    let param = request::GetDeviceList {
        unit: Some("owner".to_string()),
        network: Some("public".to_string()),
        ..Default::default()
    };
    test_get_list(
        runtime,
        &routes_state,
        TOKEN_OWNER,
        Some(&param),
        data_size.3,
    )?;
    let param = request::GetDeviceList {
        unit: Some("manager".to_string()),
        network: Some("public".to_string()),
        ..Default::default()
    };
    test_get_list(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 1)?;
    test_get_list(runtime, &routes_state, TOKEN_MEMBER, Some(&param), 1)?;

    let param = request::GetDeviceList {
        unit: Some("manager".to_string()),
        ..Default::default()
    };
    test_get_list(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        Some(&param),
        data_size.1,
    )?;
    let param = request::GetDeviceList {
        unit: Some("owner".to_string()),
        ..Default::default()
    };
    test_get_list(
        runtime,
        &routes_state,
        TOKEN_OWNER,
        Some(&param),
        data_size.4,
    )?;

    let param = request::GetDeviceList {
        addr: Some("same-addr".to_string()),
        ..Default::default()
    };
    test_get_list(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        Some(&param),
        data_size.2,
    )?;
    let param = request::GetDeviceList {
        unit: Some("owner".to_string()),
        addr: Some("same-addr".to_string()),
        ..Default::default()
    };
    test_get_list(
        runtime,
        &routes_state,
        TOKEN_OWNER,
        Some(&param),
        data_size.5,
    )?;

    let param = request::GetDeviceList {
        unit: Some("manager".to_string()),
        addr: Some("same-addr".to_string()),
        ..Default::default()
    };
    test_get_list(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 1)?;
    let param = request::GetDeviceList {
        network: Some("public".to_string()),
        addr: Some("same-addr".to_string()),
        ..Default::default()
    };
    test_get_list(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 0)?;
    let param = request::GetDeviceList {
        unit: Some("owner".to_string()),
        addr: Some("same-addr".to_string()),
        ..Default::default()
    };
    test_get_list(runtime, &routes_state, TOKEN_OWNER, Some(&param), 1)?;
    let param = request::GetDeviceList {
        unit: Some("owner".to_string()),
        network: Some("public".to_string()),
        addr: Some("same-addr".to_string()),
        ..Default::default()
    };
    test_get_list(runtime, &routes_state, TOKEN_OWNER, Some(&param), 0)?;
    let param = request::GetDeviceList {
        unit: Some("manager".to_string()),
        addr: Some("same-addr".to_string()),
        ..Default::default()
    };
    test_get_list(runtime, &routes_state, TOKEN_MEMBER, Some(&param), 1)?;
    let param = request::GetDeviceList {
        unit: Some("manager".to_string()),
        network: Some("public".to_string()),
        addr: Some("same-addr".to_string()),
        ..Default::default()
    };
    test_get_list(runtime, &routes_state, TOKEN_MEMBER, Some(&param), 0)
}

pub fn get_list_sort(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    count_list_dataset(runtime, &routes_state)?;

    let mut param = request::GetDeviceList {
        ..Default::default()
    };
    test_get_list_sort(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        &mut param,
        &[
            "manager",
            "owner1-1",
            "owner1-2",
            "owner1-3",
            "owner2-1",
            "owner2-2",
            "manager-public",
            "owner-public",
        ],
    )?;

    param.sort_vec = Some(vec![("network", true), ("addr", true)]);
    test_get_list_sort(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        &mut param,
        &[
            "manager",
            "owner1-1",
            "owner1-2",
            "owner1-3",
            "owner2-1",
            "owner2-2",
            "manager-public",
            "owner-public",
        ],
    )?;
    param.sort_vec = Some(vec![("network", true), ("addr", false)]);
    test_get_list_sort(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        &mut param,
        &[
            "manager",
            "owner1-3",
            "owner1-2",
            "owner1-1",
            "owner2-2",
            "owner2-1",
            "owner-public",
            "manager-public",
        ],
    )?;
    param.sort_vec = Some(vec![("network", false), ("addr", true)]);
    test_get_list_sort(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        &mut param,
        &[
            "manager-public",
            "owner-public",
            "owner2-1",
            "owner2-2",
            "owner1-1",
            "owner1-2",
            "owner1-3",
            "manager",
        ],
    )?;
    param.sort_vec = Some(vec![("network", false), ("addr", false)]);
    test_get_list_sort(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        &mut param,
        &[
            "owner-public",
            "manager-public",
            "owner2-2",
            "owner2-1",
            "owner1-3",
            "owner1-2",
            "owner1-1",
            "manager",
        ],
    )?;

    param.sort_vec = Some(vec![("addr", true)]);
    test_get_list_sort(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        &mut param,
        &[
            "manager",
            "manager-public",
            "owner-public",
            "owner1-1",
            "owner1-2",
            "owner1-3",
            "owner2-1",
            "owner2-2",
        ],
    )?;
    param.sort_vec = Some(vec![("addr", false)]);
    test_get_list_sort(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        &mut param,
        &[
            "owner2-2",
            "owner2-1",
            "owner1-3",
            "owner1-2",
            "owner1-1",
            "owner-public",
            "manager-public",
            "manager",
        ],
    )?;

    param.sort_vec = Some(vec![("name", true)]);
    test_get_list_sort(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        &mut param,
        &[
            "manager",
            "manager-public",
            "owner-public",
            "owner1-1",
            "owner1-2",
            "owner1-3",
            "owner2-2",
            "owner2-1",
        ],
    )?;
    param.sort_vec = Some(vec![("name", false)]);
    test_get_list_sort(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        &mut param,
        &[
            "owner2-1",
            "owner2-2",
            "owner1-3",
            "owner1-2",
            "owner1-1",
            "owner-public",
            "manager-public",
            "manager",
        ],
    )?;

    param.sort_vec = Some(vec![("created", true)]);
    test_get_list_sort(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        &mut param,
        &[
            "manager-public",
            "owner-public",
            "manager",
            "owner1-1",
            "owner1-2",
            "owner1-3",
            "owner2-1",
            "owner2-2",
        ],
    )?;
    param.sort_vec = Some(vec![("created", false)]);
    test_get_list_sort(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        &mut param,
        &[
            "owner2-2",
            "owner2-1",
            "owner1-3",
            "owner1-2",
            "owner1-1",
            "manager",
            "owner-public",
            "manager-public",
        ],
    )?;

    param.sort_vec = Some(vec![("modified", true)]);
    test_get_list_sort(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        &mut param,
        &[
            "manager-public",
            "owner-public",
            "owner2-2",
            "owner2-1",
            "owner1-3",
            "owner1-2",
            "owner1-1",
            "manager",
        ],
    )?;
    param.sort_vec = Some(vec![("modified", false)]);
    test_get_list_sort(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        &mut param,
        &[
            "manager",
            "owner1-1",
            "owner1-2",
            "owner1-3",
            "owner2-1",
            "owner2-2",
            "owner-public",
            "manager-public",
        ],
    )
}

pub fn get_list_offset_limit(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    add_unit_model(runtime, routes_state, "manager", vec![], "manager")?;
    add_network_model(runtime, routes_state, "manager", "manager", "amqp://host")?;

    let unit_id = "manager";
    let network_id = "manager";

    for i in 100..302 {
        add_device_model(
            runtime,
            &routes_state,
            unit_id,
            network_id,
            format!("device_{}", i).as_str(),
            false,
        )?;
    }

    let mut param = request::GetDeviceList {
        unit: Some(unit_id.to_string()),
        contains: Some("D".to_string()),
        ..Default::default()
    };
    test_get_list_offset_limit(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        &param,
        (100..200).collect(),
    )?;

    param.network = Some(network_id.to_string());
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
    param.network = Some("".to_string());
    param.addr = Some("".to_string());
    test_get_list_offset_limit(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        &param,
        (100..302).collect(),
    )?;
    param.unit = None;
    param.network = None;
    param.addr = None;

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
        add_device_model(
            runtime,
            &routes_state,
            unit_id,
            "public",
            format!("device_{}", i).as_str(),
            true,
        )?;
    }

    let mut param = request::GetDeviceList {
        network: Some("public".to_string()),
        contains: Some("D".to_string()),
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
    param.network = None;
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
    add_network_model(runtime, routes_state, "manager", "manager", "amqp://host")?;

    let unit_id = "manager";
    let network_id = "manager";

    for i in 100..302 {
        add_device_model(
            runtime,
            &routes_state,
            unit_id,
            network_id,
            format!("device_{}", i).as_str(),
            false,
        )?;
    }

    let mut param = request::GetDeviceList {
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

    let uri = "/broker/api/v1/device/list";
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

    let uri = "/broker/api/v1/device/list";
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

    let req = TestRequest::get().uri("/broker/api/v1/device/list");
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
    add_network_model(runtime, routes_state, "manager", "manager", "amqp://host")?;
    add_network_model(runtime, routes_state, "", "public", "amqp://host")?;
    add_network_model(runtime, routes_state, "owner1", "owner1", "amqp://host")?;
    add_network_model(runtime, routes_state, "owner2", "owner2", "amqp://host")?;
    add_device_model(
        runtime,
        routes_state,
        "manager",
        "public",
        "manager-public",
        true,
    )?;
    add_device_model(
        runtime,
        routes_state,
        "manager",
        "manager",
        "manager",
        false,
    )?;
    add_device_model(
        runtime,
        routes_state,
        "owner1",
        "public",
        "owner-public",
        true,
    )?;
    add_device_model(runtime, routes_state, "owner1", "owner1", "owner1", false)?;
    add_device_model(runtime, routes_state, "owner2", "owner2", "owner2", false)?;

    test_get(runtime, routes_state, TOKEN_MANAGER, "manager-public")?;
    test_get(runtime, routes_state, TOKEN_MANAGER, "manager")?;
    test_get(runtime, routes_state, TOKEN_MANAGER, "owner1")?;
    test_get(runtime, routes_state, TOKEN_MANAGER, "owner2")?;
    test_get(runtime, routes_state, TOKEN_OWNER, "owner-public")?;
    test_get(runtime, routes_state, TOKEN_OWNER, "owner1")?;
    test_get(runtime, routes_state, TOKEN_OWNER, "owner2")?;
    test_get(runtime, routes_state, TOKEN_MEMBER, "manager-public")?;
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
    add_device_model(
        runtime,
        routes_state,
        "manager",
        "public",
        "manager-public",
        true,
    )?;
    add_device_model(
        runtime,
        routes_state,
        "manager",
        "manager",
        "manager",
        false,
    )?;
    add_device_model(
        runtime,
        routes_state,
        "owner1",
        "public",
        "owner-public",
        true,
    )?;
    add_device_model(runtime, routes_state, "owner1", "owner1", "owner1", false)?;
    add_device_model(runtime, routes_state, "owner2", "owner2", "owner2", false)?;

    test_get_wrong_id(runtime, routes_state, TOKEN_MANAGER, "manager1")?;
    test_get_wrong_id(runtime, routes_state, TOKEN_OWNER, "manager-public")?;
    test_get_wrong_id(runtime, routes_state, TOKEN_OWNER, "manager")?;
    test_get_wrong_id(runtime, routes_state, TOKEN_MEMBER, "owner-public")?;
    test_get_wrong_id(runtime, routes_state, TOKEN_MEMBER, "owner1")
}

pub fn get_invalid_token(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    let req = TestRequest::get().uri("/broker/api/v1/device/id");
    test_invalid_token(runtime, &routes_state, req)
}

pub fn patch(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    add_unit_model(runtime, routes_state, "manager", vec![], "manager")?;
    add_unit_model(runtime, routes_state, "owner", vec![], "owner")?;
    add_network_model(runtime, routes_state, "manager", "manager", "amqp://host")?;
    add_network_model(runtime, routes_state, "", "public", "amqp://host")?;
    add_network_model(runtime, routes_state, "owner", "owner", "amqp://host")?;

    test_patch(
        runtime,
        routes_state,
        TOKEN_MANAGER,
        "manager",
        "public",
        "manager-public",
        true,
    )?;
    test_patch(
        runtime,
        routes_state,
        TOKEN_MANAGER,
        "manager",
        "manager",
        "manager",
        false,
    )?;
    test_patch(
        runtime,
        routes_state,
        TOKEN_OWNER,
        "owner",
        "public",
        "owner-public",
        true,
    )?;
    test_patch(
        runtime,
        routes_state,
        TOKEN_OWNER,
        "owner",
        "owner",
        "owner",
        false,
    )
}

pub fn patch_wrong_id(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    add_unit_model(runtime, routes_state, "manager", vec!["member"], "manager")?;
    add_unit_model(runtime, routes_state, "", vec!["member"], "public")?;
    add_network_model(runtime, routes_state, "manager", "manager", "amqp://host")?;
    add_network_model(runtime, routes_state, "", "public", "amqp://host")?;
    add_network_model(runtime, routes_state, "owner", "owner", "amqp://host")?;
    add_device_model(
        runtime,
        routes_state,
        "manager",
        "public",
        "manager-public",
        true,
    )?;
    add_device_model(
        runtime,
        routes_state,
        "manager",
        "manager",
        "manager",
        false,
    )?;
    add_device_model(
        runtime,
        routes_state,
        "owner",
        "public",
        "owner-public",
        true,
    )?;
    add_device_model(runtime, routes_state, "owner", "public", "owner", true)?;

    test_patch_wrong_id(runtime, routes_state, TOKEN_MANAGER, "manager1")?;
    test_patch_wrong_id(runtime, routes_state, TOKEN_OWNER, "manager-public")?;
    test_patch_wrong_id(runtime, routes_state, TOKEN_OWNER, "manager")?;
    test_patch_wrong_id(runtime, routes_state, TOKEN_MEMBER, "manager-public")?;
    test_patch_wrong_id(runtime, routes_state, TOKEN_MEMBER, "manager")?;
    test_patch_wrong_id(runtime, routes_state, TOKEN_MEMBER, "owner-public")?;
    test_patch_wrong_id(runtime, routes_state, TOKEN_MEMBER, "owner")
}

pub fn patch_invalid_param(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    add_unit_model(runtime, routes_state, "manager", vec![], "manager")?;
    add_network_model(runtime, routes_state, "manager", "manager", "amqp://host")?;
    add_device_model(
        runtime,
        routes_state,
        "manager",
        "manager",
        "manager",
        false,
    )?;

    test_patch_invalid_param(runtime, routes_state, TOKEN_MANAGER, "manager", None)?;

    let param = request::PatchDevice {
        ..Default::default()
    };
    test_patch_invalid_param(
        runtime,
        routes_state,
        TOKEN_MANAGER,
        "manager",
        Some(&param),
    )?;

    let param = request::PatchDevice {
        data: request::PatchDeviceData {
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
    let param = request::PatchDevice {
        data: request::PatchDeviceData {
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

    let req = TestRequest::patch().uri("/broker/api/v1/device/id");
    test_invalid_token(runtime, &routes_state, req)
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
    add_device_model(
        runtime,
        routes_state,
        "manager",
        "public",
        "manager-public",
        true,
    )?;
    add_device_model(
        runtime,
        routes_state,
        "manager",
        "manager",
        "manager",
        false,
    )?;
    add_device_model(
        runtime,
        routes_state,
        "owner",
        "public",
        "owner-public",
        true,
    )?;
    add_device_model(
        runtime,
        routes_state,
        "owner",
        "public",
        "owner-public2",
        true,
    )?;
    add_device_model(runtime, routes_state, "owner", "owner", "owner", false)?;
    add_device_model(runtime, routes_state, "owner", "owner", "owner2", false)?;

    let mut app = runtime.block_on(async {
        test::init_service(
            App::new()
                .wrap(NormalizePath::trim())
                .service(routes::new_service(&routes_state)),
        )
        .await
    });

    let req = TestRequest::delete()
        .uri("/broker/api/v1/device/id")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", TOKEN_MANAGER)))
        .to_request();
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    expect(resp.status()).to_equal(StatusCode::NO_CONTENT)?;
    let _ = get_device_model(runtime, &routes_state, "manager", true)?;

    let req = TestRequest::delete()
        .uri("/broker/api/v1/device/manager-public")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", TOKEN_OWNER)))
        .to_request();
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    expect(resp.status()).to_equal(StatusCode::NO_CONTENT)?;
    let _ = get_device_model(runtime, &routes_state, "manager-public", true)?;

    let req = TestRequest::delete()
        .uri("/broker/api/v1/device/manager-public")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", TOKEN_MEMBER)))
        .to_request();
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    expect(resp.status()).to_equal(StatusCode::NO_CONTENT)?;
    let _ = get_device_model(runtime, &routes_state, "manager-public", true)?;

    let req = TestRequest::delete()
        .uri("/broker/api/v1/device/manager-public")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", TOKEN_MANAGER)))
        .to_request();
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    expect(resp.status()).to_equal(StatusCode::NO_CONTENT)?;
    let _ = get_device_model(runtime, &routes_state, "manager-public", false)?;

    let req = TestRequest::delete()
        .uri("/broker/api/v1/device/manager")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", TOKEN_OWNER)))
        .to_request();
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    expect(resp.status()).to_equal(StatusCode::NO_CONTENT)?;
    let _ = get_device_model(runtime, &routes_state, "manager", true)?;

    let req = TestRequest::delete()
        .uri("/broker/api/v1/device/manager")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", TOKEN_MEMBER)))
        .to_request();
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    expect(resp.status()).to_equal(StatusCode::NO_CONTENT)?;
    let _ = get_device_model(runtime, &routes_state, "manager", true)?;

    let req = TestRequest::delete()
        .uri("/broker/api/v1/device/manager")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", TOKEN_MANAGER)))
        .to_request();
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    expect(resp.status()).to_equal(StatusCode::NO_CONTENT)?;
    let _ = get_device_model(runtime, &routes_state, "manager", false)?;

    let req = TestRequest::delete()
        .uri("/broker/api/v1/device/owner-public")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", TOKEN_MEMBER)))
        .to_request();
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    expect(resp.status()).to_equal(StatusCode::NO_CONTENT)?;
    let _ = get_device_model(runtime, &routes_state, "owner-public", true)?;

    let req = TestRequest::delete()
        .uri("/broker/api/v1/device/owner-public")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", TOKEN_OWNER)))
        .to_request();
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    expect(resp.status()).to_equal(StatusCode::NO_CONTENT)?;
    let _ = get_device_model(runtime, &routes_state, "owner-public", false)?;

    let req = TestRequest::delete()
        .uri("/broker/api/v1/device/owner-public2")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", TOKEN_MANAGER)))
        .to_request();
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    expect(resp.status()).to_equal(StatusCode::NO_CONTENT)?;
    let _ = get_device_model(runtime, &routes_state, "owner-public2", false)?;

    let req = TestRequest::delete()
        .uri("/broker/api/v1/device/owner")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", TOKEN_MEMBER)))
        .to_request();
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    expect(resp.status()).to_equal(StatusCode::NO_CONTENT)?;
    let _ = get_device_model(runtime, &routes_state, "owner", true)?;

    let req = TestRequest::delete()
        .uri("/broker/api/v1/device/owner")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", TOKEN_OWNER)))
        .to_request();
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    expect(resp.status()).to_equal(StatusCode::NO_CONTENT)?;
    let _ = get_device_model(runtime, &routes_state, "owner", false)?;

    let req = TestRequest::delete()
        .uri("/broker/api/v1/device/owner")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", TOKEN_MANAGER)))
        .to_request();
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    expect(resp.status()).to_equal(StatusCode::NO_CONTENT)?;
    let _ = get_device_model(runtime, &routes_state, "owner", false)?;

    libs::clear_all_data(runtime, state);
    add_delete_rsc(runtime, routes_state)?;
    let req = TestRequest::delete()
        .uri("/broker/api/v1/device/manager-public")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", TOKEN_MANAGER)))
        .to_request();
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    expect(resp.status()).to_equal(StatusCode::NO_CONTENT)?;
    let _ = get_unit_model(runtime, routes_state, "manager", true)?;
    let _ = get_unit_model(runtime, routes_state, "owner", true)?;
    let _ = get_application_model(runtime, routes_state, "manager", true)?;
    let _ = get_application_model(runtime, routes_state, "owner", true)?;
    let _ = get_network_model(runtime, routes_state, "public", true)?;
    let _ = get_network_model(runtime, routes_state, "manager", true)?;
    let _ = get_network_model(runtime, routes_state, "owner", true)?;
    let _ = get_device_model(runtime, routes_state, "manager-public", false)?;
    let _ = get_device_model(runtime, routes_state, "manager", true)?;
    let _ = get_device_model(runtime, routes_state, "owner-public", true)?;
    let _ = get_device_model(runtime, routes_state, "owner1", true)?;
    let _ = get_device_model(runtime, routes_state, "owner2", true)?;
    let _ = get_network_route_model(runtime, routes_state, "public-manager", true)?;
    let _ = get_network_route_model(runtime, routes_state, "manager-manager", true)?;
    let _ = get_network_route_model(runtime, routes_state, "owner-owner", true)?;
    let _ = get_device_route_model(runtime, routes_state, "manager-public-manager", false)?;
    let _ = get_device_route_model(runtime, routes_state, "manager-manager", true)?;
    let _ = get_device_route_model(runtime, routes_state, "owner-public-owner", true)?;
    let _ = get_device_route_model(runtime, routes_state, "owner1-owner", true)?;
    let _ = get_device_route_model(runtime, routes_state, "owner2-owner", true)?;
    let _ = get_dldata_buffer_model(runtime, routes_state, "manager-public-manager", false)?;
    let _ = get_dldata_buffer_model(runtime, routes_state, "manager-manager", true)?;
    let _ = get_dldata_buffer_model(runtime, routes_state, "owner-public-owner", true)?;
    let _ = get_dldata_buffer_model(runtime, routes_state, "owner1-owner", true)?;
    let _ = get_dldata_buffer_model(runtime, routes_state, "owner2-owner", true)?;

    libs::clear_all_data(runtime, state);
    add_delete_rsc(runtime, routes_state)?;
    let req = TestRequest::delete()
        .uri("/broker/api/v1/device/manager")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", TOKEN_MANAGER)))
        .to_request();
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    expect(resp.status()).to_equal(StatusCode::NO_CONTENT)?;
    let _ = get_unit_model(runtime, routes_state, "manager", true)?;
    let _ = get_unit_model(runtime, routes_state, "owner", true)?;
    let _ = get_application_model(runtime, routes_state, "manager", true)?;
    let _ = get_application_model(runtime, routes_state, "owner", true)?;
    let _ = get_network_model(runtime, routes_state, "public", true)?;
    let _ = get_network_model(runtime, routes_state, "manager", true)?;
    let _ = get_network_model(runtime, routes_state, "owner", true)?;
    let _ = get_device_model(runtime, routes_state, "manager-public", true)?;
    let _ = get_device_model(runtime, routes_state, "manager", false)?;
    let _ = get_device_model(runtime, routes_state, "owner-public", true)?;
    let _ = get_device_model(runtime, routes_state, "owner1", true)?;
    let _ = get_device_model(runtime, routes_state, "owner2", true)?;
    let _ = get_network_route_model(runtime, routes_state, "public-manager", true)?;
    let _ = get_network_route_model(runtime, routes_state, "manager-manager", true)?;
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
    let req = TestRequest::delete()
        .uri("/broker/api/v1/device/owner-public")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", TOKEN_OWNER)))
        .to_request();
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    expect(resp.status()).to_equal(StatusCode::NO_CONTENT)?;
    let _ = get_unit_model(runtime, routes_state, "manager", true)?;
    let _ = get_unit_model(runtime, routes_state, "owner", true)?;
    let _ = get_application_model(runtime, routes_state, "manager", true)?;
    let _ = get_application_model(runtime, routes_state, "owner", true)?;
    let _ = get_network_model(runtime, routes_state, "public", true)?;
    let _ = get_network_model(runtime, routes_state, "manager", true)?;
    let _ = get_network_model(runtime, routes_state, "owner", true)?;
    let _ = get_device_model(runtime, routes_state, "manager-public", true)?;
    let _ = get_device_model(runtime, routes_state, "manager", true)?;
    let _ = get_device_model(runtime, routes_state, "owner-public", false)?;
    let _ = get_device_model(runtime, routes_state, "owner1", true)?;
    let _ = get_device_model(runtime, routes_state, "owner2", true)?;
    let _ = get_network_route_model(runtime, routes_state, "public-manager", true)?;
    let _ = get_network_route_model(runtime, routes_state, "manager-manager", true)?;
    let _ = get_network_route_model(runtime, routes_state, "owner-owner", true)?;
    let _ = get_device_route_model(runtime, routes_state, "manager-public-manager", true)?;
    let _ = get_device_route_model(runtime, routes_state, "manager-manager", true)?;
    let _ = get_device_route_model(runtime, routes_state, "owner-public-owner", false)?;
    let _ = get_device_route_model(runtime, routes_state, "owner1-owner", true)?;
    let _ = get_device_route_model(runtime, routes_state, "owner2-owner", true)?;
    let _ = get_dldata_buffer_model(runtime, routes_state, "manager-public-manager", true)?;
    let _ = get_dldata_buffer_model(runtime, routes_state, "manager-manager", true)?;
    let _ = get_dldata_buffer_model(runtime, routes_state, "owner-public-owner", false)?;
    let _ = get_dldata_buffer_model(runtime, routes_state, "owner1-owner", true)?;
    let _ = get_dldata_buffer_model(runtime, routes_state, "owner2-owner", true)?;

    libs::clear_all_data(runtime, state);
    add_delete_rsc(runtime, routes_state)?;
    let req = TestRequest::delete()
        .uri("/broker/api/v1/device/owner1")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", TOKEN_OWNER)))
        .to_request();
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    expect(resp.status()).to_equal(StatusCode::NO_CONTENT)?;
    let _ = get_unit_model(runtime, routes_state, "manager", true)?;
    let _ = get_unit_model(runtime, routes_state, "owner", true)?;
    let _ = get_application_model(runtime, routes_state, "manager", true)?;
    let _ = get_application_model(runtime, routes_state, "owner", true)?;
    let _ = get_network_model(runtime, routes_state, "public", true)?;
    let _ = get_network_model(runtime, routes_state, "manager", true)?;
    let _ = get_network_model(runtime, routes_state, "owner", true)?;
    let _ = get_device_model(runtime, routes_state, "manager-public", true)?;
    let _ = get_device_model(runtime, routes_state, "manager", true)?;
    let _ = get_device_model(runtime, routes_state, "owner-public", true)?;
    let _ = get_device_model(runtime, routes_state, "owner1", false)?;
    let _ = get_device_model(runtime, routes_state, "owner2", true)?;
    let _ = get_network_route_model(runtime, routes_state, "public-manager", true)?;
    let _ = get_network_route_model(runtime, routes_state, "manager-manager", true)?;
    let _ = get_network_route_model(runtime, routes_state, "owner-owner", true)?;
    let _ = get_device_route_model(runtime, routes_state, "manager-public-manager", true)?;
    let _ = get_device_route_model(runtime, routes_state, "manager-manager", true)?;
    let _ = get_device_route_model(runtime, routes_state, "owner-public-owner", true)?;
    let _ = get_device_route_model(runtime, routes_state, "owner1-owner", false)?;
    let _ = get_device_route_model(runtime, routes_state, "owner2-owner", true)?;
    let _ = get_dldata_buffer_model(runtime, routes_state, "manager-public-manager", true)?;
    let _ = get_dldata_buffer_model(runtime, routes_state, "manager-manager", true)?;
    let _ = get_dldata_buffer_model(runtime, routes_state, "owner-public-owner", true)?;
    let _ = get_dldata_buffer_model(runtime, routes_state, "owner1-owner", false)?;
    let _ = get_dldata_buffer_model(runtime, routes_state, "owner2-owner", true)?;

    Ok(())
}

pub fn delete_invalid_token(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    let req = TestRequest::delete().uri("/broker/api/v1/device/id");
    test_invalid_token(runtime, &routes_state, req)
}

fn test_post(
    runtime: &Runtime,
    state: &routes::State,
    token: &str,
    param: &request::PostDevice,
    expect_code: &str,
    expect_unit_code: Option<&str>,
    expect_network_code: &str,
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
        .uri("/broker/api/v1/device")
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
    let body: response::PostDevice = runtime.block_on(async { test::read_body_json(resp).await });
    expect(body.data.device_id.len() > 0).to_equal(true)?;

    let device_info = match runtime.block_on(async {
        let cond = QueryCond {
            device_id: Some(body.data.device_id.as_str()),
            ..Default::default()
        };
        state.model.device().get(&cond).await
    }) {
        Err(e) => return Err(format!("get device model error: {}", e)),
        Ok(info) => match info {
            None => return Err("add device then get none device".to_string()),
            Some(info) => info,
        },
    };
    expect(device_info.unit_id.as_str()).to_equal(param.data.unit_id.as_str())?;
    match device_info.unit_code.as_ref() {
        None => expect(expect_unit_code.is_none()).to_equal(true)?,
        Some(unit_code) => expect(Some(unit_code.as_str())).to_equal(expect_unit_code)?,
    }
    expect(device_info.network_id.as_str()).to_equal(param.data.network_id.as_str())?;
    expect(device_info.network_code.as_str()).to_equal(expect_network_code)?;
    expect(device_info.network_addr.as_str())
        .to_equal(param.data.network_addr.to_lowercase().as_str())?;
    expect(device_info.created_at.ge(&time_before)).to_equal(true)?;
    expect(device_info.created_at.le(&time_after)).to_equal(true)?;
    expect(device_info.modified_at.ge(&time_before)).to_equal(true)?;
    expect(device_info.modified_at.le(&time_after)).to_equal(true)?;
    match param.data.name.as_ref() {
        None => expect(device_info.name.len()).to_equal(0)?,
        Some(name) => expect(device_info.name.as_str()).to_equal(name.as_str())?,
    }
    match param.data.info.as_ref() {
        None => expect(device_info.info).to_equal(Map::<String, Value>::new()),
        Some(info) => expect(device_info.info).to_equal(info.clone()),
    }
}

fn test_post_invalid_param(
    runtime: &Runtime,
    state: &routes::State,
    token: &str,
    param: Option<&request::PostDevice>,
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
        .uri("/broker/api/v1/device")
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

fn test_post_bulk(
    runtime: &Runtime,
    state: &routes::State,
    token: &str,
    use_post: bool,
    param: &request::PostDeviceBulk,
    expect_code: &str,
    expect_unit_code: Option<&str>,
    expect_network_code: &str,
    expect_network_addrs: &Vec<String>,
) -> Result<(), String> {
    let mut app = runtime.block_on(async {
        test::init_service(
            App::new()
                .wrap(NormalizePath::trim())
                .service(routes::new_service(state)),
        )
        .await
    });

    let req = match use_post {
        false => TestRequest::post().uri("/broker/api/v1/device/bulk-delete"),
        true => TestRequest::post().uri("/broker/api/v1/device/bulk"),
    };
    let req = req
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)))
        .set_json(param)
        .to_request();
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    if resp.status() != StatusCode::NO_CONTENT {
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
            "API not 204, status: {}, code: {}, message: {}",
            status,
            body.code.as_str(),
            message
        ));
    }

    let device_list = match runtime.block_on(async {
        let cond = ListQueryCond {
            unit_id: Some(param.data.unit_id.as_str()),
            network_id: Some(param.data.network_id.as_str()),
            ..Default::default()
        };
        let sort = vec![SortCond {
            key: SortKey::NetworkAddr,
            asc: true,
        }];
        let opts = ListOptions {
            cond: &cond,
            offset: None,
            limit: None,
            sort: Some(sort.as_slice()),
            cursor_max: None,
        };
        state.model.device().list(&opts, None).await
    }) {
        Err(e) => return Err(format!("get device model error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(device_list.len()).to_equal(expect_network_addrs.len())?;
    for i in 0..device_list.len() {
        let device = &device_list[i];
        match device.unit_code.as_ref() {
            None => expect(expect_unit_code).to_equal(None)?,
            Some(unit_code) => expect(Some(unit_code.as_str())).to_equal(expect_unit_code)?,
        }
        expect(device.network_code.as_str()).to_equal(expect_network_code)?;
        expect(device.network_addr.as_str()).to_equal(expect_network_addrs[i].as_str())?;
    }

    Ok(())
}

fn test_post_bulk_invalid_param(
    runtime: &Runtime,
    state: &routes::State,
    token: &str,
    use_post: bool,
    param: Option<&request::PostDeviceBulk>,
) -> Result<(), String> {
    let mut app = runtime.block_on(async {
        test::init_service(
            App::new()
                .wrap(NormalizePath::trim())
                .service(routes::new_service(state)),
        )
        .await
    });

    let req = match use_post {
        false => TestRequest::post().uri("/broker/api/v1/device/bulk-delete"),
        true => TestRequest::post().uri("/broker/api/v1/device/bulk"),
    };
    let req = req
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)))
        .set_json(&param)
        .to_request();
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    expect(resp.status()).to_equal(StatusCode::BAD_REQUEST)?;
    let body: ApiError = runtime.block_on(async { test::read_body_json(resp).await });
    if body.code.as_str() != err::E_PARAM {
        return Err(format!("unexpected 400 error: {}", body.code.as_str()));
    }
    Ok(())
}

fn test_post_range(
    runtime: &Runtime,
    state: &routes::State,
    token: &str,
    use_post: bool,
    param: &request::PostDeviceRange,
    expect_code: &str,
    expect_unit_code: Option<&str>,
    expect_network_code: &str,
    expect_network_addrs: &Vec<String>,
) -> Result<(), String> {
    let mut app = runtime.block_on(async {
        test::init_service(
            App::new()
                .wrap(NormalizePath::trim())
                .service(routes::new_service(state)),
        )
        .await
    });

    let req = match use_post {
        false => TestRequest::post().uri("/broker/api/v1/device/range-delete"),
        true => TestRequest::post().uri("/broker/api/v1/device/range"),
    };
    let req = req
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)))
        .set_json(param)
        .to_request();
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    if resp.status() != StatusCode::NO_CONTENT {
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
            "API not 204, status: {}, code: {}, message: {}",
            status,
            body.code.as_str(),
            message
        ));
    }

    let device_list = match runtime.block_on(async {
        let cond = ListQueryCond {
            unit_id: Some(param.data.unit_id.as_str()),
            network_id: Some(param.data.network_id.as_str()),
            ..Default::default()
        };
        let sort = vec![SortCond {
            key: SortKey::NetworkAddr,
            asc: true,
        }];
        let opts = ListOptions {
            cond: &cond,
            offset: None,
            limit: None,
            sort: Some(sort.as_slice()),
            cursor_max: None,
        };
        state.model.device().list(&opts, None).await
    }) {
        Err(e) => return Err(format!("get device model error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(device_list.len()).to_equal(expect_network_addrs.len())?;
    for i in 0..device_list.len() {
        let device = &device_list[i];
        match device.unit_code.as_ref() {
            None => expect(expect_unit_code).to_equal(None)?,
            Some(unit_code) => expect(Some(unit_code.as_str())).to_equal(expect_unit_code)?,
        }
        expect(device.network_code.as_str()).to_equal(expect_network_code)?;
        expect(device.network_addr.as_str()).to_equal(expect_network_addrs[i].as_str())?;
    }

    Ok(())
}

fn test_post_range_invalid_param(
    runtime: &Runtime,
    state: &routes::State,
    token: &str,
    use_post: bool,
    param: Option<&request::PostDeviceRange>,
) -> Result<(), String> {
    let mut app = runtime.block_on(async {
        test::init_service(
            App::new()
                .wrap(NormalizePath::trim())
                .service(routes::new_service(state)),
        )
        .await
    });

    let req = match use_post {
        false => TestRequest::post().uri("/broker/api/v1/device/range-delete"),
        true => TestRequest::post().uri("/broker/api/v1/device/range"),
    };
    let req = req
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)))
        .set_json(&param)
        .to_request();
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
    param: Option<&request::GetDeviceCount>,
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
        None => "/broker/api/v1/device/count".to_string(),
        Some(param) => format!(
            "/broker/api/v1/device/count?{}",
            serde_urlencoded::to_string(&param).unwrap()
        ),
    };
    let req = TestRequest::get()
        .uri(uri.as_str())
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)))
        .to_request();
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    expect(resp.status()).to_equal(StatusCode::OK)?;
    let body: response::GetDeviceCount =
        runtime.block_on(async { test::read_body_json(resp).await });
    expect(body.data.count).to_equal(expect_count)
}

fn test_get_list(
    runtime: &Runtime,
    state: &routes::State,
    token: &str,
    param: Option<&request::GetDeviceList>,
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
        None => "/broker/api/v1/device/list".to_string(),
        Some(param) => format!(
            "/broker/api/v1/device/list?{}",
            serde_urlencoded::to_string(&param).unwrap()
        ),
    };
    let req = TestRequest::get()
        .uri(uri.as_str())
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)))
        .to_request();
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    expect(resp.status()).to_equal(StatusCode::OK)?;
    let body: response::GetDeviceList =
        runtime.block_on(async { test::read_body_json(resp).await });
    expect(body.data.len()).to_equal(expect_count)?;

    let mut code_min = "";
    let mut addr_min = "";
    for info in body.data.iter() {
        if let Err(_) = expect(info.network_code.as_str().ge(code_min)).to_equal(true) {
            return Err(format!(
                "code order error: {}/{} - {}/{}",
                code_min,
                addr_min,
                info.network_code.as_str(),
                info.network_addr.as_str()
            ));
        } else if let Err(_) = expect(info.network_addr.as_str().ge(addr_min)).to_equal(true) {
            return Err(format!(
                "addr order error: {}/{} - {}/{}",
                code_min,
                addr_min,
                info.network_code.as_str(),
                info.network_addr.as_str()
            ));
        }
        if code_min.cmp(info.network_code.as_str()) != Ordering::Equal {
            addr_min = "";
        }
        code_min = info.network_code.as_str();
    }
    Ok(())
}

fn test_get_list_sort(
    runtime: &Runtime,
    state: &routes::State,
    token: &str,
    param: &mut request::GetDeviceList,
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
        "/broker/api/v1/device/list?{}",
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
    let body: response::GetDeviceList =
        runtime.block_on(async { test::read_body_json(resp).await });
    expect(body.data.len()).to_equal(expect_ids.len())?;

    let mut index = 0;
    for data in body.data.iter() {
        expect(data.device_id.as_str()).to_equal(expect_ids[index])?;
        index += 1;
    }
    Ok(())
}

fn test_get_list_offset_limit(
    runtime: &Runtime,
    state: &routes::State,
    token: &str,
    param: &request::GetDeviceList,
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
        "/broker/api/v1/device/list?{}",
        serde_urlencoded::to_string(&param).unwrap()
    );
    let req = TestRequest::get()
        .uri(uri.as_str())
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)))
        .to_request();
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    expect(resp.status()).to_equal(StatusCode::OK)?;
    let body: response::GetDeviceList =
        runtime.block_on(async { test::read_body_json(resp).await });
    expect(body.data.len()).to_equal(expect_ids.len())?;

    let mut index = 0;
    for data in body.data.iter() {
        expect(data.device_id.as_str())
            .to_equal(format!("device_{}", expect_ids[index]).as_str())?;
        index += 1;
    }
    Ok(())
}

fn test_get_list_format_array(
    runtime: &Runtime,
    state: &routes::State,
    token: &str,
    param: &request::GetDeviceList,
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
        "/broker/api/v1/device/list?{}",
        serde_urlencoded::to_string(&param).unwrap()
    );
    let req = TestRequest::get()
        .uri(uri.as_str())
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)))
        .to_request();
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    expect(resp.status()).to_equal(StatusCode::OK)?;
    let body: Vec<response::GetDeviceListData> =
        runtime.block_on(async { test::read_body_json(resp).await });
    expect(body.len()).to_equal(expect_ids.len())?;

    let mut index = 0;
    for data in body.iter() {
        expect(data.device_id.as_str())
            .to_equal(format!("device_{}", expect_ids[index]).as_str())?;
        index += 1;
    }
    Ok(())
}

fn test_get(
    runtime: &Runtime,
    state: &routes::State,
    token: &str,
    device_id: &str,
) -> Result<(), String> {
    let mut app = runtime.block_on(async {
        test::init_service(
            App::new()
                .wrap(NormalizePath::trim())
                .service(routes::new_service(state)),
        )
        .await
    });

    let device_info = get_device_model(runtime, state, device_id, true)?.unwrap();

    let uri = format!("/broker/api/v1/device/{}", device_id);
    let req = TestRequest::get()
        .uri(uri.as_str())
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)))
        .to_request();
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    if let Err(_) = expect(resp.status()).to_equal(StatusCode::OK) {
        return Err(format!("token:{}, app: {}", token, device_id));
    }
    let body: response::GetDevice = runtime.block_on(async { test::read_body_json(resp).await });
    expect(body.data.device_id.as_str()).to_equal(device_info.device_id.as_str())?;
    expect(body.data.unit_id.as_str()).to_equal(device_info.unit_id.as_str())?;
    expect(body.data.unit_code.as_ref()).to_equal(device_info.unit_code.as_ref())?;
    expect(body.data.network_id.as_str()).to_equal(device_info.network_id.as_str())?;
    expect(body.data.network_code.as_str()).to_equal(device_info.network_code.as_str())?;
    expect(body.data.network_addr.as_str()).to_equal(device_info.network_addr.as_str())?;
    expect(
        DateTime::parse_from_rfc3339(body.data.created_at.as_str())
            .unwrap()
            .timestamp_millis(),
    )
    .to_equal(device_info.created_at.timestamp_millis())?;
    expect(
        DateTime::parse_from_rfc3339(body.data.modified_at.as_str())
            .unwrap()
            .timestamp_millis(),
    )
    .to_equal(device_info.modified_at.timestamp_millis())?;
    expect(body.data.name.as_str()).to_equal(device_info.name.as_str())?;
    expect(body.data.info).to_equal(device_info.info)
}

fn test_get_wrong_id(
    runtime: &Runtime,
    state: &routes::State,
    token: &str,
    device_id: &str,
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
        .uri(format!("/broker/api/v1/device/{}", device_id).as_str())
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
    network_id: &str,
    device_id: &str,
    is_public: bool,
) -> Result<(), String> {
    add_device_model(runtime, state, unit_id, network_id, device_id, is_public)?;

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
    let body = request::PatchDevice {
        data: request::PatchDeviceData {
            name: Some("name changes".to_string()),
            info: Some(info.clone()),
        },
    };
    let req = TestRequest::patch()
        .uri(format!("/broker/api/v1/device/{}", device_id).as_str())
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)))
        .set_json(&body)
        .to_request();
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    expect(resp.status()).to_equal(StatusCode::NO_CONTENT)?;

    let time_after = Utc::now().trunc_subsecs(3);
    let device_info = get_device_model(runtime, state, device_id, true)?.unwrap();
    expect(device_info.modified_at.ge(&time_before)).to_equal(true)?;
    expect(device_info.modified_at.le(&time_after)).to_equal(true)?;
    expect(device_info.name.as_str()).to_equal("name changes")?;
    expect(device_info.info).to_equal(info)?;

    let body = request::PatchDevice {
        data: request::PatchDeviceData {
            name: Some("".to_string()),
            info: Some(Map::<String, Value>::new()),
            ..Default::default()
        },
    };
    let req = TestRequest::patch()
        .uri(format!("/broker/api/v1/device/{}", device_id).as_str())
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)))
        .set_json(&body)
        .to_request();
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    expect(resp.status()).to_equal(StatusCode::NO_CONTENT)?;

    let device_info = get_device_model(runtime, state, device_id, true)?.unwrap();
    expect(device_info.name.as_str()).to_equal("")?;
    expect(device_info.info).to_equal(Map::<String, Value>::new())
}

fn test_patch_wrong_id(
    runtime: &Runtime,
    state: &routes::State,
    token: &str,
    device_id: &str,
) -> Result<(), String> {
    let mut app = runtime.block_on(async {
        test::init_service(
            App::new()
                .wrap(NormalizePath::trim())
                .service(routes::new_service(state)),
        )
        .await
    });

    let body = request::PatchDevice {
        data: request::PatchDeviceData {
            name: Some("".to_string()),
            info: Some(Map::<String, Value>::new()),
            ..Default::default()
        },
    };
    let req = TestRequest::patch()
        .uri(format!("/broker/api/v1/device/{}", device_id).as_str())
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
    device_id: &str,
    param: Option<&request::PatchDevice>,
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
        .uri(format!("/broker/api/v1/device/{}", device_id).as_str())
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
    add_network_model(runtime, state, "", "public", "amqp://host")?;
    add_network_model(runtime, state, "manager", "manager", "amqp://host")?;
    add_network_model(runtime, state, "owner1", "owner1", "amqp://host")?;
    add_network_model(runtime, state, "owner2", "owner2", "amqp://host")?;
    let now = Utc::now();

    let mut device = create_device("manager", "public", "manager-public", true);
    device.created_at = now - Duration::milliseconds(2);
    device.modified_at = now - Duration::milliseconds(2);
    runtime.block_on(async {
        if let Err(e) = state.model.device().add(&device).await {
            return Err(format!("add device {} error: {}", device.device_id, e));
        }
        Ok(())
    })?;

    let mut device = create_device("owner1", "public", "owner-public", true);
    device.created_at = now - Duration::milliseconds(1);
    device.modified_at = now - Duration::milliseconds(1);
    runtime.block_on(async {
        if let Err(e) = state.model.device().add(&device).await {
            return Err(format!("add device {} error: {}", device.device_id, e));
        }
        Ok(())
    })?;

    let mut device = create_device("manager", "manager", "manager", true);
    device.created_at = now;
    device.modified_at = now + Duration::milliseconds(5);
    runtime.block_on(async {
        if let Err(e) = state.model.device().add(&device).await {
            return Err(format!("add device {} error: {}", device.device_id, e));
        }
        Ok(())
    })?;

    let mut device = create_device("owner1", "owner1", "owner1-1", true);
    device.created_at = now + Duration::milliseconds(1);
    device.modified_at = now + Duration::milliseconds(4);
    runtime.block_on(async {
        if let Err(e) = state.model.device().add(&device).await {
            return Err(format!("add device {} error: {}", device.device_id, e));
        }
        Ok(())
    })?;

    let mut device = create_device("owner1", "owner1", "owner1-2", true);
    device.created_at = now + Duration::milliseconds(2);
    device.modified_at = now + Duration::milliseconds(3);
    runtime.block_on(async {
        if let Err(e) = state.model.device().add(&device).await {
            return Err(format!("add device {} error: {}", device.device_id, e));
        }
        Ok(())
    })?;

    let mut device = create_device("owner1", "owner1", "owner1-3", true);
    device.created_at = now + Duration::milliseconds(3);
    device.modified_at = now + Duration::milliseconds(2);
    runtime.block_on(async {
        if let Err(e) = state.model.device().add(&device).await {
            return Err(format!("add device {} error: {}", device.device_id, e));
        }
        Ok(())
    })?;

    let mut device = create_device("owner2", "owner2", "owner2-1", true);
    device.created_at = now + Duration::milliseconds(4);
    device.modified_at = now + Duration::milliseconds(1);
    device.name = "owner2-2".to_string();
    runtime.block_on(async {
        if let Err(e) = state.model.device().add(&device).await {
            return Err(format!("add device {} error: {}", device.device_id, e));
        }
        Ok(())
    })?;

    let mut device = create_device("owner2", "owner2", "owner2-2", true);
    device.created_at = now + Duration::milliseconds(5);
    device.modified_at = now;
    device.name = "owner2-1".to_string();
    runtime.block_on(async {
        if let Err(e) = state.model.device().add(&device).await {
            return Err(format!("add device {} error: {}", device.device_id, e));
        }
        Ok(())
    })?;

    Ok((8, 4, 2, 3))
}

/// Generate dataset for extra conditions (network, addr).
/// Returns (manager_public, manager, manager_same-addr, owner_public, owner, owner_same-addr)
/// tuple.
fn count_list_extra_dataset(
    runtime: &Runtime,
    state: &routes::State,
) -> Result<(usize, usize, usize, usize, usize, usize), String> {
    add_unit_model(runtime, state, "manager", vec!["member"], "manager")?;
    add_unit_model(runtime, state, "owner", vec![], "owner")?;
    add_network_model(runtime, state, "", "public", "amqp://host")?;
    add_network_model(runtime, state, "manager", "manager", "amqp://host")?;
    add_network_model(runtime, state, "owner", "owner", "amqp://host")?;

    let mut device = create_device("manager", "public", "manager-addr", true);
    device.device_id = "manager-addr-public".to_string();
    runtime.block_on(async {
        if let Err(e) = state.model.device().add(&device).await {
            return Err(format!("add device {} error: {}", device.device_id, e));
        }
        Ok(())
    })?;

    let device = create_device("manager", "manager", "manager-addr", true);
    runtime.block_on(async {
        if let Err(e) = state.model.device().add(&device).await {
            return Err(format!("add device {} error: {}", device.device_id, e));
        }
        Ok(())
    })?;

    let device = create_device("manager", "manager", "manager-addr2", true);
    runtime.block_on(async {
        if let Err(e) = state.model.device().add(&device).await {
            return Err(format!("add device {} error: {}", device.device_id, e));
        }
        Ok(())
    })?;

    let mut device = create_device("manager", "manager", "same-addr", true);
    device.device_id = "same-addr-manager".to_string();
    runtime.block_on(async {
        if let Err(e) = state.model.device().add(&device).await {
            return Err(format!("add device {} error: {}", device.device_id, e));
        }
        Ok(())
    })?;

    let mut device = create_device("owner", "public", "owner-addr", true);
    device.device_id = "owner-addr-public".to_string();
    runtime.block_on(async {
        if let Err(e) = state.model.device().add(&device).await {
            return Err(format!("add device {} error: {}", device.device_id, e));
        }
        Ok(())
    })?;

    let device = create_device("owner", "owner", "owner-addr", true);
    runtime.block_on(async {
        if let Err(e) = state.model.device().add(&device).await {
            return Err(format!("add device {} error: {}", device.device_id, e));
        }
        Ok(())
    })?;

    let mut device = create_device("owner", "owner", "same-addr", true);
    device.device_id = "same-addr-owner".to_string();
    runtime.block_on(async {
        if let Err(e) = state.model.device().add(&device).await {
            return Err(format!("add device {} error: {}", device.device_id, e));
        }
        Ok(())
    })?;

    Ok((2, 4, 2, 1, 3, 1))
}
