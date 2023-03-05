use std::cmp::Ordering;

use actix_web::{
    http::{header, StatusCode},
    middleware::NormalizePath,
    test::{self, TestRequest},
    App,
};
use chrono::{DateTime, Duration, SubsecRound, TimeZone, Utc};
use laboratory::{expect, SpecContext};
use serde_json::{Map, Value};
use serde_urlencoded;
use tokio::runtime::Runtime;

use sylvia_iot_broker::routes;
use sylvia_iot_corelib::err;

use super::{
    super::{
        super::libs::{
            add_application_model, add_network_model, add_network_route_model, add_unit_model,
            create_network_route, get_application_model, get_network_model,
            get_network_route_model, test_get_400, test_invalid_token, ApiError,
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
    add_network_model(runtime, routes_state, "owner", "owner2", "amqp://host")?;

    let param = request::PostNetworkRoute {
        data: request::PostNetworkRouteData {
            network_id: "public".to_string(),
            application_id: "manager".to_string(),
        },
    };
    test_post(runtime, routes_state, TOKEN_MANAGER, &param, "")?;

    let param = request::PostNetworkRoute {
        data: request::PostNetworkRouteData {
            network_id: "manager".to_string(),
            application_id: "manager".to_string(),
        },
    };
    test_post(runtime, routes_state, TOKEN_MANAGER, &param, "")?;

    let param = request::PostNetworkRoute {
        data: request::PostNetworkRouteData {
            network_id: "owner".to_string(),
            application_id: "owner".to_string(),
        },
    };
    test_post(runtime, routes_state, TOKEN_OWNER, &param, "")?;

    let param = request::PostNetworkRoute {
        data: request::PostNetworkRouteData {
            network_id: "owner2".to_string(),
            application_id: "owner".to_string(),
        },
    };
    test_post(runtime, routes_state, TOKEN_MANAGER, &param, "")
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

    let param = request::PostNetworkRoute {
        data: request::PostNetworkRouteData {
            network_id: "public".to_string(),
            application_id: "manager".to_string(),
        },
    };
    test_post(runtime, routes_state, TOKEN_MANAGER, &param, "")?;

    let param = request::PostNetworkRoute {
        data: request::PostNetworkRouteData {
            network_id: "public".to_string(),
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

    let param = request::PostNetworkRoute {
        data: request::PostNetworkRouteData {
            network_id: "manager".to_string(),
            application_id: "manager".to_string(),
        },
    };
    test_post(runtime, routes_state, TOKEN_MANAGER, &param, "")?;

    let param = request::PostNetworkRoute {
        data: request::PostNetworkRouteData {
            network_id: "manager".to_string(),
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

    let param = request::PostNetworkRoute {
        data: request::PostNetworkRouteData {
            network_id: "id".to_string(),
            application_id: "manager".to_string(),
        },
    };
    test_post(
        runtime,
        routes_state,
        TOKEN_MANAGER,
        &param,
        "err_broker_network_not_exist",
    )?;

    let param = request::PostNetworkRoute {
        data: request::PostNetworkRouteData {
            network_id: "manager".to_string(),
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

    let mut param = request::PostNetworkRoute {
        data: request::PostNetworkRouteData {
            network_id: "public".to_string(),
            application_id: "owner".to_string(),
        },
    };
    test_post(
        runtime,
        routes_state,
        TOKEN_OWNER,
        &param,
        "err_broker_network_not_exist",
    )?;
    param.data.network_id = "manager".to_string();
    test_post(
        runtime,
        routes_state,
        TOKEN_OWNER,
        &param,
        "err_broker_network_not_exist",
    )?;

    let param = request::PostNetworkRoute {
        data: request::PostNetworkRouteData {
            network_id: "owner".to_string(),
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
    add_unit_model(runtime, routes_state, "owner2", vec![], "owner")?;
    add_application_model(runtime, routes_state, "manager", "manager", "amqp://host")?;
    add_application_model(runtime, routes_state, "owner", "owner", "amqp://host")?;
    add_network_model(runtime, routes_state, "", "public", "amqp://host")?;
    add_network_model(runtime, routes_state, "manager", "manager", "amqp://host")?;
    add_network_model(runtime, routes_state, "owner", "owner", "amqp://host")?;
    add_network_model(runtime, routes_state, "owner", "owner2", "amqp://host")?;

    let param = request::PostNetworkRoute {
        data: request::PostNetworkRouteData {
            network_id: "manager".to_string(),
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
    let param = request::PostNetworkRoute {
        data: request::PostNetworkRouteData {
            network_id: "owner".to_string(),
            application_id: "manager".to_string(),
        },
    };
    test_post(
        runtime,
        routes_state,
        TOKEN_MANAGER,
        &param,
        "err_broker_unit_not_match",
    )?;

    let param = request::PostNetworkRoute {
        data: request::PostNetworkRouteData {
            network_id: "owner2".to_string(),
            application_id: "owner".to_string(),
        },
    };
    test_post(
        runtime,
        routes_state,
        TOKEN_OWNER,
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

    let param = request::PostNetworkRoute {
        data: request::PostNetworkRouteData {
            network_id: "".to_string(),
            application_id: "id".to_string(),
        },
    };
    test_post_invalid_param(runtime, &routes_state, TOKEN_MANAGER, Some(&param))?;

    let param = request::PostNetworkRoute {
        data: request::PostNetworkRouteData {
            network_id: "id".to_string(),
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

    let req = TestRequest::post().uri("/broker/api/v1/network-route");
    test_invalid_token(runtime, &routes_state, req)
}

pub fn get_count(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    count_list_dataset(runtime, &routes_state)?;

    test_get_count(runtime, &routes_state, TOKEN_MANAGER, None, 6)?;

    let param = request::GetNetworkRouteCount {
        unit: Some("".to_string()),
        ..Default::default()
    };
    test_get_count(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 6)?;

    let param = request::GetNetworkRouteCount {
        unit: Some("".to_string()),
        application: Some("".to_string()),
        network: Some("".to_string()),
        ..Default::default()
    };
    test_get_count(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 6)?;

    let param = request::GetNetworkRouteCount {
        unit: Some("manager".to_string()),
        ..Default::default()
    };
    test_get_count(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 2)?;
    test_get_count(runtime, &routes_state, TOKEN_MEMBER, Some(&param), 2)?;

    let param = request::GetNetworkRouteCount {
        unit: Some("owner1".to_string()),
        ..Default::default()
    };
    test_get_count(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 3)?;
    test_get_count(runtime, &routes_state, TOKEN_OWNER, Some(&param), 3)?;

    let param = request::GetNetworkRouteCount {
        unit: Some("owner2".to_string()),
        ..Default::default()
    };
    test_get_count(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 1)?;
    test_get_count(runtime, &routes_state, TOKEN_OWNER, Some(&param), 1)?;
    test_get_count(runtime, &routes_state, TOKEN_MEMBER, Some(&param), 1)?;

    let mut param = request::GetNetworkRouteCount {
        application: Some("manager".to_string()),
        ..Default::default()
    };
    test_get_count(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 2)?;
    param.unit = Some("manager".to_string());
    test_get_count(runtime, &routes_state, TOKEN_MEMBER, Some(&param), 2)?;

    let mut param = request::GetNetworkRouteCount {
        application: Some("owner1-1".to_string()),
        ..Default::default()
    };
    test_get_count(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 2)?;
    param.unit = Some("owner1".to_string());
    test_get_count(runtime, &routes_state, TOKEN_OWNER, Some(&param), 2)?;

    let mut param = request::GetNetworkRouteCount {
        application: Some("owner1-2".to_string()),
        ..Default::default()
    };
    test_get_count(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 1)?;
    param.unit = Some("owner2".to_string());
    test_get_count(runtime, &routes_state, TOKEN_OWNER, Some(&param), 0)?;
    test_get_count(runtime, &routes_state, TOKEN_MEMBER, Some(&param), 0)?;

    let param = request::GetNetworkRouteCount {
        unit: Some("owner2".to_string()),
        application: Some("owner2".to_string()),
        ..Default::default()
    };
    test_get_count(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 1)?;
    test_get_count(runtime, &routes_state, TOKEN_OWNER, Some(&param), 1)?;
    test_get_count(runtime, &routes_state, TOKEN_MEMBER, Some(&param), 1)?;

    let mut param = request::GetNetworkRouteCount {
        network: Some("public".to_string()),
        ..Default::default()
    };
    test_get_count(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 1)?;
    param.unit = Some("manager".to_string());
    test_get_count(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 1)?;
    test_get_count(runtime, &routes_state, TOKEN_MEMBER, Some(&param), 1)?;
    param.unit = Some("owner1".to_string());
    test_get_count(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 0)?;
    test_get_count(runtime, &routes_state, TOKEN_OWNER, Some(&param), 0)?;

    let mut param = request::GetNetworkRouteCount {
        network: Some("manager".to_string()),
        ..Default::default()
    };
    test_get_count(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 1)?;
    param.unit = Some("manager".to_string());
    test_get_count(runtime, &routes_state, TOKEN_MEMBER, Some(&param), 1)?;

    let mut param = request::GetNetworkRouteCount {
        network: Some("owner1-1".to_string()),
        ..Default::default()
    };
    test_get_count(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 1)?;
    param.unit = Some("owner1".to_string());
    test_get_count(runtime, &routes_state, TOKEN_OWNER, Some(&param), 1)?;
    param.unit = Some("owner2".to_string());
    test_get_count(runtime, &routes_state, TOKEN_OWNER, Some(&param), 0)?;

    let param = request::GetNetworkRouteCount {
        unit: Some("manager".to_string()),
        application: Some("manager".to_string()),
        network: Some("manager".to_string()),
        ..Default::default()
    };
    test_get_count(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 1)?;
    test_get_count(runtime, &routes_state, TOKEN_MEMBER, Some(&param), 1)?;

    let param = request::GetNetworkRouteCount {
        unit: Some("owner1".to_string()),
        application: Some("owner1-1".to_string()),
        network: Some("owner1-2".to_string()),
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

    let uri = "/broker/api/v1/network-route/count";
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

    let uri = "/broker/api/v1/network-route/count";
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

    let req = TestRequest::get().uri("/broker/api/v1/network-route/count");
    test_invalid_token(runtime, &routes_state, req)
}

pub fn get_list(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    count_list_dataset(runtime, &routes_state)?;

    test_get_list(runtime, &routes_state, TOKEN_MANAGER, None, 6)?;

    let param = request::GetNetworkRouteList {
        unit: Some("".to_string()),
        ..Default::default()
    };
    test_get_list(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 6)?;

    let param = request::GetNetworkRouteList {
        unit: Some("".to_string()),
        application: Some("".to_string()),
        network: Some("".to_string()),
        ..Default::default()
    };
    test_get_list(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 6)?;

    let param = request::GetNetworkRouteList {
        unit: Some("manager".to_string()),
        ..Default::default()
    };
    test_get_list(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 2)?;
    test_get_list(runtime, &routes_state, TOKEN_MEMBER, Some(&param), 2)?;

    let param = request::GetNetworkRouteList {
        unit: Some("owner1".to_string()),
        ..Default::default()
    };
    test_get_list(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 3)?;
    test_get_list(runtime, &routes_state, TOKEN_OWNER, Some(&param), 3)?;

    let param = request::GetNetworkRouteList {
        unit: Some("owner2".to_string()),
        ..Default::default()
    };
    test_get_list(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 1)?;
    test_get_list(runtime, &routes_state, TOKEN_OWNER, Some(&param), 1)?;
    test_get_list(runtime, &routes_state, TOKEN_MEMBER, Some(&param), 1)?;

    let mut param = request::GetNetworkRouteList {
        application: Some("manager".to_string()),
        ..Default::default()
    };
    test_get_list(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 2)?;
    param.unit = Some("manager".to_string());
    test_get_list(runtime, &routes_state, TOKEN_MEMBER, Some(&param), 2)?;

    let mut param = request::GetNetworkRouteList {
        application: Some("owner1-1".to_string()),
        ..Default::default()
    };
    test_get_list(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 2)?;
    param.unit = Some("owner1".to_string());
    test_get_list(runtime, &routes_state, TOKEN_OWNER, Some(&param), 2)?;

    let mut param = request::GetNetworkRouteList {
        application: Some("owner1-2".to_string()),
        ..Default::default()
    };
    test_get_list(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 1)?;
    param.unit = Some("owner2".to_string());
    test_get_list(runtime, &routes_state, TOKEN_OWNER, Some(&param), 0)?;
    test_get_list(runtime, &routes_state, TOKEN_MEMBER, Some(&param), 0)?;

    let param = request::GetNetworkRouteList {
        unit: Some("owner2".to_string()),
        application: Some("owner2".to_string()),
        ..Default::default()
    };
    test_get_list(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 1)?;
    test_get_list(runtime, &routes_state, TOKEN_OWNER, Some(&param), 1)?;
    test_get_list(runtime, &routes_state, TOKEN_MEMBER, Some(&param), 1)?;

    let mut param = request::GetNetworkRouteList {
        network: Some("public".to_string()),
        ..Default::default()
    };
    test_get_list(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 1)?;
    param.unit = Some("manager".to_string());
    test_get_list(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 1)?;
    test_get_list(runtime, &routes_state, TOKEN_MEMBER, Some(&param), 1)?;
    param.unit = Some("owner1".to_string());
    test_get_list(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 0)?;
    test_get_list(runtime, &routes_state, TOKEN_OWNER, Some(&param), 0)?;

    let mut param = request::GetNetworkRouteList {
        network: Some("manager".to_string()),
        ..Default::default()
    };
    test_get_list(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 1)?;
    param.unit = Some("manager".to_string());
    test_get_list(runtime, &routes_state, TOKEN_MEMBER, Some(&param), 1)?;

    let mut param = request::GetNetworkRouteList {
        network: Some("owner1-1".to_string()),
        ..Default::default()
    };
    test_get_list(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 1)?;
    param.unit = Some("owner1".to_string());
    test_get_list(runtime, &routes_state, TOKEN_OWNER, Some(&param), 1)?;
    param.unit = Some("owner2".to_string());
    test_get_list(runtime, &routes_state, TOKEN_OWNER, Some(&param), 0)?;

    let param = request::GetNetworkRouteList {
        unit: Some("manager".to_string()),
        application: Some("manager".to_string()),
        network: Some("manager".to_string()),
        ..Default::default()
    };
    test_get_list(runtime, &routes_state, TOKEN_MANAGER, Some(&param), 1)?;
    test_get_list(runtime, &routes_state, TOKEN_MEMBER, Some(&param), 1)?;

    let param = request::GetNetworkRouteList {
        unit: Some("owner1".to_string()),
        application: Some("owner1-1".to_string()),
        network: Some("owner1-2".to_string()),
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

    let mut param = request::GetNetworkRouteList {
        ..Default::default()
    };
    test_get_list_sort(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        &mut param,
        &[
            "manager-manager",
            "owner1-1-owner1-1",
            "owner1-2-owner1-2",
            "owner1-2-owner1-1",
            "owner2-owner2",
            "public-manager",
        ],
    )?;

    param.sort_vec = Some(vec![("network", true), ("created", false)]);
    test_get_list_sort(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        &mut param,
        &[
            "manager-manager",
            "owner1-1-owner1-1",
            "owner1-2-owner1-2",
            "owner1-2-owner1-1",
            "owner2-owner2",
            "public-manager",
        ],
    )?;
    param.sort_vec = Some(vec![("network", true), ("created", true)]);
    test_get_list_sort(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        &mut param,
        &[
            "manager-manager",
            "owner1-1-owner1-1",
            "owner1-2-owner1-1",
            "owner1-2-owner1-2",
            "owner2-owner2",
            "public-manager",
        ],
    )?;
    param.sort_vec = Some(vec![("network", false), ("created", false)]);
    test_get_list_sort(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        &mut param,
        &[
            "public-manager",
            "owner2-owner2",
            "owner1-2-owner1-2",
            "owner1-2-owner1-1",
            "owner1-1-owner1-1",
            "manager-manager",
        ],
    )?;
    param.sort_vec = Some(vec![("network", false), ("created", true)]);
    test_get_list_sort(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        &mut param,
        &[
            "public-manager",
            "owner2-owner2",
            "owner1-2-owner1-1",
            "owner1-2-owner1-2",
            "owner1-1-owner1-1",
            "manager-manager",
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
            "public-manager",
            "owner1-2-owner1-1",
            "owner1-1-owner1-1",
            "owner1-2-owner1-2",
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
            "owner1-2-owner1-2",
            "owner1-2-owner1-1",
            "owner1-1-owner1-1",
            "manager-manager",
            "public-manager",
        ],
    )?;

    param.sort_vec = Some(vec![("created", true)]);
    test_get_list_sort(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        &mut param,
        &[
            "public-manager",
            "manager-manager",
            "owner1-1-owner1-1",
            "owner1-2-owner1-1",
            "owner1-2-owner1-2",
            "owner2-owner2",
        ],
    )?;
    param.sort_vec = Some(vec![("created", false)]);
    test_get_list_sort(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        &mut param,
        &[
            "owner2-owner2",
            "owner1-2-owner1-2",
            "owner1-2-owner1-1",
            "owner1-1-owner1-1",
            "manager-manager",
            "public-manager",
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

    for i in 100..302 {
        let network = format!("network_{}", i);
        add_network_model(
            runtime,
            &routes_state,
            "manager",
            network.as_str(),
            "amqp://host",
        )?;
        add_network_route_model(
            runtime,
            &routes_state,
            format!("route_{}", i).as_str(),
            "manager",
            "manager",
            network.as_str(),
        )?;
    }

    let param = request::GetNetworkRouteList {
        ..Default::default()
    };
    test_get_list_offset_limit(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        &param,
        (100..200).collect(),
    )?;

    let mut param = request::GetNetworkRouteList {
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

    for i in 100..302 {
        let network = format!("network_{}", i);
        add_network_model(
            runtime,
            &routes_state,
            "manager",
            network.as_str(),
            "amqp://host",
        )?;
        add_network_route_model(
            runtime,
            &routes_state,
            format!("route_{}", i).as_str(),
            "manager",
            "manager",
            network.as_str(),
        )?;
    }

    let mut param = request::GetNetworkRouteList {
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

    let uri = "/broker/api/v1/network-route/list";
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

    let uri = "/broker/api/v1/network-route/list";
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

    let req = TestRequest::get().uri("/broker/api/v1/network-route/list");
    test_invalid_token(runtime, &routes_state, req)
}

pub fn delete(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    add_unit_model(runtime, routes_state, "manager", vec![], "manager")?;
    add_application_model(runtime, routes_state, "manager", "manager", "amqp://host")?;
    add_network_model(runtime, routes_state, "manager", "manager1", "amqp://host")?;
    add_network_model(runtime, routes_state, "manager", "manager2", "amqp://host")?;
    add_network_route_model(
        runtime,
        &routes_state,
        "route1",
        "manager",
        "manager",
        "manager1",
    )?;
    add_network_route_model(
        runtime,
        &routes_state,
        "route2",
        "manager",
        "manager",
        "manager2",
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
        .uri("/broker/api/v1/network-route/id")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", TOKEN_MANAGER)))
        .to_request();
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    expect(resp.status()).to_equal(StatusCode::NO_CONTENT)?;
    let _ = get_network_route_model(runtime, &routes_state, "route1", true)?;
    let _ = get_network_route_model(runtime, &routes_state, "route2", true)?;

    let req = TestRequest::delete()
        .uri("/broker/api/v1/network-route/route1")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", TOKEN_OWNER)))
        .to_request();
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    expect(resp.status()).to_equal(StatusCode::NO_CONTENT)?;
    let _ = get_network_route_model(runtime, &routes_state, "route1", true)?;
    let _ = get_network_route_model(runtime, &routes_state, "route2", true)?;

    let req = TestRequest::delete()
        .uri("/broker/api/v1/network-route/route1")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", TOKEN_MANAGER)))
        .to_request();
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    expect(resp.status()).to_equal(StatusCode::NO_CONTENT)?;
    let _ = get_network_route_model(runtime, &routes_state, "route1", false)?;
    let _ = get_network_route_model(runtime, &routes_state, "route2", true)?;

    Ok(())
}

pub fn delete_invalid_token(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    let req = TestRequest::delete().uri("/broker/api/v1/network-route/id");
    test_invalid_token(runtime, &routes_state, req)
}

fn test_post(
    runtime: &Runtime,
    state: &routes::State,
    token: &str,
    param: &request::PostNetworkRoute,
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
        .uri("/broker/api/v1/network-route")
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
    let body: response::PostNetworkRoute =
        runtime.block_on(async { test::read_body_json(resp).await });
    expect(body.data.route_id.len() > 0).to_equal(true)?;

    let network_id = param.data.network_id.as_str();
    let application_id = param.data.application_id.as_str();

    let network_info = get_network_model(runtime, state, network_id, true)?.unwrap();
    let application_info = get_application_model(runtime, state, application_id, true)?.unwrap();
    let route_info = match runtime.block_on(async {
        state
            .model
            .network_route()
            .get(body.data.route_id.as_str())
            .await
    }) {
        Err(e) => return Err(format!("get network route model error: {}", e)),
        Ok(info) => match info {
            None => return Err("add network route then get none network route".to_string()),
            Some(info) => info,
        },
    };
    expect(route_info.unit_id.as_str()).to_equal(application_info.unit_id.as_str())?;
    expect(route_info.application_id.as_str()).to_equal(application_id)?;
    expect(route_info.application_code.as_str()).to_equal(application_info.code.as_str())?;
    expect(route_info.network_id.as_str()).to_equal(network_id)?;
    expect(route_info.network_code.as_str()).to_equal(network_info.code.as_str())?;
    expect(route_info.created_at.ge(&time_before)).to_equal(true)?;
    expect(route_info.created_at.le(&time_after)).to_equal(true)
}

fn test_post_invalid_param(
    runtime: &Runtime,
    state: &routes::State,
    token: &str,
    param: Option<&request::PostNetworkRoute>,
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
        .uri("/broker/api/v1/network-route")
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
    param: Option<&request::GetNetworkRouteCount>,
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
        None => "/broker/api/v1/network-route/count".to_string(),
        Some(param) => format!(
            "/broker/api/v1/network-route/count?{}",
            serde_urlencoded::to_string(&param).unwrap()
        ),
    };
    let req = TestRequest::get()
        .uri(uri.as_str())
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)))
        .to_request();
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    expect(resp.status()).to_equal(StatusCode::OK)?;
    let body: response::GetNetworkRouteCount =
        runtime.block_on(async { test::read_body_json(resp).await });
    expect(body.data.count).to_equal(expect_count)
}

fn test_get_list(
    runtime: &Runtime,
    state: &routes::State,
    token: &str,
    param: Option<&request::GetNetworkRouteList>,
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
        None => "/broker/api/v1/network-route/list".to_string(),
        Some(param) => format!(
            "/broker/api/v1/network-route/list?{}",
            serde_urlencoded::to_string(&param).unwrap()
        ),
    };
    let req = TestRequest::get()
        .uri(uri.as_str())
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)))
        .to_request();
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    expect(resp.status()).to_equal(StatusCode::OK)?;
    let body: response::GetNetworkRouteList =
        runtime.block_on(async { test::read_body_json(resp).await });
    expect(body.data.len()).to_equal(expect_count)?;

    let now = Utc::now();
    let mut code_min = "";
    let mut created_at = now;
    for info in body.data.iter() {
        if let Err(_) = expect(info.network_code.as_str().ge(code_min)).to_equal(true) {
            return Err(format!(
                "code order error: {} - {}/{}",
                code_min,
                info.network_code.as_str(),
                info.application_code.as_str(),
            ));
        }
        let info_created_at = match DateTime::parse_from_rfc3339(info.created_at.as_str()) {
            Err(e) => return Err(format!("created_at not RFC3339: {}", e)),
            Ok(dt) => Utc.timestamp_nanos(dt.timestamp_nanos()),
        };
        if let Err(_) = expect(info_created_at.le(&created_at)).to_equal(true) {
            return Err(format!(
                "created_at order error: {} - {}/{}",
                code_min,
                info.network_code.as_str(),
                info.application_code.as_str()
            ));
        }
        if code_min.cmp(info.network_code.as_str()) != Ordering::Equal {
            created_at = now;
        }
        code_min = info.network_code.as_str();
    }
    Ok(())
}

fn test_get_list_sort(
    runtime: &Runtime,
    state: &routes::State,
    token: &str,
    param: &mut request::GetNetworkRouteList,
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
        "/broker/api/v1/network-route/list?{}",
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
    let body: response::GetNetworkRouteList =
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
    param: &request::GetNetworkRouteList,
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
        "/broker/api/v1/network-route/list?{}",
        serde_urlencoded::to_string(&param).unwrap()
    );
    let req = TestRequest::get()
        .uri(uri.as_str())
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)))
        .to_request();
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    expect(resp.status()).to_equal(StatusCode::OK)?;
    let body: response::GetNetworkRouteList =
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
    param: &request::GetNetworkRouteList,
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
        "/broker/api/v1/network-route/list?{}",
        serde_urlencoded::to_string(&param).unwrap()
    );
    let req = TestRequest::get()
        .uri(uri.as_str())
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)))
        .to_request();
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    expect(resp.status()).to_equal(StatusCode::OK)?;
    let body: Vec<response::GetNetworkRouteListData> =
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
    let now = Utc::now();

    let mut route = create_network_route("public-manager", "manager", "manager", "public");
    route.created_at = now;
    runtime.block_on(async {
        if let Err(e) = state.model.network_route().add(&route).await {
            return Err(format!("add network route {} error: {}", route.route_id, e));
        }
        Ok(())
    })?;

    let mut route = create_network_route("manager-manager", "manager", "manager", "manager");
    route.created_at = now + Duration::milliseconds(1);
    runtime.block_on(async {
        if let Err(e) = state.model.network_route().add(&route).await {
            return Err(format!("add network route {} error: {}", route.route_id, e));
        }
        Ok(())
    })?;

    let mut route = create_network_route("owner1-1-owner1-1", "owner1", "owner1-1", "owner1-1");
    route.created_at = now + Duration::milliseconds(2);
    runtime.block_on(async {
        if let Err(e) = state.model.network_route().add(&route).await {
            return Err(format!("add network route {} error: {}", route.route_id, e));
        }
        Ok(())
    })?;

    let mut route = create_network_route("owner1-2-owner1-1", "owner1", "owner1-1", "owner1-2");
    route.created_at = now + Duration::milliseconds(3);
    runtime.block_on(async {
        if let Err(e) = state.model.network_route().add(&route).await {
            return Err(format!("add network route {} error: {}", route.route_id, e));
        }
        Ok(())
    })?;

    let mut route = create_network_route("owner1-2-owner1-2", "owner1", "owner1-2", "owner1-2");
    route.created_at = now + Duration::milliseconds(4);
    runtime.block_on(async {
        if let Err(e) = state.model.network_route().add(&route).await {
            return Err(format!("add network route {} error: {}", route.route_id, e));
        }
        Ok(())
    })?;

    let mut route = create_network_route("owner2-owner2", "owner2", "owner2", "owner2");
    route.created_at = now + Duration::milliseconds(5);
    runtime.block_on(async {
        if let Err(e) = state.model.network_route().add(&route).await {
            return Err(format!("add network route {} error: {}", route.route_id, e));
        }
        Ok(())
    })
}
