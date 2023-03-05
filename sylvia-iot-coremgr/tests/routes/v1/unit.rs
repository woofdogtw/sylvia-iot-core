use std::collections::HashMap;

use actix_web::{
    http::{header, StatusCode},
    middleware::NormalizePath,
    test::{self, TestRequest},
    App,
};
use laboratory::SpecContext;

use sylvia_iot_broker::models::Model;
use sylvia_iot_coremgr::routes;

use super::{
    super::libs::{
        create_application, create_network, create_unit, test_invalid_param, test_invalid_token,
        test_list,
    },
    STATE, TOKEN_MANAGER,
};
use crate::TestState;

const DELETE_UNIT_ID: &'static str = "unit_delete";

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

    let req = TestRequest::post().uri("/coremgr/api/v1/unit");
    test_invalid_token(runtime, &routes_state, req)
}

pub fn get_count(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    let req = TestRequest::get().uri("/coremgr/api/v1/unit/count");
    test_invalid_token(runtime, &routes_state, req)
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

    let req = TestRequest::get().uri("/coremgr/api/v1/unit/id");
    test_invalid_token(runtime, &routes_state, req)
}

pub fn patch(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    let req = TestRequest::patch().uri("/coremgr/api/v1/unit/id");
    test_invalid_token(runtime, &routes_state, req)
}

pub fn delete(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    let mut app = runtime.block_on(async {
        test::init_service(
            App::new()
                .wrap(NormalizePath::trim())
                .service(routes::new_service(routes_state)),
        )
        .await
    });

    let req = TestRequest::delete()
        .uri(format!("/coremgr/api/v1/unit/{}", DELETE_UNIT_ID).as_str())
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", TOKEN_MANAGER)))
        .to_request();
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    if resp.status() != StatusCode::NO_CONTENT {
        let status = resp.status();
        let body = runtime.block_on(async { test::read_body(resp).await });
        let body = match String::from_utf8(body.to_vec()) {
            Err(e) => format!("(no body with error: {})", e),
            Ok(body) => body,
        };
        return Err(format!("API not 204, status: {}, body: {}", status, body));
    }
    Ok(())
}

pub fn delete_invalid(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    let req = TestRequest::delete().uri("/coremgr/api/v1/unit/test");
    test_invalid_param(runtime, routes_state, req, "err_param")
}
