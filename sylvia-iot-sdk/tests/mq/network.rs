use std::{
    collections::HashMap,
    error::Error as StdError,
    sync::{Arc, Mutex},
    time::Duration,
};

use async_trait::async_trait;
use chrono::{TimeZone, Utc};
use laboratory::{SpecContext, expect};
use serde::{self, Deserialize, Serialize};
use serde_json::{self, Map, Value};
use tokio::time;

use general_mq::{
    AmqpQueueOptions, MqttQueueOptions, Queue, QueueOptions,
    queue::{
        EventHandler as MqEventHandler, GmqQueue, Message, MessageHandler as MqMessageHandler,
        Status as MqStatus,
    },
};
use sylvia_iot_sdk::{
    mq::{
        Connection, MgrStatus, Options,
        network::{DlData, DlDataResult, EventHandler, NetworkCtrlMsg, NetworkMgr, UlData},
    },
    util::strings,
};

use super::{MqEngine, STATE, conn_host_uri, new_connection};
use crate::{TestState, WAIT_COUNT, WAIT_TICK};

/// Uplink data from network to broker.
#[derive(Debug, Deserialize)]
struct NetUlData {
    time: String,
    #[serde(rename = "networkAddr")]
    network_addr: String,
    data: String,
    extension: Option<Map<String, Value>>,
}

/// Downlink data from broker to network.
#[derive(Serialize)]
struct NetDlData {
    #[serde(rename = "dataId")]
    data_id: String,
    #[serde(rename = "pub")]
    publish: String,
    #[serde(rename = "expiresIn")]
    expires_in: i64,
    #[serde(rename = "networkAddr")]
    network_addr: String,
    data: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    extension: Option<Map<String, Value>>,
}

/// Downlink data result when processing or completing data transfer to the device.
#[derive(Debug, Deserialize)]
struct NetDlDataResult {
    #[serde(rename = "dataId")]
    data_id: String,
    status: i32,
    message: Option<String>,
}

/// Control message from broker to network servers.
#[derive(Serialize)]
#[serde(untagged)]
enum SendNetCtrlMsg {
    AddDevice {
        time: String,
        operation: String,
        new: NetCtrlAddr,
    },
    AddDeviceBulk {
        time: String,
        operation: String,
        new: NetCtrlAddrs,
    },
    AddDeviceRange {
        time: String,
        operation: String,
        new: NetCtrlAddrRange,
    },
    DelDevice {
        time: String,
        operation: String,
        new: NetCtrlAddr,
    },
    DelDeviceBulk {
        time: String,
        operation: String,
        new: NetCtrlAddrs,
    },
    DelDeviceRange {
        time: String,
        operation: String,
        new: NetCtrlAddrRange,
    },
}

/// Shared structure to keep simple design.
#[derive(Serialize)]
struct NetCtrlAddr {
    #[serde(rename = "networkAddr")]
    network_addr: String,
}

/// Shared structure to keep simple design.
#[derive(Serialize)]
struct NetCtrlAddrs {
    #[serde(rename = "networkAddrs")]
    network_addrs: Vec<String>,
}

/// Shared structure to keep simple design.
#[derive(Serialize)]
struct NetCtrlAddrRange {
    #[serde(rename = "startAddr")]
    pub start_addr: String,
    #[serde(rename = "endAddr")]
    pub end_addr: String,
}

struct NetCtrlMsgOp;

struct TestHandler {
    // Use Mutex to implement interior mutability.
    status_changed: Arc<Mutex<bool>>,
    recv_dldata: Arc<Mutex<Vec<Box<DlData>>>>,
    recv_ctrl: Arc<Mutex<Vec<Box<NetworkCtrlMsg>>>>,
    is_dldata_recv: Arc<Mutex<bool>>,
    is_ctrl_recv: Arc<Mutex<bool>>,
}

#[derive(Clone)]
struct TestUlDataHandler {
    // Use Mutex to implement interior mutability.
    status_connected: Arc<Mutex<bool>>,
    recv_uldata: Arc<Mutex<Vec<Box<NetUlData>>>>,
}

#[derive(Clone)]
struct TestDlDataResultHandler {
    // Use Mutex to implement interior mutability.
    status_connected: Arc<Mutex<bool>>,
    recv_dldata_result: Arc<Mutex<Vec<Box<NetDlDataResult>>>>,
}

impl NetCtrlMsgOp {
    const ADD_DEVICE: &'static str = "add-device";
    const ADD_DEVICE_BULK: &'static str = "add-device-bulk";
    const ADD_DEVICE_RANGE: &'static str = "add-device-range";
    const DEL_DEVICE: &'static str = "del-device";
    const DEL_DEVICE_BULK: &'static str = "del-device-bulk";
    const DEL_DEVICE_RANGE: &'static str = "del-device-range";
}

