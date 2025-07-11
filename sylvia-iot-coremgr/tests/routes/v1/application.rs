use std::{collections::HashMap, sync::Arc, time::Duration};

use async_trait::async_trait;
use axum::http::{HeaderValue, Method, StatusCode, header};
use base64::{Engine, engine::general_purpose};
use chrono::{DateTime, SubsecRound, Utc};
use hex;
use laboratory::{SpecContext, expect};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use tokio::{runtime::Runtime, time};

use general_mq::{
    AmqpConnection, AmqpConnectionOptions, AmqpQueue, AmqpQueueOptions, MqttConnection,
    MqttConnectionOptions, MqttQueue, MqttQueueOptions,
    connection::{GmqConnection, Status as ConnStatus},
    queue::{GmqQueue, Message, MessageHandler, Status as QueueStatus},
};
use sylvia_iot_broker::models::{Model, device, network};
use sylvia_iot_corelib::{constants::ContentType, err};
use sylvia_iot_coremgr::{
    libs::mq::{self, QueueType, emqx, rabbitmq, to_username},
    routes,
};

use crate::{WAIT_COUNT, WAIT_TICK, routes::libs::new_test_server};

use super::{
    super::{
        TestState,
        libs::{
            ApiError, TOKEN_MANAGER, TOKEN_MEMBER, create_device, create_network, create_unit,
            test_invalid_param, test_invalid_token, test_list,
        },
    },
    STATE, Stats, remove_unit,
};

struct TestDummyHandler;

#[async_trait]
impl MessageHandler for TestDummyHandler {
    async fn on_message(&self, _queue: Arc<dyn GmqQueue>, _msg: Box<dyn Message>) {}
}

#[derive(Serialize)]
struct PostApplication {
    data: PostApplicationData,
}

#[derive(Serialize)]
struct PostApplicationData {
    code: String,
    #[serde(rename = "unitId")]
    unit_id: String,
    #[serde(rename = "hostUri")]
    host_uri: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    info: Option<Map<String, Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    ttl: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    length: Option<usize>,
}

#[derive(Serialize)]
struct PatchApplication {
    data: PatchApplicationData,
}

#[derive(Serialize)]
struct PatchApplicationData {
    #[serde(rename = "hostUri", skip_serializing_if = "Option::is_none")]
    host_uri: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    info: Option<Map<String, Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    ttl: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    length: Option<usize>,
    password: Option<String>,
}

#[derive(Serialize)]
pub struct PostApplicationDlDataBody {
    pub data: PostApplicationDlData,
}

#[derive(Serialize)]
pub struct PostApplicationDlData {
    #[serde(rename = "deviceId")]
    pub device_id: String,
    pub payload: String,
}

#[derive(Deserialize)]
struct PostApplicationRes {
    data: PostApplicationResData,
}

#[derive(Deserialize)]
struct PostApplicationResData {
    #[serde(rename = "applicationId")]
    application_id: String,
    password: String,
}

#[derive(Deserialize)]
struct GetApplicationRes {
    data: GetApplicationResData,
}

#[derive(Deserialize)]
struct GetApplicationResData {
    #[serde(rename = "applicationId")]
    application_id: String,
    code: String,
    #[serde(rename = "unitId")]
    unit_id: String,
    #[serde(rename = "unitCode")]
    _unit_code: String,
    #[serde(rename = "createdAt")]
    created_at: String,
    #[serde(rename = "modifiedAt")]
    modified_at: String,
    #[serde(rename = "hostUri")]
    host_uri: String,
    name: String,
    info: Map<String, Value>,
    ttl: Option<usize>,
    length: Option<usize>,
}

#[derive(Deserialize)]
struct GetApplicationStatsRes {
    data: GetApplicationStatsResData,
}

#[derive(Deserialize)]
struct GetApplicationStatsResData {
    uldata: Stats,
    #[serde(rename = "dldataResp")]
    _dldata_resp: Stats,
    #[serde(rename = "dldataResult")]
    _dldata_result: Stats,
}

const UNIT_OWNER: &'static str = "manager";
const UNIT_CODE: &'static str = "manager";
const APP_CODE: &'static str = "manager";
const APP2_CODE: &'static str = "manager2";

pub fn after_each_fn(state: &mut HashMap<&'static str, TestState>) -> () {
    let state = state.get_mut(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let broker_db = state.broker_db.as_ref().unwrap();
    let client = state.client.as_ref().unwrap();
    let mq_opts = state.mq_opts.as_ref().unwrap();
    let host = crate::TEST_MQ_HOST;

    runtime.block_on(async {
        time::sleep(Duration::from_secs(2)).await; // FIXME: remove this after general-mq using event-driven
        if let Err(e) = remove_unit(client, UNIT_CODE).await {
            println!("remove unit error: {}", e);
        }
        let cond = device::QueryCond {
            unit_id: Some(UNIT_CODE),
            ..Default::default()
        };
        if let Err(e) = broker_db.device().del(&cond).await {
            println!("remove device error: {}", e);
        }
        let cond = network::QueryCond {
            unit_id: Some(Some(UNIT_CODE)),
            ..Default::default()
        };
        if let Err(e) = broker_db.network().del(&cond).await {
            println!("remove network error: {}", e);
        }

        let apps = vec![APP_CODE, APP2_CODE];
        for app in apps {
            let username = mq::to_username(QueueType::Application, UNIT_CODE, app);
            let username = username.as_str();
            let q_type = QueueType::Application;
            let _ = rabbitmq::delete_user(client, &mq_opts.0, host, username).await;
            let _ = rabbitmq::delete_vhost(client, &mq_opts.0, host, username).await;
            if state.rumqttd_handles.is_none() {
                let _ = emqx::delete_user(client, &mq_opts.1, host, username).await;
                let _ = emqx::delete_acl(client, &mq_opts.1, host, username).await;
                let _ =
                    emqx::delete_topic_metrics(client, &mq_opts.1, host, q_type, username).await;
            }
        }
    })
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
        "/coremgr/api/v1/application/count",
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
        "/coremgr/api/v1/application/list",
        TOKEN_MANAGER,
        "applicationId,code,unitId,unitCode,createdAt,modifiedAt,hostUri,name,info",
    )
}

