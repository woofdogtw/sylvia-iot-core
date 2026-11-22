use axum::http::{HeaderValue, Method, StatusCode, header};
use laboratory::{SpecContext, expect};
use serde_json::Value;

use sylvia_iot_coremgr::libs::config::MqHost;

use super::{
    super::libs::{TOKEN_MEMBER, new_test_server, test_invalid_token},
    STATE,
};
use crate::TestState;

pub fn get_hosts(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    test_invalid_token(
        runtime,
        &routes_state,
        Method::GET,
        "/coremgr/api/v1/mq/hosts",
    )?;

    // No Authorization header.
    let server = new_test_server(routes_state)?;
    let req = server.get("/coremgr/api/v1/mq/hosts");
    let resp = runtime.block_on(async { req.await });
    expect(resp.status_code()).to_equal(StatusCode::UNAUTHORIZED)?;

    // No hosts.
    let req = server.get("/coremgr/api/v1/mq/hosts").add_header(
        header::AUTHORIZATION,
        HeaderValue::from_str(format!("Bearer {}", TOKEN_MEMBER).as_str()).unwrap(),
    );
    let resp = runtime.block_on(async { req.await });
    expect(resp.status_code()).to_equal(StatusCode::OK)?;
    let body: Value = match serde_json::from_str(resp.text().as_str()) {
        Err(e) => return Err(format!("unexpected response: {}", e)),
        Ok(body) => body,
    };
    expect(body["data"].as_array().map(|v| v.len())).to_equal(Some(0))?;

    // Any user can get hosts, and internal hosts are not exposed.
    let mut hosts_state = routes_state.clone();
    hosts_state.mq_hosts.amqp = vec![MqHost {
        name: "amqp".to_string(),
        host: "amqp.internal".to_string(),
        external: "amqp.example.com".to_string(),
        active: true,
    }];
    hosts_state.mq_hosts.mqtt = vec![MqHost {
        name: "mqtt".to_string(),
        host: "mqtt.internal".to_string(),
        external: "mqtt.example.com".to_string(),
        active: false,
    }];
    let server = new_test_server(&hosts_state)?;
    let req = server.get("/coremgr/api/v1/mq/hosts").add_header(
        header::AUTHORIZATION,
        HeaderValue::from_str(format!("Bearer {}", TOKEN_MEMBER).as_str()).unwrap(),
    );
    let resp = runtime.block_on(async { req.await });
    expect(resp.status_code()).to_equal(StatusCode::OK)?;
    let body: Value = match serde_json::from_str(resp.text().as_str()) {
        Err(e) => return Err(format!("unexpected response: {}", e)),
        Ok(body) => body,
    };
    let expect_data = serde_json::json!([
        {
            "type": "amqp",
            "name": "amqp",
            "external": "amqp.example.com",
            "active": true
        },
        {
            "type": "mqtt",
            "name": "mqtt",
            "external": "mqtt.example.com",
            "active": false
        }
    ]);
    expect(&body["data"]).to_equal(&expect_data)
}
