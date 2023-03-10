use std::{collections::HashMap, sync::Arc, time::Duration};

use chrono::{TimeZone, Utc};
use general_mq::{
    connection::Connection as MqConnection,
    queue::{Queue as MqQueue, Status},
    AmqpConnection, AmqpConnectionOptions, AmqpQueueOptions, MqttConnection, MqttConnectionOptions,
    MqttQueueOptions, Queue, QueueOptions,
};
use laboratory::SpecContext;
use serde::Serialize;
use serde_json::{Map, Value};
use tokio::time;

use sylvia_iot_corelib::{constants::MqEngine, strings};
use sylvia_iot_data::{
    libs::{config::DataData as DataMqConfig, mq::coremgr},
    models::{
        coremgr_opdata::{ListOptions as CmOpDataOpts, ListQueryCond as CmOpDataCond},
        Model,
    },
};

use super::STATE;
use crate::{TestState, WAIT_COUNT, WAIT_TICK};

#[derive(Serialize)]
#[serde(untagged)]
enum SendDataMsg {
    CmOpData { kind: String, data: CmOpData },
}

#[derive(Clone, Serialize)]
struct CmOpData {
    #[serde(rename = "dataId")]
    data_id: String,
    #[serde(rename = "reqTime")]
    req_time: String,
    #[serde(rename = "resTime")]
    res_time: String,
    #[serde(rename = "latencyMs")]
    latency_ms: i64,
    status: isize,
    #[serde(rename = "sourceIp")]
    source_ip: String,
    method: String,
    path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    body: Option<Map<String, Value>>,
    #[serde(rename = "userId")]
    user_id: String,
    #[serde(rename = "clientId")]
    client_id: String,
    #[serde(rename = "errCode", skip_serializing_if = "Option::is_none")]
    err_code: Option<String>,
    #[serde(rename = "errMessage", skip_serializing_if = "Option::is_none")]
    err_message: Option<String>,
}

