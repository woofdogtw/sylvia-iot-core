use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::Duration,
};

use async_trait::async_trait;
use general_mq::{
    queue::{Event as MqEvent, EventHandler as MqEventHandler, Message, Queue, Status as MqStatus},
    AmqpQueueOptions, MqttQueueOptions, Queue as MqQueue, QueueOptions as MqQueueOptions,
};
use laboratory::{expect, SpecContext};
use serde::{self, Deserialize, Serialize};
use serde_json::{self, Map, Value};
use tokio::time;

use sylvia_iot_broker::libs::mq::{
    application::{ApplicationMgr, DlData, DlDataResp, DlDataResult, EventHandler, UlData},
    Connection, MgrStatus, Options,
};

use super::{new_connection, STATE};
use crate::{libs::libs::conn_host_uri, TestState, WAIT_COUNT, WAIT_TICK};

/// Downlink data from application to broker.
#[derive(Debug, Default, Serialize)]
pub struct AppDlData {
    #[serde(rename = "correlationId")]
    pub correlation_id: String,
    #[serde(rename = "deviceId", skip_serializing_if = "Option::is_none")]
    pub device_id: Option<String>,
    #[serde(rename = "networkCode", skip_serializing_if = "Option::is_none")]
    pub network_code: Option<String>,
    #[serde(rename = "networkAddr", skip_serializing_if = "Option::is_none")]
    pub network_addr: Option<String>,
    pub data: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extension: Option<Map<String, Value>>,
}

/// Uplink data.
#[derive(Debug, Deserialize)]
pub struct AppUlData {
    #[serde(rename = "dataId")]
    pub data_id: String,
    pub time: String,
    #[serde(rename = "pub")]
    pub publish: String,
    #[serde(rename = "deviceId")]
    pub device_id: String,
    #[serde(rename = "networkId")]
    pub network_id: String,
    #[serde(rename = "networkCode")]
    pub network_code: String,
    #[serde(rename = "networkAddr")]
    pub network_addr: String,
    #[serde(rename = "isPublic")]
    pub is_public: bool,
    pub profile: String,
    pub data: String,
    pub extension: Option<Map<String, Value>>,
}

/// Downlink data response for [`DlData`].
#[derive(Debug, Deserialize)]
pub struct AppDlDataResp {
    #[serde(rename = "correlationId")]
    pub correlation_id: String,
    #[serde(rename = "dataId")]
    pub data_id: Option<String>,
    pub error: Option<String>,
    pub message: Option<String>,
}

/// Downlink data result.
#[derive(Debug, Deserialize)]
pub struct AppDlDataResult {
    #[serde(rename = "dataId")]
    pub data_id: String,
    pub status: i32,
    pub message: Option<String>,
}

struct TestHandler {
    // Use Mutex to implement interior mutability.
    status_changed: Arc<Mutex<bool>>,
    recv_dldata: Arc<Mutex<Vec<Box<DlData>>>>,
}

struct TestUlDataHandler {
    // Use Mutex to implement interior mutability.
    status_connected: Arc<Mutex<bool>>,
    recv_uldata: Arc<Mutex<Vec<Box<AppUlData>>>>,
}

struct TestDlDataRespHandler {
    // Use Mutex to implement interior mutability.
    status_connected: Arc<Mutex<bool>>,
    recv_data_resp: Arc<Mutex<Vec<Box<AppDlDataResp>>>>,
}

struct TestDlDataResultHandler {
    // Use Mutex to implement interior mutability.
    status_connected: Arc<Mutex<bool>>,
    recv_data_result: Arc<Mutex<Vec<Box<AppDlDataResult>>>>,
}

impl TestHandler {
    fn new() -> Self {
        TestHandler {
            status_changed: Arc::new(Mutex::new(false)),
            recv_dldata: Arc::new(Mutex::new(vec![])),
        }
    }
}

#[async_trait]
impl EventHandler for TestHandler {
    async fn on_status_change(&self, _mgr: &ApplicationMgr, _status: MgrStatus) {
        *self.status_changed.lock().unwrap() = true;
    }