pub fn post(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();
    let broker_db = state.broker_db.as_ref().unwrap();
    let client = state.client.as_ref().unwrap();
    let mq_opts = state.mq_opts.as_ref().unwrap();
    let host = crate::TEST_MQ_HOST;

    match runtime.block_on(async {
        let unit = create_unit(UNIT_CODE, UNIT_OWNER);
        broker_db.unit().add(&unit).await
    }) {
        Err(e) => return Err(format!("add unit model info error: {}", e)),
        Ok(_) => (),
    }

    let mut info = Map::<String, Value>::new();
    info.insert("info".to_string(), Value::String("value".to_string()));
    let mut param = PostApplication {
        data: PostApplicationData {
            code: APP_CODE.to_string(),
            unit_id: UNIT_CODE.to_string(),
            host_uri: format!("amqp://{}", host),
            name: Some(UNIT_CODE.to_string()),
            info: Some(info),
            ttl: Some(1000),
            length: Some(2),
        },
    };
    let info = create_application(runtime, routes_state, &param)?;
    runtime.block_on(async {
        let host = param.data.host_uri.as_str();
        check_queue(host, info.password.as_str(), UNIT_CODE, APP_CODE).await
    })?;

    param.data.code = APP2_CODE.to_string();
    param.data.host_uri = match state.rumqttd_handles.is_some() {
        false => format!("mqtt://{}", host),
        true => format!("mqtt://{}:{}", host, crate::TEST_RUMQTTD_MQTT_PORT),
    };
    param.data.info = None;
    let info = create_application(runtime, routes_state, &param)?;
    runtime.block_on(async {
        let host = param.data.host_uri.as_str();
        check_queue(host, info.password.as_str(), UNIT_CODE, APP2_CODE).await
    })?;

    runtime.block_on(async {
        for _ in 0..100 {
            let username = mq::to_username(QueueType::Application, UNIT_CODE, APP_CODE);
            let username = username.as_str();
            if let Ok(stats) = rabbitmq::stats(client, &mq_opts.0, host, username, "dldata").await {
                if stats.consumers >= 1 {
                    return Ok(());
                }
            }
            time::sleep(Duration::from_millis(100)).await;
        }
        Err("broker does not consume application dldata".to_string())
    })?;

    create_application_dup(runtime, routes_state, &param)
}

pub fn post_invalid(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();
    let broker_db = state.broker_db.as_ref().unwrap();
    let host = crate::TEST_MQ_HOST;

    match runtime.block_on(async {
        let unit = create_unit(UNIT_CODE, UNIT_OWNER);
        broker_db.unit().add(&unit).await
    }) {
        Err(e) => return Err(format!("add unit model info error: {}", e)),
        Ok(_) => (),
    }

    let server = new_test_server(routes_state)?;

    let param = PostApplication {
        data: PostApplicationData {
            code: APP_CODE.to_string(),
            unit_id: UNIT_CODE.to_string(),
            host_uri: match state.rumqttd_handles.is_some() {
                false => format!("mqtt://{}", host),
                true => format!("mqtt://{}:{}", host, crate::TEST_RUMQTTD_MQTT_PORT),
            },
            name: Some(UNIT_CODE.to_string()),
            info: None,
            ttl: None,
            length: None,
        },
    };
    let req = server.post("/coremgr/api/v1/application").json(&param);
    test_invalid_param(runtime, req, "err_param")?;

    let req = server
        .post("/coremgr/api/v1/application")
        .add_header(
            header::AUTHORIZATION,
            HeaderValue::from_str(format!("Bearer {}", TOKEN_MANAGER).as_str()).unwrap(),
        )
        .add_header(
            header::CONTENT_TYPE,
            HeaderValue::from_str(ContentType::JSON).unwrap(),
        )
        .bytes("{".into());
    test_invalid_param(runtime, req, "err_param")?;

    let param = PostApplication {
        data: PostApplicationData {
            code: "code+".to_string(),
            unit_id: UNIT_CODE.to_string(),
            host_uri: match state.rumqttd_handles.is_some() {
                false => format!("mqtt://{}", host),
                true => format!("mqtt://{}:{}", host, crate::TEST_RUMQTTD_MQTT_PORT),
            },
            name: Some(UNIT_CODE.to_string()),
            info: None,
            ttl: None,
            length: None,
        },
    };
    let req = server
        .post("/coremgr/api/v1/application")
        .add_header(
            header::AUTHORIZATION,
            HeaderValue::from_str(format!("Bearer {}", TOKEN_MANAGER).as_str()).unwrap(),
        )
        .json(&param);
    test_invalid_param(runtime, req, "err_param")?;

    let param = PostApplication {
        data: PostApplicationData {
            code: APP_CODE.to_string(),
            unit_id: "".to_string(),
            host_uri: match state.rumqttd_handles.is_some() {
                false => format!("mqtt://{}", host),
                true => format!("mqtt://{}:{}", host, crate::TEST_RUMQTTD_MQTT_PORT),
            },
            name: Some(UNIT_CODE.to_string()),
            info: None,
            ttl: None,
            length: None,
        },
    };
    let req = server
        .post("/coremgr/api/v1/application")
        .add_header(
            header::AUTHORIZATION,
            HeaderValue::from_str(format!("Bearer {}", TOKEN_MANAGER).as_str()).unwrap(),
        )
        .json(&param);
    test_invalid_param(runtime, req, "err_param")?;

    let param = PostApplication {
        data: PostApplicationData {
            code: APP_CODE.to_string(),
            unit_id: UNIT_CODE.to_string(),
            host_uri: format!("://{}", host),
            name: Some(UNIT_CODE.to_string()),
            info: None,
            ttl: None,
            length: None,
        },
    };
    let req = server
        .post("/coremgr/api/v1/application")
        .add_header(
            header::AUTHORIZATION,
            HeaderValue::from_str(format!("Bearer {}", TOKEN_MANAGER).as_str()).unwrap(),
        )
        .json(&param);
    test_invalid_param(runtime, req, "err_param")?;

    let param = PostApplication {
        data: PostApplicationData {
            code: APP_CODE.to_string(),
            unit_id: UNIT_CODE.to_string(),
            host_uri: "mqtt://".to_string(),
            name: Some(UNIT_CODE.to_string()),
            info: None,
            ttl: None,
            length: None,
        },
    };
    let req = server
        .post("/coremgr/api/v1/application")
        .add_header(
            header::AUTHORIZATION,
            HeaderValue::from_str(format!("Bearer {}", TOKEN_MANAGER).as_str()).unwrap(),
        )
        .json(&param);
    test_invalid_param(runtime, req, "err_param")?;

    let param = PostApplication {
        data: PostApplicationData {
            code: APP_CODE.to_string(),
            unit_id: UNIT_CODE.to_string(),
            host_uri: "mqtt://".to_string(),
            name: Some(UNIT_CODE.to_string()),
            info: None,
            ttl: None,
            length: None,
        },
    };
    let req = server
        .post("/coremgr/api/v1/application")
        .add_header(
            header::AUTHORIZATION,
            HeaderValue::from_str(format!("Bearer {}", TOKEN_MEMBER).as_str()).unwrap(),
        )
        .json(&param);
    test_invalid_param(runtime, req, "err_broker_unit_not_exist")?;

    Ok(())
}