pub fn before_all_fn(state: &mut HashMap<&'static str, TestState>) -> () {
    let state = state.get_mut(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let mq_engine = state.mq_engine.as_ref().unwrap();

    let mut queue = match mq_engine.as_str() {
        MqEngine::RABBITMQ => {
            let opts = AmqpConnectionOptions {
                uri: crate::TEST_AMQP_HOST_URI.to_string(),
                ..Default::default()
            };
            let mut conn = match AmqpConnection::new(opts) {
                Err(e) => panic!("create AMQP connection error: {}", e),
                Ok(conn) => conn,
            };
            let _ = conn.connect();
            state.mq_conn = Some(Box::new(conn.clone()));

            let opts = QueueOptions::Amqp(
                AmqpQueueOptions {
                    name: "coremgr.data".to_string(),
                    is_recv: false,
                    reliable: true,
                    broadcast: false,
                    ..Default::default()
                },
                &conn,
            );
            match Queue::new(opts) {
                Err(e) => panic!("create AMQP send queue error: {}", e),
                Ok(q) => q,
            }
        }
        MqEngine::EMQX => {
            let opts = MqttConnectionOptions {
                uri: crate::TEST_MQTT_HOST_URI.to_string(),
                ..Default::default()
            };
            let mut conn = match MqttConnection::new(opts) {
                Err(e) => panic!("create MQTT connection error: {}", e),
                Ok(conn) => conn,
            };
            let _ = conn.connect();
            state.mq_conn = Some(Box::new(conn.clone()));

            let opts = QueueOptions::Mqtt(
                MqttQueueOptions {
                    name: "coremgr.data".to_string(),
                    is_recv: false,
                    reliable: true,
                    broadcast: false,
                    shared_prefix: Some("$share/sylvia-iot-data/".to_string()),
                    ..Default::default()
                },
                &conn,
            );
            match Queue::new(opts) {
                Err(e) => panic!("create MQTT send queue error: {}", e),
                Ok(q) => q,
            }
        }
        s => panic!("unsupport engine {}", s),
    };
    if let Err(e) = queue.connect() {
        panic!("connect queue error: {}", e);
    }
    state.data_queue = Some(queue.clone());

    runtime.block_on(async {
        for _ in 0..WAIT_COUNT {
            if queue.status() == Status::Connected {
                return;
            }
            time::sleep(Duration::from_millis(WAIT_TICK)).await;
        }
        panic!("cannot connect to send queue");
    });
}

pub fn after_all_fn(state: &mut HashMap<&'static str, TestState>) -> () {
    let state = state.get_mut(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();

    if let Some(mut q) = state.data_queue.take() {
        runtime.block_on(async move {
            let _ = q.close().await;
        });
    }

    if let Some(mut conn) = state.mq_conn.take() {
        runtime.block_on(async move {
            let _ = conn.close().await;
        });
    }
}

/// Test new data queue with default options.
pub fn new_default(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let mut state = context.state.borrow_mut();
    let state = state.get_mut(STATE).unwrap();

    create_default_queue(state)
}

/// Test new data queue with manual options.
pub fn new_manual(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let mut state = context.state.borrow_mut();
    let state = state.get_mut(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let mq_engine = state.mq_engine.as_ref().unwrap().as_str();
    let mq_conns = state.recv_conns.as_mut().unwrap();
    let model = Arc::new(state.sqlite.as_ref().unwrap().clone());

    let queue = match mq_engine {
        MqEngine::RABBITMQ => {
            let conf = DataMqConfig {
                url: Some(crate::TEST_AMQP_HOST_URI.to_string()),
                prefetch: Some(1),
                ..Default::default()
            };
            match coremgr::new(model, mq_conns, &conf) {
                Err(e) => return Err(e.to_string()),
                Ok(q) => q,
            }
        }
        MqEngine::EMQX => {
            let conf = DataMqConfig {
                url: Some(crate::TEST_MQTT_HOST_URI.to_string()),
                shared_prefix: Some("$share/sylvia-iot-data/".to_string()),
                ..Default::default()
            };
            match coremgr::new(model, mq_conns, &conf) {
                Err(e) => return Err(e.to_string()),
                Ok(q) => q,
            }
        }
        _ => return Err(format!("unsupport MQ engine {}", mq_engine)),
    };
    for _ in 0..WAIT_COUNT {
        if queue.status() == Status::Connected {
            state.recv_queue = Some(queue);
            return Ok(());
        }
        runtime.block_on(async { time::sleep(Duration::from_millis(WAIT_TICK)).await });
    }
    state.recv_queue = Some(queue);
    Err("queue is not connected".to_string())
}

/// Test new data queue with the same host.
pub fn new_same_host(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let mut state = context.state.borrow_mut();
    let state = state.get_mut(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let mq_engine = state.mq_engine.as_ref().unwrap().as_str();
    let mq_conns = state.recv_conns.as_mut().unwrap();
    let model = Arc::new(state.sqlite.as_ref().unwrap().clone());

    let mut queue = match mq_engine {
        MqEngine::RABBITMQ => {
            let conf = DataMqConfig {
                url: Some(crate::TEST_AMQP_HOST_URI.to_string()),
                prefetch: Some(1),
                ..Default::default()
            };
            match coremgr::new(model, mq_conns, &conf) {
                Err(e) => return Err(e.to_string()),
                Ok(q) => q,
            }
        }
        MqEngine::EMQX => {
            let conf = DataMqConfig {
                url: Some(crate::TEST_MQTT_HOST_URI.to_string()),
                shared_prefix: Some("$share/sylvia-iot-data/".to_string()),
                ..Default::default()
            };
            match coremgr::new(model, mq_conns, &conf) {
                Err(e) => return Err(e.to_string()),
                Ok(q) => q,
            }
        }
        _ => return Err(format!("unsupport MQ engine {}", mq_engine)),
    };
    runtime.block_on(async move {
        for _ in 0..WAIT_COUNT {
            if queue.status() == Status::Connected {
                if let Err(e) = queue.close().await {
                    return Err(format!("close queue error: {}", e));
                }
                return Ok(());
            }
            time::sleep(Duration::from_millis(WAIT_TICK)).await;
        }
        if let Err(e) = queue.close().await {
            return Err(format!("close queue error: {}", e));
        }
        Err("queue is not connected".to_string())
    })?;

    let model = Arc::new(state.sqlite.as_ref().unwrap().clone());
    let queue = match mq_engine {
        MqEngine::RABBITMQ => {
            let conf = DataMqConfig {
                url: Some(crate::TEST_AMQP_HOST_URI.to_string()),
                prefetch: Some(1),
                ..Default::default()
            };
            match coremgr::new(model, mq_conns, &conf) {
                Err(e) => return Err(e.to_string()),
                Ok(q) => q,
            }
        }
        MqEngine::EMQX => {
            let conf = DataMqConfig {
                url: Some(crate::TEST_MQTT_HOST_URI.to_string()),
                shared_prefix: Some("$share/sylvia-iot-data/".to_string()),
                ..Default::default()
            };
            match coremgr::new(model, mq_conns, &conf) {
                Err(e) => return Err(e.to_string()),
                Ok(q) => q,
            }
        }
        _ => return Err(format!("unsupport MQ engine {}", mq_engine)),
    };
    for _ in 0..WAIT_COUNT {
        if queue.status() == Status::Connected {
            state.recv_queue = Some(queue);
            return Ok(());
        }
        runtime.block_on(async { time::sleep(Duration::from_millis(WAIT_TICK)).await });
    }
    state.recv_queue = Some(queue);

    Ok(())
}

/// Test new managers with wrong options.
pub fn new_wrong_opts(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let mut state = context.state.borrow_mut();
    let state = state.get_mut(STATE).unwrap();
    let mq_conns = state.recv_conns.as_mut().unwrap();
    let model = state.sqlite.as_ref().unwrap();

    let conf = DataMqConfig {
        url: None,
        prefetch: Some(1),
        ..Default::default()
    };
    if let Ok(q) = coremgr::new(Arc::new(model.clone()), mq_conns, &conf) {
        state.recv_queue = Some(q);
        return Err("data queue should not be created".to_string());
    }

    let conf = DataMqConfig {
        url: Some("".to_string()),
        prefetch: Some(1),
        ..Default::default()
    };
    if let Ok(q) = coremgr::new(Arc::new(model.clone()), mq_conns, &conf) {
        state.recv_queue = Some(q);
        return Err("data queue should not be created".to_string());
    }

    let conf = DataMqConfig {
        url: Some("http://localhost".to_string()),
        prefetch: Some(1),
        ..Default::default()
    };
    if let Ok(q) = coremgr::new(Arc::new(model.clone()), mq_conns, &conf) {
        state.recv_queue = Some(q);
        return Err("data queue should not be created".to_string());
    }

    Ok(())
}

/// Test operation kind data.
pub fn operation(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let mut state = context.state.borrow_mut();
    let state = state.get_mut(STATE).unwrap();

    create_default_queue(state)?;
    let send_queue = state.data_queue.as_mut().unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap();

    runtime.block_on(async {
        let now = Utc::now();
        let data = SendDataMsg::CmOpData {
            kind: "operation".to_string(),
            data: CmOpData {
                data_id: "data_id1".to_string(),
                req_time: strings::time_str(&now),
                res_time: strings::time_str(
                    &(Utc.timestamp_nanos(now.timestamp_nanos() + 1000000)),
                ),
                latency_ms: 1,
                status: 200,
                source_ip: "::1".to_string(),
                method: "GET".to_string(),
                path: "/path".to_string(),
                body: None,
                user_id: "user_id1".to_string(),
                client_id: "client_id1".to_string(),
                err_code: None,
                err_message: None,
            },
        };
        let payload = match serde_json::to_vec(&data) {
            Err(e) => return Err(format!("marshal data1 error: {}", e)),
            Ok(payload) => payload,
        };
        if let Err(e) = send_queue.send_msg(payload).await {
            return Err(format!("send data1 error: {}", e));
        }

        let data = SendDataMsg::CmOpData {
            kind: "operation".to_string(),
            data: CmOpData {
                data_id: "data_id2".to_string(),
                req_time: strings::time_str(&now),
                res_time: strings::time_str(
                    &(Utc.timestamp_nanos(now.timestamp_nanos() + 2000000)),
                ),
                latency_ms: 2,
                status: 400,
                source_ip: "::1".to_string(),
                method: "POST".to_string(),
                path: "/path".to_string(),
                body: Some(Map::new()),
                user_id: "user_id2".to_string(),
                client_id: "client_id2".to_string(),
                err_code: Some("err_param".to_string()),
                err_message: Some("error paramter".to_string()),
            },
        };
        let payload = match serde_json::to_vec(&data) {
            Err(e) => return Err(format!("marshal data2 error: {}", e)),
            Ok(payload) => payload,
        };
        if let Err(e) = send_queue.send_msg(payload).await {
            return Err(format!("send data2 error: {}", e));
        }

        let mut count = 0;
        for _ in 0..20 {
            let cond = CmOpDataCond::default();
            let opts = CmOpDataOpts {
                cond: &cond,
                offset: None,
                limit: None,
                sort: None,
                cursor_max: None,
            };
            match model.coremgr_opdata().list(&opts, None).await {
                Err(e) => return Err(format!("get data error: {}", e)),
                Ok((list, _cursor)) => {
                    count = list.len();
                    if count != 2 {
                        time::sleep(Duration::from_millis(100)).await;
                        continue;
                    }
                    return Ok(());
                }
            }
        }
        Err(format!("data count error: {}/2", count))
    })
}

/// Test operation kind data with wrong content.
pub fn operation_wrong(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let mut state = context.state.borrow_mut();
    let state = state.get_mut(STATE).unwrap();

    create_default_queue(state)?;
    let send_queue = state.data_queue.as_mut().unwrap();
    let runtime = state.runtime.as_ref().unwrap();

    runtime.block_on(async {
        if let Err(e) = send_queue.send_msg(vec![]).await {
            return Err(format!("send kind error: {}", e));
        }

        let now = Utc::now();
        let mut content = CmOpData {
            data_id: "data_id1".to_string(),
            req_time: "2022-20-28T00:00:00Z".to_string(),
            res_time: strings::time_str(&now),
            latency_ms: 1,
            status: 200,
            source_ip: "::1".to_string(),
            method: "GET".to_string(),
            path: "/path".to_string(),
            body: None,
            user_id: "user_id1".to_string(),
            client_id: "client_id1".to_string(),
            err_code: None,
            err_message: None,
        };
        let data = SendDataMsg::CmOpData {
            kind: "operation".to_string(),
            data: content.clone(),
        };
        let payload = match serde_json::to_vec(&data) {
            Err(e) => return Err(format!("marshal wrong req_time error: {}", e)),
            Ok(payload) => payload,
        };
        if let Err(e) = send_queue.send_msg(payload).await {
            return Err(format!("send wrong req_time error: {}", e));
        }

        content.req_time = strings::time_str(&now);
        content.res_time = "2022-20-28T00:00:00Z".to_string();
        let data = SendDataMsg::CmOpData {
            kind: "operation".to_string(),
            data: content.clone(),
        };
        let payload = match serde_json::to_vec(&data) {
            Err(e) => return Err(format!("marshal wrong res_time error: {}", e)),
            Ok(payload) => payload,
        };
        if let Err(e) = send_queue.send_msg(payload).await {
            return Err(format!("send wrong res_time error: {}", e));
        }

        Ok(())
    })
}

/// Create receive queue for the mq library with default configurations.
fn create_default_queue(state: &mut TestState) -> Result<(), String> {
    let runtime = state.runtime.as_ref().unwrap();
    let mq_engine = state.mq_engine.as_ref().unwrap().as_str();
    let mq_conns = state.recv_conns.as_mut().unwrap();
    let model = Arc::new(state.sqlite.as_ref().unwrap().clone());

    let queue = match mq_engine {
        MqEngine::RABBITMQ => {
            let conf = DataMqConfig {
                url: Some(crate::TEST_AMQP_HOST_URI.to_string()),
                ..Default::default()
            };
            match coremgr::new(model, mq_conns, &conf) {
                Err(e) => return Err(e.to_string()),
                Ok(q) => q,
            }
        }
        MqEngine::EMQX => {
            let conf = DataMqConfig {
                url: Some(crate::TEST_MQTT_HOST_URI.to_string()),
                shared_prefix: Some("$share/sylvia-iot-data/".to_string()),
                ..Default::default()
            };
            match coremgr::new(model, mq_conns, &conf) {
                Err(e) => return Err(e.to_string()),
                Ok(q) => q,
            }
        }
        _ => return Err(format!("unsupport MQ engine {}", mq_engine)),
    };
    for _ in 0..WAIT_COUNT {
        if queue.status() == Status::Connected {
            state.recv_queue = Some(queue);
            return Ok(());
        }
        runtime.block_on(async { time::sleep(Duration::from_millis(WAIT_TICK)).await });
    }
    state.recv_queue = Some(queue);
    Err("queue is not connected".to_string())
}