    async fn on_dldata(
        &self,
        _mgr: &ApplicationMgr,
        data: Box<DlData>,
    ) -> Result<Box<DlDataResp>, ()> {
        let correlation_id = data.correlation_id.clone();
        let count;
        {
            let mut mutex = self.recv_dldata.lock().unwrap();
            count = mutex.len();
            mutex.push(data);
        }
        if count == 0 {
            Err(())
        } else if count == 1 {
            if correlation_id.as_str() == "1" {
                Ok(Box::new(DlDataResp {
                    correlation_id: correlation_id.clone(),
                    data_id: Some(correlation_id.clone()),
                    ..Default::default()
                }))
            } else {
                Ok(Box::new(DlDataResp {
                    correlation_id: correlation_id.clone(),
                    error: Some(correlation_id.clone()),
                    message: Some(format!("message {}", correlation_id.clone())),
                    ..Default::default()
                }))
            }
        } else {
            if correlation_id.as_str() == "1" {
                Ok(Box::new(DlDataResp {
                    correlation_id: correlation_id.clone(),
                    data_id: Some(correlation_id.clone()),
                    ..Default::default()
                }))
            } else {
                Ok(Box::new(DlDataResp {
                    correlation_id: correlation_id.clone(),
                    error: Some(correlation_id.clone()),
                    message: Some(format!("message {}", correlation_id.clone())),
                    ..Default::default()
                }))
            }
        }
    }
}

impl TestUlDataHandler {
    fn new() -> Self {
        TestUlDataHandler {
            status_connected: Arc::new(Mutex::new(false)),
            recv_uldata: Arc::new(Mutex::new(vec![])),
        }
    }
}

impl TestDlDataRespHandler {
    fn new() -> Self {
        TestDlDataRespHandler {
            status_connected: Arc::new(Mutex::new(false)),
            recv_data_resp: Arc::new(Mutex::new(vec![])),
        }
    }
}

impl TestDlDataResultHandler {
    fn new() -> Self {
        TestDlDataResultHandler {
            status_connected: Arc::new(Mutex::new(false)),
            recv_data_result: Arc::new(Mutex::new(vec![])),
        }
    }
}

#[async_trait]
impl MqEventHandler for TestUlDataHandler {
    async fn on_event(&self, _queue: Arc<dyn Queue>, ev: MqEvent) {
        if let MqEvent::Status(status) = ev {
            if status == MqStatus::Connected {
                *self.status_connected.lock().unwrap() = true;
            }
        }
    }

    async fn on_message(&self, _queue: Arc<dyn Queue>, msg: Box<dyn Message>) {
        let data = match serde_json::from_slice::<AppUlData>(msg.payload()) {
            Err(_) => return,
            Ok(data) => Box::new(data),
        };
        {
            self.recv_uldata.lock().unwrap().push(data);
        }
        let _ = msg.ack().await;
    }
}

#[async_trait]
impl MqEventHandler for TestDlDataRespHandler {
    async fn on_event(&self, _queue: Arc<dyn Queue>, ev: MqEvent) {
        if let MqEvent::Status(status) = ev {
            if status == MqStatus::Connected {
                *self.status_connected.lock().unwrap() = true;
            }
        }
    }

    async fn on_message(&self, _queue: Arc<dyn Queue>, msg: Box<dyn Message>) {
        let data = match serde_json::from_slice::<AppDlDataResp>(msg.payload()) {
            Err(_) => return,
            Ok(data) => Box::new(data),
        };
        {
            self.recv_data_resp.lock().unwrap().push(data);
        }
        let _ = msg.ack().await;
    }
}

#[async_trait]
impl MqEventHandler for TestDlDataResultHandler {
    async fn on_event(&self, _queue: Arc<dyn Queue>, ev: MqEvent) {
        if let MqEvent::Status(status) = ev {
            if status == MqStatus::Connected {
                *self.status_connected.lock().unwrap() = true;
            }
        }
    }

