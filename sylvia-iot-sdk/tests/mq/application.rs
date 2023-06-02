use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::Duration,
};

use async_trait::async_trait;
use chrono::{TimeZone, Utc};
use laboratory::{expect, SpecContext};
use serde::{self, Deserialize, Serialize};
use serde_json::{self, Map, Value};
use tokio::time;

use general_mq::{
    queue::{
        Event as MqEvent, EventHandler as MqEventHandler, GmqQueue, Message, Status as MqStatus,
    },
    AmqpQueueOptions, MqttQueueOptions, Queue, QueueOptions,
};
use sylvia_iot_sdk::{
    mq::{
        application::{ApplicationMgr, DlData, DlDataResp, DlDataResult, EventHandler, UlData},
        Connection, MgrStatus, Options,
    },
    util::strings,
};

use super::{conn_host_uri, new_connection, MqEngine, STATE};
use crate::{TestState, WAIT_COUNT, WAIT_TICK};

/// Uplink data from broker to application.
#[derive(Serialize)]
struct AppUlData {
    #[serde(rename = "dataId")]
    data_id: String,
    time: String,
    #[serde(rename = "pub")]
    publish: String,
    #[serde(rename = "deviceId")]
    device_id: String,
    #[serde(rename = "networkId")]
    network_id: String,
    #[serde(rename = "networkCode")]
    network_code: String,
    #[serde(rename = "networkAddr")]
    network_addr: String,
    #[serde(rename = "isPublic")]
    is_public: bool,
    data: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    extension: Option<Map<String, Value>>,
}

/// Downlink data from application to broker.
#[derive(Debug, Deserialize)]
struct AppDlData {
    #[serde(rename = "correlationId")]
    correlation_id: String,
    #[serde(rename = "deviceId")]
    device_id: Option<String>,
    #[serde(rename = "networkCode")]
    network_code: Option<String>,
    #[serde(rename = "networkAddr")]
    network_addr: Option<String>,
    data: String,
    extension: Option<Map<String, Value>>,
}

/// Downlink response data from broker to application.
#[derive(Serialize)]
struct AppDlDataResp {
    #[serde(rename = "correlationId")]
    pub correlation_id: String,
    #[serde(rename = "dataId", skip_serializing_if = "Option::is_none")]
    pub data_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// Downlink result data from broker to application.
#[derive(Serialize)]
struct AppDlDataResult {
    #[serde(rename = "dataId")]
    pub data_id: String,
    pub status: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

struct TestHandler {
    // Use Mutex to implement interior mutability.
    status_changed: Arc<Mutex<bool>>,
    recv_uldata: Arc<Mutex<Vec<Box<UlData>>>>,
    recv_dldata_resp: Arc<Mutex<Vec<Box<DlDataResp>>>>,
    recv_dldata_result: Arc<Mutex<Vec<Box<DlDataResult>>>>,
    is_uldata_recv: Arc<Mutex<bool>>,
    is_dldata_resp_recv: Arc<Mutex<bool>>,
    is_dldata_result_recv: Arc<Mutex<bool>>,
}

#[derive(Clone)]
struct TestDlDataHandler {
    // Use Mutex to implement interior mutability.
    status_connected: Arc<Mutex<bool>>,
    recv_dldata: Arc<Mutex<Vec<Box<AppDlData>>>>,
}

impl TestHandler {
    fn new() -> Self {
        TestHandler {
            status_changed: Arc::new(Mutex::new(false)),
            recv_uldata: Arc::new(Mutex::new(vec![])),
            recv_dldata_resp: Arc::new(Mutex::new(vec![])),
            recv_dldata_result: Arc::new(Mutex::new(vec![])),
            is_uldata_recv: Arc::new(Mutex::new(false)),
            is_dldata_resp_recv: Arc::new(Mutex::new(false)),
            is_dldata_result_recv: Arc::new(Mutex::new(false)),
        }
    }
}

#[async_trait]
impl EventHandler for TestHandler {
    async fn on_status_change(&self, _mgr: &ApplicationMgr, _status: MgrStatus) {
        *self.status_changed.lock().unwrap() = true;
    }

    async fn on_uldata(&self, _mgr: &ApplicationMgr, data: Box<UlData>) -> Result<(), ()> {
        {
            let mut mutex = self.is_uldata_recv.lock().unwrap();
            if !*mutex {
                *mutex = true;
                return Err(()); // test AMQP NACK.
            }
        }

        let mut mutex = self.recv_uldata.lock().unwrap();
        mutex.push(data);
        Ok(())
    }

    async fn on_dldata_resp(&self, _mgr: &ApplicationMgr, data: Box<DlDataResp>) -> Result<(), ()> {
        {
            let mut mutex = self.is_dldata_resp_recv.lock().unwrap();
            if !*mutex {
                *mutex = true;
                return Err(()); // test AMQP NACK.
            }
        }

        let mut mutex = self.recv_dldata_resp.lock().unwrap();
        mutex.push(data);
        Ok(())
    }

    async fn on_dldata_result(
        &self,
        _mgr: &ApplicationMgr,
        data: Box<DlDataResult>,
    ) -> Result<(), ()> {
        {
            let mut mutex = self.is_dldata_result_recv.lock().unwrap();
            if !*mutex {
                *mutex = true;
                return Err(()); // test AMQP NACK.
            }
        }

        let mut mutex = self.recv_dldata_result.lock().unwrap();
        mutex.push(data);
        Ok(())
    }
}

impl TestDlDataHandler {
    fn new() -> Self {
        TestDlDataHandler {
            status_connected: Arc::new(Mutex::new(false)),
            recv_dldata: Arc::new(Mutex::new(vec![])),
        }
    }
}

#[async_trait]
impl MqEventHandler for TestDlDataHandler {
    async fn on_event(&self, _queue: Arc<dyn GmqQueue>, ev: MqEvent) {
        if let MqEvent::Status(status) = ev {
            if status == MqStatus::Connected {
                *self.status_connected.lock().unwrap() = true;
            }
        }
    }