impl TestHandler {
    fn new() -> Self {
        TestHandler {
            status_changed: Arc::new(Mutex::new(false)),
            recv_dldata: Arc::new(Mutex::new(vec![])),
            recv_ctrl: Arc::new(Mutex::new(vec![])),
            is_dldata_recv: Arc::new(Mutex::new(false)),
            is_ctrl_recv: Arc::new(Mutex::new(false)),
        }
    }
}

#[async_trait]
impl EventHandler for TestHandler {
    async fn on_status_change(&self, _mgr: &NetworkMgr, _status: MgrStatus) {
        *self.status_changed.lock().unwrap() = true;
    }

    async fn on_dldata(&self, _mgr: &NetworkMgr, data: Box<DlData>) -> Result<(), ()> {
        {
            let mut mutex = self.is_dldata_recv.lock().unwrap();
            if !*mutex {
                *mutex = true;
                return Err(()); // test AMQP NACK.
            }
        }

        let mut mutex = self.recv_dldata.lock().unwrap();
        mutex.push(data);
        Ok(())
    }

    async fn on_ctrl(&self, _mgr: &NetworkMgr, data: Box<NetworkCtrlMsg>) -> Result<(), ()> {
        {
            let mut mutex = self.is_ctrl_recv.lock().unwrap();
            if !*mutex {
                *mutex = true;
            }
        }

        let mut mutex = self.recv_ctrl.lock().unwrap();
        mutex.push(data);
        Ok(())
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

impl TestDlDataResultHandler {
    fn new() -> Self {
        TestDlDataResultHandler {
            status_connected: Arc::new(Mutex::new(false)),
            recv_dldata_result: Arc::new(Mutex::new(vec![])),
        }
    }
}

#[async_trait]
impl MqEventHandler for TestUlDataHandler {
    async fn on_error(&self, _queue: Arc<dyn GmqQueue>, _err: Box<dyn StdError + Send + Sync>) {}

    async fn on_status(&self, _queue: Arc<dyn GmqQueue>, status: MqStatus) {
        if status == MqStatus::Connected {
            *self.status_connected.lock().unwrap() = true;
        }
    }
}

#[async_trait]
impl MqMessageHandler for TestUlDataHandler {
    async fn on_message(&self, _queue: Arc<dyn GmqQueue>, msg: Box<dyn Message>) {
        let data = match serde_json::from_slice::<NetUlData>(msg.payload()) {
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
impl MqEventHandler for TestDlDataResultHandler {
    async fn on_error(&self, _queue: Arc<dyn GmqQueue>, _err: Box<dyn StdError + Send + Sync>) {}

    async fn on_status(&self, _queue: Arc<dyn GmqQueue>, status: MqStatus) {
        if status == MqStatus::Connected {
            *self.status_connected.lock().unwrap() = true;
        }
    }
}

#[async_trait]
impl MqMessageHandler for TestDlDataResultHandler {
    async fn on_message(&self, _queue: Arc<dyn GmqQueue>, msg: Box<dyn Message>) {
        let data = match serde_json::from_slice::<NetDlDataResult>(msg.payload()) {
            Err(_) => return,
            Ok(data) => Box::new(data),
        };
        {
            self.recv_dldata_result.lock().unwrap().push(data);
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

    let opts = Options {
        unit_id: "unit_id".to_string(),
        unit_code: "unit_code".to_string(),
        id: "id_network".to_string(),
        name: "code_network".to_string(),
        shared_prefix: state.mqtt_shared_prefix.clone(),
        ..Default::default()
    };
    let handler = Arc::new(TestHandler::new());
    let mgr = NetworkMgr::new(conn_pool, &host_uri, opts, handler.clone())?;
    let mq_status = mgr.mq_status();
    state.net_mgrs = Some(vec![mgr.clone()]);

    expect(mgr.unit_id()).equals("unit_id")?;
    expect(mgr.unit_code()).equals("unit_code")?;
    expect(mgr.id()).equals("id_network")?;
    expect(mgr.name()).equals("code_network")?;
    expect(mq_status.dldata_resp == MqStatus::Closed).equals(true)?;

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
    expect(mq_status.dldata_resp == MqStatus::Closed).equals(true)?;
    expect(mq_status.dldata_result == MqStatus::Connected).equals(true)?;
    expect(mq_status.ctrl == MqStatus::Connected).equals(true)?;

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
        unit_id: "".to_string(),
        unit_code: "".to_string(),
        id: "id_network".to_string(),
        name: "code_network".to_string(),
        prefetch: Some(0),
        shared_prefix: state.mqtt_shared_prefix.clone(),
        ..Default::default()
    };
    let mgr = NetworkMgr::new(conn_pool.clone(), &host_uri, opts, handler.clone())?;
    state.net_mgrs = Some(vec![mgr]);

    let opts = Options {
        unit_id: "unit_id".to_string(),
        unit_code: "unit_code".to_string(),
        id: "id_network".to_string(),
        name: "code_network".to_string(),
        prefetch: Some(0),
        persistent: Some(false),
        shared_prefix: state.mqtt_shared_prefix.clone(),
        ..Default::default()
    };
    let mgr = NetworkMgr::new(conn_pool.clone(), &host_uri, opts, handler.clone())?;
    state.net_mgrs.as_mut().unwrap().push(mgr);

    let opts = Options {
        unit_id: "unit_id".to_string(),
        unit_code: "unit_code".to_string(),
        id: "id_network".to_string(),
        name: "code_network".to_string(),
        prefetch: Some(1),
        persistent: Some(true),
        shared_prefix: state.mqtt_shared_prefix.clone(),
        ..Default::default()
    };
    let mgr = NetworkMgr::new(conn_pool.clone(), &host_uri, opts, handler.clone())?;
    state.net_mgrs.as_mut().unwrap().push(mgr);

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
        unit_id: "".to_string(),
        unit_code: "unit_code".to_string(),
        ..Default::default()
    };
    expect(NetworkMgr::new(conn_pool.clone(), &host_uri, opts, handler.clone()).is_err())
        .equals(true)?;
    let opts = Options {
        unit_id: "unit_id".to_string(),
        unit_code: "".to_string(),
        ..Default::default()
    };
    expect(NetworkMgr::new(conn_pool.clone(), &host_uri, opts, handler.clone()).is_err())
        .equals(true)?;
    let opts = Options {
        unit_id: "unit_id".to_string(),
        unit_code: "unit_code".to_string(),
        ..Default::default()
    };
    expect(NetworkMgr::new(conn_pool.clone(), &host_uri, opts, handler.clone()).is_err())
        .equals(true)?;
    let opts = Options {
        unit_id: "unit_id".to_string(),
        unit_code: "unit_code".to_string(),
        id: "id".to_string(),
        ..Default::default()
    };
    expect(NetworkMgr::new(conn_pool.clone(), &host_uri, opts, handler.clone()).is_err())
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
        id: "id_network".to_string(),
        name: "code_network".to_string(),
        prefetch: Some(0),
        shared_prefix: state.mqtt_shared_prefix.clone(),
        ..Default::default()
    };
    let mgr = NetworkMgr::new(conn_pool.clone(), &host_uri, opts, handler.clone())?;

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
    let conn_pool = state.mgr_conns.as_ref().unwrap();

    state.app_net_conn = Some(new_connection(runtime, mq_engine)?);
    let conn = state.app_net_conn.as_ref().unwrap();

    let host_uri = conn_host_uri(mq_engine)?;
    let mgr_handler = Arc::new(TestHandler::new());

    let opts = Options {
        unit_id: "unit_id".to_string(),
        unit_code: "unit_code".to_string(),
        id: "id_network".to_string(),
        name: "code_network".to_string(),
        shared_prefix: state.mqtt_shared_prefix.clone(),
        ..Default::default()
    };
    let mgr = NetworkMgr::new(conn_pool.clone(), &host_uri, opts, mgr_handler.clone())?;
    state.net_mgrs = Some(vec![mgr.clone()]);

    let handler = TestUlDataHandler::new();
    let queue = match conn {
        Connection::Amqp(conn, _) => {
            let opts = QueueOptions::Amqp(
                AmqpQueueOptions {
                    name: "broker.network.unit_code.code_network.uldata".to_string(),
                    is_recv: true,
                    reliable: true,
                    broadcast: false,
                    ..Default::default()
                },
                conn,
            );
            let mut queue_result = Queue::new(opts)?;
            queue_result.set_handler(Arc::new(handler.clone()));
            queue_result.set_msg_handler(Arc::new(handler.clone()));
            if let Err(e) = queue_result.connect() {
                return Err(format!("connect uldata queue error: {}", e));
            }
            queue_result
        }
        Connection::Mqtt(conn, _) => {
            let opts = QueueOptions::Mqtt(
                MqttQueueOptions {
                    name: "broker.network.unit_code.code_network.uldata".to_string(),
                    is_recv: true,
                    reliable: true,
                    broadcast: false,
                    ..Default::default()
                },
                conn,
            );
            let mut queue_result = Queue::new(opts)?;
            queue_result.set_handler(Arc::new(handler.clone()));
            queue_result.set_msg_handler(Arc::new(handler.clone()));
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

        let now = Utc::now();
        let ts_nanos = match now.timestamp_nanos_opt() {
            None => i64::MAX,
            Some(ts) => ts,
        };
        let mut ext = Map::<String, Value>::new();
        ext.insert("key".to_string(), Value::String("value".to_string()));
        let data1 = UlData {
            time: now,
            network_addr: "addr1".to_string(),
            data: vec![1],
            extension: Some(ext),
        };
        if let Err(e) = mgr.send_uldata(&data1) {
            return Err(format!("send data1 error: {}", e));
        }
        let data2 = UlData {
            time: Utc.timestamp_nanos(ts_nanos + 1000000),
            network_addr: "addr2".to_string(),
            data: vec![2],
            extension: None,
        };
        if let Err(e) = mgr.send_uldata(&data2) {
            return Err(format!("send data2 error: {}", e));
        }

        let expect_count = 2;
        for _ in 0..WAIT_COUNT {
            let count = { handler.recv_uldata.lock().unwrap().len() };
            if count < expect_count {
                time::sleep(Duration::from_millis(WAIT_TICK)).await;
                continue;
            }
        }
        let count = { handler.recv_uldata.lock().unwrap().len() };
        if count < expect_count {
            return Err(format!("receive {}/{} data", count, expect_count));
        }

        for i in 0..expect_count {
            let data = match { handler.recv_uldata.lock().unwrap().pop() } {
                None => return Err(format!("only receive {}/{} data", i, expect_count)),
                Some(data) => data,
            };
            let network_addr = data.network_addr.as_str();
            if network_addr == "addr1" {
                expect(data.time).equals(strings::time_str(&data1.time))?;
                expect(data.data.as_str()).equals(hex::encode(&data1.data).as_str())?;
                expect(data.extension.as_ref()).equals(data1.extension.as_ref())?;
            } else if network_addr == "addr2" {
                expect(data.time).equals(strings::time_str(&data2.time))?;
                expect(data.data.as_str()).equals(hex::encode(&data2.data).as_str())?;
                expect(data.extension.as_ref()).equals(data2.extension.as_ref())?;
            } else {
                return Err(format!("receive wrong data {}", network_addr));
            }
        }

        Ok(())
    })?;

    Ok(())
}

/// Test sending uldata with wrong content.
pub fn uldata_wrong(context: &mut SpecContext<TestState>) -> Result<(), String> {
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
        id: "id_network".to_string(),
        name: "code_network".to_string(),
        shared_prefix: state.mqtt_shared_prefix.clone(),
        ..Default::default()
    };
    let mgr = NetworkMgr::new(conn_pool.clone(), &host_uri, opts, mgr_handler.clone())?;
    state.net_mgrs = Some(vec![mgr.clone()]);

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

        let data = UlData {
            time: Utc::now(),
            network_addr: "".to_string(),
            data: vec![0],
            extension: None,
        };
        expect(mgr.send_uldata(&data).is_err()).equals(true)?;

        Ok(())
    })?;

    Ok(())
}

/// Test receiving dldata.
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
        id: "id_network".to_string(),
        name: "code_network".to_string(),
        shared_prefix: state.mqtt_shared_prefix.clone(),
        ..Default::default()
    };
    let mgr = NetworkMgr::new(conn_pool.clone(), &host_uri, opts, mgr_handler.clone())?;
    state.net_mgrs = Some(vec![mgr.clone()]);

    let queue = match conn {
        Connection::Amqp(conn, _) => {
            let opts = QueueOptions::Amqp(
                AmqpQueueOptions {
                    name: "broker.network.unit_code.code_network.dldata".to_string(),
                    is_recv: false,
                    reliable: true,
                    broadcast: false,
                    ..Default::default()
                },
                conn,
            );
            let mut queue_result = Queue::new(opts)?;
            if let Err(e) = queue_result.connect() {
                return Err(format!("connect dldata queue error: {}", e));
            }
            queue_result
        }
        Connection::Mqtt(conn, _) => {
            let opts = QueueOptions::Mqtt(
                MqttQueueOptions {
                    name: "broker.network.unit_code.code_network.dldata".to_string(),
                    is_recv: false,
                    reliable: true,
                    broadcast: false,
                    ..Default::default()
                },
                conn,
            );
            let mut queue_result = Queue::new(opts)?;
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
        let ts_nanos = match now.timestamp_nanos_opt() {
            None => i64::MAX,
            Some(ts) => ts,
        };
        let data1 = NetDlData {
            data_id: "1".to_string(),
            publish: strings::time_str(&now),
            expires_in: 1000,
            network_addr: "addr1".to_string(),
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
        let data2 = NetDlData {
            data_id: "2".to_string(),
            publish: strings::time_str(&Utc.timestamp_nanos(ts_nanos + 1000000)),
            expires_in: 2000,
            network_addr: "addr2".to_string(),
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
        let data3 = NetDlData {
            data_id: "3".to_string(),
            publish: strings::time_str(&Utc.timestamp_nanos(ts_nanos + 2000000)),
            expires_in: 3000,
            network_addr: "addr3".to_string(),
            data: "03".to_string(),
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
            let count = { mgr_handler.recv_dldata.lock().unwrap().len() };
            if count < expect_count {
                time::sleep(Duration::from_millis(WAIT_TICK)).await;
                continue;
            }
        }
        let count = { mgr_handler.recv_dldata.lock().unwrap().len() };
        if count < expect_count {
            return Err(format!("receive {}/{} data", count, expect_count));
        }

        for i in 0..expect_count {
            let data = match { mgr_handler.recv_dldata.lock().unwrap().pop() } {
                None => return Err(format!("only receive {}/{} data", i, expect_count)),
                Some(data) => data,
            };
            let data_id = data.data_id.as_str();
            if data_id == "1" {
                expect(mq_engine).equals(MqEngine::RABBITMQ)?;
                expect(data.publish.timestamp_millis()).equals(now.timestamp_millis())?;
                expect(data.expires_in).equals(data1.expires_in)?;
                expect(data.network_addr.as_str()).equals(data1.network_addr.as_str())?;
                expect(hex::encode(data.data).as_str()).equals(data1.data.as_str())?;
                expect(data.extension.as_ref()).equals(data1.extension.as_ref())?;
            } else if data_id == "2" {
                expect(data.publish.timestamp_millis()).equals(now.timestamp_millis() + 1)?;
                expect(data.expires_in).equals(data2.expires_in)?;
                expect(data.network_addr.as_str()).equals(data2.network_addr.as_str())?;
                expect(hex::encode(data.data).as_str()).equals(data2.data.as_str())?;
                expect(data.extension.as_ref()).equals(data2.extension.as_ref())?;
            } else if data_id == "3" {
                expect(data.publish.timestamp_millis()).equals(now.timestamp_millis() + 2)?;
                expect(data.expires_in).equals(data3.expires_in)?;
                expect(data.network_addr.as_str()).equals(data3.network_addr.as_str())?;
                expect(hex::encode(data.data).as_str()).equals(data3.data.as_str())?;
                expect(data.extension.as_ref()).equals(data3.extension.as_ref())?;
            } else {
                return Err(format!("receive wrong data {}", data_id));
            }
        }

        Ok(())
    })?;

    Ok(())
}

/// Test receiving dldata with wrong content.
pub fn dldata_wrong(context: &mut SpecContext<TestState>) -> Result<(), String> {
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
        id: "id_network".to_string(),
        name: "code_network".to_string(),
        shared_prefix: state.mqtt_shared_prefix.clone(),
        ..Default::default()
    };
    let mgr = NetworkMgr::new(conn_pool.clone(), &host_uri, opts, mgr_handler.clone())?;
    state.net_mgrs = Some(vec![mgr.clone()]);

    let queue = match conn {
        Connection::Amqp(conn, _) => {
            let opts = QueueOptions::Amqp(
                AmqpQueueOptions {
                    name: "broker.network.unit_code.code_network.dldata".to_string(),
                    is_recv: false,
                    reliable: true,
                    broadcast: false,
                    ..Default::default()
                },
                conn,
            );
            let mut queue_result = Queue::new(opts)?;
            if let Err(e) = queue_result.connect() {
                return Err(format!("connect dldata queue error: {}", e));
            }
            queue_result
        }
        Connection::Mqtt(conn, _) => {
            let opts = QueueOptions::Mqtt(
                MqttQueueOptions {
                    name: "broker.network.unit_code.code_network.dldata".to_string(),
                    is_recv: false,
                    reliable: true,
                    broadcast: false,
                    ..Default::default()
                },
                conn,
            );
            let mut queue_result = Queue::new(opts)?;
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

        let data = NetDlData {
            data_id: "1".to_string(),
            publish: "2022-20-29T11:45:00.000Z".to_string(),
            expires_in: 1000,
            network_addr: "addr1".to_string(),
            data: "00".to_string(),
            extension: None,
        };
        let payload = match serde_json::to_vec(&data) {
            Err(e) => return Err(format!("marshal pub error: {}", e)),
            Ok(payload) => payload,
        };
        if let Err(e) = queue.send_msg(payload).await {
            return Err(format!("send pub error: {}", e));
        }

        time::sleep(Duration::from_secs(1)).await;
        let count = { mgr_handler.recv_dldata.lock().unwrap().len() };
        expect(count).equals(0)
    })?;

    Ok(())
}