pub fn get(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();
    let broker_db = state.broker_db.as_ref().unwrap();
    let host = crate::TEST_MQ_HOST;

    match runtime.block_on(async {
        let unit = create_unit(UNIT_CODE, UNIT_OWNER);
        broker_db.unit().add(&unit).await
    }) {
        Err(e) => return Err(format!("add unit model info error: {}", e)),
        Ok(_) => (),
    }

    let mut param = PostApplication {
        data: PostApplicationData {
            code: APP_CODE.to_string(),
            unit_id: UNIT_CODE.to_string(),
            host_uri: format!("amqp://{}", host),
            name: Some(UNIT_CODE.to_string()),
            info: None,
            ttl: Some(1000),
            length: Some(2),
        },
    };
    test_get(runtime, routes_state, &param)?;

    param.data.code = APP2_CODE.to_string();
    param.data.host_uri = match state.rumqttd_handles.is_some() {
        false => format!("mqtt://{}", host),
        true => format!("mqtt://{}:{}", host, crate::TEST_RUMQTTD_MQTT_PORT),
    };
    test_get(runtime, routes_state, &param)?;

    Ok(())
}

pub fn get_invalid(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    let server = new_test_server(routes_state)?;

    let req = server.get("/coremgr/api/v1/application/test");
    test_invalid_param(runtime, req, "err_param")?;

    let req = server.get("/coremgr/api/v1/application/test").add_header(
        header::AUTHORIZATION,
        HeaderValue::from_str(format!("Bearer {}", TOKEN_MANAGER).as_str()).unwrap(),
    );
    test_invalid_param(runtime, req, "err_not_found")
}

/// Test PATCH API with the following steps:
/// 1. create MQTT application.
/// 2. update to AMQP with TTL/length.
/// 3. update password and TTL/length to zero.
/// 4. update name.
/// 5. update to MQTT.
/// 6. update password.
pub fn patch(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();
    let broker_db = state.broker_db.as_ref().unwrap();
    let host = crate::TEST_MQ_HOST;

    match runtime.block_on(async {
        let unit = create_unit(UNIT_CODE, UNIT_OWNER);
        broker_db.unit().add(&unit).await
    }) {
        Err(e) => return Err(format!("add unit model info error: {}", e)),
        Ok(_) => (),
    }

    // Step 1.
    let param = PostApplication {
        data: PostApplicationData {
            code: APP_CODE.to_string(),
            unit_id: UNIT_CODE.to_string(),
            host_uri: match state.rumqttd_handles.is_some() {
                false => format!("mqtt://{}", host),
                true => format!("mqtt://{}:{}", host, crate::TEST_RUMQTTD_MQTT_PORT),
            },
            name: Some(UNIT_CODE.to_string()),
            info: None,
            ttl: None,
            length: None,
        },
    };
    let info = create_application(runtime, routes_state, &param)?;
    runtime.block_on(async {
        let host = param.data.host_uri.as_str();
        check_queue(host, info.password.as_str(), UNIT_CODE, APP_CODE).await
    })?;
    let application_id = info.application_id.as_str();

    // Step 2.
    let mut info = Map::<String, Value>::new();
    info.insert("info".to_string(), Value::String("value".to_string()));
    let param = PatchApplication {
        data: PatchApplicationData {
            host_uri: Some(format!("amqp://{}", host)),
            name: Some(UNIT_CODE.to_string()),
            info: Some(info),
            ttl: Some(1000),
            length: Some(2),
            password: Some("password2".to_string()),
        },
    };
    test_patch(
        runtime,
        routes_state,
        &param,
        application_id,
        UNIT_CODE,
        APP_CODE,
    )?;

    // Step 3.
    let param = PatchApplication {
        data: PatchApplicationData {
            host_uri: None,
            name: None,
            info: None,
            ttl: Some(0),
            length: Some(0),
            password: Some("password3".to_string()),
        },
    };
    test_patch(
        runtime,
        routes_state,
        &param,
        application_id,
        UNIT_CODE,
        APP_CODE,
    )?;

    // Step 4.
    let param = PatchApplication {
        data: PatchApplicationData {
            host_uri: None,
            name: Some("changed name".to_string()),
            info: None,
            ttl: None,
            length: None,
            password: None,
        },
    };
    test_patch(
        runtime,
        routes_state,
        &param,
        application_id,
        UNIT_CODE,
        APP_CODE,
    )?;

    // Step 5.
    runtime.block_on(async {
        time::sleep(Duration::from_secs(30)).await;
    });
    let param = PatchApplication {
        data: PatchApplicationData {
            host_uri: Some(match state.rumqttd_handles.is_some() {
                false => format!("mqtt://{}", host),
                true => format!("mqtt://{}:{}", host, crate::TEST_RUMQTTD_MQTT_PORT),
            }),
            name: None,
            info: None,
            ttl: None,
            length: None,
            password: Some("password".to_string()),
        },
    };
    test_patch(
        runtime,
        routes_state,
        &param,
        application_id,
        UNIT_CODE,
        APP_CODE,
    )?;

    // Step 6.
    let param = PatchApplication {
        data: PatchApplicationData {
            host_uri: None,
            name: None,
            info: None,
            ttl: None,
            length: None,
            password: Some("password2".to_string()),
        },
    };
    test_patch(
        runtime,
        routes_state,
        &param,
        application_id,
        UNIT_CODE,
        APP_CODE,
    )
}

