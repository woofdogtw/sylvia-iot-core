use std::cmp::Ordering;

use actix_web::{
    http::{header, StatusCode},
    middleware::NormalizePath,
    test::{self, TestRequest},
    App,
};
use chrono::{Duration, SubsecRound, Utc};
use laboratory::{expect, SpecContext};
use serde_json::{Map, Value};
use serde_urlencoded;
use tokio::runtime::Runtime;

use sylvia_iot_broker::{
    models::device_route::{ListOptions, ListQueryCond, SortCond, SortKey},
    routes,
};
use sylvia_iot_corelib::{err, strings};

use crate::routes::libs::{add_device_bulk_model, get_network_model, rm_device_bulk_model};

use super::{
    super::{
        super::libs::{
            add_application_model, add_device_model, add_device_route_model, add_network_model,
            add_unit_model, create_device_route, get_application_model, get_device_model,
            get_device_route_model, test_get_400, test_invalid_token, ApiError,
        },
        TestState, STATE, TOKEN_MANAGER, TOKEN_MEMBER, TOKEN_OWNER,
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
    add_application_model(runtime, routes_state, "manager", "manager", "amqp://host")?;
    add_application_model(runtime, routes_state, "owner", "owner", "amqp://host")?;
    add_network_model(runtime, routes_state, "", "public", "amqp://host")?;
    add_network_model(runtime, routes_state, "manager", "manager", "amqp://host")?;
    add_network_model(runtime, routes_state, "owner", "owner", "amqp://host")?;
    add_device_model(
        runtime,
        routes_state,
        "manager",
        "public",
        "manager-public",
        true,
        "",
    )?;
    add_device_model(
        runtime,
        routes_state,
        "manager",
        "manager",
        "manager",
        false,
        "",
    )?;
    add_device_model(
        runtime,
        routes_state,
        "owner",
        "public",
        "owner-public",
        true,
        "",
    )?;
    add_device_model(
        runtime,
        routes_state,
        "owner",
        "public",
        "owner-public2",
        true,
        "",
    )?;
    add_device_model(runtime, routes_state, "owner", "owner", "owner", false, "")?;
    add_device_model(runtime, routes_state, "owner", "owner", "owner2", false, "")?;

    let param = request::PostDeviceRoute {
        data: request::PostDeviceRouteData {
            device_id: "manager-public".to_string(),
            application_id: "manager".to_string(),
        },
    };
    test_post(runtime, routes_state, TOKEN_MANAGER, &param, "")?;

    let param = request::PostDeviceRoute {
        data: request::PostDeviceRouteData {
            device_id: "manager".to_string(),
            application_id: "manager".to_string(),
        },
    };
    test_post(runtime, routes_state, TOKEN_MANAGER, &param, "")?;

    let param = request::PostDeviceRoute {
        data: request::PostDeviceRouteData {
            device_id: "owner-public2".to_string(),
            application_id: "owner".to_string(),
        },
    };
    test_post(runtime, routes_state, TOKEN_MANAGER, &param, "")?;

    let param = request::PostDeviceRoute {
        data: request::PostDeviceRouteData {
            device_id: "owner2".to_string(),
            application_id: "owner".to_string(),
        },
    };
    test_post(runtime, routes_state, TOKEN_MANAGER, &param, "")?;

    let param = request::PostDeviceRoute {
        data: request::PostDeviceRouteData {
            device_id: "owner-public".to_string(),
            application_id: "owner".to_string(),
        },
    };
    test_post(runtime, routes_state, TOKEN_OWNER, &param, "")?;

    let param = request::PostDeviceRoute {
        data: request::PostDeviceRouteData {
            device_id: "owner".to_string(),
            application_id: "owner".to_string(),
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
    add_application_model(runtime, routes_state, "manager", "manager", "amqp://host")?;
    add_network_model(runtime, routes_state, "", "public", "amqp://host")?;
    add_network_model(runtime, routes_state, "manager", "manager", "amqp://host")?;
    add_device_model(
        runtime,
        routes_state,
        "manager",
        "public",
        "manager-public",
        true,
        "",
    )?;
    add_device_model(
        runtime,
        routes_state,
        "manager",
        "manager",
        "manager",
        false,
        "",
    )?;

    let param = request::PostDeviceRoute {
        data: request::PostDeviceRouteData {
            device_id: "manager-public".to_string(),
            application_id: "manager".to_string(),
        },
    };
    test_post(runtime, routes_state, TOKEN_MANAGER, &param, "")?;

    let param = request::PostDeviceRoute {
        data: request::PostDeviceRouteData {
            device_id: "manager-public".to_string(),
            application_id: "manager".to_string(),
        },
    };
    test_post(
        runtime,
        routes_state,
        TOKEN_MANAGER,
        &param,
        "err_broker_route_exist",
    )?;

    let param = request::PostDeviceRoute {
        data: request::PostDeviceRouteData {
            device_id: "manager".to_string(),
            application_id: "manager".to_string(),
        },
    };
    test_post(runtime, routes_state, TOKEN_MANAGER, &param, "")?;

    let param = request::PostDeviceRoute {
        data: request::PostDeviceRouteData {
            device_id: "manager".to_string(),
            application_id: "manager".to_string(),
        },
    };
    test_post(
        runtime,
        routes_state,
        TOKEN_MANAGER,
        &param,
        "err_broker_route_exist",
    )
}

pub fn post_not_exist(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    add_unit_model(runtime, routes_state, "manager", vec![], "manager")?;
    add_unit_model(runtime, routes_state, "owner", vec![], "owner")?;
    add_application_model(runtime, routes_state, "manager", "manager", "amqp://host")?;
    add_application_model(runtime, routes_state, "owner", "owner", "amqp://host")?;
    add_network_model(runtime, routes_state, "", "public", "amqp://host")?;
    add_network_model(runtime, routes_state, "manager", "manager", "amqp://host")?;
    add_network_model(runtime, routes_state, "owner", "owner", "amqp://host")?;
    add_device_model(
        runtime,
        routes_state,
        "manager",
        "public",
        "manager-public",
        true,
        "",
    )?;
    add_device_model(
        runtime,
        routes_state,
        "manager",
        "manager",
        "manager",
        false,
        "",
    )?;
    add_device_model(runtime, routes_state, "owner", "owner", "owner", false, "")?;

    let param = request::PostDeviceRoute {
        data: request::PostDeviceRouteData {
            device_id: "id".to_string(),
            application_id: "manager".to_string(),
        },
    };
    test_post(
        runtime,
        routes_state,
        TOKEN_MANAGER,
        &param,
        "err_broker_device_not_exist",
    )?;

    let param = request::PostDeviceRoute {
        data: request::PostDeviceRouteData {
            device_id: "manager".to_string(),
            application_id: "id".to_string(),
        },
    };
    test_post(
        runtime,
        routes_state,
        TOKEN_MANAGER,
        &param,
        "err_broker_application_not_exist",
    )?;

    let param = request::PostDeviceRoute {
        data: request::PostDeviceRouteData {
            device_id: "manager-public".to_string(),
            application_id: "owner".to_string(),
        },
    };
    test_post(
        runtime,
        routes_state,
        TOKEN_OWNER,
        &param,
        "err_broker_device_not_exist",
    )?;

    let param = request::PostDeviceRoute {
        data: request::PostDeviceRouteData {
            device_id: "manager".to_string(),
            application_id: "owner".to_string(),
        },
    };
    test_post(
        runtime,
        routes_state,
        TOKEN_OWNER,
        &param,
        "err_broker_device_not_exist",
    )?;

    let param = request::PostDeviceRoute {
        data: request::PostDeviceRouteData {
            device_id: "owner".to_string(),
            application_id: "manager".to_string(),
        },
    };
    test_post(
        runtime,
        routes_state,
        TOKEN_OWNER,
        &param,
        "err_broker_application_not_exist",
    )
}

pub fn post_not_match_unit(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    add_unit_model(runtime, routes_state, "manager", vec![], "manager")?;
    add_unit_model(runtime, routes_state, "owner", vec![], "owner")?;
    add_application_model(runtime, routes_state, "manager", "manager", "amqp://host")?;
    add_application_model(runtime, routes_state, "owner", "owner", "amqp://host")?;
    add_network_model(runtime, routes_state, "manager", "manager", "amqp://host")?;
    add_device_model(
        runtime,
        routes_state,
        "manager",
        "manager",
        "manager",
        false,
        "",
    )?;
    add_device_model(runtime, routes_state, "owner", "owner", "owner", false, "")?;

    let param = request::PostDeviceRoute {
        data: request::PostDeviceRouteData {
            device_id: "manager".to_string(),
            application_id: "owner".to_string(),
        },
    };
    test_post(
        runtime,
        routes_state,
        TOKEN_MANAGER,
        &param,
        "err_broker_unit_not_match",
    )?;

    let param = request::PostDeviceRoute {
        data: request::PostDeviceRouteData {
            device_id: "owner".to_string(),
            application_id: "manager".to_string(),
        },
    };
    test_post(
        runtime,
        routes_state,
        TOKEN_MANAGER,
        &param,
        "err_broker_unit_not_match",
    )
}

pub fn post_invalid_param(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    test_post_invalid_param(runtime, &routes_state, TOKEN_MANAGER, None)?;

    let param = request::PostDeviceRoute {
        data: request::PostDeviceRouteData {
            device_id: "".to_string(),
            application_id: "id".to_string(),
        },
    };
    test_post_invalid_param(runtime, &routes_state, TOKEN_MANAGER, Some(&param))?;

    let param = request::PostDeviceRoute {
        data: request::PostDeviceRouteData {
            device_id: "id".to_string(),
            application_id: "".to_string(),
        },
    };
    test_post_invalid_param(runtime, &routes_state, TOKEN_MANAGER, Some(&param))
}

pub fn post_invalid_token(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    let req = TestRequest::post().uri("/broker/api/v1/device-route");
    test_invalid_token(runtime, &routes_state, req)
}

pub fn post_bulk(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    add_unit_model(runtime, routes_state, "manager", vec![], "manager")?;
    add_unit_model(runtime, routes_state, "owner", vec![], "owner")?;
    add_application_model(runtime, routes_state, "manager", "manager", "amqp://host")?;
    add_application_model(runtime, routes_state, "owner", "owner", "amqp://host")?;
    add_network_model(runtime, routes_state, "", "public", "amqp://host")?;
    add_network_model(runtime, routes_state, "owner", "owner", "amqp://host")?;
    let mut network_addrs = vec![];
    for i in 0..1025 {
        network_addrs.push(strings::u128_to_addr(i, 32));
    }

    add_device_bulk_model(
        runtime,
        routes_state,
        "manager",
        "public",
        &network_addrs,
        true,
        "",
    )?;
    let mut network_addrs1 = vec![];
    for i in 0..1024 {
        network_addrs1.push(strings::u128_to_addr(i, 32));
    }
    let mut param = request::PostDeviceRouteBulk {
        data: request::PostDeviceRouteBulkData {
            application_id: "manager".to_string(),
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
        &network_addrs1,
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
        &network_addrs1,
    )?;
    rm_device_bulk_model(runtime, routes_state, "manager", "public", &network_addrs)?;

    add_device_bulk_model(
        runtime,
        routes_state,
        "owner",
        "owner",
        &network_addrs,
        true,
        "",
    )?;
    let mut network_addrs1 = vec![];
    for i in 0..1024 {
        network_addrs1.push(strings::u128_to_addr(i, 32));
    }
    let mut param = request::PostDeviceRouteBulk {
        data: request::PostDeviceRouteBulkData {
            application_id: "owner".to_string(),
            network_id: "owner".to_string(),
            network_addrs: network_addrs1.clone(),
        },
    };
    test_post_bulk(
        runtime,
        routes_state,
        TOKEN_OWNER,
        true,
        &param,
        "",
        &network_addrs1,
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
        TOKEN_OWNER,
        true,
        &param,
        "",
        &network_addrs1,
    )
}

pub fn post_bulk_not_exist(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    add_unit_model(runtime, routes_state, "manager", vec![], "manager")?;
    add_unit_model(runtime, routes_state, "owner", vec![], "owner")?;
    add_application_model(runtime, routes_state, "manager", "manager", "amqp://host")?;
    add_application_model(runtime, routes_state, "owner", "owner", "amqp://host")?;
    add_network_model(runtime, routes_state, "", "public", "amqp://host")?;
    add_network_model(runtime, routes_state, "owner", "owner", "amqp://host")?;

    add_device_bulk_model(
        runtime,
        routes_state,
        "manager",
        "public",
        &vec!["0000".to_string()],
        true,
        "",
    )?;
    add_device_bulk_model(
        runtime,
        routes_state,
        "owner",
        "owner",
        &vec!["0001".to_string()],
        true,
        "",
    )?;

    let param = request::PostDeviceRouteBulk {
        data: request::PostDeviceRouteBulkData {
            application_id: "id".to_string(),
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
        "err_broker_application_not_exist",
        &vec!["0000".to_string()],
    )?;

    let param = request::PostDeviceRouteBulk {
        data: request::PostDeviceRouteBulkData {
            application_id: "manager".to_string(),
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
        &vec!["0000".to_string()],
    )?;

    let param = request::PostDeviceRouteBulk {
        data: request::PostDeviceRouteBulkData {
            application_id: "manager".to_string(),
            network_id: "public".to_string(),
            network_addrs: vec!["0001".to_string()],
        },
    };
    test_post_bulk(
        runtime,
        routes_state,
        TOKEN_MANAGER,
        true,
        &param,
        "err_broker_device_not_exist",
        &vec!["0000".to_string()],
    )?;

    let param = request::PostDeviceRouteBulk {
        data: request::PostDeviceRouteBulkData {
            application_id: "manager".to_string(),
            network_id: "owner".to_string(),
            network_addrs: vec!["0001".to_string()],
        },
    };
    test_post_bulk(
        runtime,
        routes_state,
        TOKEN_OWNER,
        true,
        &param,
        "err_broker_application_not_exist",
        &vec!["0001".to_string()],
    )?;

    let param = request::PostDeviceRouteBulk {
        data: request::PostDeviceRouteBulkData {
            application_id: "owner".to_string(),
            network_id: "public".to_string(),
            network_addrs: vec!["0001".to_string()],
        },
    };
    test_post_bulk(
        runtime,
        routes_state,
        TOKEN_OWNER,
        true,
        &param,
        "err_broker_network_not_exist",
        &vec!["0001".to_string()],
    )?;

    let param = request::PostDeviceRouteBulk {
        data: request::PostDeviceRouteBulkData {
            application_id: "owner".to_string(),
            network_id: "owner".to_string(),
            network_addrs: vec!["0000".to_string()],
        },
    };
    test_post_bulk(
        runtime,
        routes_state,
        TOKEN_OWNER,
        true,
        &param,
        "err_broker_device_not_exist",
        &vec!["0001".to_string()],
    )
}

pub fn post_bulk_invalid_param(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    test_post_bulk_invalid_param(runtime, &routes_state, TOKEN_MANAGER, true, None)?;

    let param = request::PostDeviceRouteBulk {
        data: request::PostDeviceRouteBulkData {
            application_id: "".to_string(),
            network_id: "manager".to_string(),
            network_addrs: vec!["0000".to_string()],
        },
    };
    test_post_bulk_invalid_param(runtime, &routes_state, TOKEN_MANAGER, true, Some(&param))?;

    let param = request::PostDeviceRouteBulk {
        data: request::PostDeviceRouteBulkData {
            application_id: "manager".to_string(),
            network_id: "".to_string(),
            network_addrs: vec!["0000".to_string()],
        },
    };
    test_post_bulk_invalid_param(runtime, &routes_state, TOKEN_MANAGER, true, Some(&param))?;

    let param = request::PostDeviceRouteBulk {
        data: request::PostDeviceRouteBulkData {
            application_id: "manager".to_string(),
            network_id: "manager".to_string(),
            network_addrs: vec![],
        },
    };
    test_post_bulk_invalid_param(runtime, &routes_state, TOKEN_MANAGER, true, Some(&param))?;

    let mut network_addrs = vec![];
    for i in 0..1025 {
        network_addrs.push(format!("{:#04}", i));
    }
    let param = request::PostDeviceRouteBulk {
        data: request::PostDeviceRouteBulkData {
            application_id: "manager".to_string(),
            network_id: "manager".to_string(),
            network_addrs,
        },
    };
    test_post_bulk_invalid_param(runtime, &routes_state, TOKEN_MANAGER, true, Some(&param))?;

    let param = request::PostDeviceRouteBulk {
        data: request::PostDeviceRouteBulkData {
            application_id: "manager".to_string(),
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

    let req = TestRequest::post().uri("/broker/api/v1/device-route/bulk");
    test_invalid_token(runtime, &routes_state, req)
}

pub fn post_bulk_del(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    add_unit_model(runtime, routes_state, "manager", vec![], "manager")?;
    add_unit_model(runtime, routes_state, "owner", vec![], "owner")?;
    add_application_model(runtime, routes_state, "manager", "manager", "amqp://host")?;
    add_application_model(runtime, routes_state, "owner", "owner", "amqp://host")?;
    add_network_model(runtime, routes_state, "", "public", "amqp://host")?;
    add_network_model(runtime, routes_state, "owner", "owner", "amqp://host")?;
    let mut network_addrs = vec![];
    for i in 0..1025 {
        network_addrs.push(strings::u128_to_addr(i, 32));
    }

    // Test for manager.
    add_device_bulk_model(
        runtime,
        routes_state,
        "manager",
        "public",
        &network_addrs,
        true,
        "",
    )?;
    let mut network_addrs1 = vec![];
    for i in 0..1024 {
        network_addrs1.push(strings::u128_to_addr(i, 32));
    }
    let mut param = request::PostDeviceRouteBulk {
        data: request::PostDeviceRouteBulkData {
            application_id: "manager".to_string(),
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
        &network_addrs1,
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
        &network_addrs1,
    )?;

    let mut network_addrs1 = vec![];
    for i in 0..1024 {
        network_addrs1.push(strings::u128_to_addr(i, 32));
    }
    let param = request::PostDeviceRouteBulk {
        data: request::PostDeviceRouteBulkData {
            application_id: "manager".to_string(),
            network_id: "public".to_string(),
            network_addrs: network_addrs1,
        },
    };
    test_post_bulk(
        runtime,
        routes_state,
        TOKEN_MANAGER,
        false,
        &param,
        "",
        &vec![strings::u128_to_addr(1024, 32)],
    )?;
    rm_device_bulk_model(runtime, routes_state, "manager", "public", &network_addrs)?;

    // Test for owner.
    add_device_bulk_model(
        runtime,
        routes_state,
        "owner",
        "owner",
        &network_addrs,
        true,
        "",
    )?;
    let mut network_addrs1 = vec![];
    for i in 0..1024 {
        network_addrs1.push(strings::u128_to_addr(i, 32));
    }
    let mut param = request::PostDeviceRouteBulk {
        data: request::PostDeviceRouteBulkData {
            application_id: "owner".to_string(),
            network_id: "owner".to_string(),
            network_addrs: network_addrs1.clone(),
        },
    };
    test_post_bulk(
        runtime,
        routes_state,
        TOKEN_OWNER,
        true,
        &param,
        "",
        &network_addrs1,
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
        TOKEN_OWNER,
        true,
        &param,
        "",
        &network_addrs1,
    )?;

    let mut network_addrs1 = vec![];
    for i in 0..1024 {
        network_addrs1.push(strings::u128_to_addr(i, 32));
    }
    let param = request::PostDeviceRouteBulk {
        data: request::PostDeviceRouteBulkData {
            application_id: "owner".to_string(),
            network_id: "owner".to_string(),
            network_addrs: network_addrs1,
        },
    };
    test_post_bulk(
        runtime,
        routes_state,
        TOKEN_OWNER,
        false,
        &param,
        "",
        &vec![strings::u128_to_addr(1024, 32)],
    )
}

pub fn post_bulk_del_not_exist(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    add_unit_model(runtime, routes_state, "manager", vec![], "manager")?;
    add_unit_model(runtime, routes_state, "owner", vec![], "owner")?;
    add_application_model(runtime, routes_state, "manager", "manager", "amqp://host")?;
    add_application_model(runtime, routes_state, "owner", "owner", "amqp://host")?;
    add_network_model(runtime, routes_state, "", "public", "amqp://host")?;
    add_network_model(runtime, routes_state, "owner", "owner", "amqp://host")?;

    let param = request::PostDeviceRouteBulk {
        data: request::PostDeviceRouteBulkData {
            application_id: "id".to_string(),
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
        "err_broker_application_not_exist",
        &vec!["0000".to_string()],
    )?;

    let param = request::PostDeviceRouteBulk {
        data: request::PostDeviceRouteBulkData {
            application_id: "manager".to_string(),
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
        &vec!["0000".to_string()],
    )?;

    let param = request::PostDeviceRouteBulk {
        data: request::PostDeviceRouteBulkData {
            application_id: "manager".to_string(),
            network_id: "owner".to_string(),
            network_addrs: vec!["0001".to_string()],
        },
    };
    test_post_bulk(
        runtime,
        routes_state,
        TOKEN_OWNER,
        false,
        &param,
        "err_broker_application_not_exist",
        &vec!["0001".to_string()],
    )?;

    let param = request::PostDeviceRouteBulk {
        data: request::PostDeviceRouteBulkData {
            application_id: "owner".to_string(),
            network_id: "public".to_string(),
            network_addrs: vec!["0001".to_string()],
        },
    };
    test_post_bulk(
        runtime,
        routes_state,
        TOKEN_OWNER,
        false,
        &param,
        "err_broker_network_not_exist",
        &vec!["0001".to_string()],
    )
}

pub fn post_bulk_del_invalid_param(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    test_post_bulk_invalid_param(runtime, &routes_state, TOKEN_MANAGER, false, None)?;

    let param = request::PostDeviceRouteBulk {
        data: request::PostDeviceRouteBulkData {
            application_id: "".to_string(),
            network_id: "manager".to_string(),
            network_addrs: vec!["0000".to_string()],
        },
    };
    test_post_bulk_invalid_param(runtime, &routes_state, TOKEN_MANAGER, false, Some(&param))?;

    let param = request::PostDeviceRouteBulk {
        data: request::PostDeviceRouteBulkData {
            application_id: "manager".to_string(),
            network_id: "".to_string(),
            network_addrs: vec!["0000".to_string()],
        },
    };
    test_post_bulk_invalid_param(runtime, &routes_state, TOKEN_MANAGER, false, Some(&param))?;

    let param = request::PostDeviceRouteBulk {
        data: request::PostDeviceRouteBulkData {
            application_id: "manager".to_string(),
            network_id: "manager".to_string(),
            network_addrs: vec![],
        },
    };
    test_post_bulk_invalid_param(runtime, &routes_state, TOKEN_MANAGER, false, Some(&param))?;

    let mut network_addrs = vec![];
    for i in 0..1025 {
        network_addrs.push(format!("{:#04}", i));
    }
    let param = request::PostDeviceRouteBulk {
        data: request::PostDeviceRouteBulkData {
            application_id: "manager".to_string(),
            network_id: "manager".to_string(),
            network_addrs,
        },
    };
    test_post_bulk_invalid_param(runtime, &routes_state, TOKEN_MANAGER, false, Some(&param))?;

    let param = request::PostDeviceRouteBulk {
        data: request::PostDeviceRouteBulkData {
            application_id: "manager".to_string(),
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

    let req = TestRequest::post().uri("/broker/api/v1/device-route/bulk-delete");
    test_invalid_token(runtime, &routes_state, req)
}

pub fn post_range(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    add_unit_model(runtime, routes_state, "manager", vec![], "manager")?;
    add_unit_model(runtime, routes_state, "owner", vec![], "owner")?;
    add_application_model(runtime, routes_state, "manager", "manager", "amqp://host")?;
    add_application_model(runtime, routes_state, "owner", "owner", "amqp://host")?;
    add_network_model(runtime, routes_state, "", "public", "amqp://host")?;
    add_network_model(runtime, routes_state, "owner", "owner", "amqp://host")?;
    let mut network_addrs = vec![];
    for i in 0..1025 {
        network_addrs.push(strings::u128_to_addr(i, 32));
    }

    add_device_bulk_model(
        runtime,
        routes_state,
        "manager",
        "public",
        &network_addrs,
        true,
        "",
    )?;
    let mut network_addrs1 = vec![];
    for i in 0..1024 {
        network_addrs1.push(strings::u128_to_addr(i, 32));
    }
    let mut param = request::PostDeviceRouteRange {
        data: request::PostDeviceRouteRangeData {
            application_id: "manager".to_string(),
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
        &network_addrs1,
    )?;
    rm_device_bulk_model(runtime, routes_state, "manager", "public", &network_addrs)?;

    add_device_bulk_model(
        runtime,
        routes_state,
        "owner",
        "owner",
        &network_addrs,
        true,
        "",
    )?;
    network_addrs1.pop();
    let mut param = request::PostDeviceRouteRange {
        data: request::PostDeviceRouteRangeData {
            application_id: "owner".to_string(),
            network_id: "owner".to_string(),
            start_addr: strings::u128_to_addr(0, 32),
            end_addr: strings::u128_to_addr(1023, 32),
        },
    };
    test_post_range(
        runtime,
        routes_state,
        TOKEN_OWNER,
        true,
        &param,
        "",
        &network_addrs1,
    )?;

    network_addrs1.push(strings::u128_to_addr(1024, 32));
    param.data.start_addr = strings::u128_to_addr(1, 32);
    param.data.end_addr = strings::u128_to_addr(1024, 32);
    test_post_range(
        runtime,
        routes_state,
        TOKEN_OWNER,
        true,
        &param,
        "",
        &network_addrs1,
    )
}

pub fn post_range_not_exist(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    add_unit_model(runtime, routes_state, "manager", vec![], "manager")?;
    add_unit_model(runtime, routes_state, "owner", vec![], "owner")?;
    add_application_model(runtime, routes_state, "manager", "manager", "amqp://host")?;
    add_application_model(runtime, routes_state, "owner", "owner", "amqp://host")?;
    add_network_model(runtime, routes_state, "", "public", "amqp://host")?;
    add_network_model(runtime, routes_state, "owner", "owner", "amqp://host")?;

    add_device_bulk_model(
        runtime,
        routes_state,
        "manager",
        "public",
        &vec!["0000".to_string()],
        true,
        "",
    )?;
    add_device_bulk_model(
        runtime,
        routes_state,
        "owner",
        "owner",
        &vec!["0001".to_string()],
        true,
        "",
    )?;

    let param = request::PostDeviceRouteRange {
        data: request::PostDeviceRouteRangeData {
            application_id: "id".to_string(),
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
        "err_broker_application_not_exist",
        &vec!["0000".to_string()],
    )?;

    let param = request::PostDeviceRouteRange {
        data: request::PostDeviceRouteRangeData {
            application_id: "manager".to_string(),
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
        &vec!["0000".to_string()],
    )?;

    let param = request::PostDeviceRouteRange {
        data: request::PostDeviceRouteRangeData {
            application_id: "manager".to_string(),
            network_id: "public".to_string(),
            start_addr: "0001".to_string(),
            end_addr: "0001".to_string(),
        },
    };
    test_post_range(
        runtime,
        routes_state,
        TOKEN_MANAGER,
        true,
        &param,
        "err_broker_device_not_exist",
        &vec!["0000".to_string()],
    )?;

    let param = request::PostDeviceRouteRange {
        data: request::PostDeviceRouteRangeData {
            application_id: "manager".to_string(),
            network_id: "owner".to_string(),
            start_addr: "0001".to_string(),
            end_addr: "0001".to_string(),
        },
    };
    test_post_range(
        runtime,
        routes_state,
        TOKEN_OWNER,
        true,
        &param,
        "err_broker_application_not_exist",
        &vec!["0001".to_string()],
    )?;

    let param = request::PostDeviceRouteRange {
        data: request::PostDeviceRouteRangeData {
            application_id: "owner".to_string(),
            network_id: "public".to_string(),
            start_addr: "0001".to_string(),
            end_addr: "0001".to_string(),
        },
    };
    test_post_range(
        runtime,
        routes_state,
        TOKEN_OWNER,
        true,
        &param,
        "err_broker_network_not_exist",
        &vec!["0001".to_string()],
    )?;

    let param = request::PostDeviceRouteRange {
        data: request::PostDeviceRouteRangeData {
            application_id: "owner".to_string(),
            network_id: "owner".to_string(),
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
        "err_broker_device_not_exist",
        &vec!["0001".to_string()],
    )
}

pub fn post_range_invalid_param(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    test_post_range_invalid_param(runtime, &routes_state, TOKEN_MANAGER, true, None)?;

    let param = request::PostDeviceRouteRange {
        data: request::PostDeviceRouteRangeData {
            application_id: "".to_string(),
            network_id: "manager".to_string(),
            start_addr: "0000".to_string(),
            end_addr: "0000".to_string(),
        },
    };
    test_post_range_invalid_param(runtime, &routes_state, TOKEN_MANAGER, true, Some(&param))?;

    let param = request::PostDeviceRouteRange {
        data: request::PostDeviceRouteRangeData {
            application_id: "manager".to_string(),
            network_id: "".to_string(),
            start_addr: "0000".to_string(),
            end_addr: "0000".to_string(),
        },
    };
    test_post_range_invalid_param(runtime, &routes_state, TOKEN_MANAGER, true, Some(&param))?;

    let param = request::PostDeviceRouteRange {
        data: request::PostDeviceRouteRangeData {
            application_id: "manager".to_string(),
            network_id: "manager".to_string(),
            start_addr: "".to_string(),
            end_addr: "0000".to_string(),
        },
    };
    test_post_range_invalid_param(runtime, &routes_state, TOKEN_MANAGER, true, Some(&param))?;

    let param = request::PostDeviceRouteRange {
        data: request::PostDeviceRouteRangeData {
            application_id: "manager".to_string(),
            network_id: "manager".to_string(),
            start_addr: "0000".to_string(),
            end_addr: "".to_string(),
        },
    };
    test_post_range_invalid_param(runtime, &routes_state, TOKEN_MANAGER, true, Some(&param))?;

    let param = request::PostDeviceRouteRange {
        data: request::PostDeviceRouteRangeData {
            application_id: "manager".to_string(),
            network_id: "manager".to_string(),
            start_addr: "000g".to_string(),
            end_addr: "0000".to_string(),
        },
    };
    test_post_range_invalid_param(runtime, &routes_state, TOKEN_MANAGER, true, Some(&param))?;

    let param = request::PostDeviceRouteRange {
        data: request::PostDeviceRouteRangeData {
            application_id: "manager".to_string(),
            network_id: "manager".to_string(),
            start_addr: "0000".to_string(),
            end_addr: "000g".to_string(),
        },
    };
    test_post_range_invalid_param(runtime, &routes_state, TOKEN_MANAGER, true, Some(&param))?;

    let param = request::PostDeviceRouteRange {
        data: request::PostDeviceRouteRangeData {
            application_id: "manager".to_string(),
            network_id: "manager".to_string(),
            start_addr: "0000".to_string(),
            end_addr: "000000".to_string(),
        },
    };
    test_post_range_invalid_param(runtime, &routes_state, TOKEN_MANAGER, true, Some(&param))?;

    let param = request::PostDeviceRouteRange {
        data: request::PostDeviceRouteRangeData {
            application_id: "manager".to_string(),
            network_id: "manager".to_string(),
            start_addr: "0000".to_string(),
            end_addr: "0400".to_string(),
        },
    };
    test_post_range_invalid_param(runtime, &routes_state, TOKEN_MANAGER, true, Some(&param))?;

    let param = request::PostDeviceRouteRange {
        data: request::PostDeviceRouteRangeData {
            application_id: "manager".to_string(),
            network_id: "manager".to_string(),
            start_addr: "0001".to_string(),
            end_addr: "0000".to_string(),
        },
    };
    test_post_range_invalid_param(runtime, &routes_state, TOKEN_MANAGER, true, Some(&param))?;

    let param = request::PostDeviceRouteRange {
        data: request::PostDeviceRouteRangeData {
            application_id: "manager".to_string(),
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

    let req = TestRequest::post().uri("/broker/api/v1/device-route/range");
    test_invalid_token(runtime, &routes_state, req)
}

pub fn post_range_del(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    add_unit_model(runtime, routes_state, "manager", vec![], "manager")?;
    add_unit_model(runtime, routes_state, "owner", vec![], "owner")?;
    add_application_model(runtime, routes_state, "manager", "manager", "amqp://host")?;
    add_application_model(runtime, routes_state, "owner", "owner", "amqp://host")?;
    add_network_model(runtime, routes_state, "", "public", "amqp://host")?;
    add_network_model(runtime, routes_state, "owner", "owner", "amqp://host")?;
    let mut network_addrs = vec![];
    for i in 0..1025 {
        network_addrs.push(strings::u128_to_addr(i, 32));
    }

    // Test for manager.
    add_device_bulk_model(
        runtime,
        routes_state,
        "manager",
        "public",
        &network_addrs,
        true,
        "",
    )?;
    let mut network_addrs1 = vec![];
    for i in 0..1024 {
        network_addrs1.push(strings::u128_to_addr(i, 32));
    }
    let mut param = request::PostDeviceRouteRange {
        data: request::PostDeviceRouteRangeData {
            application_id: "manager".to_string(),
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
        &network_addrs1,
    )?;

    let param = request::PostDeviceRouteRange {
        data: request::PostDeviceRouteRangeData {
            application_id: "manager".to_string(),
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
        &vec![strings::u128_to_addr(1024, 32)],
    )?;
    rm_device_bulk_model(runtime, routes_state, "manager", "public", &network_addrs)?;

    // Test for owner.
    add_device_bulk_model(
        runtime,
        routes_state,
        "owner",
        "owner",
        &network_addrs,
        true,
        "",
    )?;
    network_addrs1.pop();
    let mut param = request::PostDeviceRouteRange {
        data: request::PostDeviceRouteRangeData {
            application_id: "owner".to_string(),
            network_id: "owner".to_string(),
            start_addr: strings::u128_to_addr(0, 32),
            end_addr: strings::u128_to_addr(1023, 32),
        },
    };
    test_post_range(
        runtime,
        routes_state,
        TOKEN_OWNER,
        true,
        &param,
        "",
        &network_addrs1,
    )?;
    network_addrs1.push(strings::u128_to_addr(1024, 32));
    param.data.start_addr = strings::u128_to_addr(1024, 32);
    param.data.end_addr = strings::u128_to_addr(1024, 32);
    test_post_range(
        runtime,
        routes_state,
        TOKEN_OWNER,
        true,
        &param,
        "",
        &network_addrs1,
    )?;

    let param = request::PostDeviceRouteRange {
        data: request::PostDeviceRouteRangeData {
            application_id: "owner".to_string(),
            network_id: "owner".to_string(),
            start_addr: strings::u128_to_addr(0, 32),
            end_addr: strings::u128_to_addr(1023, 32),
        },
    };
    test_post_range(
        runtime,
        routes_state,
        TOKEN_OWNER,
        false,
        &param,
        "",
        &vec![strings::u128_to_addr(1024, 32)],
    )
}

pub fn post_range_del_not_exist(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    add_unit_model(runtime, routes_state, "manager", vec![], "manager")?;
    add_unit_model(runtime, routes_state, "owner", vec![], "owner")?;
    add_application_model(runtime, routes_state, "manager", "manager", "amqp://host")?;
    add_application_model(runtime, routes_state, "owner", "owner", "amqp://host")?;
    add_network_model(runtime, routes_state, "", "public", "amqp://host")?;
    add_network_model(runtime, routes_state, "owner", "owner", "amqp://host")?;

    add_device_bulk_model(
        runtime,
        routes_state,
        "manager",
        "public",
        &vec!["0000".to_string()],
        true,
        "",
    )?;
    add_device_bulk_model(
        runtime,
        routes_state,
        "owner",
        "owner",
        &vec!["0001".to_string()],
        true,
        "",
    )?;

    let param = request::PostDeviceRouteRange {
        data: request::PostDeviceRouteRangeData {
            application_id: "id".to_string(),
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
        "err_broker_application_not_exist",
        &vec!["0000".to_string()],
    )?;

    let param = request::PostDeviceRouteRange {
        data: request::PostDeviceRouteRangeData {
            application_id: "manager".to_string(),
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
        &vec!["0000".to_string()],
    )?;

    let param = request::PostDeviceRouteRange {
        data: request::PostDeviceRouteRangeData {
            application_id: "manager".to_string(),
            network_id: "owner".to_string(),
            start_addr: "0001".to_string(),
            end_addr: "0001".to_string(),
        },
    };
    test_post_range(
        runtime,
        routes_state,
        TOKEN_OWNER,
        false,
        &param,
        "err_broker_application_not_exist",
        &vec!["0001".to_string()],
    )?;

    let param = request::PostDeviceRouteRange {
        data: request::PostDeviceRouteRangeData {
            application_id: "owner".to_string(),
            network_id: "public".to_string(),
            start_addr: "0001".to_string(),
            end_addr: "0001".to_string(),
        },
    };
    test_post_range(
        runtime,
        routes_state,
        TOKEN_OWNER,
        false,
        &param,
        "err_broker_network_not_exist",
        &vec!["0001".to_string()],
    )
}

pub fn post_range_del_invalid_param(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    test_post_range_invalid_param(runtime, &routes_state, TOKEN_MANAGER, false, None)?;

    let param = request::PostDeviceRouteRange {
        data: request::PostDeviceRouteRangeData {
            application_id: "".to_string(),
            network_id: "manager".to_string(),
            start_addr: "0000".to_string(),
            end_addr: "0000".to_string(),
        },
    };
    test_post_range_invalid_param(runtime, &routes_state, TOKEN_MANAGER, false, Some(&param))?;

    let param = request::PostDeviceRouteRange {
        data: request::PostDeviceRouteRangeData {
            application_id: "manager".to_string(),
            network_id: "".to_string(),
            start_addr: "0000".to_string(),
            end_addr: "0000".to_string(),
        },
    };
    test_post_range_invalid_param(runtime, &routes_state, TOKEN_MANAGER, false, Some(&param))?;

    let param = request::PostDeviceRouteRange {
        data: request::PostDeviceRouteRangeData {
            application_id: "manager".to_string(),
            network_id: "manager".to_string(),
            start_addr: "".to_string(),
            end_addr: "0000".to_string(),
        },
    };
    test_post_range_invalid_param(runtime, &routes_state, TOKEN_MANAGER, false, Some(&param))?;

    let param = request::PostDeviceRouteRange {
        data: request::PostDeviceRouteRangeData {
            application_id: "manager".to_string(),
            network_id: "manager".to_string(),
            start_addr: "0000".to_string(),
            end_addr: "".to_string(),
        },
    };
    test_post_range_invalid_param(runtime, &routes_state, TOKEN_MANAGER, false, Some(&param))?;

    let param = request::PostDeviceRouteRange {
        data: request::PostDeviceRouteRangeData {
            application_id: "manager".to_string(),
            network_id: "manager".to_string(),
            start_addr: "000g".to_string(),
            end_addr: "0000".to_string(),
        },
    };
    test_post_range_invalid_param(runtime, &routes_state, TOKEN_MANAGER, false, Some(&param))?;

    let param = request::PostDeviceRouteRange {
        data: request::PostDeviceRouteRangeData {
            application_id: "manager".to_string(),
            network_id: "manager".to_string(),
            start_addr: "0000".to_string(),
            end_addr: "000g".to_string(),
        },
    };
    test_post_range_invalid_param(runtime, &routes_state, TOKEN_MANAGER, false, Some(&param))?;

    let param = request::PostDeviceRouteRange {
        data: request::PostDeviceRouteRangeData {
            application_id: "manager".to_string(),
            network_id: "manager".to_string(),
            start_addr: "0000".to_string(),
            end_addr: "000000".to_string(),
        },
    };
    test_post_range_invalid_param(runtime, &routes_state, TOKEN_MANAGER, false, Some(&param))?;

    let param = request::PostDeviceRouteRange {
        data: request::PostDeviceRouteRangeData {
            application_id: "manager".to_string(),
            network_id: "manager".to_string(),
            start_addr: "0000".to_string(),
            end_addr: "0400".to_string(),
        },
    };
    test_post_range_invalid_param(runtime, &routes_state, TOKEN_MANAGER, false, Some(&param))?;

    let param = request::PostDeviceRouteRange {
        data: request::PostDeviceRouteRangeData {
            application_id: "manager".to_string(),
            network_id: "manager".to_string(),
            start_addr: "0001".to_string(),
            end_addr: "0000".to_string(),
        },
    };
    test_post_range_invalid_param(runtime, &routes_state, TOKEN_MANAGER, false, Some(&param))?;

    let param = request::PostDeviceRouteRange {
        data: request::PostDeviceRouteRangeData {
            application_id: "manager".to_string(),
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

    let req = TestRequest::post().uri("/broker/api/v1/device-route/range-delete");
    test_invalid_token(runtime, &routes_state, req)
}

pub fn get_count(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    count_list_dataset(runtime, &routes_state)?;

    test_get_count(runtime, &routes_state, TOKEN_MANAGER, None, 6)?;

    let param = request::GetDeviceRouteCount {
        unit: Some("".to_string()),
        ..Default::default()
    };
    test_get_count(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 6)?;

    let param = request::GetDeviceRouteCount {
        unit: Some("".to_string()),
        application: Some("".to_string()),
        network: Some("".to_string()),
        device: Some("".to_string()),
        ..Default::default()
    };
    test_get_count(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 6)?;

    let param = request::GetDeviceRouteCount {
        unit: Some("manager".to_string()),
        ..Default::default()
    };
    test_get_count(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 2)?;
    test_get_count(runtime, &routes_state, TOKEN_MEMBER, Some(&param), 2)?;

    let param = request::GetDeviceRouteCount {
        unit: Some("owner1".to_string()),
        ..Default::default()
    };
    test_get_count(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 3)?;
    test_get_count(runtime, &routes_state, TOKEN_OWNER, Some(&param), 3)?;

    let param = request::GetDeviceRouteCount {
        unit: Some("owner2".to_string()),
        ..Default::default()
    };
    test_get_count(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 1)?;
    test_get_count(runtime, &routes_state, TOKEN_OWNER, Some(&param), 1)?;
    test_get_count(runtime, &routes_state, TOKEN_MEMBER, Some(&param), 1)?;

    let mut param = request::GetDeviceRouteCount {
        application: Some("manager".to_string()),
        ..Default::default()
    };
    test_get_count(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 2)?;
    param.unit = Some("manager".to_string());
    test_get_count(runtime, &routes_state, TOKEN_MEMBER, Some(&param), 2)?;

    let mut param = request::GetDeviceRouteCount {
        application: Some("owner1-1".to_string()),
        ..Default::default()
    };
    test_get_count(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 2)?;
    param.unit = Some("owner1".to_string());
    test_get_count(runtime, &routes_state, TOKEN_OWNER, Some(&param), 2)?;

    let mut param = request::GetDeviceRouteCount {
        application: Some("owner1-2".to_string()),
        ..Default::default()
    };
    test_get_count(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 1)?;
    param.unit = Some("owner2".to_string());
    test_get_count(runtime, &routes_state, TOKEN_OWNER, Some(&param), 0)?;
    test_get_count(runtime, &routes_state, TOKEN_MEMBER, Some(&param), 0)?;

    let param = request::GetDeviceRouteCount {
        unit: Some("owner2".to_string()),
        application: Some("owner2".to_string()),
        ..Default::default()
    };
    test_get_count(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 1)?;
    test_get_count(runtime, &routes_state, TOKEN_OWNER, Some(&param), 1)?;
    test_get_count(runtime, &routes_state, TOKEN_MEMBER, Some(&param), 1)?;

    let mut param = request::GetDeviceRouteCount {
        network: Some("public".to_string()),
        ..Default::default()
    };
    test_get_count(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 3)?;
    param.unit = Some("manager".to_string());
    test_get_count(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 1)?;
    test_get_count(runtime, &routes_state, TOKEN_MEMBER, Some(&param), 1)?;
    param.unit = Some("owner1".to_string());
    test_get_count(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 2)?;
    test_get_count(runtime, &routes_state, TOKEN_OWNER, Some(&param), 2)?;

    let mut param = request::GetDeviceRouteCount {
        network: Some("manager".to_string()),
        ..Default::default()
    };
    test_get_count(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 1)?;
    param.unit = Some("manager".to_string());
    test_get_count(runtime, &routes_state, TOKEN_MEMBER, Some(&param), 1)?;

    let mut param = request::GetDeviceRouteCount {
        network: Some("owner1-1".to_string()),
        ..Default::default()
    };
    test_get_count(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 1)?;
    param.unit = Some("owner1".to_string());
    test_get_count(runtime, &routes_state, TOKEN_OWNER, Some(&param), 1)?;
    param.unit = Some("owner2".to_string());
    test_get_count(runtime, &routes_state, TOKEN_OWNER, Some(&param), 0)?;

    let param = request::GetDeviceRouteCount {
        unit: Some("manager".to_string()),
        application: Some("manager".to_string()),
        network: Some("manager".to_string()),
        ..Default::default()
    };
    test_get_count(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 1)?;
    test_get_count(runtime, &routes_state, TOKEN_MEMBER, Some(&param), 1)?;

    let param = request::GetDeviceRouteCount {
        unit: Some("owner1".to_string()),
        application: Some("owner1-2".to_string()),
        network: Some("public".to_string()),
        ..Default::default()
    };
    test_get_count(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 1)?;
    test_get_count(runtime, &routes_state, TOKEN_OWNER, Some(&param), 1)?;

    let mut param = request::GetDeviceRouteCount {
        device: Some("manager-public".to_string()),
        ..Default::default()
    };
    test_get_count(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 1)?;
    param.device = Some("owner1-public".to_string());
    test_get_count(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 2)?;

    let param = request::GetDeviceRouteCount {
        unit: Some("manager".to_string()),
        device: Some("manager-public".to_string()),
        ..Default::default()
    };
    test_get_count(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 1)?;
    test_get_count(runtime, &routes_state, TOKEN_MEMBER, Some(&param), 1)?;

    let param = request::GetDeviceRouteCount {
        unit: Some("owner1".to_string()),
        device: Some("owner1-public".to_string()),
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

    let uri = "/broker/api/v1/device-route/count";
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

    let uri = "/broker/api/v1/device-route/count";
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

    let req = TestRequest::get().uri("/broker/api/v1/device-route/count");
    test_invalid_token(runtime, &routes_state, req)
}

pub fn get_list(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    count_list_dataset(runtime, &routes_state)?;

    test_get_list(runtime, &routes_state, TOKEN_MANAGER, None, 6)?;

    let param = request::GetDeviceRouteList {
        unit: Some("".to_string()),
        ..Default::default()
    };
    test_get_list(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 6)?;

    test_get_list(runtime, &routes_state, TOKEN_MANAGER, None, 6)?;

    let param = request::GetDeviceRouteList {
        unit: Some("".to_string()),
        application: Some("".to_string()),
        network: Some("".to_string()),
        device: Some("".to_string()),
        ..Default::default()
    };
    test_get_list(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 6)?;

    let param = request::GetDeviceRouteList {
        unit: Some("manager".to_string()),
        ..Default::default()
    };
    test_get_list(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 2)?;
    test_get_list(runtime, &routes_state, TOKEN_MEMBER, Some(&param), 2)?;

    let param = request::GetDeviceRouteList {
        unit: Some("owner1".to_string()),
        ..Default::default()
    };
    test_get_list(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 3)?;
    test_get_list(runtime, &routes_state, TOKEN_OWNER, Some(&param), 3)?;

    let param = request::GetDeviceRouteList {
        unit: Some("owner2".to_string()),
        ..Default::default()
    };
    test_get_list(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 1)?;
    test_get_list(runtime, &routes_state, TOKEN_OWNER, Some(&param), 1)?;
    test_get_list(runtime, &routes_state, TOKEN_MEMBER, Some(&param), 1)?;

    let mut param = request::GetDeviceRouteList {
        application: Some("manager".to_string()),
        ..Default::default()
    };
    test_get_list(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 2)?;
    param.unit = Some("manager".to_string());
    test_get_list(runtime, &routes_state, TOKEN_MEMBER, Some(&param), 2)?;

    let mut param = request::GetDeviceRouteList {
        application: Some("owner1-1".to_string()),
        ..Default::default()
    };
    test_get_list(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 2)?;
    param.unit = Some("owner1".to_string());
    test_get_list(runtime, &routes_state, TOKEN_OWNER, Some(&param), 2)?;

    let mut param = request::GetDeviceRouteList {
        application: Some("owner1-2".to_string()),
        ..Default::default()
    };
    test_get_list(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 1)?;
    param.unit = Some("owner2".to_string());
    test_get_list(runtime, &routes_state, TOKEN_OWNER, Some(&param), 0)?;
    test_get_list(runtime, &routes_state, TOKEN_MEMBER, Some(&param), 0)?;

    let param = request::GetDeviceRouteList {
        unit: Some("owner2".to_string()),
        application: Some("owner2".to_string()),
        ..Default::default()
    };
    test_get_list(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 1)?;
    test_get_list(runtime, &routes_state, TOKEN_OWNER, Some(&param), 1)?;
    test_get_list(runtime, &routes_state, TOKEN_MEMBER, Some(&param), 1)?;

    let mut param = request::GetDeviceRouteList {
        network: Some("public".to_string()),
        ..Default::default()
    };
    test_get_list(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 3)?;
    param.unit = Some("manager".to_string());
    test_get_list(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 1)?;
    test_get_list(runtime, &routes_state, TOKEN_MEMBER, Some(&param), 1)?;
    param.unit = Some("owner1".to_string());
    test_get_list(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 2)?;
    test_get_list(runtime, &routes_state, TOKEN_OWNER, Some(&param), 2)?;

    let mut param = request::GetDeviceRouteList {
        network: Some("manager".to_string()),
        ..Default::default()
    };
    test_get_list(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 1)?;
    param.unit = Some("manager".to_string());
    test_get_list(runtime, &routes_state, TOKEN_MEMBER, Some(&param), 1)?;

    let mut param = request::GetDeviceRouteList {
        network: Some("owner1-1".to_string()),
        ..Default::default()
    };
    test_get_list(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 1)?;
    param.unit = Some("owner1".to_string());
    test_get_list(runtime, &routes_state, TOKEN_OWNER, Some(&param), 1)?;
    param.unit = Some("owner2".to_string());
    test_get_list(runtime, &routes_state, TOKEN_OWNER, Some(&param), 0)?;

    let param = request::GetDeviceRouteList {
        unit: Some("manager".to_string()),
        application: Some("manager".to_string()),
        network: Some("manager".to_string()),
        ..Default::default()
    };
    test_get_list(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 1)?;
    test_get_list(runtime, &routes_state, TOKEN_MEMBER, Some(&param), 1)?;

    let param = request::GetDeviceRouteList {
        unit: Some("owner1".to_string()),
        application: Some("owner1-2".to_string()),
        network: Some("public".to_string()),
        ..Default::default()
    };
    test_get_list(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 1)?;
    test_get_list(runtime, &routes_state, TOKEN_OWNER, Some(&param), 1)?;

    let mut param = request::GetDeviceRouteList {
        device: Some("manager-public".to_string()),
        ..Default::default()
    };
    test_get_list(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 1)?;
    param.device = Some("owner1-public".to_string());
    test_get_list(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 2)?;

    let param = request::GetDeviceRouteList {
        unit: Some("manager".to_string()),
        device: Some("manager-public".to_string()),
        ..Default::default()
    };
    test_get_list(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 1)?;
    test_get_list(runtime, &routes_state, TOKEN_MEMBER, Some(&param), 1)?;

    let param = request::GetDeviceRouteList {
        unit: Some("owner1".to_string()),
        device: Some("owner1-public".to_string()),
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

    let mut param = request::GetDeviceRouteList {
        ..Default::default()
    };
    test_get_list_sort(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        &mut param,
        &[
            "manager-manager",
            "owner1-1-1-owner1-1",
            "owner2-1-owner2",
            "manager-public-manager",
            "owner1-public-owner1-2",
            "owner1-public-owner1-1",
        ],
    )?;

    param.sort_vec = Some(vec![("network", true), ("addr", true), ("created", false)]);
    test_get_list_sort(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        &mut param,
        &[
            "manager-manager",
            "owner1-1-1-owner1-1",
            "owner2-1-owner2",
            "manager-public-manager",
            "owner1-public-owner1-2",
            "owner1-public-owner1-1",
        ],
    )?;
    param.sort_vec = Some(vec![("network", true), ("addr", true), ("created", true)]);
    test_get_list_sort(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        &mut param,
        &[
            "manager-manager",
            "owner1-1-1-owner1-1",
            "owner2-1-owner2",
            "manager-public-manager",
            "owner1-public-owner1-1",
            "owner1-public-owner1-2",
        ],
    )?;
    param.sort_vec = Some(vec![("network", false), ("addr", true), ("created", false)]);
    test_get_list_sort(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        &mut param,
        &[
            "manager-public-manager",
            "owner1-public-owner1-2",
            "owner1-public-owner1-1",
            "owner2-1-owner2",
            "owner1-1-1-owner1-1",
            "manager-manager",
        ],
    )?;
    param.sort_vec = Some(vec![("network", false), ("addr", true), ("created", true)]);
    test_get_list_sort(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        &mut param,
        &[
            "manager-public-manager",
            "owner1-public-owner1-1",
            "owner1-public-owner1-2",
            "owner2-1-owner2",
            "owner1-1-1-owner1-1",
            "manager-manager",
        ],
    )?;

    param.sort_vec = Some(vec![("addr", true), ("created", true)]);
    test_get_list_sort(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        &mut param,
        &[
            "manager-manager",
            "manager-public-manager",
            "owner1-1-1-owner1-1",
            "owner1-public-owner1-1",
            "owner1-public-owner1-2",
            "owner2-1-owner2",
        ],
    )?;
    param.sort_vec = Some(vec![("addr", false), ("created", true)]);
    test_get_list_sort(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        &mut param,
        &[
            "owner2-1-owner2",
            "owner1-public-owner1-1",
            "owner1-public-owner1-2",
            "owner1-1-1-owner1-1",
            "manager-public-manager",
            "manager-manager",
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
            "owner1-public-owner1-1",
            "owner1-1-1-owner1-1",
            "owner1-public-owner1-2",
            "owner2-1-owner2",
        ],
    )?;
    param.sort_vec = Some(vec![("application", false), ("created", true)]);
    test_get_list_sort(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        &mut param,
        &[
            "owner2-1-owner2",
            "owner1-public-owner1-2",
            "owner1-public-owner1-1",
            "owner1-1-1-owner1-1",
            "manager-public-manager",
            "manager-manager",
        ],
    )?;

    param.sort_vec = Some(vec![("created", true)]);
    test_get_list_sort(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        &mut param,
        &[
            "manager-public-manager",
            "manager-manager",
            "owner1-public-owner1-1",
            "owner1-public-owner1-2",
            "owner1-1-1-owner1-1",
            "owner2-1-owner2",
        ],
    )?;
    param.sort_vec = Some(vec![("created", false)]);
    test_get_list_sort(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        &mut param,
        &[
            "owner2-1-owner2",
            "owner1-1-1-owner1-1",
            "owner1-public-owner1-2",
            "owner1-public-owner1-1",
            "manager-manager",
            "manager-public-manager",
        ],
    )?;

    param.sort_vec = Some(vec![("modified", true)]);
    test_get_list_sort(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        &mut param,
        &[
            "owner2-1-owner2",
            "owner1-1-1-owner1-1",
            "owner1-public-owner1-2",
            "owner1-public-owner1-1",
            "manager-manager",
            "manager-public-manager",
        ],
    )?;
    param.sort_vec = Some(vec![("modified", false)]);
    test_get_list_sort(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        &mut param,
        &[
            "manager-public-manager",
            "manager-manager",
            "owner1-public-owner1-1",
            "owner1-public-owner1-2",
            "owner1-1-1-owner1-1",
            "owner2-1-owner2",
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

    for i in 100..302 {
        let addr = format!("device_{}", i);
        add_device_model(
            runtime,
            &routes_state,
            "manager",
            "manager",
            addr.as_str(),
            false,
            "",
        )?;
        add_device_route_model(
            runtime,
            &routes_state,
            format!("route_{}", i).as_str(),
            "manager",
            "manager",
            "manager",
            addr.as_str(),
            "",
        )?;
    }

    let param = request::GetDeviceRouteList {
        ..Default::default()
    };
    test_get_list_offset_limit(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        &param,
        (100..200).collect(),
    )?;

    let mut param = request::GetDeviceRouteList {
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
    add_application_model(runtime, routes_state, "manager", "manager", "amqp://host")?;
    add_network_model(runtime, routes_state, "manager", "manager", "amqp://host")?;

    for i in 100..302 {
        let addr = format!("device_{}", i);
        add_device_model(
            runtime,
            &routes_state,
            "manager",
            "manager",
            addr.as_str(),
            false,
            "",
        )?;
        add_device_route_model(
            runtime,
            &routes_state,
            format!("route_{}", i).as_str(),
            "manager",
            "manager",
            "manager",
            addr.as_str(),
            "",
        )?;
    }

    let mut param = request::GetDeviceRouteList {
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

    let uri = "/broker/api/v1/device-route/list";
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

    let uri = "/broker/api/v1/device-route/list";
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
        Value::String("created:asc,network:true".to_string()),
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

    let req = TestRequest::get().uri("/broker/api/v1/device-route/list");
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
        "addr1",
        false,
        "",
    )?;
    add_device_model(
        runtime,
        &routes_state,
        "manager",
        "manager",
        "addr2",
        false,
        "",
    )?;
    add_device_route_model(
        runtime,
        &routes_state,
        "route1",
        "manager",
        "manager",
        "manager",
        "addr1",
        "",
    )?;
    add_device_route_model(
        runtime,
        &routes_state,
        "route2",
        "manager",
        "manager",
        "manager",
        "addr2",
        "",
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
        .uri("/broker/api/v1/device-route/id")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", TOKEN_MANAGER)))
        .to_request();
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    expect(resp.status()).to_equal(StatusCode::NO_CONTENT)?;
    let _ = get_device_route_model(runtime, &routes_state, "route1", true)?;
    let _ = get_device_route_model(runtime, &routes_state, "route2", true)?;

    let req = TestRequest::delete()
        .uri("/broker/api/v1/device-route/route1")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", TOKEN_OWNER)))
        .to_request();
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    expect(resp.status()).to_equal(StatusCode::NO_CONTENT)?;
    let _ = get_device_route_model(runtime, &routes_state, "route1", true)?;
    let _ = get_device_route_model(runtime, &routes_state, "route2", true)?;

    let req = TestRequest::delete()
        .uri("/broker/api/v1/device-route/route1")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", TOKEN_MANAGER)))
        .to_request();
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    expect(resp.status()).to_equal(StatusCode::NO_CONTENT)?;
    let _ = get_device_route_model(runtime, &routes_state, "route1", false)?;
    let _ = get_device_route_model(runtime, &routes_state, "route2", true)?;

    Ok(())
}

pub fn delete_invalid_token(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    let req = TestRequest::delete().uri("/broker/api/v1/device-route/id");
    test_invalid_token(runtime, &routes_state, req)
}

fn test_post(
    runtime: &Runtime,
    state: &routes::State,
    token: &str,
    param: &request::PostDeviceRoute,
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
        .uri("/broker/api/v1/device-route")
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
    let body: response::PostDeviceRoute =
        runtime.block_on(async { test::read_body_json(resp).await });
    expect(body.data.route_id.len() > 0).to_equal(true)?;

    let device_id = param.data.device_id.as_str();
    let application_id = param.data.application_id.as_str();

    let device_info = get_device_model(runtime, state, device_id, true)?.unwrap();
    let application_info = get_application_model(runtime, state, application_id, true)?.unwrap();
    let route_info = match runtime.block_on(async {
        state
            .model
            .device_route()
            .get(body.data.route_id.as_str())
            .await
    }) {
        Err(e) => return Err(format!("get device route model error: {}", e)),
        Ok(info) => match info {
            None => return Err("add device route then get none device route".to_string()),
            Some(info) => info,
        },
    };
    expect(route_info.unit_id.as_str()).to_equal(application_info.unit_id.as_str())?;
    expect(route_info.application_id.as_str()).to_equal(application_id)?;
    expect(route_info.application_code.as_str()).to_equal(application_info.code.as_str())?;
    expect(route_info.device_id.as_str()).to_equal(device_id)?;
    expect(route_info.network_code.as_str()).to_equal(device_info.network_code.as_str())?;
    expect(route_info.network_addr.as_str()).to_equal(device_info.network_addr.as_str())?;
    expect(route_info.created_at.ge(&time_before)).to_equal(true)?;
    expect(route_info.created_at.le(&time_after)).to_equal(true)
}

fn test_post_invalid_param(
    runtime: &Runtime,
    state: &routes::State,
    token: &str,
    param: Option<&request::PostDeviceRoute>,
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
        .uri("/broker/api/v1/device-route")
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
    param: &request::PostDeviceRouteBulk,
    expect_code: &str,
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
        false => TestRequest::post().uri("/broker/api/v1/device-route/bulk-delete"),
        true => TestRequest::post().uri("/broker/api/v1/device-route/bulk"),
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

    let application_id = param.data.application_id.as_str();
    let network_id = param.data.network_id.as_str();

    let application_info = get_application_model(runtime, state, application_id, true)?.unwrap();
    let network_info = get_network_model(runtime, state, network_id, true)?.unwrap();
    let route_list = match runtime.block_on(async {
        let cond = ListQueryCond {
            application_id: Some(param.data.application_id.as_str()),
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
        state.model.device_route().list(&opts, None).await
    }) {
        Err(e) => return Err(format!("get device route model error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(route_list.len()).to_equal(expect_network_addrs.len())?;
    for i in 0..route_list.len() {
        let route = &route_list[i];
        expect(route.unit_id.as_str()).to_equal(application_info.unit_id.as_str())?;
        expect(route.unit_code.as_str()).to_equal(application_info.unit_code.as_str())?;
        expect(route.application_id.as_str()).to_equal(application_id)?;
        expect(route.application_code.as_str()).to_equal(application_info.code.as_str())?;
        expect(route.network_id.as_str()).to_equal(network_info.network_id.as_str())?;
        expect(route.network_code.as_str()).to_equal(network_info.code.as_str())?;
        expect(route.network_addr.as_str()).to_equal(expect_network_addrs[i].as_str())?;
    }

    Ok(())
}

fn test_post_bulk_invalid_param(
    runtime: &Runtime,
    state: &routes::State,
    token: &str,
    use_post: bool,
    param: Option<&request::PostDeviceRouteBulk>,
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
        false => TestRequest::post().uri("/broker/api/v1/device-route/bulk-delete"),
        true => TestRequest::post().uri("/broker/api/v1/device-route/bulk"),
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
    param: &request::PostDeviceRouteRange,
    expect_code: &str,
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
        false => TestRequest::post().uri("/broker/api/v1/device-route/range-delete"),
        true => TestRequest::post().uri("/broker/api/v1/device-route/range"),
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

    let application_id = param.data.application_id.as_str();
    let network_id = param.data.network_id.as_str();

    let application_info = get_application_model(runtime, state, application_id, true)?.unwrap();
    let network_info = get_network_model(runtime, state, network_id, true)?.unwrap();
    let route_list = match runtime.block_on(async {
        let cond = ListQueryCond {
            application_id: Some(param.data.application_id.as_str()),
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
        state.model.device_route().list(&opts, None).await
    }) {
        Err(e) => return Err(format!("get device route model error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(route_list.len()).to_equal(expect_network_addrs.len())?;
    for i in 0..route_list.len() {
        let route = &route_list[i];
        expect(route.unit_id.as_str()).to_equal(application_info.unit_id.as_str())?;
        expect(route.unit_code.as_str()).to_equal(application_info.unit_code.as_str())?;
        expect(route.application_id.as_str()).to_equal(application_id)?;
        expect(route.application_code.as_str()).to_equal(application_info.code.as_str())?;
        expect(route.network_id.as_str()).to_equal(network_info.network_id.as_str())?;
        expect(route.network_code.as_str()).to_equal(network_info.code.as_str())?;
        expect(route.network_addr.as_str()).to_equal(expect_network_addrs[i].as_str())?;
    }

    Ok(())
}

fn test_post_range_invalid_param(
    runtime: &Runtime,
    state: &routes::State,
    token: &str,
    use_post: bool,
    param: Option<&request::PostDeviceRouteRange>,
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
        false => TestRequest::post().uri("/broker/api/v1/device-route/range-delete"),
        true => TestRequest::post().uri("/broker/api/v1/device-route/range"),
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
    param: Option<&request::GetDeviceRouteCount>,
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
        None => "/broker/api/v1/device-route/count".to_string(),
        Some(param) => format!(
            "/broker/api/v1/device-route/count?{}",
            serde_urlencoded::to_string(&param).unwrap()
        ),
    };
    let req = TestRequest::get()
        .uri(uri.as_str())
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)))
        .to_request();
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    expect(resp.status()).to_equal(StatusCode::OK)?;
    let body: response::GetDeviceRouteCount =
        runtime.block_on(async { test::read_body_json(resp).await });
    expect(body.data.count).to_equal(expect_count)
}

fn test_get_list(
    runtime: &Runtime,
    state: &routes::State,
    token: &str,
    param: Option<&request::GetDeviceRouteList>,
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
        None => "/broker/api/v1/device-route/list".to_string(),
        Some(param) => format!(
            "/broker/api/v1/device-route/list?{}",
            serde_urlencoded::to_string(&param).unwrap()
        ),
    };
    let req = TestRequest::get()
        .uri(uri.as_str())
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)))
        .to_request();
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    expect(resp.status()).to_equal(StatusCode::OK)?;
    let body: response::GetDeviceRouteList =
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
    param: &mut request::GetDeviceRouteList,
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
        "/broker/api/v1/device-route/list?{}",
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
    let body: response::GetDeviceRouteList =
        runtime.block_on(async { test::read_body_json(resp).await });
    expect(body.data.len()).to_equal(expect_ids.len())?;

    let mut index = 0;
    for data in body.data.iter() {
        expect(data.route_id.as_str()).to_equal(expect_ids[index])?;
        index += 1;
    }
    Ok(())
}

fn test_get_list_offset_limit(
    runtime: &Runtime,
    state: &routes::State,
    token: &str,
    param: &request::GetDeviceRouteList,
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
        "/broker/api/v1/device-route/list?{}",
        serde_urlencoded::to_string(&param).unwrap()
    );
    let req = TestRequest::get()
        .uri(uri.as_str())
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)))
        .to_request();
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    expect(resp.status()).to_equal(StatusCode::OK)?;
    let body: response::GetDeviceRouteList =
        runtime.block_on(async { test::read_body_json(resp).await });
    expect(body.data.len()).to_equal(expect_ids.len())?;

    let mut index = 0;
    for data in body.data.iter() {
        expect(data.route_id.as_str()).to_equal(format!("route_{}", expect_ids[index]).as_str())?;
        index += 1;
    }
    Ok(())
}

fn test_get_list_format_array(
    runtime: &Runtime,
    state: &routes::State,
    token: &str,
    param: &request::GetDeviceRouteList,
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
        "/broker/api/v1/device-route/list?{}",
        serde_urlencoded::to_string(&param).unwrap()
    );
    let req = TestRequest::get()
        .uri(uri.as_str())
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)))
        .to_request();
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    expect(resp.status()).to_equal(StatusCode::OK)?;
    let body: Vec<response::GetDeviceRouteListData> =
        runtime.block_on(async { test::read_body_json(resp).await });
    expect(body.len()).to_equal(expect_ids.len())?;

    let mut index = 0;
    for data in body.iter() {
        expect(data.route_id.as_str()).to_equal(format!("route_{}", expect_ids[index]).as_str())?;
        index += 1;
    }
    Ok(())
}

fn count_list_dataset(runtime: &Runtime, state: &routes::State) -> Result<(), String> {
    add_unit_model(runtime, state, "manager", vec!["member"], "manager")?;
    add_unit_model(runtime, state, "owner1", vec![], "owner")?;
    add_unit_model(runtime, state, "owner2", vec!["member"], "owner")?;
    add_application_model(runtime, state, "manager", "manager", "amqp://host")?;
    add_application_model(runtime, state, "owner1", "owner1-1", "amqp://host")?;
    add_application_model(runtime, state, "owner1", "owner1-2", "amqp://host")?;
    add_application_model(runtime, state, "owner2", "owner2", "amqp://host")?;
    add_network_model(runtime, state, "", "public", "amqp://host")?;
    add_network_model(runtime, state, "manager", "manager", "amqp://host")?;
    add_network_model(runtime, state, "owner1", "owner1-1", "amqp://host")?;
    add_network_model(runtime, state, "owner1", "owner1-2", "amqp://host")?;
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
    add_device_model(
        runtime,
        state,
        "owner1",
        "owner1-1",
        "owner1-1-1",
        false,
        "",
    )?;
    add_device_model(runtime, state, "owner2", "owner2", "owner2-1", false, "")?;
    let now = Utc::now();

    let mut route = create_device_route(
        "manager-public-manager",
        "manager",
        "manager",
        "public",
        "manager-public",
        "",
    );
    route.created_at = now;
    route.modified_at = now + Duration::milliseconds(5);
    runtime.block_on(async {
        if let Err(e) = state.model.device_route().add(&route).await {
            return Err(format!("add device route {} error: {}", route.route_id, e));
        }
        Ok(())
    })?;

    let mut route = create_device_route(
        "manager-manager",
        "manager",
        "manager",
        "manager",
        "manager",
        "",
    );
    route.created_at = now + Duration::milliseconds(1);
    route.modified_at = now + Duration::milliseconds(4);
    runtime.block_on(async {
        if let Err(e) = state.model.device_route().add(&route).await {
            return Err(format!("add device route {} error: {}", route.route_id, e));
        }
        Ok(())
    })?;

    let mut route = create_device_route(
        "owner1-public-owner1-1",
        "owner1",
        "owner1-1",
        "public",
        "owner1-public",
        "",
    );
    route.created_at = now + Duration::milliseconds(2);
    route.modified_at = now + Duration::milliseconds(3);
    runtime.block_on(async {
        if let Err(e) = state.model.device_route().add(&route).await {
            return Err(format!("add device route {} error: {}", route.route_id, e));
        }
        Ok(())
    })?;

    let mut route = create_device_route(
        "owner1-public-owner1-2",
        "owner1",
        "owner1-2",
        "public",
        "owner1-public",
        "",
    );
    route.created_at = now + Duration::milliseconds(3);
    route.modified_at = now + Duration::milliseconds(2);
    runtime.block_on(async {
        if let Err(e) = state.model.device_route().add(&route).await {
            return Err(format!("add device route {} error: {}", route.route_id, e));
        }
        Ok(())
    })?;

    let mut route = create_device_route(
        "owner1-1-1-owner1-1",
        "owner1",
        "owner1-1",
        "owner1-1",
        "owner1-1-1",
        "",
    );
    route.created_at = now + Duration::milliseconds(4);
    route.modified_at = now + Duration::milliseconds(1);
    runtime.block_on(async {
        if let Err(e) = state.model.device_route().add(&route).await {
            return Err(format!("add device route {} error: {}", route.route_id, e));
        }
        Ok(())
    })?;

    let mut route = create_device_route(
        "owner2-1-owner2",
        "owner2",
        "owner2",
        "owner2",
        "owner2-1",
        "",
    );
    route.created_at = now + Duration::milliseconds(5);
    route.modified_at = now;
    runtime.block_on(async {
        if let Err(e) = state.model.device_route().add(&route).await {
            return Err(format!("add device route {} error: {}", route.route_id, e));
        }
        Ok(())
    })?;

    Ok(())
}