/// Test generating dldata-result.
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
        id: "id_network".to_string(),
        name: "code_network".to_string(),
        shared_prefix: state.mqtt_shared_prefix.clone(),
        ..Default::default()
    };
    let mgr = NetworkMgr::new(conn_pool.clone(), &host_uri, opts, mgr_handler.clone())?;
    state.net_mgrs = Some(vec![mgr.clone()]);

    let handler = TestDlDataResultHandler::new();
    let queue = match conn {
        Connection::Amqp(conn, _) => {
            let opts = QueueOptions::Amqp(
                AmqpQueueOptions {
                    name: "broker.network.unit_code.code_network.dldata-result".to_string(),
                    is_recv: true,
                    reliable: true,
                    broadcast: false,
                    ..Default::default()
                },
                conn,
            );
            let mut queue_result = Queue::new(opts)?;
            queue_result.set_handler(Arc::new(handler.clone()));
            queue_result.set_msg_handler(Arc::new(handler.clone()));
            if let Err(e) = queue_result.connect() {
                return Err(format!("connect dldata-result queue error: {}", e));
            }
            queue_result
        }
        Connection::Mqtt(conn, _) => {
            let opts = QueueOptions::Mqtt(
                MqttQueueOptions {
                    name: "broker.network.unit_code.code_network.dldata-result".to_string(),
                    is_recv: true,
                    reliable: true,
                    broadcast: false,
                    ..Default::default()
                },
                conn,
            );
            let mut queue_result = Queue::new(opts)?;
            queue_result.set_handler(Arc::new(handler.clone()));
            queue_result.set_msg_handler(Arc::new(handler.clone()));
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
        let data1 = DlDataResult {
            data_id: "1".to_string(),
            status: -1,
            message: None,
        };
        if let Err(e) = mgr.send_dldata_result(&data1) {
            return Err(format!("send data1 error: {}", e));
        }
        let data2 = DlDataResult {
            data_id: "2".to_string(),
            status: 1,
            message: Some("error".to_string()),
        };
        if let Err(e) = mgr.send_dldata_result(&data2) {
            return Err(format!("send data2 error: {}", e));
        }

        let expect_count = 2;
        for _ in 0..WAIT_COUNT {
            let count = { handler.recv_dldata_result.lock().unwrap().len() };
            if count < expect_count {
                time::sleep(Duration::from_millis(WAIT_TICK)).await;
                continue;
            }
        }
        let count = { handler.recv_dldata_result.lock().unwrap().len() };
        if count < expect_count {
            return Err(format!("receive {}/{} data", count, expect_count));
        }

        for i in 0..expect_count {
            let data = match { handler.recv_dldata_result.lock().unwrap().pop() } {
                None => return Err(format!("only receive {}/{} data", i, expect_count)),
                Some(data) => data,
            };
            let data_id = data.data_id.as_str();
            if data_id == "1" {
                expect(data.status).equals(data1.status)?;
                expect(data.message.as_ref()).equals(data1.message.as_ref())?;
            } else if data_id == "2" {
                expect(data.status).equals(data2.status)?;
                expect(data.message.as_ref()).equals(data2.message.as_ref())?;
            } else {
                return Err(format!("receive wrong data {}", data_id));
            }
        }

        Ok(())
    })?;

    Ok(())
}