pub fn patch_invalid(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();
    let broker_db = state.broker_db.as_ref().unwrap();
    let host = crate::TEST_MQ_HOST;

    let server = new_test_server(routes_state)?;

    let param = PatchApplication {
        data: PatchApplicationData {
            host_uri: None,
            name: None,
            info: None,
            ttl: None,
            length: None,
            password: None,
        },
    };
    let req = server
        .patch("/coremgr/api/v1/application/test")
        .json(&param);
    test_invalid_param(runtime, req, "err_param")?;

    let param = PatchApplication {
        data: PatchApplicationData {
            host_uri: None,
            name: None,
            info: None,
            ttl: None,
            length: None,
            password: None,
        },
    };
    let req = server
        .patch("/coremgr/api/v1/application/test")
        .add_header(
            header::AUTHORIZATION,
            HeaderValue::from_str(format!("Bearer {}", TOKEN_MANAGER).as_str()).unwrap(),
        )
        .json(&param);
    test_invalid_param(runtime, req, "err_param")?;

    let param = PatchApplication {
        data: PatchApplicationData {
            host_uri: None,
            name: Some("name".to_string()),
            info: None,
            ttl: None,
            length: None,
            password: None,
        },
    };
    let req = server
        .patch("/coremgr/api/v1/application/test")
        .add_header(
            header::AUTHORIZATION,
            HeaderValue::from_str(format!("Bearer {}", TOKEN_MANAGER).as_str()).unwrap(),
        )
        .json(&param);
    test_invalid_param(runtime, req, "err_not_found")?;

    // Create unit and application.
    match runtime.block_on(async {
        let unit = create_unit(UNIT_CODE, UNIT_OWNER);
        broker_db.unit().add(&unit).await
    }) {
        Err(e) => return Err(format!("add unit model info error: {}", e)),
        Ok(_) => (),
    }
    let param = PostApplication {
        data: PostApplicationData {
            code: APP_CODE.to_string(),
            unit_id: UNIT_CODE.to_string(),
            host_uri: match state.rumqttd_handles.is_some() {
                false => format!("mqtt://{}", host),
                true => format!("mqtt://{}:{}", host, crate::TEST_RUMQTTD_MQTT_PORT),
            },
            name: Some(UNIT_CODE.to_string()),
            info: None,
            ttl: None,
            length: None,
        },
    };
    let info = create_application(runtime, routes_state, &param)?;
    runtime.block_on(async {
        let host = param.data.host_uri.as_str();
        check_queue(host, info.password.as_str(), UNIT_CODE, APP_CODE).await
    })?;
    let application_id = info.application_id.as_str();

    let param = PatchApplication {
        data: PatchApplicationData {
            host_uri: Some(format!("amqp://{}", host)),
            name: None,
            info: None,
            ttl: None,
            length: None,
            password: None,
        },
    };
    let req = server
        .patch(format!("/coremgr/api/v1/application/{}", application_id).as_str())
        .add_header(
            header::AUTHORIZATION,
            HeaderValue::from_str(format!("Bearer {}", TOKEN_MANAGER).as_str()).unwrap(),
        )
        .json(&param);
    test_invalid_param(runtime, req, "err_param")?;

    let param = PatchApplication {
        data: PatchApplicationData {
            host_uri: Some(format!("amqp://{}", host)),
            name: None,
            info: None,
            ttl: None,
            length: None,
            password: Some("".to_string()),
        },
    };
    let req = server
        .patch(format!("/coremgr/api/v1/application/{}", application_id).as_str())
        .add_header(
            header::AUTHORIZATION,
            HeaderValue::from_str(format!("Bearer {}", TOKEN_MANAGER).as_str()).unwrap(),
        )
        .json(&param);
    test_invalid_param(runtime, req, "err_param")?;

    let param = PatchApplication {
        data: PatchApplicationData {
            host_uri: None,
            name: None,
            info: None,
            ttl: None,
            length: None,
            password: Some("".to_string()),
        },
    };
    let req = server
        .patch(format!("/coremgr/api/v1/application/{}", application_id).as_str())
        .add_header(
            header::AUTHORIZATION,
            HeaderValue::from_str(format!("Bearer {}", TOKEN_MANAGER).as_str()).unwrap(),
        )
        .json(&param);
    test_invalid_param(runtime, req, "err_param")?;

    let param = PatchApplication {
        data: PatchApplicationData {
            host_uri: Some(format!("://{}", host)),
            name: None,
            info: None,
            ttl: None,
            length: None,
            password: Some("test".to_string()),
        },
    };
    let req = server
        .patch(format!("/coremgr/api/v1/application/{}", application_id).as_str())
        .add_header(
            header::AUTHORIZATION,
            HeaderValue::from_str(format!("Bearer {}", TOKEN_MANAGER).as_str()).unwrap(),
        )
        .json(&param);
    test_invalid_param(runtime, req, "err_param")?;

    let param = PatchApplication {
        data: PatchApplicationData {
            host_uri: Some("mqtt://".to_string()),
            name: None,
            info: None,
            ttl: None,
            length: None,
            password: Some("test".to_string()),
        },
    };
    let req = server
        .patch(format!("/coremgr/api/v1/application/{}", application_id).as_str())
        .add_header(
            header::AUTHORIZATION,
            HeaderValue::from_str(format!("Bearer {}", TOKEN_MANAGER).as_str()).unwrap(),
        )
        .json(&param);
    test_invalid_param(runtime, req, "err_param")?;

    Ok(())
}

pub fn delete(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();
    let broker_db = state.broker_db.as_ref().unwrap();
    let host = crate::TEST_MQ_HOST;
    let is_rumqttd = state.rumqttd_handles.is_some();

    match runtime.block_on(async {
        let unit = create_unit(UNIT_CODE, UNIT_OWNER);
        broker_db.unit().add(&unit).await
    }) {
        Err(e) => return Err(format!("add unit model info error: {}", e)),
        Ok(_) => (),
    }

    let mut param = PostApplication {
        data: PostApplicationData {
            code: APP_CODE.to_string(),
            unit_id: UNIT_CODE.to_string(),
            host_uri: format!("amqp://{}", host),
            name: Some(UNIT_CODE.to_string()),
            info: None,
            ttl: None,
            length: None,
        },
    };
    let info = create_application(runtime, routes_state, &param)?;
    runtime.block_on(async {
        let host = param.data.host_uri.as_str();
        check_queue(host, info.password.as_str(), UNIT_CODE, APP_CODE).await
    })?;
    let application_id = info.application_id.as_str();
    test_delete(
        runtime,
        routes_state,
        application_id,
        param.data.host_uri.as_str(),
        UNIT_CODE,
        APP_CODE,
        is_rumqttd,
    )?;

    param.data.host_uri = match state.rumqttd_handles.is_some() {
        false => format!("mqtt://{}", host),
        true => format!("mqtt://{}:{}", host, crate::TEST_RUMQTTD_MQTT_PORT),
    };
    let info = create_application(runtime, routes_state, &param)?;
    runtime.block_on(async {
        let host = param.data.host_uri.as_str();
        check_queue(host, info.password.as_str(), UNIT_CODE, APP_CODE).await
    })?;
    let application_id = info.application_id.as_str();
    test_delete(
        runtime,
        routes_state,
        application_id,
        param.data.host_uri.as_str(),
        UNIT_CODE,
        APP_CODE,
        is_rumqttd,
    )
}

pub fn delete_invalid(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    let server = new_test_server(routes_state)?;

    let req = server.delete("/coremgr/api/v1/application/test");
    test_invalid_param(runtime, req, "err_param")?;

    let req = server
        .delete("/coremgr/api/v1/application/test")
        .add_header(
            header::AUTHORIZATION,
            HeaderValue::from_str(format!("Bearer {}", TOKEN_MANAGER).as_str()).unwrap(),
        );
    test_invalid_param(runtime, req, "err_not_found")
}

