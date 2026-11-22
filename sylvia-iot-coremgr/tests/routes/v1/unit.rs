use std::collections::HashMap;

use axum::http::{HeaderValue, Method, StatusCode, header};
use laboratory::{SpecContext, expect};
use serde_json::{Value, json};

use sylvia_iot_broker::models::Model;

use super::{
    super::libs::{
        TOKEN_MEMBER, TOKEN_OWNER, create_application, create_network, create_unit,
        test_invalid_param, test_invalid_token, test_list,
    },
    STATE, TOKEN_MANAGER, application,
};
use crate::{TestState, routes::libs::new_test_server};

const DELETE_UNIT_ID: &'static str = "unit_delete";
const NOT_OWNER_UNIT_ID: &'static str = "unit_not_owner";

pub fn create_delete_rsc(state: &mut HashMap<&'static str, TestState>) -> () {
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let broker_db = state.broker_db.as_ref().unwrap();

    let user_id = "manager";
    let unit_id = DELETE_UNIT_ID;

    let item = create_unit(unit_id, user_id);
    if let Err(e) = runtime.block_on(async { broker_db.unit().add(&item).await }) {
        panic!("create delete unit error: {}", e);
    }

    for i in 100..150 {
        let name = format!("delete{}", i);
        let name = name.as_str();
        let item = create_application(name, format!("amqp://localhost/{}", name).as_str(), unit_id);
        if let Err(e) = runtime.block_on(async { broker_db.application().add(&item).await }) {
            panic!("create delete application error: {}", e);
        }
        let item = create_network(name, format!("amqp://localhost/{}", name).as_str(), unit_id);
        if let Err(e) = runtime.block_on(async { broker_db.network().add(&item).await }) {
            panic!("create delete network error: {}", e);
        }
    }
    for i in 150..201 {
        let name = format!("delete{}", i);
        let name = name.as_str();
        let item = create_application(name, "mqtt://localhost", unit_id);
        if let Err(e) = runtime.block_on(async { broker_db.application().add(&item).await }) {
            panic!("create delete application error: {}", e);
        }
        let item = create_network(name, "mqtt://localhost", unit_id);
        if let Err(e) = runtime.block_on(async { broker_db.network().add(&item).await }) {
            panic!("create delete network error: {}", e);
        }
    }
}

pub fn post(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    test_invalid_token(runtime, &routes_state, Method::POST, "/coremgr/api/v1/unit")
}

pub fn get_count(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    test_invalid_token(
        runtime,
        &routes_state,
        Method::GET,
        "/coremgr/api/v1/unit/count",
    )
}

pub fn get_list(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    test_list(
        runtime,
        routes_state,
        "/coremgr/api/v1/unit/list",
        TOKEN_MANAGER,
        "unitId,code,createdAt,modifiedAt,ownerId,memberIds,name,info",
    )
}

pub fn get(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    test_invalid_token(
        runtime,
        &routes_state,
        Method::GET,
        "/coremgr/api/v1/unit/id",
    )
}

pub fn patch(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    test_invalid_token(
        runtime,
        &routes_state,
        Method::PATCH,
        "/coremgr/api/v1/unit/id",
    )
}

pub fn delete(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    let server = new_test_server(routes_state)?;

    let req = server
        .delete(format!("/coremgr/api/v1/unit/{}", DELETE_UNIT_ID).as_str())
        .add_header(
            header::AUTHORIZATION,
            HeaderValue::from_str(format!("Bearer {}", TOKEN_MANAGER).as_str()).unwrap(),
        );
    let resp = runtime.block_on(async { req.await });
    let status = resp.status_code();
    if status != StatusCode::NO_CONTENT {
        return Err(format!(
            "API not 204, status: {}, body: {}",
            status,
            resp.text()
        ));
    }
    Ok(())
}

pub fn delete_invalid(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    let server = new_test_server(routes_state)?;
    let req = server.delete("/coremgr/api/v1/unit/test");
    test_invalid_param(runtime, req, "err_param")
}

/// Non-owners cannot clear message broker resources with the DELETE API.
pub fn delete_not_owner(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();
    let broker_db = state.broker_db.as_ref().unwrap();
    let host_uri = format!("amqp://{}", crate::TEST_MQ_HOST);
    let unit_id = NOT_OWNER_UNIT_ID;
    let app_code = "not_owner";

    let mut unit = create_unit(unit_id, "owner");
    unit.member_ids.push("member".to_string());
    if let Err(e) = runtime.block_on(async { broker_db.unit().add(&unit).await }) {
        return Err(format!("add unit model info error: {}", e));
    }
    let server = new_test_server(routes_state)?;
    let req = server
        .post("/coremgr/api/v1/application")
        .add_header(
            header::AUTHORIZATION,
            HeaderValue::from_str(format!("Bearer {}", TOKEN_OWNER).as_str()).unwrap(),
        )
        .json(&json!({"data": {"code": app_code, "unitId": unit_id, "hostUri": host_uri}}));
    let resp = runtime.block_on(async { req.await });
    expect(resp.status_code()).to_equal(StatusCode::OK)?;
    let body: Value = match serde_json::from_str(resp.text().as_str()) {
        Err(e) => return Err(format!("unexpected response: {}", e)),
        Ok(body) => body,
    };
    let password = match body["data"]["password"].as_str() {
        None => return Err(format!("no password: {}", body)),
        Some(password) => password.to_string(),
    };

    // The broker does nothing for non-owners, and the resources must be kept.
    let uri = format!("/coremgr/api/v1/unit/{}", unit_id);
    let req = server.delete(uri.as_str()).add_header(
        header::AUTHORIZATION,
        HeaderValue::from_str(format!("Bearer {}", TOKEN_MEMBER).as_str()).unwrap(),
    );
    let resp = runtime.block_on(async { req.await });
    expect(resp.status_code()).to_equal(StatusCode::NO_CONTENT)?;
    runtime.block_on(async {
        application::check_queue(host_uri.as_str(), password.as_str(), unit_id, app_code).await
    })?;

    let req = server.delete(uri.as_str()).add_header(
        header::AUTHORIZATION,
        HeaderValue::from_str(format!("Bearer {}", TOKEN_OWNER).as_str()).unwrap(),
    );
    let resp = runtime.block_on(async { req.await });
    expect(resp.status_code()).to_equal(StatusCode::NO_CONTENT)
}