    async fn on_message(&self, _queue: Arc<dyn GmqQueue>, msg: Box<dyn Message>) {
        let data = match serde_json::from_slice::<AppDlData>(msg.payload()) {
            Err(_) => return,
            Ok(data) => Box::new(data),
        };
        {
            self.recv_dldata.lock().unwrap().push(data);
        }
        let _ = msg.ack().await;
    }
}

/// Test new managers with default options.
pub fn new_default(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let mut state = context.state.borrow_mut();
    let state = state.get_mut(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let mq_engine = state.mq_engine.as_ref().unwrap().as_str();

    let conn_pool = Arc::new(Mutex::new(HashMap::new()));
    let host_uri = conn_host_uri(mq_engine)?;
    let handler = Arc::new(TestHandler::new());

    let opts = Options {
        unit_id: "unit_id".to_string(),
        unit_code: "unit_code".to_string(),
        id: "id_application".to_string(),
        name: "code_application".to_string(),
        shared_prefix: state.mqtt_shared_prefix.clone(),
        ..Default::default()
    };
    let mgr = ApplicationMgr::new(conn_pool, &host_uri, opts, handler.clone())?;
    let status = mgr.status();
    let mq_status = mgr.mq_status();
    state.app_mgrs = Some(vec![mgr.clone()]);

    expect(mgr.unit_id()).equals("unit_id")?;
    expect(mgr.unit_code()).equals("unit_code")?;
    expect(mgr.id()).equals("id_application")?;
    expect(mgr.name()).equals("code_application")?;
    expect(status == MgrStatus::NotReady).equals(true)?;
    expect(mq_status.uldata == MqStatus::Connecting).equals(true)?;
    expect(mq_status.dldata == MqStatus::Connecting).equals(true)?;
    expect(mq_status.dldata_resp == MqStatus::Connecting).equals(true)?;
    expect(mq_status.dldata_result == MqStatus::Connecting).equals(true)?;

    for _ in 0..WAIT_COUNT {
        if *handler.status_changed.lock().unwrap() {
            break;
        }
        runtime.block_on(async { time::sleep(Duration::from_millis(WAIT_TICK)).await });
    }
    let status = mgr.status();
    let mq_status = mgr.mq_status();
    expect(status == MgrStatus::Ready).equals(true)?;
    expect(mq_status.uldata == MqStatus::Connected).equals(true)?;
    expect(mq_status.dldata == MqStatus::Connected).equals(true)?;
    expect(mq_status.dldata_resp == MqStatus::Connected).equals(true)?;
    expect(mq_status.dldata_result == MqStatus::Connected).equals(true)?;

    Ok(())
}

/// Test new managers with manual options.
pub fn new_manual(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let mut state = context.state.borrow_mut();
    let state = state.get_mut(STATE).unwrap();
    let mq_engine = state.mq_engine.as_ref().unwrap().as_str();

    let conn_pool = Arc::new(Mutex::new(HashMap::new()));
    let host_uri = conn_host_uri(mq_engine)?;
    let handler = Arc::new(TestHandler::new());

    let opts = Options {
        unit_id: "unit_id".to_string(),
        unit_code: "unit_code".to_string(),
        id: "id_application".to_string(),
        name: "code_application".to_string(),
        prefetch: Some(0),
        shared_prefix: state.mqtt_shared_prefix.clone(),
        ..Default::default()
    };
    let mgr = ApplicationMgr::new(conn_pool.clone(), &host_uri, opts, handler.clone())?;
    state.app_mgrs = Some(vec![mgr]);

    let opts = Options {
        unit_id: "unit_id".to_string(),
        unit_code: "unit_code".to_string(),
        id: "id_application".to_string(),
        name: "code_application".to_string(),
        prefetch: Some(1),
        shared_prefix: state.mqtt_shared_prefix.clone(),
        ..Default::default()
    };
    let mgr = ApplicationMgr::new(conn_pool.clone(), &host_uri, opts, handler.clone())?;
    state.app_mgrs.as_mut().unwrap().push(mgr);

    Ok(())
}

/// Test new managers with wrong options.
pub fn new_wrong_opts(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let mut state = context.state.borrow_mut();
    let state = state.get_mut(STATE).unwrap();
    let mq_engine = state.mq_engine.as_ref().unwrap().as_str();

    let conn_pool = Arc::new(Mutex::new(HashMap::new()));
    let host_uri = conn_host_uri(mq_engine)?;
    let handler = Arc::new(TestHandler::new());

    let opts = Options {
        ..Default::default()
    };
    expect(ApplicationMgr::new(conn_pool.clone(), &host_uri, opts, handler.clone()).is_err())
        .equals(true)?;
    let opts = Options {
        unit_id: "".to_string(),
        unit_code: "unit_code".to_string(),
        ..Default::default()
    };
    expect(ApplicationMgr::new(conn_pool.clone(), &host_uri, opts, handler.clone()).is_err())
        .equals(true)?;
    let opts = Options {
        unit_id: "unit_id".to_string(),
        unit_code: "".to_string(),
        ..Default::default()
    };
    expect(ApplicationMgr::new(conn_pool.clone(), &host_uri, opts, handler.clone()).is_err())
        .equals(true)?;
    let opts = Options {
        unit_id: "unit_id".to_string(),
        unit_code: "unit_code".to_string(),
        ..Default::default()
    };
    expect(ApplicationMgr::new(conn_pool.clone(), &host_uri, opts, handler.clone()).is_err())
        .equals(true)?;
    let opts = Options {
        unit_id: "unit_id".to_string(),
        unit_code: "unit_code".to_string(),
        id: "id".to_string(),
        ..Default::default()
    };
    expect(ApplicationMgr::new(conn_pool.clone(), &host_uri, opts, handler.clone()).is_err())
        .equals(true)
}

/// Test `close()`.
pub fn close(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let mut state = context.state.borrow_mut();
    let state = state.get_mut(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let mq_engine = state.mq_engine.as_ref().unwrap().as_str();

    let conn_pool = Arc::new(Mutex::new(HashMap::new()));
    let host_uri = conn_host_uri(mq_engine)?;
    let handler = Arc::new(TestHandler::new());

    let opts = Options {
        unit_id: "unit_id".to_string(),
        unit_code: "unit_code".to_string(),
        id: "id_application".to_string(),
        name: "code_application".to_string(),
        prefetch: Some(0),
        shared_prefix: state.mqtt_shared_prefix.clone(),
        ..Default::default()
    };
    let mgr = ApplicationMgr::new(conn_pool.clone(), &host_uri, opts, handler.clone())?;

    match runtime.block_on(async move { mgr.close().await }) {
        Err(e) => Err(format!("close with error: {}", e)),
        Ok(_) => Ok(()),
    }
}

/// Test receiving uldata.
pub fn uldata(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let mut state = context.state.borrow_mut();
    let state = state.get_mut(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let mq_engine = state.mq_engine.as_ref().unwrap().as_str();
    let conn_pool = state.mgr_conns.as_ref().unwrap();

    state.app_net_conn = Some(new_connection(runtime, mq_engine)?);
    let conn = state.app_net_conn.as_ref().unwrap();

    let host_uri = conn_host_uri(mq_engine)?;
    let mgr_handler = Arc::new(TestHandler::new());

    let opts = Options {
        unit_id: "unit_id".to_string(),
        unit_code: "unit_code".to_string(),
        id: "id_application".to_string(),
        name: "code_application".to_string(),
        shared_prefix: state.mqtt_shared_prefix.clone(),
        ..Default::default()
    };
    let mgr = ApplicationMgr::new(conn_pool.clone(), &host_uri, opts, mgr_handler.clone())?;
    state.app_mgrs = Some(vec![mgr.clone()]);

    let queue = match conn {
        Connection::Amqp(conn, _) => {
            let opts = QueueOptions::Amqp(
                AmqpQueueOptions {
                    name: "broker.application.unit_code.code_application.uldata".to_string(),
                    is_recv: false,
                    reliable: true,
                    broadcast: false,
                    ..Default::default()
                },
                conn,
            );
            let mut queue_result = Queue::new(opts)?;
            if let Err(e) = queue_result.connect() {
                return Err(format!("connect uldata queue error: {}", e));
            }
            queue_result
        }
        Connection::Mqtt(conn, _) => {
            let opts = QueueOptions::Mqtt(
                MqttQueueOptions {
                    name: "broker.application.unit_code.code_application.uldata".to_string(),
                    is_recv: false,
                    reliable: true,
                    broadcast: false,
                    ..Default::default()
                },
                conn,
            );
            let mut queue_result = Queue::new(opts)?;
            if let Err(e) = queue_result.connect() {
                return Err(format!("connect uldata queue error: {}", e));
            }
            queue_result
        }
    };

    runtime.block_on(async move {
        for _ in 0..WAIT_COUNT {
            if queue.status() == MqStatus::Connected {
                break;
            }
            time::sleep(Duration::from_millis(WAIT_TICK)).await;
        }
        if queue.status() != MqStatus::Connected {
            return Err("send queue not connected".to_string());
        }
        for _ in 0..WAIT_COUNT {
            if mgr.status() == MgrStatus::Ready {
                break;
            }
            time::sleep(Duration::from_millis(WAIT_TICK)).await;
        }
        if mgr.status() != MgrStatus::Ready {
            return Err("manager not ready".to_string());
        }

        let now = Utc::now();
        let data1 = AppUlData {
            data_id: "1".to_string(),
            time: strings::time_str(&now),
            publish: strings::time_str(&Utc.timestamp_nanos(now.timestamp_nanos() + 1000000)),
            device_id: "device_id1".to_string(),
            network_id: "network_id1".to_string(),
            network_code: "network_code1".to_string(),
            network_addr: "network_addr1".to_string(),
            is_public: true,
            data: "01".to_string(),
            extension: None,
        };
        let payload = match serde_json::to_vec(&data1) {
            Err(e) => return Err(format!("marshal data1 error: {}", e)),
            Ok(payload) => payload,
        };
        if let Err(e) = queue.send_msg(payload).await {
            return Err(format!("send data1 error: {}", e));
        }
        let mut ext = Map::<String, Value>::new();
        ext.insert("key".to_string(), Value::String("value".to_string()));
        let data2 = AppUlData {
            data_id: "2".to_string(),
            time: strings::time_str(&Utc.timestamp_nanos(now.timestamp_nanos() + 1000000)),
            publish: strings::time_str(&Utc.timestamp_nanos(now.timestamp_nanos() + 2000000)),
            device_id: "device_id2".to_string(),
            network_id: "network_id2".to_string(),
            network_code: "network_code2".to_string(),
            network_addr: "network_addr2".to_string(),
            is_public: false,
            data: "02".to_string(),
            extension: Some(ext),
        };
        let payload = match serde_json::to_vec(&data2) {
            Err(e) => return Err(format!("marshal data2 error: {}", e)),
            Ok(payload) => payload,
        };
        if let Err(e) = queue.send_msg(payload).await {
            return Err(format!("send data2 error: {}", e));
        }
        let data3 = AppUlData {
            data_id: "3".to_string(),
            time: strings::time_str(&Utc.timestamp_nanos(now.timestamp_nanos() + 2000000)),
            publish: strings::time_str(&Utc.timestamp_nanos(now.timestamp_nanos() + 3000000)),
            device_id: "device_id3".to_string(),
            network_id: "network_id3".to_string(),
            network_code: "network_code3".to_string(),
            network_addr: "network_addr3".to_string(),
            is_public: true,
            data: "".to_string(),
            extension: None,
        };
        let payload = match serde_json::to_vec(&data3) {
            Err(e) => return Err(format!("marshal data3 error: {}", e)),
            Ok(payload) => payload,
        };
        if let Err(e) = queue.send_msg(payload).await {
            return Err(format!("send data3 error: {}", e));
        }

        let expect_count = match mq_engine {
            MqEngine::RABBITMQ => 3,
            _ => 2,
        };
        for _ in 0..WAIT_COUNT {
            let count = { mgr_handler.recv_uldata.lock().unwrap().len() };
            if count < expect_count {
                time::sleep(Duration::from_millis(WAIT_TICK)).await;
                continue;
            }
        }
        let count = { mgr_handler.recv_uldata.lock().unwrap().len() };
        if count < expect_count {
            return Err(format!("receive {}/{} data", count, expect_count));
        }

        for i in 0..expect_count {
            let data = match { mgr_handler.recv_uldata.lock().unwrap().pop() } {
                None => return Err(format!("only receive {}/{} data", i, expect_count)),
                Some(data) => data,
            };
            let data_id = data.data_id.as_str();
            if data_id == "1" {
                expect(mq_engine).equals(MqEngine::RABBITMQ)?;
                expect(data.time.timestamp_millis()).equals(now.timestamp_millis())?;
                expect(data.publish.timestamp_millis()).equals(now.timestamp_millis() + 1)?;
                expect(data.device_id.as_str()).equals(data1.device_id.as_str())?;
                expect(data.network_id.as_str()).equals(data1.network_id.as_str())?;
                expect(data.network_code.as_str()).equals(data1.network_code.as_str())?;
                expect(data.network_addr.as_str()).equals(data1.network_addr.as_str())?;
                expect(data.is_public).equals(data1.is_public)?;
                expect(data.data.as_str()).equals(data1.data.as_str())?;
                expect(data.extension.as_ref()).equals(data1.extension.as_ref())?;
            } else if data_id == "2" {
                expect(data.time.timestamp_millis()).equals(now.timestamp_millis() + 1)?;
                expect(data.publish.timestamp_millis()).equals(now.timestamp_millis() + 2)?;
                expect(data.device_id.as_str()).equals(data2.device_id.as_str())?;
                expect(data.network_id.as_str()).equals(data2.network_id.as_str())?;
                expect(data.network_code.as_str()).equals(data2.network_code.as_str())?;
                expect(data.network_addr.as_str()).equals(data2.network_addr.as_str())?;
                expect(data.is_public).equals(data2.is_public)?;
                expect(data.data.as_str()).equals(data2.data.as_str())?;
                expect(data.extension.as_ref()).equals(data2.extension.as_ref())?;
            } else if data_id == "3" {
                expect(data.time.timestamp_millis()).equals(now.timestamp_millis() + 2)?;
                expect(data.publish.timestamp_millis()).equals(now.timestamp_millis() + 3)?;
                expect(data.device_id.as_str()).equals(data3.device_id.as_str())?;
                expect(data.network_id.as_str()).equals(data3.network_id.as_str())?;
                expect(data.network_code.as_str()).equals(data3.network_code.as_str())?;
                expect(data.network_addr.as_str()).equals(data3.network_addr.as_str())?;
                expect(data.is_public).equals(data3.is_public)?;
                expect(data.data.as_str()).equals(data3.data.as_str())?;
                expect(data.extension.as_ref()).equals(data3.extension.as_ref())?;
            } else {
                return Err(format!("receive wrong data {}", data_id));
            }
        }

        Ok(())
    })?;

    Ok(())
}

/// Test receiving uldata with wrong content.
pub fn uldata_wrong(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let mut state = context.state.borrow_mut();
    let state = state.get_mut(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let mq_engine = state.mq_engine.as_ref().unwrap().as_str();
    let conn_pool = state.mgr_conns.as_ref().unwrap();

    state.app_net_conn = Some(new_connection(runtime, mq_engine)?);
    let conn = state.app_net_conn.as_ref().unwrap();

    let host_uri = conn_host_uri(mq_engine)?;
    let mgr_handler = Arc::new(TestHandler::new());

    let opts = Options {
        unit_id: "unit_id".to_string(),
        unit_code: "unit_code".to_string(),
        id: "id_application".to_string(),
        name: "code_application".to_string(),
        shared_prefix: state.mqtt_shared_prefix.clone(),
        ..Default::default()
    };
    let mgr = ApplicationMgr::new(conn_pool.clone(), &host_uri, opts, mgr_handler.clone())?;
    state.app_mgrs = Some(vec![mgr.clone()]);

    let queue = match conn {
        Connection::Amqp(conn, _) => {
            let opts = QueueOptions::Amqp(
                AmqpQueueOptions {
                    name: "broker.application.unit_code.code_application.uldata".to_string(),
                    is_recv: false,
                    reliable: true,
                    broadcast: false,
                    ..Default::default()
                },
                conn,
            );
            let mut queue_result = Queue::new(opts)?;
            if let Err(e) = queue_result.connect() {
                return Err(format!("connect uldata queue error: {}", e));
            }
            queue_result
        }
        Connection::Mqtt(conn, _) => {
            let opts = QueueOptions::Mqtt(
                MqttQueueOptions {
                    name: "broker.application.unit_code.code_application.uldata".to_string(),
                    is_recv: false,
                    reliable: true,
                    broadcast: false,
                    ..Default::default()
                },
                conn,
            );
            let mut queue_result = Queue::new(opts)?;
            if let Err(e) = queue_result.connect() {
                return Err(format!("connect uldata queue error: {}", e));
            }
            queue_result
        }
    };

    runtime.block_on(async move {
        for _ in 0..WAIT_COUNT {
            if queue.status() == MqStatus::Connected {
                break;
            }
            time::sleep(Duration::from_millis(WAIT_TICK)).await;
        }
        if queue.status() != MqStatus::Connected {
            return Err("send queue not connected".to_string());
        }
        for _ in 0..WAIT_COUNT {
            if mgr.status() == MgrStatus::Ready {
                break;
            }
            time::sleep(Duration::from_millis(WAIT_TICK)).await;
        }
        if mgr.status() != MgrStatus::Ready {
            return Err("manager not ready".to_string());
        }

        if let Err(e) = queue.send_msg(vec![]).await {
            return Err(format!("send empty error: {}", e));
        }

        let now = Utc::now();
        let mut data = AppUlData {
            data_id: "id".to_string(),
            time: "2022-20-29T11:45:00.000Z".to_string(),
            publish: strings::time_str(&now),
            device_id: "device_id".to_string(),
            network_id: "network_id".to_string(),
            network_code: "network_code".to_string(),
            network_addr: "network_addr".to_string(),
            is_public: true,
            data: "00".to_string(),
            extension: None,
        };
        let payload = match serde_json::to_vec(&data) {
            Err(e) => return Err(format!("marshal time error: {}", e)),
            Ok(payload) => payload,
        };
        if let Err(e) = queue.send_msg(payload).await {
            return Err(format!("send time error: {}", e));
        }
        data.time = strings::time_str(&now);
        data.publish = "2022-20-29T11:45:00.000Z".to_string();
        let payload = match serde_json::to_vec(&data) {
            Err(e) => return Err(format!("marshal pub error: {}", e)),
            Ok(payload) => payload,
        };
        if let Err(e) = queue.send_msg(payload).await {
            return Err(format!("send pub error: {}", e));
        }
        data.publish = strings::time_str(&now);
        data.data = "gg".to_string();
        let payload = match serde_json::to_vec(&data) {
            Err(e) => return Err(format!("marshal data error: {}", e)),
            Ok(payload) => payload,
        };
        if let Err(e) = queue.send_msg(payload).await {
            return Err(format!("send data error: {}", e));
        }

        time::sleep(Duration::from_secs(1)).await;
        let count = { mgr_handler.recv_uldata.lock().unwrap().len() };
        expect(count).equals(0)
    })?;

    Ok(())
}

/// Test generating dldata.
pub fn dldata(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let mut state = context.state.borrow_mut();
    let state = state.get_mut(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let mq_engine = state.mq_engine.as_ref().unwrap().as_str();
    let conn_pool = state.mgr_conns.as_ref().unwrap();

    state.app_net_conn = Some(new_connection(runtime, mq_engine)?);
    let conn = state.app_net_conn.as_ref().unwrap();

    let host_uri = conn_host_uri(mq_engine)?;
    let mgr_handler = Arc::new(TestHandler::new());

    let opts = Options {
        unit_id: "unit_id".to_string(),
        unit_code: "unit_code".to_string(),
        id: "id_application".to_string(),
        name: "code_application".to_string(),
        shared_prefix: state.mqtt_shared_prefix.clone(),
        ..Default::default()
    };
    let mgr = ApplicationMgr::new(conn_pool.clone(), &host_uri, opts, mgr_handler.clone())?;
    state.app_mgrs = Some(vec![mgr.clone()]);

    let handler = TestDlDataHandler::new();
    let queue = match conn {
        Connection::Amqp(conn, _) => {
            let opts = QueueOptions::Amqp(
                AmqpQueueOptions {
                    name: "broker.application.unit_code.code_application.dldata".to_string(),
                    is_recv: true,
                    reliable: true,
                    broadcast: false,
                    ..Default::default()
                },
                conn,
            );
            let mut queue_result = Queue::new(opts)?;
            queue_result.set_handler(Arc::new(handler.clone()));
            if let Err(e) = queue_result.connect() {
                return Err(format!("connect dldata queue error: {}", e));
            }
            queue_result
        }
        Connection::Mqtt(conn, _) => {
            let opts = QueueOptions::Mqtt(
                MqttQueueOptions {
                    name: "broker.application.unit_code.code_application.dldata".to_string(),
                    is_recv: true,
                    reliable: true,
                    broadcast: false,
                    ..Default::default()
                },
                conn,
            );
            let mut queue_result = Queue::new(opts)?;
            queue_result.set_handler(Arc::new(handler.clone()));
            if let Err(e) = queue_result.connect() {
                return Err(format!("connect dldata queue error: {}", e));
            }
            queue_result
        }
    };

    runtime.block_on(async move {
        for _ in 0..WAIT_COUNT {
            if queue.status() == MqStatus::Connected {
                break;
            }
            time::sleep(Duration::from_millis(WAIT_TICK)).await;
        }
        if queue.status() != MqStatus::Connected {
            return Err("recv queue not connected".to_string());
        }
        for _ in 0..WAIT_COUNT {
            if mgr.status() == MgrStatus::Ready {
                break;
            }
            time::sleep(Duration::from_millis(WAIT_TICK)).await;
        }
        if mgr.status() != MgrStatus::Ready {
            return Err("manager not ready".to_string());
        }

        let mut ext = Map::<String, Value>::new();
        ext.insert("key".to_string(), Value::String("value".to_string()));
        let data1 = DlData {
            correlation_id: "1".to_string(),
            device_id: Some("device1".to_string()),
            network_code: None,
            network_addr: None,
            data: "01".to_string(),
            extension: Some(ext),
        };
        if let Err(e) = mgr.send_dldata(&data1) {
            return Err(format!("send DlData 1 error: {}", e));
        }
        let data2 = DlData {
            correlation_id: "2".to_string(),
            device_id: None,
            network_code: Some("code".to_string()),
            network_addr: Some("addr2".to_string()),
            data: "02".to_string(),
            extension: None,
        };
        if let Err(e) = mgr.send_dldata(&data2) {
            return Err(format!("send DlData 2 error: {}", e));
        }

        let expect_count = 2;
        for _ in 0..WAIT_COUNT {
            let count = { handler.recv_dldata.lock().unwrap().len() };
            if count < expect_count {
                time::sleep(Duration::from_millis(WAIT_TICK)).await;
                continue;
            }
        }
        let count = { handler.recv_dldata.lock().unwrap().len() };
        if count < expect_count {
            return Err(format!("receive {}/{} data", count, expect_count));
        }

        for i in 0..expect_count {
            let data = match { handler.recv_dldata.lock().unwrap().pop() } {
                None => return Err(format!("only receive {}/{} data", i, expect_count)),
                Some(data) => data,
            };
            let correlation_id = data.correlation_id.as_str();
            if correlation_id == "1" {
                expect(data.device_id.as_ref()).equals(data1.device_id.as_ref())?;
                expect(data.network_code.as_ref()).equals(data1.network_code.as_ref())?;
                expect(data.network_addr.as_ref()).equals(data1.network_addr.as_ref())?;
                expect(data.data.as_str()).equals(data1.data.as_str())?;
                expect(data.extension.as_ref()).equals(data1.extension.as_ref())?;
            } else if correlation_id == "2" {
                expect(data.device_id.as_ref()).equals(data2.device_id.as_ref())?;
                expect(data.network_code.as_ref()).equals(data2.network_code.as_ref())?;
                expect(data.network_addr.as_ref()).equals(data2.network_addr.as_ref())?;
                expect(data.data.as_str()).equals(data2.data.as_str())?;
                expect(data.extension.as_ref()).equals(data2.extension.as_ref())?;
            } else {
                return Err(format!("receive wrong data {}", correlation_id));
            }
        }

        Ok(())
    })?;

    Ok(())
}

/// Test sending dldata with wrong content.
pub fn dldata_wrong(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let mut state = context.state.borrow_mut();
    let state = state.get_mut(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let mq_engine = state.mq_engine.as_ref().unwrap().as_str();
    let conn_pool = state.mgr_conns.as_ref().unwrap();

    let host_uri = conn_host_uri(mq_engine)?;
    let mgr_handler = Arc::new(TestHandler::new());

    let opts = Options {
        unit_id: "unit_id".to_string(),
        unit_code: "unit_code".to_string(),
        id: "id_application".to_string(),
        name: "code_application".to_string(),
        shared_prefix: state.mqtt_shared_prefix.clone(),
        ..Default::default()
    };
    let mgr = ApplicationMgr::new(conn_pool.clone(), &host_uri, opts, mgr_handler.clone())?;
    state.app_mgrs = Some(vec![mgr.clone()]);

    runtime.block_on(async move {
        for _ in 0..WAIT_COUNT {
            if mgr.status() == MgrStatus::Ready {
                break;
            }
            time::sleep(Duration::from_millis(WAIT_TICK)).await;
        }
        if mgr.status() != MgrStatus::Ready {
            return Err("manager not ready".to_string());
        }

        let mut data = DlData {
            correlation_id: "".to_string(),
            device_id: Some("device".to_string()),
            network_code: None,
            network_addr: None,
            data: "00".to_string(),
            extension: None,
        };
        expect(mgr.send_dldata(&data).is_err()).equals(true)?;
        data.correlation_id = "1".to_string();
        data.device_id = None;
        expect(mgr.send_dldata(&data).is_err()).equals(true)?;
        data.network_code = Some("".to_string());
        data.network_addr = Some("".to_string());
        expect(mgr.send_dldata(&data).is_err()).equals(true)?;
        data.device_id = Some("".to_string());
        data.network_code = None;
        data.network_addr = None;
        expect(mgr.send_dldata(&data).is_err()).equals(true)?;
        data.device_id = Some("device_id".to_string());
        data.data = "gg".to_string();
        expect(mgr.send_dldata(&data).is_err()).equals(true)?;

        Ok(())
    })?;

    Ok(())
}

/// Test receiving dldata-resp.
pub fn dldata_resp(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let mut state = context.state.borrow_mut();
    let state = state.get_mut(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let mq_engine = state.mq_engine.as_ref().unwrap().as_str();
    let conn_pool = state.mgr_conns.as_ref().unwrap();

    state.app_net_conn = Some(new_connection(runtime, mq_engine)?);
    let conn = state.app_net_conn.as_ref().unwrap();

    let host_uri = conn_host_uri(mq_engine)?;
    let mgr_handler = Arc::new(TestHandler::new());

    let opts = Options {
        unit_id: "unit_id".to_string(),
        unit_code: "unit_code".to_string(),
        id: "id_application".to_string(),
        name: "code_application".to_string(),
        shared_prefix: state.mqtt_shared_prefix.clone(),
        ..Default::default()
    };
    let mgr = ApplicationMgr::new(conn_pool.clone(), &host_uri, opts, mgr_handler.clone())?;
    state.app_mgrs = Some(vec![mgr.clone()]);

    let queue = match conn {
        Connection::Amqp(conn, _) => {
            let opts = QueueOptions::Amqp(
                AmqpQueueOptions {
                    name: "broker.application.unit_code.code_application.dldata-resp".to_string(),
                    is_recv: false,
                    reliable: true,
                    broadcast: false,
                    ..Default::default()
                },
                conn,
            );
            let mut queue_result = Queue::new(opts)?;
            if let Err(e) = queue_result.connect() {
                return Err(format!("connect dldata-resp queue error: {}", e));
            }
            queue_result
        }
        Connection::Mqtt(conn, _) => {
            let opts = QueueOptions::Mqtt(
                MqttQueueOptions {
                    name: "broker.application.unit_code.code_application.dldata-resp".to_string(),
                    is_recv: false,
                    reliable: true,
                    broadcast: false,
                    ..Default::default()
                },
                conn,
            );
            let mut queue_result = Queue::new(opts)?;
            if let Err(e) = queue_result.connect() {
                return Err(format!("connect dldata-resp queue error: {}", e));
            }
            queue_result
        }
    };

    runtime.block_on(async move {
        for _ in 0..WAIT_COUNT {
            if queue.status() == MqStatus::Connected {
                break;
            }
            time::sleep(Duration::from_millis(WAIT_TICK)).await;
        }
        if queue.status() != MqStatus::Connected {
            return Err("send queue not connected".to_string());
        }
        for _ in 0..WAIT_COUNT {
            if mgr.status() == MgrStatus::Ready {
                break;
            }
            time::sleep(Duration::from_millis(WAIT_TICK)).await;
        }
        if mgr.status() != MgrStatus::Ready {
            return Err("manager not ready".to_string());
        }

        let data1 = AppDlDataResp {
            correlation_id: "1".to_string(),
            data_id: Some("data_id1".to_string()),
            error: None,
            message: None,
        };
        let payload = match serde_json::to_vec(&data1) {
            Err(e) => return Err(format!("marshal data1 error: {}", e)),
            Ok(payload) => payload,
        };
        if let Err(e) = queue.send_msg(payload).await {
            return Err(format!("send data1 error: {}", e));
        }
        let data2 = AppDlDataResp {
            correlation_id: "2".to_string(),
            data_id: Some("data_id2".to_string()),
            error: None,
            message: None,
        };
        let payload = match serde_json::to_vec(&data2) {
            Err(e) => return Err(format!("marshal data2 error: {}", e)),
            Ok(payload) => payload,
        };
        if let Err(e) = queue.send_msg(payload).await {
            return Err(format!("send data2 error: {}", e));
        }
        let data3 = AppDlDataResp {
            correlation_id: "3".to_string(),
            data_id: None,
            error: Some("error3".to_string()),
            message: Some("message3".to_string()),
        };
        let payload = match serde_json::to_vec(&data3) {
            Err(e) => return Err(format!("marshal data3 error: {}", e)),
            Ok(payload) => payload,
        };
        if let Err(e) = queue.send_msg(payload).await {
            return Err(format!("send data3 error: {}", e));
        }

        let expect_count = match mq_engine {
            MqEngine::RABBITMQ => 3,
            _ => 2,
        };
        for _ in 0..WAIT_COUNT {
            let count = { mgr_handler.recv_dldata_resp.lock().unwrap().len() };
            if count < expect_count {
                time::sleep(Duration::from_millis(WAIT_TICK)).await;
                continue;
            }
        }
        let count = { mgr_handler.recv_dldata_resp.lock().unwrap().len() };
        if count < expect_count {
            return Err(format!("receive {}/{} data", count, expect_count));
        }

        for i in 0..expect_count {
            let data = match { mgr_handler.recv_dldata_resp.lock().unwrap().pop() } {
                None => return Err(format!("only receive {}/{} data", i, expect_count)),
                Some(data) => data,
            };
            let correlation_id = data.correlation_id.as_str();
            if correlation_id == "1" {
                expect(mq_engine).equals(MqEngine::RABBITMQ)?;
                expect(data.data_id.as_ref()).equals(data1.data_id.as_ref())?;
                expect(data.error.as_ref()).equals(data1.error.as_ref())?;
                expect(data.message.as_ref()).equals(data1.message.as_ref())?;
            } else if correlation_id == "2" {
                expect(data.data_id.as_ref()).equals(data2.data_id.as_ref())?;
                expect(data.error.as_ref()).equals(data2.error.as_ref())?;
                expect(data.message.as_ref()).equals(data2.message.as_ref())?;
            } else if correlation_id == "3" {
                expect(data.data_id.as_ref()).equals(data3.data_id.as_ref())?;
                expect(data.error.as_ref()).equals(data3.error.as_ref())?;
                expect(data.message.as_ref()).equals(data3.message.as_ref())?;
            } else {
                return Err(format!("receive wrong data {}", correlation_id));
            }
        }

        Ok(())
    })?;

    Ok(())
}

/// Test receiving dldata-resp with wrong content.
pub fn dldata_resp_wrong(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let mut state = context.state.borrow_mut();
    let state = state.get_mut(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let mq_engine = state.mq_engine.as_ref().unwrap().as_str();
    let conn_pool = state.mgr_conns.as_ref().unwrap();

    state.app_net_conn = Some(new_connection(runtime, mq_engine)?);
    let conn = state.app_net_conn.as_ref().unwrap();

    let host_uri = conn_host_uri(mq_engine)?;
    let mgr_handler = Arc::new(TestHandler::new());

    let opts = Options {
        unit_id: "unit_id".to_string(),
        unit_code: "unit_code".to_string(),
        id: "id_application".to_string(),
        name: "code_application".to_string(),
        shared_prefix: state.mqtt_shared_prefix.clone(),
        ..Default::default()
    };
    let mgr = ApplicationMgr::new(conn_pool.clone(), &host_uri, opts, mgr_handler.clone())?;
    state.app_mgrs = Some(vec![mgr.clone()]);

    let queue = match conn {
        Connection::Amqp(conn, _) => {
            let opts = QueueOptions::Amqp(
                AmqpQueueOptions {
                    name: "broker.application.unit_code.code_application.dldata-resp".to_string(),
                    is_recv: false,
                    reliable: true,
                    broadcast: false,
                    ..Default::default()
                },
                conn,
            );
            let mut queue_result = Queue::new(opts)?;
            if let Err(e) = queue_result.connect() {
                return Err(format!("connect dldata-resp queue error: {}", e));
            }
            queue_result
        }
        Connection::Mqtt(conn, _) => {
            let opts = QueueOptions::Mqtt(
                MqttQueueOptions {
                    name: "broker.application.unit_code.code_application.dldata-resp".to_string(),
                    is_recv: false,
                    reliable: true,
                    broadcast: false,
                    ..Default::default()
                },
                conn,
            );
            let mut queue_result = Queue::new(opts)?;
            if let Err(e) = queue_result.connect() {
                return Err(format!("connect dldata-resp queue error: {}", e));
            }
            queue_result
        }
    };

    runtime.block_on(async move {
        for _ in 0..WAIT_COUNT {
            if queue.status() == MqStatus::Connected {
                break;
            }
            time::sleep(Duration::from_millis(WAIT_TICK)).await;
        }
        if queue.status() != MqStatus::Connected {
            return Err("send queue not connected".to_string());
        }
        for _ in 0..WAIT_COUNT {
            if mgr.status() == MgrStatus::Ready {
                break;
            }
            time::sleep(Duration::from_millis(WAIT_TICK)).await;
        }
        if mgr.status() != MgrStatus::Ready {
            return Err("manager not ready".to_string());
        }

        if let Err(e) = queue.send_msg(vec![]).await {
            return Err(format!("send empty error: {}", e));
        }

        time::sleep(Duration::from_secs(1)).await;
        let count = { mgr_handler.recv_dldata_resp.lock().unwrap().len() };
        expect(count).equals(0)
    })?;

    Ok(())
}

/// Test receiving dldata-result.
pub fn dldata_result(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let mut state = context.state.borrow_mut();
    let state = state.get_mut(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let mq_engine = state.mq_engine.as_ref().unwrap().as_str();
    let conn_pool = state.mgr_conns.as_ref().unwrap();

    state.app_net_conn = Some(new_connection(runtime, mq_engine)?);
    let conn = state.app_net_conn.as_ref().unwrap();

    let host_uri = conn_host_uri(mq_engine)?;
    let mgr_handler = Arc::new(TestHandler::new());

    let opts = Options {
        unit_id: "unit_id".to_string(),
        unit_code: "unit_code".to_string(),
        id: "id_application".to_string(),
        name: "code_application".to_string(),
        shared_prefix: state.mqtt_shared_prefix.clone(),
        ..Default::default()
    };
    let mgr = ApplicationMgr::new(conn_pool.clone(), &host_uri, opts, mgr_handler.clone())?;
    state.app_mgrs = Some(vec![mgr.clone()]);

    let queue = match conn {
        Connection::Amqp(conn, _) => {
            let opts = QueueOptions::Amqp(
                AmqpQueueOptions {
                    name: "broker.application.unit_code.code_application.dldata-result".to_string(),
                    is_recv: false,
                    reliable: true,
                    broadcast: false,
                    ..Default::default()
                },
                conn,
            );
            let mut queue_result = Queue::new(opts)?;
            if let Err(e) = queue_result.connect() {
                return Err(format!("connect dldata-result queue error: {}", e));
            }
            queue_result
        }
        Connection::Mqtt(conn, _) => {
            let opts = QueueOptions::Mqtt(
                MqttQueueOptions {
                    name: "broker.application.unit_code.code_application.dldata-result".to_string(),
                    is_recv: false,
                    reliable: true,
                    broadcast: false,
                    ..Default::default()
                },
                conn,
            );
            let mut queue_result = Queue::new(opts)?;
            if let Err(e) = queue_result.connect() {
                return Err(format!("connect dldata-result queue error: {}", e));
            }
            queue_result
        }
    };

    runtime.block_on(async move {
        for _ in 0..WAIT_COUNT {
            if queue.status() == MqStatus::Connected {
                break;
            }
            time::sleep(Duration::from_millis(WAIT_TICK)).await;
        }
        if queue.status() != MqStatus::Connected {
            return Err("send queue not connected".to_string());
        }
        for _ in 0..WAIT_COUNT {
            if mgr.status() == MgrStatus::Ready {
                break;
            }
            time::sleep(Duration::from_millis(WAIT_TICK)).await;
        }
        if mgr.status() != MgrStatus::Ready {
            return Err("manager not ready".to_string());
        }

        let data1 = AppDlDataResult {
            data_id: "1".to_string(),
            status: -1,
            message: None,
        };
        let payload = match serde_json::to_vec(&data1) {
            Err(e) => return Err(format!("marshal data1 error: {}", e)),
            Ok(payload) => payload,
        };
        if let Err(e) = queue.send_msg(payload).await {
            return Err(format!("send data1 error: {}", e));
        }
        let data2 = AppDlDataResult {
            data_id: "2".to_string(),
            status: 0,
            message: None,
        };
        let payload = match serde_json::to_vec(&data2) {
            Err(e) => return Err(format!("marshal data2 error: {}", e)),
            Ok(payload) => payload,
        };
        if let Err(e) = queue.send_msg(payload).await {
            return Err(format!("send data2 error: {}", e));
        }
        let data3 = AppDlDataResult {
            data_id: "3".to_string(),
            status: 1,
            message: Some("error".to_string()),
        };
        let payload = match serde_json::to_vec(&data3) {
            Err(e) => return Err(format!("marshal data3 error: {}", e)),
            Ok(payload) => payload,
        };
        if let Err(e) = queue.send_msg(payload).await {
            return Err(format!("send data3 error: {}", e));
        }

        let expect_count = match mq_engine {
            MqEngine::RABBITMQ => 3,
            _ => 2,
        };
        for _ in 0..WAIT_COUNT {
            let count = { mgr_handler.recv_dldata_result.lock().unwrap().len() };
            if count < expect_count {
                time::sleep(Duration::from_millis(WAIT_TICK)).await;
                continue;
            }
        }
        let count = { mgr_handler.recv_dldata_result.lock().unwrap().len() };
        if count < expect_count {
            return Err(format!("receive {}/{} data", count, expect_count));
        }

        for i in 0..expect_count {
            let data = match { mgr_handler.recv_dldata_result.lock().unwrap().pop() } {
                None => return Err(format!("only receive {}/{} data", i, expect_count)),
                Some(data) => data,
            };
            let data_id = data.data_id.as_str();
            if data_id == "1" {
                expect(mq_engine).equals(MqEngine::RABBITMQ)?;
                expect(data.status).equals(data1.status)?;
                expect(data.message.as_ref()).equals(data1.message.as_ref())?;
            } else if data_id == "2" {
                expect(data.status).equals(data2.status)?;
                expect(data.message.as_ref()).equals(data2.message.as_ref())?;
            } else if data_id == "3" {
                expect(data.status).equals(data3.status)?;
                expect(data.message.as_ref()).equals(data3.message.as_ref())?;
            } else {
                return Err(format!("receive wrong data {}", data_id));
            }
        }

        Ok(())
    })?;

    Ok(())
}

/// Test receiving dldata-result with wrong content.
pub fn dldata_result_wrong(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let mut state = context.state.borrow_mut();
    let state = state.get_mut(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let mq_engine = state.mq_engine.as_ref().unwrap().as_str();
    let conn_pool = state.mgr_conns.as_ref().unwrap();

    state.app_net_conn = Some(new_connection(runtime, mq_engine)?);
    let conn = state.app_net_conn.as_ref().unwrap();

    let host_uri = conn_host_uri(mq_engine)?;
    let mgr_handler = Arc::new(TestHandler::new());

    let opts = Options {
        unit_id: "unit_id".to_string(),
        unit_code: "unit_code".to_string(),
        id: "id_application".to_string(),
        name: "code_application".to_string(),
        shared_prefix: state.mqtt_shared_prefix.clone(),
        ..Default::default()
    };
    let mgr = ApplicationMgr::new(conn_pool.clone(), &host_uri, opts, mgr_handler.clone())?;
    state.app_mgrs = Some(vec![mgr.clone()]);

    let queue = match conn {
        Connection::Amqp(conn, _) => {
            let opts = QueueOptions::Amqp(
                AmqpQueueOptions {
                    name: "broker.application.unit_code.code_application.dldata-result".to_string(),
                    is_recv: false,
                    reliable: true,
                    broadcast: false,
                    ..Default::default()
                },
                conn,
            );
            let mut queue_result = Queue::new(opts)?;
            if let Err(e) = queue_result.connect() {
                return Err(format!("connect dldata-result queue error: {}", e));
            }
            queue_result
        }
        Connection::Mqtt(conn, _) => {
            let opts = QueueOptions::Mqtt(
                MqttQueueOptions {
                    name: "broker.application.unit_code.code_application.dldata-result".to_string(),
                    is_recv: false,
                    reliable: true,
                    broadcast: false,
                    ..Default::default()
                },
                conn,
            );
            let mut queue_result = Queue::new(opts)?;
            if let Err(e) = queue_result.connect() {
                return Err(format!("connect dldata-result queue error: {}", e));
            }
            queue_result
        }
    };

    runtime.block_on(async move {
        for _ in 0..WAIT_COUNT {
            if queue.status() == MqStatus::Connected {
                break;
            }
            time::sleep(Duration::from_millis(WAIT_TICK)).await;
        }
        if queue.status() != MqStatus::Connected {
            return Err("send queue not connected".to_string());
        }
        for _ in 0..WAIT_COUNT {
            if mgr.status() == MgrStatus::Ready {
                break;
            }
            time::sleep(Duration::from_millis(WAIT_TICK)).await;
        }
        if mgr.status() != MgrStatus::Ready {
            return Err("manager not ready".to_string());
        }

        if let Err(e) = queue.send_msg(vec![]).await {
            return Err(format!("send empty error: {}", e));
        }

        time::sleep(Duration::from_secs(1)).await;
        let count = { mgr_handler.recv_dldata_result.lock().unwrap().len() };
        expect(count).equals(0)
    })?;

    Ok(())
}