pub fn stats(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();
    let broker_db = state.broker_db.as_ref().unwrap();
    let client = state.client.as_ref().unwrap();
    let mq_opts = state.mq_opts.as_ref().unwrap();
    let host = crate::TEST_MQ_HOST;
    let is_rumqttd = state.rumqttd_handles.is_some();

    match runtime.block_on(async {
        let unit = create_unit(UNIT_CODE, UNIT_OWNER);
        broker_db.unit().add(&unit).await
    }) {
        Err(e) => return Err(format!("add unit model info error: {}", e)),
        Ok(_) => (),
    }

    let mut param = PostApplication {
        data: PostApplicationData {
            code: APP_CODE.to_string(),
            unit_id: UNIT_CODE.to_string(),
            host_uri: format!("amqp://{}", host),
            name: Some(UNIT_CODE.to_string()),
            info: None,
            ttl: None,
            length: None,
        },
    };
    let info = create_application(runtime, routes_state, &param)?;
    runtime.block_on(async {
        let host = param.data.host_uri.as_str();
        check_queue(host, info.password.as_str(), UNIT_CODE, APP_CODE).await
    })?;
    let application_id = info.application_id.as_str();
    let username = mq::to_username(QueueType::Application, UNIT_CODE, APP_CODE);
    let username = username.as_str();
    let payload = general_purpose::STANDARD.encode("amqp");
    if let Err(e) = runtime.block_on(async {
        rabbitmq::publish_message(client, &mq_opts.0, host, username, "uldata", payload).await
    }) {
        return Err(format!("publish AMQP payload error: {}", e));
    }
    test_stats(runtime, routes_state, application_id, is_rumqttd)?;

    param.data.code = APP2_CODE.to_string();
    param.data.host_uri = match state.rumqttd_handles.is_some() {
        false => format!("mqtt://{}", host),
        true => format!("mqtt://{}:{}", host, crate::TEST_RUMQTTD_MQTT_PORT),
    };
    let info = create_application(runtime, routes_state, &param)?;
    runtime.block_on(async {
        let host = param.data.host_uri.as_str();
        check_queue(host, info.password.as_str(), UNIT_CODE, APP2_CODE).await
    })?;
    let application_id = info.application_id.as_str();
    if !is_rumqttd {
        let username = mq::to_username(QueueType::Application, UNIT_CODE, APP2_CODE);
        let username = username.as_str();
        let payload = general_purpose::STANDARD.encode("mqtt");
        if let Err(e) = runtime.block_on(async {
            emqx::publish_message(client, &mq_opts.1, host, username, "uldata", payload).await
        }) {
            return Err(format!("publish MQTT payload error: {}", e));
        }
    }
    test_stats(runtime, routes_state, application_id, is_rumqttd)
}

pub fn stats_invalid(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    let server = new_test_server(routes_state)?;

    let req = server.get("/coremgr/api/v1/application/test/stats");
    test_invalid_param(runtime, req, "err_param")?;

    let req = server
        .get("/coremgr/api/v1/application/test/stats")
        .add_header(
            header::AUTHORIZATION,
            HeaderValue::from_str(format!("Bearer {}", TOKEN_MANAGER).as_str()).unwrap(),
        );
    test_invalid_param(runtime, req, "err_not_found")
}

pub fn dldata(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();
    let broker_db = state.broker_db.as_ref().unwrap();
    let client = state.client.as_ref().unwrap();
    let mq_opts = state.mq_opts.as_ref().unwrap();
    let host = crate::TEST_MQ_HOST;
    let device = "addr";
    let is_rumqttd = state.rumqttd_handles.is_some();

    match runtime.block_on(async {
        let unit = create_unit(UNIT_CODE, UNIT_OWNER);
        broker_db.unit().add(&unit).await
    }) {
        Err(e) => return Err(format!("add unit model info error: {}", e)),
        Ok(_) => (),
    }
    match runtime.block_on(async {
        let network = create_network(APP_CODE, host, UNIT_OWNER);
        broker_db.network().add(&network).await
    }) {
        Err(e) => return Err(format!("add network model info error: {}", e)),
        Ok(_) => (),
    }
    match runtime.block_on(async {
        let device = create_device(UNIT_CODE, APP_CODE, device, false);
        broker_db.device().add(&device).await
    }) {
        Err(e) => return Err(format!("add device model info error: {}", e)),
        Ok(_) => (),
    }

    let username = mq::to_username(QueueType::Application, UNIT_CODE, APP_CODE);
    let username = username.as_str();
    let param = PostApplication {
        data: PostApplicationData {
            code: APP_CODE.to_string(),
            unit_id: UNIT_CODE.to_string(),
            host_uri: format!("amqp://{}", host),
            name: Some(UNIT_CODE.to_string()),
            info: None,
            ttl: None,
            length: None,
        },
    };
    let info = create_application(runtime, routes_state, &param)?;
    runtime.block_on(async {
        let host = param.data.host_uri.as_str();
        check_queue(host, info.password.as_str(), UNIT_CODE, APP_CODE).await
    })?;
    let application_id = info.application_id.as_str();
    let body = PostApplicationDlDataBody {
        data: PostApplicationDlData {
            device_id: device.to_string(),
            payload: hex::encode("amqp"),
        },
    };
    test_dldata(runtime, routes_state, application_id, &body, false)?;
    runtime.block_on(async {
        for _ in 0..100 {
            time::sleep(Duration::from_millis(100)).await;
            match rabbitmq::stats(client, &mq_opts.0, host, username, "dldata-resp").await {
                Err(e) => return Err(format!("get RabbitMQ stats error: {}", e)),
                Ok(stats) => {
                    if stats.messages > 0 || stats.publish_rate > 0.0 {
                        return Ok(());
                    }
                }
            }
        }
        Err("publish AMQP error".to_string())
    })?;

    let username = mq::to_username(QueueType::Application, UNIT_CODE, APP2_CODE);
    let username = username.as_str();
    let param = PostApplication {
        data: PostApplicationData {
            code: APP2_CODE.to_string(),
            unit_id: UNIT_CODE.to_string(),
            host_uri: match state.rumqttd_handles.is_some() {
                false => format!("mqtt://{}", host),
                true => format!("mqtt://{}:{}", host, crate::TEST_RUMQTTD_MQTT_PORT),
            },
            name: Some(UNIT_CODE.to_string()),
            info: None,
            ttl: None,
            length: None,
        },
    };
    let info = create_application(runtime, routes_state, &param)?;
    runtime.block_on(async {
        let host = param.data.host_uri.as_str();
        check_queue(host, info.password.as_str(), UNIT_CODE, APP2_CODE).await
    })?;
    let application_id = info.application_id.as_str();
    let body = PostApplicationDlDataBody {
        data: PostApplicationDlData {
            device_id: device.to_string(),
            payload: hex::encode("mqtt"),
        },
    };
    runtime.block_on(async { time::sleep(Duration::from_secs(2)).await });
    test_dldata(runtime, routes_state, application_id, &body, is_rumqttd)?;
    if is_rumqttd {
        return Ok(());
    }
    runtime.block_on(async {
        for _ in 0..100 {
            time::sleep(Duration::from_millis(100)).await;
            match emqx::stats(client, &mq_opts.1, host, username, "dldata-resp").await {
                Err(e) => return Err(format!("get EMQX stats error: {}", e)),
                Ok(stats) => {
                    if stats.publish_rate > 0.0 {
                        return Ok(());
                    }
                }
            }
        }
        Err("publish MQTT error".to_string())
    })
}

