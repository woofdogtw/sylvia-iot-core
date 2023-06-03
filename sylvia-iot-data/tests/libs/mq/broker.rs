use std::{collections::HashMap, sync::Arc, time::Duration};

use chrono::{TimeZone, Utc};
use laboratory::SpecContext;
use serde::Serialize;
use serde_json::{Map, Value};
use tokio::time;

use general_mq::{
    connection::GmqConnection,
    queue::{GmqQueue, Status},
    AmqpConnection, AmqpConnectionOptions, AmqpQueueOptions, MqttConnection, MqttConnectionOptions,
    MqttQueueOptions, Queue, QueueOptions,
};
use sylvia_iot_corelib::{constants::MqEngine, strings};
use sylvia_iot_data::{
    libs::{config::DataData as DataMqConfig, mq::broker},
    models::{
        application_dldata::{ListOptions as AppDlDataOpts, ListQueryCond as AppDlDataCond},
        application_uldata::{ListOptions as AppUlDataOpts, ListQueryCond as AppUlDataCond},
        network_dldata::{ListOptions as NetDlDataOpts, ListQueryCond as NetDlDataCond},
        network_uldata::{ListOptions as NetUlDataOpts, ListQueryCond as NetUlDataCond},
        Model,
    },
};

use super::STATE;
use crate::{TestState, WAIT_COUNT, WAIT_TICK};

#[derive(Serialize)]
#[serde(untagged)]
enum SendDataMsg {
    AppUlData { kind: String, data: AppUlData },
    AppDlData { kind: String, data: AppDlData },
    AppDlDataResult { kind: String, data: AppDlDataResult },
    NetUlData { kind: String, data: NetUlData },
    NetDlData { kind: String, data: NetDlData },
    NetDlDataResult { kind: String, data: NetDlDataResult },
}

#[derive(Clone, Serialize)]
struct AppUlData {
    #[serde(rename = "dataId")]
    data_id: String,
    proc: String,
    #[serde(rename = "pub")]
    publish: String,
    #[serde(rename = "unitCode")]
    unit_code: Option<String>,
    #[serde(rename = "networkCode")]
    network_code: String,
    #[serde(rename = "networkAddr")]
    network_addr: String,
    #[serde(rename = "unitId")]
    unit_id: String,
    #[serde(rename = "deviceId")]
    device_id: String,
    time: String,
    profile: String,
    data: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    extension: Option<Map<String, Value>>,
}

#[derive(Clone, Serialize)]
struct AppDlData {
    #[serde(rename = "dataId")]
    data_id: String,
    proc: String,
    status: i32,
    #[serde(rename = "unitId")]
    unit_id: String,
    #[serde(rename = "deviceId", skip_serializing_if = "Option::is_none")]
    device_id: Option<String>,
    #[serde(rename = "networkCode", skip_serializing_if = "Option::is_none")]
    network_code: Option<String>,
    #[serde(rename = "networkAddr", skip_serializing_if = "Option::is_none")]
    network_addr: Option<String>,
    profile: String,
    data: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    extension: Option<Map<String, Value>>,
}

#[derive(Clone, Serialize)]
struct AppDlDataResult {
    #[serde(rename = "dataId")]
    data_id: String,
    resp: String,
    status: i32,
}

#[derive(Clone, Serialize)]
struct NetUlData {
    #[serde(rename = "dataId")]
    data_id: String,
    proc: String,
    #[serde(rename = "unitCode")]
    unit_code: Option<String>,
    #[serde(rename = "networkCode")]
    network_code: String,
    #[serde(rename = "networkAddr")]
    network_addr: String,
    #[serde(rename = "unitId", skip_serializing_if = "Option::is_none")]
    unit_id: Option<String>,
    #[serde(rename = "deviceId", skip_serializing_if = "Option::is_none")]
    device_id: Option<String>,
    time: String,
    profile: String,
    data: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    extension: Option<Map<String, Value>>,
}

#[derive(Clone, Serialize)]
struct NetDlData {
    #[serde(rename = "dataId")]
    data_id: String,
    proc: String,
    #[serde(rename = "pub")]
    publish: String,
    status: i32,
    #[serde(rename = "unitId")]
    unit_id: String,
    #[serde(rename = "deviceId")]
    device_id: String,
    #[serde(rename = "networkCode")]
    network_code: String,
    #[serde(rename = "networkAddr")]
    network_addr: String,
    profile: String,
    data: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    extension: Option<Map<String, Value>>,
}