    async fn on_message(&self, _queue: Arc<dyn Queue>, msg: Box<dyn Message>) {
        let data = match serde_json::from_slice::<AppDlDataResult>(msg.payload()) {
            Err(_) => return,
            Ok(data) => Box::new(data),
        };
        self.recv_data_result.lock().unwrap().push(data);
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

/// Test generating uldata.
pub fn uldata(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let mut state = context.state.borrow_mut();
    let state = state.get_mut(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let mq_engine = state.mq_engine.as_ref().unwrap().as_str();

    let conn = new_connection(runtime, mq_engine)?;
    state.mq_conn = Some(conn.clone());
    let conn_pool = Arc::new(Mutex::new(HashMap::new()));
    let host_uri = conn_host_uri(mq_engine)?;
    let handler = Arc::new(TestHandler::new());

    let opts = Options {
        unit_id: "unit_id".to_string(),
        unit_code: "unit_code".to_string(),
        id: "id_application".to_string(),
        name: "code_application".to_string(),
        prefetch: state.amqp_prefetch,
        shared_prefix: state.mqtt_shared_prefix.clone(),
        ..Default::default()
    };
    let mgr = ApplicationMgr::new(conn_pool.clone(), &host_uri, opts, handler.clone())?;
    state.app_mgrs = Some(vec![mgr.clone()]);

    let queue_handler = Arc::new(TestUlDataHandler::new());
    let _queue_result = match conn {
        Connection::Amqp(conn, _) => {
            let opts = MqQueueOptions::Amqp(
                AmqpQueueOptions {
                    name: "broker.application.unit_code.code_application.uldata".to_string(),
                    is_recv: true,
                    reliable: true,
                    broadcast: false,
                    ..Default::default()
                },
                &conn,
            );
            let mut queue_result = MqQueue::new(opts)?;
            queue_result.set_handler(queue_handler.clone());
            if let Err(e) = queue_result.connect() {
                return Err(format!("connect uldata queue error: {}", e));
            }
            queue_result
        }
        Connection::Mqtt(conn, _) => {
            let opts = MqQueueOptions::Mqtt(
                MqttQueueOptions {
                    name: "broker.application.unit_code.code_application.uldata".to_string(),
                    is_recv: true,
                    reliable: true,
                    broadcast: false,
                    ..Default::default()
                },
                &conn,
            );
            let mut queue_result = MqQueue::new(opts)?;
            queue_result.set_handler(queue_handler.clone());
            if let Err(e) = queue_result.connect() {
                return Err(format!("connect uldata queue error: {}", e));
            }
            queue_result
        }
    };

    runtime.block_on(async move {
        for _ in 0..WAIT_COUNT {
            if *queue_handler.status_connected.lock().unwrap() {
                break;
            }
            time::sleep(Duration::from_millis(WAIT_TICK)).await;
        }
        if !*queue_handler.status_connected.lock().unwrap() {
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

        let data1 = UlData {
            data_id: "1".to_string(),
            time: "time1".to_string(),
            publish: "pub1".to_string(),
            device_id: "device_id1".to_string(),
            network_id: "network_id1".to_string(),
            network_code: "network_code1".to_string(),
            network_addr: "network_addr1".to_string(),
            is_public: true,
            profile: "".to_string(),
            data: "da01".to_string(),
            extension: None,
        };
        if let Err(e) = mgr.send_uldata(&data1) {
            return Err(format!("send data1 error: {}", e));
        }
        let mut ext = Map::<String, Value>::new();
        ext.insert("key".to_string(), Value::String("value".to_string()));
        let data2 = UlData {
            data_id: "2".to_string(),
            time: "time2".to_string(),
            publish: "pub2".to_string(),
            device_id: "device_id2".to_string(),
            network_id: "network_id2".to_string(),
            network_code: "network_code2".to_string(),
            network_addr: "network_addr2".to_string(),
            is_public: false,
            profile: "profile".to_string(),
            data: "da02".to_string(),
            extension: Some(ext),
        };
        if let Err(e) = mgr.send_uldata(&data2) {
            return Err(format!("send data2 error: {}", e));
        }
        let data3 = UlData {
            data_id: "3".to_string(),
            time: "time3".to_string(),
            publish: "pub3".to_string(),
            device_id: "device_id3".to_string(),
            network_id: "network_id3".to_string(),
            network_code: "network_code3".to_string(),
            network_addr: "network_addr3".to_string(),
            is_public: true,
            profile: "".to_string(),
            data: "".to_string(),
            extension: None,
        };
        if let Err(e) = mgr.send_uldata(&data3) {
            return Err(format!("send data3 error: {}", e));
        }

        for _ in 0..WAIT_COUNT {
            if queue_handler.recv_uldata.lock().unwrap().len() < 3 {
                time::sleep(Duration::from_millis(WAIT_TICK)).await;
                continue;
            }
        }
        if queue_handler.recv_uldata.lock().unwrap().len() < 3 {
            return Err(format!(
                "receive {}/3 data",
                queue_handler.recv_uldata.lock().unwrap().len()
            ));
        }

        for i in 0..3 {
            let data = match { queue_handler.recv_uldata.lock().unwrap().pop() } {
                None => return Err(format!("only receive {}/3 data", i)),
                Some(data) => data,
            };
            let data_id = data.data_id.as_str();
            if data_id == "1" {
                expect(data.time.as_str()).equals(data1.time.as_str())?;
                expect(data.publish.as_str()).equals(data1.publish.as_str())?;
                expect(data.device_id.as_str()).equals(data1.device_id.as_str())?;
                expect(data.network_id.as_str()).equals(data1.network_id.as_str())?;
                expect(data.network_code.as_str()).equals(data1.network_code.as_str())?;
                expect(data.network_addr.as_str()).equals(data1.network_addr.as_str())?;
                expect(data.is_public).equals(data1.is_public)?;
                expect(data.profile.as_str()).equals(data1.profile.as_str())?;
                expect(data.data.as_str()).equals(data1.data.as_str())?;
                expect(data.extension.as_ref()).equals(data1.extension.as_ref())?;
            } else if data_id == "2" {
                expect(data.time.as_str()).equals(data2.time.as_str())?;
                expect(data.publish.as_str()).equals(data2.publish.as_str())?;
                expect(data.device_id.as_str()).equals(data2.device_id.as_str())?;
                expect(data.network_id.as_str()).equals(data2.network_id.as_str())?;
                expect(data.network_code.as_str()).equals(data2.network_code.as_str())?;
                expect(data.network_addr.as_str()).equals(data2.network_addr.as_str())?;
                expect(data.is_public).equals(data2.is_public)?;
                expect(data.profile.as_str()).equals(data2.profile.as_str())?;
                expect(data.data.as_str()).equals(data2.data.as_str())?;
                expect(data.extension.as_ref()).equals(data2.extension.as_ref())?;
            } else if data_id == "3" {
                expect(data.time.as_str()).equals(data3.time.as_str())?;
                expect(data.publish.as_str()).equals(data3.publish.as_str())?;
                expect(data.device_id.as_str()).equals(data3.device_id.as_str())?;
                expect(data.network_id.as_str()).equals(data3.network_id.as_str())?;
                expect(data.network_code.as_str()).equals(data3.network_code.as_str())?;
                expect(data.network_addr.as_str()).equals(data3.network_addr.as_str())?;
                expect(data.is_public).equals(data3.is_public)?;
                expect(data.profile.as_str()).equals(data3.profile.as_str())?;
                expect(data.data.as_str()).equals(data3.data.as_str())?;
                expect(data.extension.as_ref()).equals(data3.extension.as_ref())?;
            } else {
                return Err(format!("receive wrong data {}", data_id));
            }
        }

        if let Err(e) = mgr.close().await {
            return Err(format!("close manager error: {}", e));
        }
        Ok(())
    })?;
    Ok(())
}

/// Test receiving dldata and test dldata-resp.
pub fn dldata(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let mut state = context.state.borrow_mut();
    let state = state.get_mut(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let mq_engine = state.mq_engine.as_ref().unwrap().as_str();

    let conn = new_connection(runtime, mq_engine)?;
    state.mq_conn = Some(conn.clone());
    let conn_pool = Arc::new(Mutex::new(HashMap::new()));
    let host_uri = conn_host_uri(mq_engine)?;
    let handler = Arc::new(TestHandler::new());

    let opts = Options {
        unit_id: "unit_id".to_string(),
        unit_code: "unit_code".to_string(),
        id: "id_application".to_string(),
        name: "code_application".to_string(),
        prefetch: state.amqp_prefetch,
        shared_prefix: state.mqtt_shared_prefix.clone(),
        ..Default::default()
    };
    let mgr = ApplicationMgr::new(conn_pool.clone(), &host_uri, opts, handler.clone())?;
    state.app_mgrs = Some(vec![mgr.clone()]);

    let recv_dldata_count;
    let recv_dldata_resp_count;
    let queue_handler = Arc::new(TestDlDataRespHandler::new());
    let (queue_send, _queue_resp) = match conn {
        Connection::Amqp(conn, _) => {
            recv_dldata_count = 3;
            recv_dldata_resp_count = 2;
            let opts = MqQueueOptions::Amqp(
                AmqpQueueOptions {
                    name: "broker.application.unit_code.code_application.dldata".to_string(),
                    is_recv: false,
                    reliable: true,
                    broadcast: false,
                    ..Default::default()
                },
                &conn,
            );
            let mut queue_send = MqQueue::new(opts)?;
            if let Err(e) = queue_send.connect() {
                return Err(format!("connect dldata queue error: {}", e));
            }

            let opts = MqQueueOptions::Amqp(
                AmqpQueueOptions {
                    name: "broker.application.unit_code.code_application.dldata-resp".to_string(),
                    is_recv: true,
                    reliable: true,
                    broadcast: false,
                    ..Default::default()
                },
                &conn,
            );
            let mut queue_resp = MqQueue::new(opts)?;
            queue_resp.set_handler(queue_handler.clone());
            if let Err(e) = queue_resp.connect() {
                return Err(format!("connect dldata-resp queue error: {}", e));
            }
            (queue_send, queue_resp)
        }
        Connection::Mqtt(conn, _) => {
            recv_dldata_count = 2;
            recv_dldata_resp_count = 1;
            let opts = MqQueueOptions::Mqtt(
                MqttQueueOptions {
                    name: "broker.application.unit_code.code_application.dldata".to_string(),
                    is_recv: false,
                    reliable: true,
                    broadcast: false,
                    ..Default::default()
                },
                &conn,
            );
            let mut queue_send = MqQueue::new(opts)?;
            if let Err(e) = queue_send.connect() {
                return Err(format!("connect dldata queue error: {}", e));
            }

            let opts = MqQueueOptions::Mqtt(
                MqttQueueOptions {
                    name: "broker.application.unit_code.code_application.dldata-resp".to_string(),
                    is_recv: true,
                    reliable: true,
                    broadcast: false,
                    ..Default::default()
                },
                &conn,
            );
            let mut queue_resp = MqQueue::new(opts)?;
            queue_resp.set_handler(queue_handler.clone());
            if let Err(e) = queue_resp.connect() {
                return Err(format!("connect dldata-resp queue error: {}", e));
            }
            (queue_send, queue_resp)
        }
    };

    runtime.block_on(async move {
        for _ in 0..WAIT_COUNT {
            if *queue_handler.status_connected.lock().unwrap() {
                break;
            }
            time::sleep(Duration::from_millis(WAIT_TICK)).await;
        }
        if !*queue_handler.status_connected.lock().unwrap() {
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

        let mut ext = Map::<String, Value>::new();
        ext.insert("key".to_string(), Value::String("value".to_string()));
        let send_data1 = AppDlData {
            correlation_id: "1".to_string(),
            device_id: Some("device1".to_string()),
            data: "da01".to_string(),
            extension: Some(ext),
            ..Default::default()
        };
        let payload = match serde_json::to_vec(&send_data1) {
            Err(e) => return Err(format!("generate payload 1 error: {}", e)),
            Ok(data) => data,
        };
        if let Err(e) = queue_send.send_msg(payload).await {
            return Err(format!("send DlData 1 error: {}", e));
        }
        let send_data2 = AppDlData {
            correlation_id: "2".to_string(),
            network_code: Some("code".to_string()),
            network_addr: Some("addr2".to_string()),
            data: "da02".to_string(),
            extension: None,
            ..Default::default()
        };
        let payload = match serde_json::to_vec(&send_data2) {
            Err(e) => return Err(format!("generate payload 2 error: {}", e)),
            Ok(data) => data,
        };
        if let Err(e) = queue_send.send_msg(payload).await {
            return Err(format!("send DlData 2 error: {}", e));
        }
        for _ in 0..WAIT_COUNT {
            if handler.recv_dldata.lock().unwrap().len() < recv_dldata_count {
                time::sleep(Duration::from_millis(WAIT_TICK)).await;
                continue;
            } else if queue_handler.recv_data_resp.lock().unwrap().len() < recv_dldata_resp_count {
                time::sleep(Duration::from_millis(WAIT_TICK)).await;
                continue;
            }
        }
        if handler.recv_dldata.lock().unwrap().len() < recv_dldata_count {
            return Err(format!("receive {} dldata", {
                handler.recv_dldata.lock().unwrap().len()
            }));
        } else if queue_handler.recv_data_resp.lock().unwrap().len() < recv_dldata_resp_count {
            return Err(format!("receive {} dldata-resp", {
                queue_handler.recv_data_resp.lock().unwrap().len()
            }));
        }

        let mut recv_1_count = recv_dldata_count - 1;
        let mut recv_2_count = 1;
        for _ in 0..recv_dldata_count {
            let recv_data = match { handler.recv_dldata.lock().unwrap().pop() } {
                None => return Err("receive no data".to_string()),
                Some(data) => data,
            };
            let correlation_id = recv_data.correlation_id.as_str();
            if correlation_id == "1" {
                if recv_1_count == 0 {
                    return Err("receive data-1 more than expect".to_string());
                }
                recv_1_count -= 1;
                expect(recv_data.device_id.as_ref()).equals(send_data1.device_id.as_ref())?;
                expect(recv_data.network_code.as_ref()).equals(send_data1.network_code.as_ref())?;
                expect(recv_data.network_addr.as_ref()).equals(send_data1.network_addr.as_ref())?;
                expect(recv_data.data.as_str()).equals(send_data1.data.as_str())?;
                expect(recv_data.extension.as_ref()).equals(send_data1.extension.as_ref())?;
            } else if correlation_id == "2" {
                if recv_2_count == 0 {
                    return Err("receive data-2 more than expect".to_string());
                }
                recv_2_count -= 1;
                expect(recv_data.device_id.as_ref()).equals(send_data2.device_id.as_ref())?;
                expect(recv_data.network_code.as_ref()).equals(send_data2.network_code.as_ref())?;
                expect(recv_data.network_addr.as_ref()).equals(send_data2.network_addr.as_ref())?;
                expect(recv_data.data.as_str()).equals(send_data2.data.as_str())?;
                expect(recv_data.extension.as_ref()).equals(send_data2.extension.as_ref())?;
            } else {
                return Err(format!("receive wrong data correlation {}", correlation_id));
            }
        }

        let mut recv_1_count = 1;
        let mut recv_2_count = 1;
        for _ in 0..recv_dldata_resp_count {
            let resp = match { queue_handler.recv_data_resp.lock().unwrap().pop() } {
                None => return Err("receive no response".to_string()),
                Some(resp) => resp,
            };
            let correlation_id = resp.correlation_id.as_str();
            if correlation_id == "1" {
                if recv_1_count == 0 {
                    return Err("receive resp-1 more than expect".to_string());
                }
                recv_1_count -= 1;
                expect(resp.data_id).equals(Some("1".to_string()))?;
                expect(resp.error).equals(None)?;
                expect(resp.message).equals(None)?;
            } else if correlation_id == "2" {
                if recv_2_count == 0 {
                    return Err("receive resp-2 more than expect".to_string());
                }
                recv_2_count -= 1;
                expect(resp.data_id).equals(None)?;
                expect(resp.error).equals(Some("2".to_string()))?;
                expect(resp.message).equals(Some("message 2".to_string()))?;
            } else {
                return Err(format!(
                    "receive wrong response correlation {}",
                    correlation_id
                ));
            }
        }

        if let Err(e) = mgr.close().await {
            return Err(format!("close manager error: {}", e));
        }
        Ok(())
    })?;
    Ok(())
}

/// Test dldata with wrong parameters.
pub fn dldata_wrong(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let mut state = context.state.borrow_mut();
    let state = state.get_mut(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let mq_engine = state.mq_engine.as_ref().unwrap().as_str();

    let conn = new_connection(runtime, mq_engine)?;
    state.mq_conn = Some(conn.clone());
    let conn_pool = Arc::new(Mutex::new(HashMap::new()));
    let host_uri = conn_host_uri(mq_engine)?;
    let handler = Arc::new(TestHandler::new());

    let opts = Options {
        unit_id: "unit_id".to_string(),
        unit_code: "unit_code".to_string(),
        id: "id_application".to_string(),
        name: "code_application".to_string(),
        prefetch: state.amqp_prefetch,
        shared_prefix: state.mqtt_shared_prefix.clone(),
        ..Default::default()
    };
    let mgr = ApplicationMgr::new(conn_pool.clone(), &host_uri, opts, handler.clone())?;
    state.app_mgrs = Some(vec![mgr.clone()]);

    let queue_handler = Arc::new(TestDlDataRespHandler::new());
    let (queue_send, _queue_resp) = match conn {
        Connection::Amqp(conn, _) => {
            let opts = MqQueueOptions::Amqp(
                AmqpQueueOptions {
                    name: "broker.application.unit_code.code_application.dldata".to_string(),
                    is_recv: false,
                    reliable: true,
                    broadcast: false,
                    ..Default::default()
                },
                &conn,
            );
            let mut queue_send = MqQueue::new(opts)?;
            if let Err(e) = queue_send.connect() {
                return Err(format!("connect dldata queue error: {}", e));
            }

            let opts = MqQueueOptions::Amqp(
                AmqpQueueOptions {
                    name: "broker.application.unit_code.code_application.dldata-resp".to_string(),
                    is_recv: true,
                    reliable: true,
                    broadcast: false,
                    ..Default::default()
                },
                &conn,
            );
            let mut queue_resp = MqQueue::new(opts)?;
            queue_resp.set_handler(queue_handler.clone());
            if let Err(e) = queue_resp.connect() {
                return Err(format!("connect dldata-resp queue error: {}", e));
            }
            (queue_send, queue_resp)
        }
        Connection::Mqtt(conn, _) => {
            let opts = MqQueueOptions::Mqtt(
                MqttQueueOptions {
                    name: "broker.application.unit_code.code_application.dldata".to_string(),
                    is_recv: false,
                    reliable: true,
                    broadcast: false,
                    ..Default::default()
                },
                &conn,
            );
            let mut queue_send = MqQueue::new(opts)?;
            if let Err(e) = queue_send.connect() {
                return Err(format!("connect dldata queue error: {}", e));
            }

            let opts = MqQueueOptions::Mqtt(
                MqttQueueOptions {
                    name: "broker.application.unit_code.code_application.dldata-resp".to_string(),
                    is_recv: true,
                    reliable: true,
                    broadcast: false,
                    ..Default::default()
                },
                &conn,
            );
            let mut queue_resp = MqQueue::new(opts)?;
            queue_resp.set_handler(queue_handler.clone());
            if let Err(e) = queue_resp.connect() {
                return Err(format!("connect dldata-resp queue error: {}", e));
            }
            (queue_send, queue_resp)
        }
    };

    runtime.block_on(async move {
        for _ in 0..WAIT_COUNT {
            if *queue_handler.status_connected.lock().unwrap() {
                break;
            }
            time::sleep(Duration::from_millis(WAIT_TICK)).await;
        }
        if !*queue_handler.status_connected.lock().unwrap() {
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

        if let Err(e) = queue_send.send_msg("{}".as_bytes().to_vec()).await {
            return Err(format!("send DlData 0 error: {}", e));
        }
        let send_data1 = AppDlData {
            correlation_id: "".to_string(),
            data: "".to_string(),
            ..Default::default()
        };
        let payload = match serde_json::to_vec(&send_data1) {
            Err(e) => return Err(format!("generate payload 1 error: {}", e)),
            Ok(data) => data,
        };
        if let Err(e) = queue_send.send_msg(payload).await {
            return Err(format!("send DlData 1 error: {}", e));
        }
        let send_data2 = AppDlData {
            correlation_id: "2".to_string(),
            data: "".to_string(),
            ..Default::default()
        };
        let payload = match serde_json::to_vec(&send_data2) {
            Err(e) => return Err(format!("generate payload 2 error: {}", e)),
            Ok(data) => data,
        };
        if let Err(e) = queue_send.send_msg(payload).await {
            return Err(format!("send DlData 2 error: {}", e));
        }
        let send_data3 = AppDlData {
            correlation_id: "3".to_string(),
            device_id: Some("".to_string()),
            data: "".to_string(),
            ..Default::default()
        };
        let payload = match serde_json::to_vec(&send_data3) {
            Err(e) => return Err(format!("generate payload 3 error: {}", e)),
            Ok(data) => data,
        };
        if let Err(e) = queue_send.send_msg(payload).await {
            return Err(format!("send DlData 3 error: {}", e));
        }
        let send_data4 = AppDlData {
            correlation_id: "4".to_string(),
            network_code: Some("".to_string()),
            network_addr: Some("addr".to_string()),
            data: "".to_string(),
            ..Default::default()
        };
        let payload = match serde_json::to_vec(&send_data4) {
            Err(e) => return Err(format!("generate payload 4 error: {}", e)),
            Ok(data) => data,
        };
        if let Err(e) = queue_send.send_msg(payload).await {
            return Err(format!("send DlData 4 error: {}", e));
        }
        let send_data5 = AppDlData {
            correlation_id: "5".to_string(),
            network_code: Some("code".to_string()),
            network_addr: Some("".to_string()),
            data: "".to_string(),
            ..Default::default()
        };
        let payload = match serde_json::to_vec(&send_data5) {
            Err(e) => return Err(format!("generate payload 5 error: {}", e)),
            Ok(data) => data,
        };
        if let Err(e) = queue_send.send_msg(payload).await {
            return Err(format!("send DlData 5 error: {}", e));
        }
        let send_data6 = AppDlData {
            correlation_id: "6".to_string(),
            network_code: Some("code".to_string()),
            data: "".to_string(),
            ..Default::default()
        };
        let payload = match serde_json::to_vec(&send_data6) {
            Err(e) => return Err(format!("generate payload 6 error: {}", e)),
            Ok(data) => data,
        };
        if let Err(e) = queue_send.send_msg(payload).await {
            return Err(format!("send DlData 6 error: {}", e));
        }
        let send_data7 = AppDlData {
            correlation_id: "7".to_string(),
            network_code: Some("code".to_string()),
            network_addr: Some("addr".to_string()),
            data: "zz".to_string(),
            ..Default::default()
        };
        let payload = match serde_json::to_vec(&send_data7) {
            Err(e) => return Err(format!("generate payload 7 error: {}", e)),
            Ok(data) => data,
        };
        if let Err(e) = queue_send.send_msg(payload).await {
            return Err(format!("send DlData 7 error: {}", e));
        }

        for _ in 0..WAIT_COUNT {
            if queue_handler.recv_data_resp.lock().unwrap().len() < 8 {
                time::sleep(Duration::from_millis(WAIT_TICK)).await;
                continue;
            }
        }
        if queue_handler.recv_data_resp.lock().unwrap().len() < 8 {
            return Err(format!(
                "receive {} dldata-resp",
                queue_handler.recv_data_resp.lock().unwrap().len()
            ));
        }

        for i in 0..8 {
            let resp = { queue_handler.recv_data_resp.lock().unwrap().pop() };
            match resp {
                None => return Err(format!("only receive {}/8 data", i)),
                Some(resp) => expect(resp.error).equals(Some("err_param".to_string()))?,
            }
        }

        if let Err(e) = mgr.close().await {
            return Err(format!("close manager error: {}", e));
        }
        Ok(())
    })?;
    Ok(())
}

/// Test generating dldata-result.
pub fn dldata_result(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let mut state = context.state.borrow_mut();
    let state = state.get_mut(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let mq_engine = state.mq_engine.as_ref().unwrap().as_str();

    let conn = new_connection(runtime, mq_engine)?;
    state.mq_conn = Some(conn.clone());
    let conn_pool = Arc::new(Mutex::new(HashMap::new()));
    let host_uri = conn_host_uri(mq_engine)?;
    let handler = Arc::new(TestHandler::new());

    let opts = Options {
        unit_id: "unit_id".to_string(),
        unit_code: "unit_code".to_string(),
        id: "id_application".to_string(),
        name: "code_application".to_string(),
        prefetch: state.amqp_prefetch,
        shared_prefix: state.mqtt_shared_prefix.clone(),
        ..Default::default()
    };
    let mgr = ApplicationMgr::new(conn_pool.clone(), &host_uri, opts, handler.clone())?;
    state.app_mgrs = Some(vec![mgr.clone()]);

    let queue_handler = Arc::new(TestDlDataResultHandler::new());
    let _queue_result = match conn {
        Connection::Amqp(conn, _) => {
            let opts = MqQueueOptions::Amqp(
                AmqpQueueOptions {
                    name: "broker.application.unit_code.code_application.dldata-result".to_string(),
                    is_recv: true,
                    reliable: true,
                    broadcast: false,
                    ..Default::default()
                },
                &conn,
            );
            let mut queue_result = MqQueue::new(opts)?;
            queue_result.set_handler(queue_handler.clone());
            if let Err(e) = queue_result.connect() {
                return Err(format!("connect dldata-result queue error: {}", e));
            }
            queue_result
        }
        Connection::Mqtt(conn, _) => {
            let opts = MqQueueOptions::Mqtt(
                MqttQueueOptions {
                    name: "broker.application.unit_code.code_application.dldata-result".to_string(),
                    is_recv: true,
                    reliable: true,
                    broadcast: false,
                    ..Default::default()
                },
                &conn,
            );
            let mut queue_result = MqQueue::new(opts)?;
            queue_result.set_handler(queue_handler.clone());
            if let Err(e) = queue_result.connect() {
                return Err(format!("connect dldata-result queue error: {}", e));
            }
            queue_result
        }
    };

    runtime.block_on(async move {
        for _ in 0..WAIT_COUNT {
            if *queue_handler.status_connected.lock().unwrap() {
                break;
            }
            time::sleep(Duration::from_millis(WAIT_TICK)).await;
        }
        if !*queue_handler.status_connected.lock().unwrap() {
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

        let result1 = DlDataResult {
            data_id: "1".to_string(),
            status: 0,
            message: None,
        };
        if let Err(e) = mgr.send_dldata_result(&result1).await {
            return Err(format!("send result1 error: {}", e));
        }
        let result2 = DlDataResult {
            data_id: "2".to_string(),
            status: 1,
            message: Some("message".to_string()),
        };
        if let Err(e) = mgr.send_dldata_result(&result2).await {
            return Err(format!("send result2 error: {}", e));
        }

        for _ in 0..WAIT_COUNT {
            if queue_handler.recv_data_result.lock().unwrap().len() < 2 {
                time::sleep(Duration::from_millis(WAIT_TICK)).await;
                continue;
            }
        }
        if queue_handler.recv_data_result.lock().unwrap().len() < 2 {
            return Err(format!(
                "receive {}/2 result",
                queue_handler.recv_data_result.lock().unwrap().len()
            ));
        }

        for i in 0..2 {
            let result = match { queue_handler.recv_data_result.lock().unwrap().pop() } {
                None => return Err(format!("only receive {}/2 results", i)),
                Some(result) => result,
            };
            if result.data_id.as_str() == "1" {
                expect(result.status).equals(0)?;
                expect(result.message).equals(None)?;
            } else {
                expect(result.status).equals(1)?;
                expect(result.message).equals(Some("message".to_string()))?;
            }
        }

        if let Err(e) = mgr.close().await {
            return Err(format!("close manager error: {}", e));
        }
        Ok(())
    })?;
    Ok(())
}