/// Test sending dldata-result with wrong content.
pub fn dldata_result_wrong(context: &mut SpecContext<TestState>) -> Result<(), String> {
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
        id: "id_network".to_string(),
        name: "code_network".to_string(),
        shared_prefix: state.mqtt_shared_prefix.clone(),
        ..Default::default()
    };
    let mgr = NetworkMgr::new(conn_pool.clone(), &host_uri, opts, mgr_handler.clone())?;
    state.net_mgrs = Some(vec![mgr.clone()]);

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

        let data = DlDataResult {
            data_id: "".to_string(),
            status: 0,
            message: None,
        };
        expect(mgr.send_dldata_result(&data).is_err()).equals(true)?;

        Ok(())
    })?;

    Ok(())
}

/// Test receiving ctrl.
pub fn ctrl(context: &mut SpecContext<TestState>) -> Result<(), String> {
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
        id: "id_network".to_string(),
        name: "code_network".to_string(),
        shared_prefix: state.mqtt_shared_prefix.clone(),
        ..Default::default()
    };
    let mgr = NetworkMgr::new(conn_pool.clone(), &host_uri, opts, mgr_handler.clone())?;
    state.net_mgrs = Some(vec![mgr.clone()]);

    let queue = match conn {
        Connection::Amqp(conn, _) => {
            let opts = QueueOptions::Amqp(
                AmqpQueueOptions {
                    name: "broker.network.unit_code.code_network.ctrl".to_string(),
                    is_recv: false,
                    reliable: true,
                    broadcast: false,
                    ..Default::default()
                },
                conn,
            );
            let mut queue_result = Queue::new(opts)?;
            if let Err(e) = queue_result.connect() {
                return Err(format!("connect ctrl queue error: {}", e));
            }
            queue_result
        }
        Connection::Mqtt(conn, _) => {
            let opts = QueueOptions::Mqtt(
                MqttQueueOptions {
                    name: "broker.network.unit_code.code_network.ctrl".to_string(),
                    is_recv: false,
                    reliable: true,
                    broadcast: false,
                    ..Default::default()
                },
                conn,
            );
            let mut queue_result = Queue::new(opts)?;
            if let Err(e) = queue_result.connect() {
                return Err(format!("connect ctrl queue error: {}", e));
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

        let now_str = strings::time_str(&Utc::now());
        let data1 = SendNetCtrlMsg::AddDevice {
            time: now_str.clone(),
            operation: NetCtrlMsgOp::ADD_DEVICE.to_string(),
            new: NetCtrlAddr {
                network_addr: "addr1".to_string(),
            },
        };
        let payload = match serde_json::to_vec(&data1) {
            Err(e) => return Err(format!("marshal data1 error: {}", e)),
            Ok(payload) => payload,
        };
        if let Err(e) = queue.send_msg(payload).await {
            return Err(format!("send data1 error: {}", e));
        }
        let data2 = SendNetCtrlMsg::AddDeviceBulk {
            time: now_str.clone(),
            operation: NetCtrlMsgOp::ADD_DEVICE_BULK.to_string(),
            new: NetCtrlAddrs {
                network_addrs: vec!["addr2".to_string()],
            },
        };
        let payload = match serde_json::to_vec(&data2) {
            Err(e) => return Err(format!("marshal data2 error: {}", e)),
            Ok(payload) => payload,
        };
        if let Err(e) = queue.send_msg(payload).await {
            return Err(format!("send data2 error: {}", e));
        }
        let data3 = SendNetCtrlMsg::AddDeviceRange {
            time: now_str.clone(),
            operation: NetCtrlMsgOp::ADD_DEVICE_RANGE.to_string(),
            new: NetCtrlAddrRange {
                start_addr: "0001".to_string(),
                end_addr: "0002".to_string(),
            },
        };
        let payload = match serde_json::to_vec(&data3) {
            Err(e) => return Err(format!("marshal data3 error: {}", e)),
            Ok(payload) => payload,
        };
        if let Err(e) = queue.send_msg(payload).await {
            return Err(format!("send data3 error: {}", e));
        }
        let data4 = SendNetCtrlMsg::DelDevice {
            time: now_str.clone(),
            operation: NetCtrlMsgOp::DEL_DEVICE.to_string(),
            new: NetCtrlAddr {
                network_addr: "addr4".to_string(),
            },
        };
        let payload = match serde_json::to_vec(&data4) {
            Err(e) => return Err(format!("marshal data4 error: {}", e)),
            Ok(payload) => payload,
        };
        if let Err(e) = queue.send_msg(payload).await {
            return Err(format!("send data4 error: {}", e));
        }
        let data5 = SendNetCtrlMsg::DelDeviceBulk {
            time: now_str.clone(),
            operation: NetCtrlMsgOp::DEL_DEVICE_BULK.to_string(),
            new: NetCtrlAddrs {
                network_addrs: vec!["addr5".to_string()],
            },
        };
        let payload = match serde_json::to_vec(&data5) {
            Err(e) => return Err(format!("marshal data5 error: {}", e)),
            Ok(payload) => payload,
        };
        if let Err(e) = queue.send_msg(payload).await {
            return Err(format!("send data5 error: {}", e));
        }
        let data6 = SendNetCtrlMsg::DelDeviceRange {
            time: now_str.clone(),
            operation: NetCtrlMsgOp::DEL_DEVICE_RANGE.to_string(),
            new: NetCtrlAddrRange {
                start_addr: "0003".to_string(),
                end_addr: "0004".to_string(),
            },
        };
        let payload = match serde_json::to_vec(&data6) {
            Err(e) => return Err(format!("marshal data6 error: {}", e)),
            Ok(payload) => payload,
        };
        if let Err(e) = queue.send_msg(payload).await {
            return Err(format!("send data6 error: {}", e));
        }

        let expect_count = match mq_engine {
            MqEngine::RABBITMQ => 6,
            _ => 5,
        };
        for _ in 0..WAIT_COUNT {
            let count = { mgr_handler.recv_ctrl.lock().unwrap().len() };
            if count < expect_count {
                time::sleep(Duration::from_millis(WAIT_TICK)).await;
                continue;
            }
        }
        let count = { mgr_handler.recv_ctrl.lock().unwrap().len() };
        if count < expect_count {
            return Err(format!("receive {}/{} data", count, expect_count));
        }

        let mut recv_dev_add = false;
        let mut recv_dev_add_bulk = false;
        let mut recv_dev_add_range = false;
        let mut recv_dev_del = false;
        let mut recv_dev_del_bulk = false;
        let mut recv_dev_del_range = false;
        for i in 0..expect_count {
            let data = match { mgr_handler.recv_ctrl.lock().unwrap().pop() } {
                None => return Err(format!("only receive {}/{} data", i, expect_count)),
                Some(data) => data,
            };
            match data.as_ref() {
                NetworkCtrlMsg::AddDevice { time, new } => {
                    expect(strings::time_str(&time).as_str()).to_equal(now_str.as_str())?;
                    expect(new.network_addr.as_str()).to_equal("addr1")?;
                    recv_dev_add = true;
                }
                NetworkCtrlMsg::AddDeviceBulk { time, new } => {
                    expect(strings::time_str(&time).as_str()).to_equal(now_str.as_str())?;
                    expect(new.network_addrs.len()).to_equal(1)?;
                    expect(new.network_addrs[0].as_str()).to_equal("addr2")?;
                    recv_dev_add_bulk = true;
                }
                NetworkCtrlMsg::AddDeviceRange { time, new } => {
                    expect(strings::time_str(&time).as_str()).to_equal(now_str.as_str())?;
                    expect(new.start_addr.as_str()).to_equal("0001")?;
                    expect(new.end_addr.as_str()).to_equal("0002")?;
                    recv_dev_add_range = true;
                }
                NetworkCtrlMsg::DelDevice { time, new } => {
                    expect(strings::time_str(&time).as_str()).to_equal(now_str.as_str())?;
                    expect(new.network_addr.as_str()).to_equal("addr4")?;
                    recv_dev_del = true;
                }
                NetworkCtrlMsg::DelDeviceBulk { time, new } => {
                    expect(strings::time_str(&time).as_str()).to_equal(now_str.as_str())?;
                    expect(new.network_addrs.len()).to_equal(1)?;
                    expect(new.network_addrs[0].as_str()).to_equal("addr5")?;
                    recv_dev_del_bulk = true;
                }
                NetworkCtrlMsg::DelDeviceRange { time, new } => {
                    expect(strings::time_str(&time).as_str()).to_equal(now_str.as_str())?;
                    expect(new.start_addr.as_str()).to_equal("0003")?;
                    expect(new.end_addr.as_str()).to_equal("0004")?;
                    recv_dev_del_range = true;
                }
            }
        }
        expect(
            recv_dev_add_bulk
                && recv_dev_add_range
                && recv_dev_del
                && recv_dev_del_bulk
                && recv_dev_del_range,
        )
        .to_equal(true)?;
        match mq_engine {
            MqEngine::RABBITMQ => expect(recv_dev_add).to_equal(true)?,
            _ => expect(recv_dev_add).to_equal(false)?,
        }

        Ok(())
    })?;

    Ok(())
}

