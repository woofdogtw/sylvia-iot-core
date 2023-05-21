use std::{collections::HashMap, time::Duration};

use actix_web::{
    http::{header, StatusCode},
    middleware::NormalizePath,
    test::{self, TestRequest},
    App,
};
use base64::{engine::general_purpose, Engine};
use chrono::{DateTime, SubsecRound, Utc};
use general_mq::{
    connection::{GmqConnection, Status as ConnStatus},
    queue::{GmqQueue, Status as QueueStatus},
    AmqpConnection, AmqpConnectionOptions, AmqpQueue, AmqpQueueOptions, MqttConnection,
    MqttConnectionOptions, MqttQueue, MqttQueueOptions,
};
use hex;
use laboratory::{expect, SpecContext};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use tokio::{runtime::Runtime, time};

use sylvia_iot_broker::models::{device, Model};
use sylvia_iot_corelib::err;
use sylvia_iot_coremgr::{
    libs::mq::{self, emqx, rabbitmq, to_username, QueueType},
    routes,
};

use crate::{WAIT_COUNT, WAIT_TICK};

use super::{
    super::{
        libs::{
            create_device, create_unit, test_invalid_param, test_invalid_token, test_list,
            ApiError, TOKEN_MANAGER, TOKEN_MEMBER,
        },
        TestState,
    },
    remove_network, remove_unit, Stats, STATE,
};

#[derive(Serialize)]
struct PostNetwork {
    data: PostNetworkData,
}

#[derive(Serialize)]
struct PostNetworkData {
    code: String,
    #[serde(rename = "unitId")]
    unit_id: Option<String>,
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
struct PatchNetwork {
    data: PatchNetworkData,
}

#[derive(Serialize)]
struct PatchNetworkData {
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
struct PostNetworkUlDataBody {
    data: PostNetworkUlData,
}

#[derive(Serialize)]
pub struct PostNetworkUlData {
    #[serde(rename = "deviceId")]
    pub device_id: String,
    pub payload: String,
}

#[derive(Deserialize)]
struct PostNetworkRes {
    data: PostNetworkResData,
}

#[derive(Deserialize)]
struct PostNetworkResData {
    #[serde(rename = "networkId")]
    network_id: String,
    password: String,
}

#[derive(Deserialize)]
struct GetNetworkRes {
    data: GetNetworkResData,
}

#[derive(Deserialize)]
struct GetNetworkResData {
    #[serde(rename = "networkId")]
    network_id: String,
    code: String,
    #[serde(rename = "unitId")]
    unit_id: Option<String>,
    #[serde(rename = "unitCode")]
    _unit_code: Option<String>,
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
struct GetNetworkStatsRes {
    data: GetNetworkStatsResData,
}

#[derive(Deserialize)]
struct GetNetworkStatsResData {
    dldata: Stats,
}

const UNIT_OWNER: &'static str = "manager";
const UNIT_CODE: &'static str = "manager";
const NET_CODE: &'static str = "manager";
const NET2_CODE: &'static str = "manager2";
const NETP_CODE: &'static str = "public";
const NETP2_CODE: &'static str = "public2";

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
        if let Err(e) = remove_network(client, NETP_CODE).await {
            println!("remove public network error: {}", e);
        }
        if let Err(e) = remove_network(client, NETP2_CODE).await {
            println!("remove public network 2 error: {}", e);
        }
        let cond = device::QueryCond {
            unit_id: Some(UNIT_CODE),
            ..Default::default()
        };
        if let Err(e) = broker_db.device().del(&cond).await {
            println!("remove device error: {}", e);
        }