pub fn dldata_invalid(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();
    let broker_db = state.broker_db.as_ref().unwrap();
    let host = crate::TEST_MQ_HOST;

    let server = new_test_server(routes_state)?;

    let mut body = PostApplicationDlDataBody {
        data: PostApplicationDlData {
            device_id: "device".to_string(),
            payload: hex::encode("payload"),
        },
    };
    let req = server
        .post("/coremgr/api/v1/application/test/dldata")
        .json(&body);
    test_invalid_param(runtime, req, "err_param")?;

    let req = server
        .post("/coremgr/api/v1/application/test/dldata")
        .add_header(
            header::AUTHORIZATION,
            HeaderValue::from_str(format!("Bearer {}", TOKEN_MANAGER).as_str()).unwrap(),
        )
        .json(&body);
    test_invalid_param(runtime, req, "err_not_found")?;

    // Create unit, network, device and application.
    match runtime.block_on(async {
        let unit = create_unit(UNIT_CODE, UNIT_OWNER);
        broker_db.unit().add(&unit).await
    }) {
        Err(e) => return Err(format!("add unit model info error: {}", e)),
        Ok(_) => (),
    }
    match runtime.block_on(async {
        let network = create_network(APP_CODE, host, UNIT_OWNER);
        broker_db.network().add(&network).await
    }) {
        Err(e) => return Err(format!("add network model info error: {}", e)),
        Ok(_) => (),
    }
    match runtime.block_on(async {
        let device = create_device(UNIT_CODE, APP_CODE, "addr", false);
        broker_db.device().add(&device).await
    }) {
        Err(e) => return Err(format!("add device model info error: {}", e)),
        Ok(_) => (),
    }
    let param = PostApplication {
        data: PostApplicationData {
            code: APP_CODE.to_string(),
            unit_id: UNIT_CODE.to_string(),
            host_uri: format!("amqp://{}", host),
            name: Some(UNIT_CODE.to_string()),
            info: None,
            ttl: None,
            length: None,
        },
    };
    let info = create_application(runtime, routes_state, &param)?;
    runtime.block_on(async {
        let host = param.data.host_uri.as_str();
        check_queue(host, info.password.as_str(), UNIT_CODE, APP_CODE).await
    })?;
    let application_id = info.application_id.as_str();

    body.data.device_id = "".to_string();
    let req = server
        .post(format!("/coremgr/api/v1/application/{}/dldata", application_id).as_str())
        .add_header(
            header::AUTHORIZATION,
            HeaderValue::from_str(format!("Bearer {}", TOKEN_MANAGER).as_str()).unwrap(),
        )
        .json(&body);
    test_invalid_param(runtime, req, "err_param")?;

    body.data.device_id = "device".to_string();
    body.data.payload = "payload".to_string();
    let req = server
        .post(format!("/coremgr/api/v1/application/{}/dldata", application_id).as_str())
        .add_header(
            header::AUTHORIZATION,
            HeaderValue::from_str(format!("Bearer {}", TOKEN_MANAGER).as_str()).unwrap(),
        )
        .json(&body);
    test_invalid_param(runtime, req, "err_param")?;

    body.data.device_id = "test".to_string();
    body.data.payload = hex::encode("payload");
    let req = server
        .post(format!("/coremgr/api/v1/application/{}/dldata", application_id).as_str())
        .add_header(
            header::AUTHORIZATION,
            HeaderValue::from_str(format!("Bearer {}", TOKEN_MANAGER).as_str()).unwrap(),
        )
        .json(&body);
    test_invalid_param(runtime, req, "err_broker_device_not_exist")
}

fn test_get(
    runtime: &Runtime,
    state: &routes::State,
    param: &PostApplication,
) -> Result<(), String> {
    let start = Utc::now().trunc_subsecs(3);
    let info = create_application(runtime, state, &param)?;
    let end = Utc::now().trunc_subsecs(3);

    let server = new_test_server(state)?;

    let req = server
        .get(format!("/coremgr/api/v1/application/{}", info.application_id).as_str())
        .add_header(
            header::AUTHORIZATION,
            HeaderValue::from_str(format!("Bearer {}", TOKEN_MANAGER).as_str()).unwrap(),
        );
    let resp = runtime.block_on(async { req.await });
    let status = resp.status_code();
    if status != StatusCode::OK {
        return Err(format!(
            "API not 200, status: {}, body: {}",
            status,
            resp.text()
        ));
    }
    let body = resp.text();
    let body = match serde_json::from_str::<GetApplicationRes>(body.as_str()) {
        Err(e) => return Err(format!("unexpected response format: {}, body: {}", e, body)),
        Ok(body) => body.data,
    };
    match DateTime::parse_from_rfc3339(body.created_at.as_str()) {
        Err(e) => return Err(format!("invalid createdAt {}: {}", body.created_at, e)),
        Ok(created_at) => match expect(start.le(&created_at)).to_equal(true) {
            Err(_) => {
                let e = format!("start {} not less than createdAt {}", start, created_at);
                return Err(e);
            }
            Ok(_) => (),
        },
    }
    match DateTime::parse_from_rfc3339(body.created_at.as_str()) {
        Err(e) => return Err(format!("invalid createdAt {}: {}", body.created_at, e)),
        Ok(created_at) => match expect(end.ge(&created_at)).to_equal(true) {
            Err(_) => {
                let e = format!("end {} not greater than createdAt {}", end, created_at);
                return Err(e);
            }
            Ok(_) => (),
        },
    }
    match DateTime::parse_from_rfc3339(body.modified_at.as_str()) {
        Err(e) => return Err(format!("invalid modifiedAt {}: {}", body.modified_at, e)),
        Ok(modified_at) => match expect(start.le(&modified_at)).to_equal(true) {
            Err(_) => {
                let e = format!("start {} not less than modifiedAt {}", start, modified_at);
                return Err(e);
            }
            Ok(_) => (),
        },
    }
    match DateTime::parse_from_rfc3339(body.modified_at.as_str()) {
        Err(e) => return Err(format!("invalid modifiedAt {}: {}", body.modified_at, e)),
        Ok(modified_at) => match expect(end.ge(&modified_at)).to_equal(true) {
            Err(_) => {
                let e = format!("end {} not greater than modifiedAt {}", end, modified_at);
                return Err(e);
            }
            Ok(_) => (),
        },
    }
    expect(body.application_id.as_str()).to_equal(info.application_id.as_str())?;
    expect(body.code.as_str()).to_equal(param.data.code.as_str())?;
    expect(body.unit_id.as_str()).to_equal(param.data.unit_id.as_str())?;
    expect(body.host_uri.as_str()).to_equal(param.data.host_uri.as_str())?;
    if body.host_uri.starts_with("amqp") {
        match param.data.ttl {
            None => expect(body.ttl).to_equal(Some(0))?,
            _ => expect(body.ttl).to_equal(param.data.ttl)?,
        }
        match param.data.length {
            None => expect(body.length).to_equal(Some(0))?,
            _ => expect(body.length).to_equal(param.data.length)?,
        }
    } else if body.host_uri.starts_with("mqtt") {
        expect(body.ttl).to_equal(None)?;
        expect(body.length).to_equal(None)?;
    } else {
        return Err(format!("invalid hostUri: {}", body.host_uri));
    }

    Ok(())
}