/// Test receiving ctrl with wrong content.
pub fn ctrl_wrong(context: &mut SpecContext<TestState>) -> Result<(), String> {
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
        id: "id_network".to_string(),
        name: "code_network".to_string(),
        shared_prefix: state.mqtt_shared_prefix.clone(),
        ..Default::default()
    };
    let mgr = NetworkMgr::new(conn_pool.clone(), &host_uri, opts, mgr_handler.clone())?;
    state.net_mgrs = Some(vec![mgr.clone()]);

    let queue = match conn {
        Connection::Amqp(conn, _) => {
            let opts = QueueOptions::Amqp(
                AmqpQueueOptions {
                    name: "broker.network.unit_code.code_network.ctrl".to_string(),
                    is_recv: false,
                    reliable: true,
                    broadcast: false,
                    ..Default::default()
                },
                conn,
            );
            let mut queue_result = Queue::new(opts)?;
            if let Err(e) = queue_result.connect() {
                return Err(format!("connect ctrl queue error: {}", e));
            }
            queue_result
        }
        Connection::Mqtt(conn, _) => {
            let opts = QueueOptions::Mqtt(
                MqttQueueOptions {
                    name: "broker.network.unit_code.code_network.ctrl".to_string(),
                    is_recv: false,
                    reliable: true,
                    broadcast: false,
                    ..Default::default()
                },
                conn,
            );
            let mut queue_result = Queue::new(opts)?;
            if let Err(e) = queue_result.connect() {
                return Err(format!("connect ctrl queue error: {}", e));
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

        let data = SendNetCtrlMsg::AddDevice {
            time: strings::time_str(&Utc::now()),
            operation: NetCtrlMsgOp::ADD_DEVICE_BULK.to_string(),
            new: NetCtrlAddr {
                network_addr: "addr5".to_string(),
            },
        };
        let payload = match serde_json::to_vec(&data) {
            Err(e) => return Err(format!("marshal data error: {}", e)),
            Ok(payload) => payload,
        };
        if let Err(e) = queue.send_msg(payload).await {
            return Err(format!("send data error: {}", e));
        }

        time::sleep(Duration::from_secs(1)).await;
        let count = { mgr_handler.recv_ctrl.lock().unwrap().len() };
        expect(count).equals(0)
    })?;

    Ok(())
}