        let nets = vec![NET_CODE, NET2_CODE];
        for net in nets {
            let username = mq::to_username(QueueType::Network, UNIT_CODE, net);
            let username = username.as_str();
            let q_type = QueueType::Network;
            let _ = rabbitmq::delete_user(client, &mq_opts.0, host, username).await;
            let _ = rabbitmq::delete_vhost(client, &mq_opts.0, host, username).await;
            if state.rumqttd_handles.is_none() {
                let _ = emqx::delete_user(client, &mq_opts.1, host, username).await;
                let _ = emqx::delete_acl(client, &mq_opts.1, host, username).await;
                let _ =
                    emqx::delete_topic_metrics(client, &mq_opts.1, host, q_type, username).await;
            }
        }
        let nets = vec![NETP_CODE, NETP2_CODE];
        for net in nets {
            let username = mq::to_username(QueueType::Network, "", net);
            let username = username.as_str();
            let q_type = QueueType::Network;
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

    let req = TestRequest::get().uri("/coremgr/api/v1/network/count");
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
        "/coremgr/api/v1/network/list",
        TOKEN_MANAGER,
        "networkId,code,unitId,createdAt,modifiedAt,hostUri,name,info",
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
    let mut param = PostNetwork {
        data: PostNetworkData {
            code: NET_CODE.to_string(),
            unit_id: Some(UNIT_CODE.to_string()),
            host_uri: format!("amqp://{}", host),
            name: Some(UNIT_CODE.to_string()),
            info: Some(info),
            ttl: Some(1000),
            length: Some(2),
        },
    };
    let info = create_network(runtime, routes_state, &param)?;
    runtime.block_on(async {
        let host = param.data.host_uri.as_str();
        check_queue(host, info.password.as_str(), UNIT_CODE, NET_CODE).await
    })?;

    param.data.code = NET2_CODE.to_string();
    param.data.host_uri = match state.rumqttd_handles.is_some() {
        false => format!("mqtt://{}", host),
        true => format!("mqtt://{}:{}", host, crate::TEST_RUMQTTD_MQTT_PORT),
    };
    param.data.info = None;
    let info = create_network(runtime, routes_state, &param)?;
    runtime.block_on(async {
        let host = param.data.host_uri.as_str();
        check_queue(host, info.password.as_str(), UNIT_CODE, NET2_CODE).await
    })?;

    param.data.unit_id = None;
    param.data.code = NETP_CODE.to_string();
    param.data.host_uri = format!("amqp://{}", host);
    let info = create_network(runtime, routes_state, &param)?;
    runtime.block_on(async {
        let host = param.data.host_uri.as_str();
        check_queue(host, info.password.as_str(), "", NETP_CODE).await
    })?;

    param.data.code = NETP2_CODE.to_string();
    param.data.host_uri = match state.rumqttd_handles.is_some() {
        false => format!("mqtt://{}", host),
        true => format!("mqtt://{}:{}", host, crate::TEST_RUMQTTD_MQTT_PORT),
    };
    let info = create_network(runtime, routes_state, &param)?;
    runtime.block_on(async {
        let host = param.data.host_uri.as_str();
        check_queue(host, info.password.as_str(), "", NETP2_CODE).await
    })?;

    runtime.block_on(async {
        let mut found_uldata = false;
        let mut found_dldata_result = false;
        let mut found_uldata_public = false;
        let mut found_dldata_result_public = false;
        for _ in 0..WAIT_COUNT {
            let username = mq::to_username(QueueType::Network, UNIT_CODE, NET_CODE);
            let username = username.as_str();
            if !found_uldata {
                if let Ok(stats) =
                    rabbitmq::stats(client, &mq_opts.0, host, username, "uldata").await
                {
                    if stats.consumers >= 1 {
                        found_uldata = true
                    }
                }
            }
            if !found_dldata_result {
                if let Ok(stats) =
                    rabbitmq::stats(client, &mq_opts.0, host, username, "dldata-result").await
                {
                    if stats.consumers >= 1 {
                        found_dldata_result = true
                    }
                }
            }
            let username = mq::to_username(QueueType::Network, "", NETP_CODE);
            let username = username.as_str();
            if !found_uldata_public {
                if let Ok(stats) =
                    rabbitmq::stats(client, &mq_opts.0, host, username, "uldata").await
                {
                    if stats.consumers >= 1 {
                        found_uldata_public = true
                    }
                }
            }
            if !found_dldata_result_public {
                if let Ok(stats) =
                    rabbitmq::stats(client, &mq_opts.0, host, username, "dldata-result").await
                {
                    if stats.consumers >= 1 {
                        found_dldata_result_public = true
                    }
                }
            }
            if found_uldata
                && found_dldata_result
                && found_uldata_public
                && found_dldata_result_public
            {
                return Ok(());
            }
            time::sleep(Duration::from_millis(WAIT_TICK)).await;
        }
        Err("broker does not consume network uldata or dldata-result".to_string())
    })
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

    let param = PostNetwork {
        data: PostNetworkData {
            code: NET_CODE.to_string(),
            unit_id: Some(UNIT_CODE.to_string()),
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
    let req = TestRequest::post()
        .uri("/coremgr/api/v1/network")
        .set_json(param);
    test_invalid_param(runtime, routes_state, req, "err_param")?;

    let req = TestRequest::post()
        .uri("/coremgr/api/v1/network")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", TOKEN_MANAGER)))
        .set_payload("{");
    test_invalid_param(runtime, routes_state, req, "err_param")?;

    let param = PostNetwork {
        data: PostNetworkData {
            code: "code+".to_string(),
            unit_id: Some(UNIT_CODE.to_string()),
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
    let req = TestRequest::post()
        .uri("/coremgr/api/v1/network")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", TOKEN_MANAGER)))
        .set_json(param);
    test_invalid_param(runtime, routes_state, req, "err_param")?;

    let param = PostNetwork {
        data: PostNetworkData {
            code: NET_CODE.to_string(),
            unit_id: Some("".to_string()),
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
    let req = TestRequest::post()
        .uri("/coremgr/api/v1/network")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", TOKEN_MANAGER)))
        .set_json(param);
    test_invalid_param(runtime, routes_state, req, "err_param")?;

    let param = PostNetwork {
        data: PostNetworkData {
            code: NET_CODE.to_string(),
            unit_id: Some(UNIT_CODE.to_string()),
            host_uri: format!("://{}", host),
            name: Some(UNIT_CODE.to_string()),
            info: None,
            ttl: None,
            length: None,
        },
    };
    let req = TestRequest::post()
        .uri("/coremgr/api/v1/network")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", TOKEN_MANAGER)))
        .set_json(param);
    test_invalid_param(runtime, routes_state, req, "err_param")?;

    let param = PostNetwork {
        data: PostNetworkData {
            code: NET_CODE.to_string(),
            unit_id: Some(UNIT_CODE.to_string()),
            host_uri: "mqtt://".to_string(),
            name: Some(UNIT_CODE.to_string()),
            info: None,
            ttl: None,
            length: None,
        },
    };
    let req = TestRequest::post()
        .uri("/coremgr/api/v1/network")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", TOKEN_MANAGER)))
        .set_json(param);
    test_invalid_param(runtime, routes_state, req, "err_param")?;

    let param = PostNetwork {
        data: PostNetworkData {
            code: NET_CODE.to_string(),
            unit_id: Some(UNIT_CODE.to_string()),
            host_uri: "mqtt://".to_string(),
            name: Some(UNIT_CODE.to_string()),
            info: None,
            ttl: None,
            length: None,
        },
    };
    let req = TestRequest::post()
        .uri("/coremgr/api/v1/network")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", TOKEN_MEMBER)))
        .set_json(param);
    test_invalid_param(runtime, routes_state, req, "err_broker_unit_not_exist")?;

    let param = PostNetwork {
        data: PostNetworkData {
            code: NET_CODE.to_string(),
            unit_id: None,
            host_uri: "mqtt://".to_string(),
            name: Some(UNIT_CODE.to_string()),
            info: None,
            ttl: None,
            length: None,
        },
    };
    let req = TestRequest::post()
        .uri("/coremgr/api/v1/network")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", TOKEN_MEMBER)))
        .set_json(param);
    test_invalid_param(runtime, routes_state, req, "err_param")?;

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

    let mut param = PostNetwork {
        data: PostNetworkData {
            code: NET_CODE.to_string(),
            unit_id: Some(UNIT_CODE.to_string()),
            host_uri: format!("amqp://{}", host),
            name: Some(UNIT_CODE.to_string()),
            info: None,
            ttl: Some(1000),
            length: Some(2),
        },
    };
    test_get(runtime, routes_state, &param)?;

    param.data.code = NET2_CODE.to_string();
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

    let req = TestRequest::get().uri("/coremgr/api/v1/network/test");
    test_invalid_param(runtime, routes_state, req, "err_param")?;

    let req = TestRequest::get()
        .uri("/coremgr/api/v1/network/test")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", TOKEN_MANAGER)));
    test_invalid_param(runtime, routes_state, req, "err_not_found")
}

/// Test PATCH API with the following steps:
/// 1. create MQTT network.
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
    let param = PostNetwork {
        data: PostNetworkData {
            code: NET_CODE.to_string(),
            unit_id: Some(UNIT_CODE.to_string()),
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
    let info = create_network(runtime, routes_state, &param)?;
    runtime.block_on(async {
        let host = param.data.host_uri.as_str();
        check_queue(host, info.password.as_str(), UNIT_CODE, NET_CODE).await
    })?;
    let network_id = info.network_id.as_str();

    // Step 2.
    let mut info = Map::<String, Value>::new();
    info.insert("info".to_string(), Value::String("value".to_string()));
    let param = PatchNetwork {
        data: PatchNetworkData {
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
        network_id,
        UNIT_CODE,
        NET_CODE,
    )?;

    // Step 3.
    let param = PatchNetwork {
        data: PatchNetworkData {
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
        network_id,
        UNIT_CODE,
        NET_CODE,
    )?;

    // Step 4.
    let param = PatchNetwork {
        data: PatchNetworkData {
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
        network_id,
        UNIT_CODE,
        NET_CODE,
    )?;

    // Step 5.
    runtime.block_on(async {
        time::sleep(Duration::from_secs(30)).await;
    });
    let param = PatchNetwork {
        data: PatchNetworkData {
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
        network_id,
        UNIT_CODE,
        NET_CODE,
    )?;

    // Step 6.
    let param = PatchNetwork {
        data: PatchNetworkData {
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
        network_id,
        UNIT_CODE,
        NET_CODE,
    )
}

pub fn patch_invalid(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();
    let broker_db = state.broker_db.as_ref().unwrap();
    let host = crate::TEST_MQ_HOST;

    let param = PatchNetwork {
        data: PatchNetworkData {
            host_uri: None,
            name: None,
            info: None,
            ttl: None,
            length: None,
            password: None,
        },
    };
    let req = TestRequest::patch()
        .uri("/coremgr/api/v1/network/test")
        .set_json(&param);
    test_invalid_param(runtime, routes_state, req, "err_param")?;

    let param = PatchNetwork {
        data: PatchNetworkData {
            host_uri: None,
            name: None,
            info: None,
            ttl: None,
            length: None,
            password: None,
        },
    };
    let req = TestRequest::patch()
        .uri("/coremgr/api/v1/network/test")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", TOKEN_MANAGER)))
        .set_json(&param);
    test_invalid_param(runtime, routes_state, req, "err_param")?;

    let param = PatchNetwork {
        data: PatchNetworkData {
            host_uri: None,
            name: Some("name".to_string()),
            info: None,
            ttl: None,
            length: None,
            password: None,
        },
    };
    let req = TestRequest::patch()
        .uri("/coremgr/api/v1/network/test")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", TOKEN_MANAGER)))
        .set_json(&param);
    test_invalid_param(runtime, routes_state, req, "err_not_found")?;

    // Create unit and network.
    match runtime.block_on(async {
        let unit = create_unit(UNIT_CODE, UNIT_OWNER);
        broker_db.unit().add(&unit).await
    }) {
        Err(e) => return Err(format!("add unit model info error: {}", e)),
        Ok(_) => (),
    }
    let param = PostNetwork {
        data: PostNetworkData {
            code: NET_CODE.to_string(),
            unit_id: Some(UNIT_CODE.to_string()),
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
    let info = create_network(runtime, routes_state, &param)?;
    runtime.block_on(async {
        let host = param.data.host_uri.as_str();
        check_queue(host, info.password.as_str(), UNIT_CODE, NET_CODE).await
    })?;
    let network_id = info.network_id.as_str();

    let param = PatchNetwork {
        data: PatchNetworkData {
            host_uri: Some(format!("amqp://{}", host)),
            name: None,
            info: None,
            ttl: None,
            length: None,
            password: None,
        },
    };
    let req = TestRequest::patch()
        .uri(format!("/coremgr/api/v1/network/{}", network_id).as_str())
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", TOKEN_MANAGER)))
        .set_json(&param);
    test_invalid_param(runtime, routes_state, req, "err_param")?;

    let param = PatchNetwork {
        data: PatchNetworkData {
            host_uri: Some(format!("amqp://{}", host)),
            name: None,
            info: None,
            ttl: None,
            length: None,
            password: Some("".to_string()),
        },
    };
    let req = TestRequest::patch()
        .uri(format!("/coremgr/api/v1/network/{}", network_id).as_str())
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", TOKEN_MANAGER)))
        .set_json(&param);
    test_invalid_param(runtime, routes_state, req, "err_param")?;

    let param = PatchNetwork {
        data: PatchNetworkData {
            host_uri: None,
            name: None,
            info: None,
            ttl: None,
            length: None,
            password: Some("".to_string()),
        },
    };
    let req = TestRequest::patch()
        .uri(format!("/coremgr/api/v1/network/{}", network_id).as_str())
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", TOKEN_MANAGER)))
        .set_json(&param);
    test_invalid_param(runtime, routes_state, req, "err_param")?;

    let param = PatchNetwork {
        data: PatchNetworkData {
            host_uri: Some(format!("://{}", host)),
            name: None,
            info: None,
            ttl: None,
            length: None,
            password: Some("test".to_string()),
        },
    };
    let req = TestRequest::patch()
        .uri(format!("/coremgr/api/v1/network/{}", network_id).as_str())
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", TOKEN_MANAGER)))
        .set_json(&param);
    test_invalid_param(runtime, routes_state, req, "err_param")?;

    let param = PatchNetwork {
        data: PatchNetworkData {
            host_uri: Some("mqtt://".to_string()),
            name: None,
            info: None,
            ttl: None,
            length: None,
            password: Some("test".to_string()),
        },
    };
    let req = TestRequest::patch()
        .uri(format!("/coremgr/api/v1/network/{}", network_id).as_str())
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", TOKEN_MANAGER)))
        .set_json(&param);
    test_invalid_param(runtime, routes_state, req, "err_param")?;

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

    let mut param = PostNetwork {
        data: PostNetworkData {
            code: NET_CODE.to_string(),
            unit_id: Some(UNIT_CODE.to_string()),
            host_uri: format!("amqp://{}", host),
            name: Some(UNIT_CODE.to_string()),
            info: None,
            ttl: None,
            length: None,
        },
    };
    let info = create_network(runtime, routes_state, &param)?;
    runtime.block_on(async {
        let host = param.data.host_uri.as_str();
        check_queue(host, info.password.as_str(), UNIT_CODE, NET_CODE).await
    })?;
    let network_id = info.network_id.as_str();
    test_delete(
        runtime,
        routes_state,
        network_id,
        param.data.host_uri.as_str(),
        UNIT_CODE,
        NET_CODE,
        is_rumqttd,
    )?;

    param.data.host_uri = match state.rumqttd_handles.is_some() {
        false => format!("mqtt://{}", host),
        true => format!("mqtt://{}:{}", host, crate::TEST_RUMQTTD_MQTT_PORT),
    };
    let info = create_network(runtime, routes_state, &param)?;
    runtime.block_on(async {
        let host = param.data.host_uri.as_str();
        check_queue(host, info.password.as_str(), UNIT_CODE, NET_CODE).await
    })?;
    let network_id = info.network_id.as_str();
    test_delete(
        runtime,
        routes_state,
        network_id,
        param.data.host_uri.as_str(),
        UNIT_CODE,
        NET_CODE,
        is_rumqttd,
    )
}

pub fn delete_invalid(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    let req = TestRequest::delete().uri("/coremgr/api/v1/network/test");
    test_invalid_param(runtime, routes_state, req, "err_param")?;

    let req = TestRequest::delete()
        .uri("/coremgr/api/v1/network/test")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", TOKEN_MANAGER)));
    test_invalid_param(runtime, routes_state, req, "err_not_found")
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

    let mut param = PostNetwork {
        data: PostNetworkData {
            code: NET_CODE.to_string(),
            unit_id: Some(UNIT_CODE.to_string()),
            host_uri: format!("amqp://{}", host),
            name: Some(UNIT_CODE.to_string()),
            info: None,
            ttl: None,
            length: None,
        },
    };
    let info = create_network(runtime, routes_state, &param)?;
    runtime.block_on(async {
        let host = param.data.host_uri.as_str();
        check_queue(host, info.password.as_str(), UNIT_CODE, NET_CODE).await
    })?;
    let network_id = info.network_id.as_str();
    let username = mq::to_username(QueueType::Network, UNIT_CODE, NET_CODE);
    let username = username.as_str();
    let payload = general_purpose::STANDARD.encode("amqp");
    if let Err(e) = runtime.block_on(async {
        rabbitmq::publish_message(client, &mq_opts.0, host, username, "dldata", payload).await
    }) {
        return Err(format!("publish AMQP payload error: {}", e));
    }
    test_stats(runtime, routes_state, network_id, is_rumqttd)?;

    param.data.code = NET2_CODE.to_string();
    param.data.host_uri = match state.rumqttd_handles.is_some() {
        false => format!("mqtt://{}", host),
        true => format!("mqtt://{}:{}", host, crate::TEST_RUMQTTD_MQTT_PORT),
    };
    let info = create_network(runtime, routes_state, &param)?;
    runtime.block_on(async {
        let host = param.data.host_uri.as_str();
        check_queue(host, info.password.as_str(), UNIT_CODE, NET2_CODE).await
    })?;
    let network_id = info.network_id.as_str();
    if !is_rumqttd {
        let username = mq::to_username(QueueType::Network, UNIT_CODE, NET2_CODE);
        let username = username.as_str();
        let payload = general_purpose::STANDARD.encode("mqtt");
        if let Err(e) = runtime.block_on(async {
            emqx::publish_message(client, &mq_opts.1, host, username, "dldata", payload).await
        }) {
            return Err(format!("publish MQTT payload error: {}", e));
        }
    }
    test_stats(runtime, routes_state, network_id, is_rumqttd)
}

pub fn stats_invalid(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    let req = TestRequest::get().uri("/coremgr/api/v1/network/test/stats");
    test_invalid_param(runtime, routes_state, req, "err_param")?;

    let req = TestRequest::get()
        .uri("/coremgr/api/v1/network/test/stats")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", TOKEN_MANAGER)));
    test_invalid_param(runtime, routes_state, req, "err_not_found")
}

pub fn uldata(context: &mut SpecContext<TestState>) -> Result<(), String> {
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
        let device = create_device(UNIT_CODE, NET_CODE, device, false);
        broker_db.device().add(&device).await
    }) {
        Err(e) => return Err(format!("add device model info error: {}", e)),
        Ok(_) => (),
    }

    let username = mq::to_username(QueueType::Network, UNIT_CODE, NET_CODE);
    let username = username.as_str();
    let param = PostNetwork {
        data: PostNetworkData {
            code: NET_CODE.to_string(),
            unit_id: Some(UNIT_CODE.to_string()),
            host_uri: format!("amqp://{}", host),
            name: Some(UNIT_CODE.to_string()),
            info: None,
            ttl: None,
            length: None,
        },
    };
    let info = create_network(runtime, routes_state, &param)?;
    runtime.block_on(async {
        let host = param.data.host_uri.as_str();
        check_queue(host, info.password.as_str(), UNIT_CODE, NET_CODE).await
    })?;
    let network_id = info.network_id.as_str();
    let body = PostNetworkUlDataBody {
        data: PostNetworkUlData {
            device_id: device.to_string(),
            payload: hex::encode("amqp"),
        },
    };
    test_uldata(runtime, routes_state, network_id, &body, false)?;
    runtime.block_on(async {
        for _ in 0..WAIT_COUNT {
            time::sleep(Duration::from_millis(WAIT_TICK)).await;
            match rabbitmq::stats(client, &mq_opts.0, host, username, "uldata").await {
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

    let username = mq::to_username(QueueType::Network, UNIT_CODE, NET2_CODE);
    let username = username.as_str();
    let param = PostNetwork {
        data: PostNetworkData {
            code: NET2_CODE.to_string(),
            unit_id: Some(UNIT_CODE.to_string()),
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
    let info = create_network(runtime, routes_state, &param)?;
    runtime.block_on(async {
        let host = param.data.host_uri.as_str();
        check_queue(host, info.password.as_str(), UNIT_CODE, NET2_CODE).await
    })?;
    let network_id = info.network_id.as_str();
    let body = PostNetworkUlDataBody {
        data: PostNetworkUlData {
            device_id: device.to_string(),
            payload: hex::encode("mqtt"),
        },
    };
    runtime.block_on(async { time::sleep(Duration::from_secs(2)).await });
    test_uldata(runtime, routes_state, network_id, &body, is_rumqttd)?;
    if is_rumqttd {
        return Ok(());
    }
    runtime.block_on(async {
        for _ in 0..WAIT_COUNT {
            time::sleep(Duration::from_millis(WAIT_TICK)).await;
            match emqx::stats(client, &mq_opts.1, host, username, "uldata").await {
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

pub fn uldata_invalid(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();
    let broker_db = state.broker_db.as_ref().unwrap();
    let host = crate::TEST_MQ_HOST;

    let mut body = PostNetworkUlDataBody {
        data: PostNetworkUlData {
            device_id: "device".to_string(),
            payload: hex::encode("payload"),
        },
    };
    let req = TestRequest::post()
        .uri("/coremgr/api/v1/network/test/uldata")
        .set_json(&body);
    test_invalid_param(runtime, routes_state, req, "err_param")?;

    let req = TestRequest::post()
        .uri("/coremgr/api/v1/network/test/uldata")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", TOKEN_MANAGER)))
        .set_json(&body);
    test_invalid_param(runtime, routes_state, req, "err_not_found")?;

    // Create unit, network, device and network.
    match runtime.block_on(async {
        let unit = create_unit(UNIT_CODE, UNIT_OWNER);
        broker_db.unit().add(&unit).await
    }) {
        Err(e) => return Err(format!("add unit model info error: {}", e)),
        Ok(_) => (),
    }
    match runtime.block_on(async {
        let device = create_device(UNIT_CODE, NET_CODE, "addr", false);
        broker_db.device().add(&device).await
    }) {
        Err(e) => return Err(format!("add device model info error: {}", e)),
        Ok(_) => (),
    }
    let param = PostNetwork {
        data: PostNetworkData {
            code: NET_CODE.to_string(),
            unit_id: Some(UNIT_CODE.to_string()),
            host_uri: format!("amqp://{}", host),
            name: Some(UNIT_CODE.to_string()),
            info: None,
            ttl: None,
            length: None,
        },
    };
    let info = create_network(runtime, routes_state, &param)?;
    runtime.block_on(async {
        let host = param.data.host_uri.as_str();
        check_queue(host, info.password.as_str(), UNIT_CODE, NET_CODE).await
    })?;
    let network_id = info.network_id.as_str();

    body.data.device_id = "".to_string();
    let req = TestRequest::post()
        .uri(format!("/coremgr/api/v1/network/{}/uldata", network_id).as_str())
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", TOKEN_MANAGER)))
        .set_json(&body);
    test_invalid_param(runtime, routes_state, req, "err_param")?;

    body.data.device_id = "device".to_string();
    body.data.payload = "payload".to_string();
    let req = TestRequest::post()
        .uri(format!("/coremgr/api/v1/network/{}/uldata", network_id).as_str())
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", TOKEN_MANAGER)))
        .set_json(&body);
    test_invalid_param(runtime, routes_state, req, "err_param")?;

    body.data.device_id = "test".to_string();
    body.data.payload = hex::encode("payload");
    let req = TestRequest::post()
        .uri(format!("/coremgr/api/v1/network/{}/uldata", network_id).as_str())
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", TOKEN_MANAGER)))
        .set_json(&body);
    test_invalid_param(runtime, routes_state, req, "err_broker_device_not_exist")
}

fn test_get(runtime: &Runtime, state: &routes::State, param: &PostNetwork) -> Result<(), String> {
    let start = Utc::now().trunc_subsecs(3);
    let info = create_network(runtime, state, &param)?;
    let end = Utc::now().trunc_subsecs(3);

    let mut app = runtime.block_on(async {
        test::init_service(
            App::new()
                .wrap(NormalizePath::trim())
                .service(routes::new_service(state)),
        )
        .await
    });

    let req = TestRequest::get()
        .uri(format!("/coremgr/api/v1/network/{}", info.network_id).as_str())
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", TOKEN_MANAGER)))
        .to_request();
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    if resp.status() != StatusCode::OK {
        let status = resp.status();
        let body = runtime.block_on(async { test::read_body(resp).await });
        let body = match String::from_utf8(body.to_vec()) {
            Err(e) => format!("(no body with error: {})", e),
            Ok(body) => body,
        };
        return Err(format!("API not 200, status: {}, body: {}", status, body));
    }
    let body = runtime.block_on(async { test::read_body(resp).await });
    let body = match String::from_utf8(body.to_vec()) {
        Err(e) => return Err(format!("response body is not UTF-8: {}", e)),
        Ok(body) => match serde_json::from_str::<GetNetworkRes>(body.as_str()) {
            Err(e) => return Err(format!("unexpected response format: {}, body: {}", e, body)),
            Ok(body) => body.data,
        },
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
    expect(body.network_id.as_str()).to_equal(info.network_id.as_str())?;
    expect(body.code.as_str()).to_equal(param.data.code.as_str())?;
    expect(body.unit_id.as_ref()).to_equal(param.data.unit_id.as_ref())?;
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
    param: &PatchNetwork,
    network_id: &str,
    unit: &str,
    code: &str,
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
        .uri(format!("/coremgr/api/v1/network/{}", network_id).as_str())
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", TOKEN_MANAGER)))
        .set_json(param)
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

    let req = TestRequest::get()
        .uri(format!("/coremgr/api/v1/network/{}", network_id).as_str())
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", TOKEN_MANAGER)))
        .to_request();
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    if resp.status() != StatusCode::OK {
        let status = resp.status();
        let body = runtime.block_on(async { test::read_body(resp).await });
        let body = match String::from_utf8(body.to_vec()) {
            Err(e) => format!("(no body with error: {})", e),
            Ok(body) => body,
        };
        return Err(format!("API not 200, status: {}, body: {}", status, body));
    }
    let body = runtime.block_on(async { test::read_body(resp).await });
    let body = match String::from_utf8(body.to_vec()) {
        Err(e) => return Err(format!("response body is not UTF-8: {}", e)),
        Ok(body) => match serde_json::from_str::<GetNetworkRes>(body.as_str()) {
            Err(e) => return Err(format!("unexpected response format: {}, body: {}", e, body)),
            Ok(body) => body.data,
        },
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
    network_id: &str,
    host: &str,
    unit: &str,
    code: &str,
    is_rumqttd: bool,
) -> Result<(), String> {
    let mut app = runtime.block_on(async {
        test::init_service(
            App::new()
                .wrap(NormalizePath::trim())
                .service(routes::new_service(state)),
        )
        .await
    });

    let req = TestRequest::delete()
        .uri(format!("/coremgr/api/v1/network/{}", network_id).as_str())
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

    runtime.block_on(async {
        let username = mq::to_username(QueueType::Network, unit, code);
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
    network_id: &str,
    is_rumqttd: bool,
) -> Result<(), String> {
    let mut app = runtime.block_on(async {
        test::init_service(
            App::new()
                .wrap(NormalizePath::trim())
                .service(routes::new_service(state)),
        )
        .await
    });

    runtime.block_on(async {
        for _ in 0..WAIT_COUNT {
            let req = TestRequest::get()
                .uri(format!("/coremgr/api/v1/network/{}/stats", network_id).as_str())
                .insert_header((header::AUTHORIZATION, format!("Bearer {}", TOKEN_MANAGER)))
                .to_request();
            let resp = test::call_service(&mut app, req).await;
            if resp.status() != StatusCode::OK {
                let status = resp.status();
                let body = test::read_body(resp).await;
                let body = match String::from_utf8(body.to_vec()) {
                    Err(e) => format!("(no body with error: {})", e),
                    Ok(body) => body,
                };
                return Err(format!("API not 200, status: {}, body: {}", status, body));
            }
            let body = test::read_body(resp).await;
            let stats = match String::from_utf8(body.to_vec()) {
                Err(e) => return Err(format!("response body is not UTF-8: {}", e)),
                Ok(body) => match serde_json::from_str::<GetNetworkStatsRes>(body.as_str()) {
                    Err(e) => {
                        return Err(format!("unexpected response format: {}, body: {}", e, body))
                    }
                    Ok(body) => body.data.dldata,
                },
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

fn test_uldata(
    runtime: &Runtime,
    state: &routes::State,
    network_id: &str,
    body: &PostNetworkUlDataBody, // use "amqp" as payload to check stats
    is_rumqttd: bool,
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
        .uri(format!("/coremgr/api/v1/network/{}/uldata", network_id).as_str())
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", TOKEN_MANAGER)))
        .set_json(body)
        .to_request();
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    if resp.status() != StatusCode::NO_CONTENT {
        let status = resp.status();
        let body = runtime.block_on(async { test::read_body(resp).await });
        let body = match String::from_utf8(body.to_vec()) {
            Err(e) => format!("(no body with error: {})", e),
            Ok(body) => body,
        };
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

fn create_network(
    runtime: &Runtime,
    state: &routes::State,
    param: &PostNetwork,
) -> Result<PostNetworkResData, String> {
    let mut app = runtime.block_on(async {
        test::init_service(
            App::new()
                .wrap(NormalizePath::trim())
                .service(routes::new_service(state)),
        )
        .await
    });

    let req = TestRequest::post()
        .uri("/coremgr/api/v1/network")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", TOKEN_MANAGER)))
        .set_json(param)
        .to_request();
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    if resp.status() != StatusCode::OK {
        let status = resp.status();
        let body = runtime.block_on(async { test::read_body(resp).await });
        let body = match String::from_utf8(body.to_vec()) {
            Err(e) => format!("(no body with error: {})", e),
            Ok(body) => body,
        };
        return Err(format!("API not 200, status: {}, body: {}", status, body));
    }
    let body = runtime.block_on(async { test::read_body(resp).await });
    match String::from_utf8(body.to_vec()) {
        Err(e) => Err(format!("response body is not UTF-8: {}", e)),
        Ok(body) => match serde_json::from_str::<PostNetworkRes>(body.as_str()) {
            Err(e) => Err(format!("unexpected response format: {}, body: {}", e, body)),
            Ok(body) => Ok(body.data),
        },
    }
}

/// Checks the specified queue that:
/// - Can connect to the specified queue.
/// - Cannot connect to the opposite type of the queue.
async fn check_queue(host_uri: &str, password: &str, unit: &str, code: &str) -> Result<(), String> {
    let username = to_username(QueueType::Network, unit, code);
    let uri = host_uri.replace("://", format!("://{}:{}@", username, password).as_str());

    if host_uri.starts_with("amqp") {
        let opts = AmqpConnectionOptions {
            uri: format!("{}/{}", uri, username),
            ..Default::default()
        };
        let mut conn = AmqpConnection::new(opts)?;
        let opts = AmqpQueueOptions {
            name: format!("broker.{}.dldata", username),
            is_recv: true,
            reliable: true,
            broadcast: false,
            ..Default::default()
        };
        let mut queue = AmqpQueue::new(opts, &conn)?;
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
                        return Err("should not connected to opposite".to_string())
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
            name: format!("broker.{}.dldata", username),
            is_recv: true,
            reliable: true,
            broadcast: false,
            ..Default::default()
        };
        let mut queue = MqttQueue::new(opts, &conn)?;
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
                        return Err("should not connected to opposite".to_string())
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