#[derive(Clone, Serialize)]
struct NetDlDataResult {
    #[serde(rename = "dataId")]
    data_id: String,
    resp: String,
    status: i32,
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
                    name: "broker.data".to_string(),
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
                    name: "broker.data".to_string(),
                    is_recv: false,
                    reliable: true,
                    broadcast: false,
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
            match broker::new(model, mq_conns, &conf) {
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
            match broker::new(model, mq_conns, &conf) {
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
            match broker::new(model, mq_conns, &conf) {
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
            match broker::new(model, mq_conns, &conf) {
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
            match broker::new(model, mq_conns, &conf) {
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
            match broker::new(model, mq_conns, &conf) {
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
    if let Ok(q) = broker::new(Arc::new(model.clone()), mq_conns, &conf) {
        state.recv_queue = Some(q);
        return Err("data queue should not be created".to_string());
    }

    let conf = DataMqConfig {
        url: Some("".to_string()),
        prefetch: Some(1),
        ..Default::default()
    };
    if let Ok(q) = broker::new(Arc::new(model.clone()), mq_conns, &conf) {
        state.recv_queue = Some(q);
        return Err("data queue should not be created".to_string());
    }

    let conf = DataMqConfig {
        url: Some("http://localhost".to_string()),
        prefetch: Some(1),
        ..Default::default()
    };
    if let Ok(q) = broker::new(Arc::new(model.clone()), mq_conns, &conf) {
        state.recv_queue = Some(q);
        return Err("data queue should not be created".to_string());
    }

    Ok(())
}

/// Test application-uldata kind data.
pub fn application_uldata(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let mut state = context.state.borrow_mut();
    let state = state.get_mut(STATE).unwrap();

    create_default_queue(state)?;
    let send_queue = state.data_queue.as_mut().unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap();

    runtime.block_on(async {
        let now = Utc::now();
        let data = SendDataMsg::AppUlData {
            kind: "application-uldata".to_string(),
            data: AppUlData {
                data_id: "data_id1".to_string(),
                proc: strings::time_str(&now),
                publish: strings::time_str(&(Utc.timestamp_nanos(now.timestamp_nanos() + 1000000))),
                unit_code: None,
                network_code: "network_code1".to_string(),
                network_addr: "network_addr1".to_string(),
                unit_id: "unit_id1".to_string(),
                device_id: "device_id1".to_string(),
                time: strings::time_str(&(Utc.timestamp_nanos(now.timestamp_nanos() + 2000000))),
                profile: "profile1".to_string(),
                data: "01".to_string(),
                extension: None,
            },
        };
        let payload = match serde_json::to_vec(&data) {
            Err(e) => return Err(format!("marshal data1 error: {}", e)),
            Ok(payload) => payload,
        };
        if let Err(e) = send_queue.send_msg(payload).await {
            return Err(format!("send data1 error: {}", e));
        }

        let data = SendDataMsg::AppUlData {
            kind: "application-uldata".to_string(),
            data: AppUlData {
                data_id: "data_id2".to_string(),
                proc: strings::time_str(&now),
                publish: strings::time_str(&(Utc.timestamp_nanos(now.timestamp_nanos() + 1000000))),
                unit_code: Some("unit_code2".to_string()),
                network_code: "network_code2".to_string(),
                network_addr: "network_addr2".to_string(),
                unit_id: "unit_id2".to_string(),
                device_id: "device_id2".to_string(),
                time: strings::time_str(&(Utc.timestamp_nanos(now.timestamp_nanos() + 2000000))),
                profile: "profile2".to_string(),
                data: "02".to_string(),
                extension: Some(Map::new()),
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
            let cond = AppUlDataCond::default();
            let opts = AppUlDataOpts {
                cond: &cond,
                offset: None,
                limit: None,
                sort: None,
                cursor_max: None,
            };
            match model.application_uldata().list(&opts, None).await {
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

/// Test application-uldata kind data with wrong content.
pub fn application_uldata_wrong(context: &mut SpecContext<TestState>) -> Result<(), String> {
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
        let mut content = AppUlData {
            data_id: "data_id".to_string(),
            proc: "2022-20-28T00:00:00Z".to_string(),
            publish: strings::time_str(&now),
            unit_code: None,
            network_code: "network_code".to_string(),
            network_addr: "network_addr".to_string(),
            unit_id: "unit_id".to_string(),
            device_id: "device_id".to_string(),
            time: strings::time_str(&now),
            profile: "profile".to_string(),
            data: "00".to_string(),
            extension: None,
        };
        let data = SendDataMsg::AppUlData {
            kind: "application-uldata".to_string(),
            data: content.clone(),
        };
        let payload = match serde_json::to_vec(&data) {
            Err(e) => return Err(format!("marshal wrong proc error: {}", e)),
            Ok(payload) => payload,
        };
        if let Err(e) = send_queue.send_msg(payload).await {
            return Err(format!("send wrong proc error: {}", e));
        }

        content.proc = strings::time_str(&now);
        content.publish = "2022-20-28T00:00:00Z".to_string();
        let data = SendDataMsg::AppUlData {
            kind: "application-uldata".to_string(),
            data: content.clone(),
        };
        let payload = match serde_json::to_vec(&data) {
            Err(e) => return Err(format!("marshal wrong publish error: {}", e)),
            Ok(payload) => payload,
        };
        if let Err(e) = send_queue.send_msg(payload).await {
            return Err(format!("send wrong publish error: {}", e));
        }

        content.publish = strings::time_str(&now);
        content.time = "2022-20-28T00:00:00Z".to_string();
        let data = SendDataMsg::AppUlData {
            kind: "application-uldata".to_string(),
            data: content,
        };
        let payload = match serde_json::to_vec(&data) {
            Err(e) => return Err(format!("marshal wrong time error: {}", e)),
            Ok(payload) => payload,
        };
        if let Err(e) = send_queue.send_msg(payload).await {
            return Err(format!("send wrong time error: {}", e));
        }

        Ok(())
    })
}

/// Test application-dldata kind data.
pub fn application_dldata(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let mut state = context.state.borrow_mut();
    let state = state.get_mut(STATE).unwrap();

    create_default_queue(state)?;
    let send_queue = state.data_queue.as_mut().unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap();

    runtime.block_on(async {
        let now = Utc::now();
        let data = SendDataMsg::AppDlData {
            kind: "application-dldata".to_string(),
            data: AppDlData {
                data_id: "data_id1".to_string(),
                proc: strings::time_str(&now),
                status: -1,
                unit_id: "unit_id1".to_string(),
                device_id: Some("device_id1".to_string()),
                network_code: None,
                network_addr: None,
                profile: "profile1".to_string(),
                data: "01".to_string(),
                extension: None,
            },
        };
        let payload = match serde_json::to_vec(&data) {
            Err(e) => return Err(format!("marshal data1 error: {}", e)),
            Ok(payload) => payload,
        };
        if let Err(e) = send_queue.send_msg(payload).await {
            return Err(format!("send data1 error: {}", e));
        }
        let data = SendDataMsg::AppDlDataResult {
            kind: "application-dldata-result".to_string(),
            data: AppDlDataResult {
                data_id: "data_id1".to_string(),
                resp: strings::time_str(&(Utc.timestamp_nanos(now.timestamp_nanos() + 1000000))),
                status: 0,
            },
        };
        let payload = match serde_json::to_vec(&data) {
            Err(e) => return Err(format!("marshal data1 result error: {}", e)),
            Ok(payload) => payload,
        };
        if let Err(e) = send_queue.send_msg(payload).await {
            return Err(format!("send data1 result error: {}", e));
        }

        let data = SendDataMsg::AppDlData {
            kind: "application-dldata".to_string(),
            data: AppDlData {
                data_id: "data_id2".to_string(),
                proc: strings::time_str(&now),
                status: -2,
                unit_id: "unit_id2".to_string(),
                device_id: None,
                network_code: Some("network_code2".to_string()),
                network_addr: Some("network_addr2".to_string()),
                profile: "profile2".to_string(),
                data: "02".to_string(),
                extension: Some(Map::new()),
            },
        };
        let payload = match serde_json::to_vec(&data) {
            Err(e) => return Err(format!("marshal data2 error: {}", e)),
            Ok(payload) => payload,
        };
        if let Err(e) = send_queue.send_msg(payload).await {
            return Err(format!("send data2 error: {}", e));
        }
        let data = SendDataMsg::AppDlDataResult {
            kind: "application-dldata-result".to_string(),
            data: AppDlDataResult {
                data_id: "data_id2".to_string(),
                resp: strings::time_str(&(Utc.timestamp_nanos(now.timestamp_nanos() + 1000000))),
                status: -1,
            },
        };
        let payload = match serde_json::to_vec(&data) {
            Err(e) => return Err(format!("marshal data2 result error: {}", e)),
            Ok(payload) => payload,
        };
        if let Err(e) = send_queue.send_msg(payload).await {
            return Err(format!("send data2 result error: {}", e));
        }

        let mut count = 0;
        for _ in 0..20 {
            let cond = AppDlDataCond::default();
            let opts = AppDlDataOpts {
                cond: &cond,
                offset: None,
                limit: None,
                sort: None,
                cursor_max: None,
            };
            match model.application_dldata().list(&opts, None).await {
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

/// Test application-dldata kind data with wrong content.
pub fn application_dldata_wrong(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let mut state = context.state.borrow_mut();
    let state = state.get_mut(STATE).unwrap();

    create_default_queue(state)?;
    let send_queue = state.data_queue.as_mut().unwrap();
    let runtime = state.runtime.as_ref().unwrap();

    runtime.block_on(async {
        if let Err(e) = send_queue.send_msg(vec![]).await {
            return Err(format!("send kind error: {}", e));
        }

        let data = SendDataMsg::AppDlData {
            kind: "application-dldata".to_string(),
            data: AppDlData {
                data_id: "data_id".to_string(),
                proc: "2022-20-28T00:00:00Z".to_string(),
                status: 0,
                unit_id: "unit_id".to_string(),
                device_id: None,
                network_code: None,
                network_addr: None,
                profile: "profile".to_string(),
                data: "00".to_string(),
                extension: None,
            },
        };
        let payload = match serde_json::to_vec(&data) {
            Err(e) => return Err(format!("marshal wrong proc error: {}", e)),
            Ok(payload) => payload,
        };
        if let Err(e) = send_queue.send_msg(payload).await {
            return Err(format!("send wrong proc error: {}", e));
        }

        let data = SendDataMsg::AppDlDataResult {
            kind: "application-dldata-result".to_string(),
            data: AppDlDataResult {
                data_id: "data_id".to_string(),
                resp: "2022-20-28T00:00:00Z".to_string(),
                status: 0,
            },
        };
        let payload = match serde_json::to_vec(&data) {
            Err(e) => return Err(format!("marshal wrong resp result error: {}", e)),
            Ok(payload) => payload,
        };
        if let Err(e) = send_queue.send_msg(payload).await {
            return Err(format!("send wrong resp result error: {}", e));
        }

        Ok(())
    })
}

/// Test network-uldata kind data.
pub fn network_uldata(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let mut state = context.state.borrow_mut();
    let state = state.get_mut(STATE).unwrap();

    create_default_queue(state)?;
    let send_queue = state.data_queue.as_mut().unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap();

    runtime.block_on(async {
        let now = Utc::now();
        let data = SendDataMsg::NetUlData {
            kind: "network-uldata".to_string(),
            data: NetUlData {
                data_id: "data_id1".to_string(),
                proc: strings::time_str(&now),
                unit_code: None,
                network_code: "network_code1".to_string(),
                network_addr: "network_addr1".to_string(),
                unit_id: None,
                device_id: None,
                time: strings::time_str(&(Utc.timestamp_nanos(now.timestamp_nanos() + 2000000))),
                profile: "profile1".to_string(),
                data: "01".to_string(),
                extension: None,
            },
        };
        let payload = match serde_json::to_vec(&data) {
            Err(e) => return Err(format!("marshal data1 error: {}", e)),
            Ok(payload) => payload,
        };
        if let Err(e) = send_queue.send_msg(payload).await {
            return Err(format!("send data1 error: {}", e));
        }

        let data = SendDataMsg::NetUlData {
            kind: "network-uldata".to_string(),
            data: NetUlData {
                data_id: "data_id2".to_string(),
                proc: strings::time_str(&now),
                unit_code: Some("unit_code2".to_string()),
                network_code: "network_code2".to_string(),
                network_addr: "network_addr2".to_string(),
                unit_id: Some("unit_id2".to_string()),
                device_id: Some("device_id2".to_string()),
                time: strings::time_str(&(Utc.timestamp_nanos(now.timestamp_nanos() + 2000000))),
                profile: "profile2".to_string(),
                data: "02".to_string(),
                extension: Some(Map::new()),
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
            let cond = NetUlDataCond::default();
            let opts = NetUlDataOpts {
                cond: &cond,
                offset: None,
                limit: None,
                sort: None,
                cursor_max: None,
            };
            match model.network_uldata().list(&opts, None).await {
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

/// Test network-uldata kind data with wrong content.
pub fn network_uldata_wrong(context: &mut SpecContext<TestState>) -> Result<(), String> {
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
        let mut content = NetUlData {
            data_id: "data_id".to_string(),
            proc: "2022-20-28T00:00:00Z".to_string(),
            unit_code: None,
            network_code: "network_code".to_string(),
            network_addr: "network_addr".to_string(),
            unit_id: None,
            device_id: None,
            time: strings::time_str(&now),
            profile: "profile".to_string(),
            data: "00".to_string(),
            extension: None,
        };
        let data = SendDataMsg::NetUlData {
            kind: "network-uldata".to_string(),
            data: content.clone(),
        };
        let payload = match serde_json::to_vec(&data) {
            Err(e) => return Err(format!("marshal wrong proc error: {}", e)),
            Ok(payload) => payload,
        };
        if let Err(e) = send_queue.send_msg(payload).await {
            return Err(format!("send wrong proc error: {}", e));
        }

        content.proc = strings::time_str(&now);
        content.time = "2022-20-28T00:00:00Z".to_string();
        let data = SendDataMsg::NetUlData {
            kind: "network-uldata".to_string(),
            data: content,
        };
        let payload = match serde_json::to_vec(&data) {
            Err(e) => return Err(format!("marshal wrong time error: {}", e)),
            Ok(payload) => payload,
        };
        if let Err(e) = send_queue.send_msg(payload).await {
            return Err(format!("send wrong time error: {}", e));
        }

        Ok(())
    })
}

/// Test network-dldata kind data.
pub fn network_dldata(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let mut state = context.state.borrow_mut();
    let state = state.get_mut(STATE).unwrap();

    create_default_queue(state)?;
    let send_queue = state.data_queue.as_mut().unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap();

    runtime.block_on(async {
        let now = Utc::now();
        let data = SendDataMsg::NetDlData {
            kind: "network-dldata".to_string(),
            data: NetDlData {
                data_id: "data_id1".to_string(),
                proc: strings::time_str(&now),
                publish: strings::time_str(&(Utc.timestamp_nanos(now.timestamp_nanos() + 1000000))),
                status: -1,
                unit_id: "unit_id1".to_string(),
                device_id: "device_id1".to_string(),
                network_code: "network_code1".to_string(),
                network_addr: "network_addr1".to_string(),
                profile: "profile1".to_string(),
                data: "01".to_string(),
                extension: None,
            },
        };
        let payload = match serde_json::to_vec(&data) {
            Err(e) => return Err(format!("marshal data1 error: {}", e)),
            Ok(payload) => payload,
        };
        if let Err(e) = send_queue.send_msg(payload).await {
            return Err(format!("send data1 error: {}", e));
        }
        let data = SendDataMsg::NetDlDataResult {
            kind: "network-dldata-result".to_string(),
            data: NetDlDataResult {
                data_id: "data_id1".to_string(),
                resp: strings::time_str(&(Utc.timestamp_nanos(now.timestamp_nanos() + 1000000))),
                status: 0,
            },
        };
        let payload = match serde_json::to_vec(&data) {
            Err(e) => return Err(format!("marshal data1 result error: {}", e)),
            Ok(payload) => payload,
        };
        if let Err(e) = send_queue.send_msg(payload).await {
            return Err(format!("send data1 result error: {}", e));
        }

        let data = SendDataMsg::NetDlData {
            kind: "network-dldata".to_string(),
            data: NetDlData {
                data_id: "data_id2".to_string(),
                proc: strings::time_str(&now),
                publish: strings::time_str(&(Utc.timestamp_nanos(now.timestamp_nanos() + 1000000))),
                status: -2,
                unit_id: "unit_id2".to_string(),
                device_id: "device_id2".to_string(),
                network_code: "network_code2".to_string(),
                network_addr: "network_addr2".to_string(),
                profile: "profile2".to_string(),
                data: "02".to_string(),
                extension: None,
            },
        };
        let payload = match serde_json::to_vec(&data) {
            Err(e) => return Err(format!("marshal data2 error: {}", e)),
            Ok(payload) => payload,
        };
        if let Err(e) = send_queue.send_msg(payload).await {
            return Err(format!("send data2 error: {}", e));
        }
        let data = SendDataMsg::NetDlDataResult {
            kind: "network-dldata-result".to_string(),
            data: NetDlDataResult {
                data_id: "data_id2".to_string(),
                resp: strings::time_str(&(Utc.timestamp_nanos(now.timestamp_nanos() + 1000000))),
                status: 0,
            },
        };
        let payload = match serde_json::to_vec(&data) {
            Err(e) => return Err(format!("marshal data2 result error: {}", e)),
            Ok(payload) => payload,
        };
        if let Err(e) = send_queue.send_msg(payload).await {
            return Err(format!("send data2 result error: {}", e));
        }

        let mut count = 0;
        for _ in 0..20 {
            let cond = NetDlDataCond::default();
            let opts = NetDlDataOpts {
                cond: &cond,
                offset: None,
                limit: None,
                sort: None,
                cursor_max: None,
            };
            match model.network_dldata().list(&opts, None).await {
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

/// Test network-dldata kind data with wrong content.
pub fn network_dldata_wrong(context: &mut SpecContext<TestState>) -> Result<(), String> {
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
        let mut content = NetDlData {
            data_id: "data_id".to_string(),
            proc: "2022-20-28T00:00:00Z".to_string(),
            publish: strings::time_str(&now),
            status: -1,
            unit_id: "unit_id".to_string(),
            device_id: "device_id".to_string(),
            network_code: "network_code".to_string(),
            network_addr: "network_addr".to_string(),
            profile: "profile".to_string(),
            data: "00".to_string(),
            extension: None,
        };
        let data = SendDataMsg::NetDlData {
            kind: "network-dldata".to_string(),
            data: content.clone(),
        };
        let payload = match serde_json::to_vec(&data) {
            Err(e) => return Err(format!("marshal wrong proc error: {}", e)),
            Ok(payload) => payload,
        };
        if let Err(e) = send_queue.send_msg(payload).await {
            return Err(format!("send wrong proc error: {}", e));
        }

        content.proc = strings::time_str(&now);
        content.publish = "2022-20-28T00:00:00Z".to_string();
        let data = SendDataMsg::NetDlData {
            kind: "network-dldata".to_string(),
            data: content,
        };
        let payload = match serde_json::to_vec(&data) {
            Err(e) => return Err(format!("marshal wrong publish error: {}", e)),
            Ok(payload) => payload,
        };
        if let Err(e) = send_queue.send_msg(payload).await {
            return Err(format!("send wrong publish error: {}", e));
        }

        let data = SendDataMsg::NetDlDataResult {
            kind: "network-dldata-result".to_string(),
            data: NetDlDataResult {
                data_id: "data_id".to_string(),
                resp: "2022-20-28T00:00:00Z".to_string(),
                status: 0,
            },
        };
        let payload = match serde_json::to_vec(&data) {
            Err(e) => return Err(format!("marshal wrong resp result error: {}", e)),
            Ok(payload) => payload,
        };
        if let Err(e) = send_queue.send_msg(payload).await {
            return Err(format!("send wrong resp result error: {}", e));
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
            match broker::new(model, mq_conns, &conf) {
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
            match broker::new(model, mq_conns, &conf) {
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