fn test_patch(
    runtime: &Runtime,
    state: &routes::State,
    param: &PatchApplication,
    application_id: &str,
    unit: &str,
    code: &str,
) -> Result<(), String> {
    let server = new_test_server(state)?;

    let req = server
        .patch(format!("/coremgr/api/v1/application/{}", application_id).as_str())
        .add_header(
            header::AUTHORIZATION,
            HeaderValue::from_str(format!("Bearer {}", TOKEN_MANAGER).as_str()).unwrap(),
        )
        .json(&param);
    let resp = runtime.block_on(async { req.await });
    let status = resp.status_code();
    if status != StatusCode::NO_CONTENT {
        return Err(format!(
            "API not 204, status: {}, body: {}",
            status,
            resp.text()
        ));
    }

    let req = server
        .get(format!("/coremgr/api/v1/application/{}", application_id).as_str())
        .add_header(
            header::AUTHORIZATION,
            HeaderValue::from_str(format!("Bearer {}", TOKEN_MANAGER).as_str()).unwrap(),
        );
    let resp = runtime.block_on(async { req.await });
    let status = resp.status_code();
    if status != StatusCode::OK {
        return Err(format!(
            "API not 200, status: {}, body: {}",
            status,
            resp.text()
        ));
    }
    let body = resp.text();
    let body = match serde_json::from_str::<GetApplicationRes>(body.as_str()) {
        Err(e) => return Err(format!("unexpected response format: {}, body: {}", e, body)),
        Ok(body) => body.data,
    };
    if let Some(host_uri) = param.data.host_uri.as_ref() {
        expect(host_uri.as_str()).to_equal(body.host_uri.as_str())?;
        let password = match param.data.password.as_ref() {
            None => "",
            Some(password) => password.as_str(),
        };
        runtime.block_on(async { check_queue(host_uri.as_str(), password, unit, code).await })?;
    } else if let Some(password) = param.data.password.as_ref() {
        let host_uri = body.host_uri.as_str();
        runtime.block_on(async { check_queue(host_uri, password, unit, code).await })?;
    }
    if let Some(host_uri) = param.data.host_uri.as_ref() {
        expect(host_uri.as_str()).to_equal(body.host_uri.as_str())?;
    }
    if let Some(name) = param.data.name.as_ref() {
        expect(name.as_str()).to_equal(body.name.as_str())?;
    }
    if let Some(info) = param.data.info.as_ref() {
        expect(info).to_equal(&body.info)?;
    }
    if let Some(ttl) = param.data.ttl {
        if body.host_uri.starts_with("amqp") {
            expect(Some(ttl)).to_equal(body.ttl)?;
        } else if body.host_uri.starts_with("mqtt") {
            expect(body.ttl).to_equal(None)?;
        } else {
            return Err("not support scheme".to_string());
        }
    }
    if let Some(length) = param.data.length {
        if body.host_uri.starts_with("amqp") {
            expect(Some(length)).to_equal(body.length)?;
        } else if body.host_uri.starts_with("mqtt") {
            expect(body.length).to_equal(None)?;
        } else {
            return Err("not support scheme".to_string());
        }
    }
    Ok(())
}

fn test_delete(
    runtime: &Runtime,
    state: &routes::State,
    application_id: &str,
    host: &str,
    unit: &str,
    code: &str,
    is_rumqttd: bool,
) -> Result<(), String> {
    let server = new_test_server(state)?;

    let req = server
        .delete(format!("/coremgr/api/v1/application/{}", application_id).as_str())
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

    runtime.block_on(async {
        let username = mq::to_username(QueueType::Application, unit, code);
        if host.starts_with("amqp") {
            let opts = AmqpConnectionOptions {
                uri: format!("{}/{}", host, username),
                ..Default::default()
            };
            let mut conn = AmqpConnection::new(opts)?;
            if let Err(e) = conn.connect() {
                return Err(format!("connect AMQP broker error: {}", e));
            }
            for _ in 0..WAIT_COUNT {
                time::sleep(Duration::from_millis(WAIT_TICK)).await;
                if conn.status() == ConnStatus::Connected {
                    let _ = conn.close().await;
                    return Err("should not connected to AMQP broker".to_string());
                }
            }
            let _ = conn.close().await;
            Ok(())
        } else if host.starts_with("mqtt") {
            if is_rumqttd {
                return Ok(());
            }
            let opts = MqttConnectionOptions {
                uri: host.to_string(),
                ..Default::default()
            };
            let mut conn = MqttConnection::new(opts)?;
            if let Err(e) = conn.connect() {
                return Err(format!("connect MQTT broker error: {}", e));
            }
            for _ in 0..WAIT_COUNT {
                time::sleep(Duration::from_millis(WAIT_TICK)).await;
                if conn.status() == ConnStatus::Connected {
                    let _ = conn.close().await;
                    return Err("should not connected to MQTT broker".to_string());
                }
            }
            let _ = conn.close().await;
            Ok(())
        } else {
            Err("not support scheme".to_string())
        }
    })
}

fn test_stats(
    runtime: &Runtime,
    state: &routes::State,
    application_id: &str,
    is_rumqttd: bool,
) -> Result<(), String> {
    let server = new_test_server(state)?;

    runtime.block_on(async {
        for _ in 0..WAIT_COUNT {
            let req = server
                .get(format!("/coremgr/api/v1/application/{}/stats", application_id).as_str())
                .add_header(
                    header::AUTHORIZATION,
                    HeaderValue::from_str(format!("Bearer {}", TOKEN_MANAGER).as_str()).unwrap(),
                );
            let resp = req.await;
            let status = resp.status_code();
            if status != StatusCode::OK {
                return Err(format!(
                    "API not 200, status: {}, body: {}",
                    status,
                    resp.text()
                ));
            }
            let body = resp.text();
            let stats = match serde_json::from_str::<GetApplicationStatsRes>(body.as_str()) {
                Err(e) => return Err(format!("unexpected response format: {}, body: {}", e, body)),
                Ok(body) => body.data.uldata,
            };

            if is_rumqttd {
                return Ok(());
            }
            time::sleep(Duration::from_millis(WAIT_TICK)).await;
            if stats.messages > 0 || stats.publish_rate > 0.0 {
                return Ok(());
            }
        }
        Err("stats not changed".to_string())
    })
}

fn test_dldata(
    runtime: &Runtime,
    state: &routes::State,
    application_id: &str,
    body: &PostApplicationDlDataBody, // use "amqp" as payload to check stats
    is_rumqttd: bool,
) -> Result<(), String> {
    let server = new_test_server(state)?;

    let req = server
        .post(format!("/coremgr/api/v1/application/{}/dldata", application_id).as_str())
        .add_header(
            header::AUTHORIZATION,
            HeaderValue::from_str(format!("Bearer {}", TOKEN_MANAGER).as_str()).unwrap(),
        )
        .json(&body);
    let resp = runtime.block_on(async { req.await });
    let status = resp.status_code();
    if status != StatusCode::NO_CONTENT {
        let body = resp.text();
        if is_rumqttd {
            if let Ok(e) = serde_json::from_str::<ApiError>(body.as_str()) {
                if e.code.as_str() == err::E_PARAM {
                    return Ok(());
                }
            }
        }
        return Err(format!("API not 204, status: {}, body: {}", status, body));
    }
    if is_rumqttd {
        return Err("rumqttd should response 400".to_string());
    }
    Ok(())
}

fn create_application(
    runtime: &Runtime,
    state: &routes::State,
    param: &PostApplication,
) -> Result<PostApplicationResData, String> {
    let server = new_test_server(state)?;

    let req = server
        .post("/coremgr/api/v1/application")
        .add_header(
            header::AUTHORIZATION,
            HeaderValue::from_str(format!("Bearer {}", TOKEN_MANAGER).as_str()).unwrap(),
        )
        .json(&param);
    let resp = runtime.block_on(async { req.await });
    let status = resp.status_code();
    if status != StatusCode::OK {
        return Err(format!(
            "API not 200, status: {}, body: {}",
            status,
            resp.text()
        ));
    }
    let body = resp.text();
    match serde_json::from_str::<PostApplicationRes>(body.as_str()) {
        Err(e) => Err(format!("unexpected response format: {}, body: {}", e, body)),
        Ok(body) => Ok(body.data),
    }
}

fn create_application_dup(
    runtime: &Runtime,
    state: &routes::State,
    param: &PostApplication,
) -> Result<(), String> {
    let server = new_test_server(state)?;

    let req = server
        .post("/coremgr/api/v1/application")
        .add_header(
            header::AUTHORIZATION,
            HeaderValue::from_str(format!("Bearer {}", TOKEN_MANAGER).as_str()).unwrap(),
        )
        .json(&param);
    let resp = runtime.block_on(async { req.await });
    let status = resp.status_code();
    if status != StatusCode::BAD_REQUEST {
        return Err(format!(
            "API not 400, status: {}, body: {}",
            status,
            resp.text()
        ));
    }
    let body = resp.text();
    match serde_json::from_str::<ApiError>(body.as_str()) {
        Err(e) => Err(format!("unexpected response format: {}, body: {}", e, body)),
        Ok(body) => expect(body.code.as_str()).to_equal("err_broker_application_exist"),
    }
}

/// Checks the specified queue that:
/// - Can connect to the specified queue.
/// - Cannot connect to the opposite type of the queue.
async fn check_queue(host_uri: &str, password: &str, unit: &str, code: &str) -> Result<(), String> {
    let username = to_username(QueueType::Application, unit, code);
    let uri = host_uri.replace("://", format!("://{}:{}@", username, password).as_str());

    if host_uri.starts_with("amqp") {
        let opts = AmqpConnectionOptions {
            uri: format!("{}/{}", uri, username),
            ..Default::default()
        };
        let mut conn = AmqpConnection::new(opts)?;
        let opts = AmqpQueueOptions {
            name: format!("broker.{}.uldata", username),
            is_recv: true,
            reliable: true,
            broadcast: false,
            ..Default::default()
        };
        let mut queue = AmqpQueue::new(opts, &conn)?;
        queue.set_msg_handler(Arc::new(TestDummyHandler {}));
        let opts = MqttConnectionOptions {
            uri: uri.replace("amqp", "mqtt"),
            ..Default::default()
        };
        let mut opposite_conn = MqttConnection::new(opts)?;
        if let Err(e) = opposite_conn.connect() {
            return Err(format!("connect opposite broker error: {}", e));
        }
        if let Err(e) = conn.connect() {
            let _ = opposite_conn.close().await;
            return Err(format!("connect AMQP broker error: {}", e));
        }
        if let Err(e) = queue.connect() {
            let _ = conn.close().await;
            let _ = opposite_conn.close().await;
            return Err(format!("connect AMQP queue error: {}", e));
        }

        for _ in 0..WAIT_COUNT {
            time::sleep(Duration::from_millis(WAIT_TICK)).await;
            if queue.status() == QueueStatus::Connected {
                let _ = queue.close().await;
                let _ = conn.close().await;
                let _ = opposite_conn.close().await;
                match opposite_conn.status() {
                    ConnStatus::Connected => {
                        return Err("should not connected to opposite".to_string());
                    }
                    _ => return Ok(()),
                }
            }
        }
        let _ = queue.close().await;
        let _ = conn.close().await;
        let _ = opposite_conn.close().await;
        Err("AMQP queue not connected".to_string())
    } else if host_uri.starts_with("mqtt") {
        let opts = AmqpConnectionOptions {
            uri: format!("{}/{}", uri, username).replace("mqtt", "amqp"),
            ..Default::default()
        };
        let mut opposite_conn = AmqpConnection::new(opts)?;
        let opts = MqttConnectionOptions {
            uri,
            ..Default::default()
        };
        let mut conn = MqttConnection::new(opts)?;
        let opts = MqttQueueOptions {
            name: format!("broker.{}.uldata", username),
            is_recv: true,
            reliable: true,
            broadcast: false,
            ..Default::default()
        };
        let mut queue = MqttQueue::new(opts, &conn)?;
        queue.set_msg_handler(Arc::new(TestDummyHandler {}));
        if let Err(e) = opposite_conn.connect() {
            return Err(format!("connect opposite broker error: {}", e));
        }
        if let Err(e) = conn.connect() {
            let _ = opposite_conn.close().await;
            return Err(format!("connect MQTT broker error: {}", e));
        }
        if let Err(e) = queue.connect() {
            let _ = conn.close().await;
            let _ = opposite_conn.close().await;
            return Err(format!("connect MQTT queue error: {}", e));
        }

        for _ in 0..WAIT_COUNT {
            time::sleep(Duration::from_millis(WAIT_TICK)).await;
            if conn.status() == ConnStatus::Connected {
                let _ = queue.close().await;
                let _ = conn.close().await;
                let _ = opposite_conn.close().await;
                match opposite_conn.status() {
                    ConnStatus::Connected => {
                        return Err("should not connected to opposite".to_string());
                    }
                    _ => return Ok(()),
                }
            }
        }
        let _ = queue.close().await;
        let _ = conn.close().await;
        let _ = opposite_conn.close().await;
        Err("MQTT queue not connected".to_string())
    } else {
        Err("not support scheme".to_string())
    }
}
