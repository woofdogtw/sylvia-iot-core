use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::Duration,
};

use async_trait::async_trait;
use general_mq::{
    queue::{
        Event as MqEvent, EventHandler as MqEventHandler, GmqQueue, Message, Status as MqStatus,
    },
    AmqpQueueOptions, MqttQueueOptions, Queue, QueueOptions,
};
use laboratory::{expect, SpecContext};
use serde::{self, Deserialize, Serialize};
use serde_json::{self, Map, Value};
use tokio::time;

use sylvia_iot_broker::libs::mq::{
    network::{DlData, DlDataResult, EventHandler, NetworkMgr, UlData},
    Connection, MgrStatus, Options,
};

use super::{new_connection, STATE};
use crate::{libs::libs::conn_host_uri, TestState, WAIT_COUNT, WAIT_TICK};

/// Downlink data from broker to network.
#[derive(Debug, Deserialize)]
pub struct NetDlData {
    #[serde(rename = "dataId")]
    pub data_id: String,
    #[serde(rename = "pub")]
    pub publish: String,
    #[serde(rename = "expiresIn")]
    pub expires_in: i64,
    #[serde(rename = "networkAddr")]
    pub network_addr: String,
    pub data: String,
    pub extension: Option<Map<String, Value>>,
}

/// Uplink data from network to broker.
#[derive(Debug, Default, Serialize)]
pub struct NetUlData {
    pub time: String,
    #[serde(rename = "networkAddr")]
    pub network_addr: String,
    pub data: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extension: Option<Map<String, Value>>,
}

/// Downlink data result when processing or completing data transfer to the device.
#[derive(Debug, Default, Serialize)]
pub struct NetDlDataResult {
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
    recv_dldata_result: Arc<Mutex<Vec<Box<DlDataResult>>>>,
}

struct TestDlDataHandler {
    // Use Mutex to implement interior mutability.
    status_connected: Arc<Mutex<bool>>,
    recv_dldata: Arc<Mutex<Vec<Box<NetDlData>>>>,
}

impl TestHandler {
    fn new() -> Self {
        TestHandler {
            status_changed: Arc::new(Mutex::new(false)),
            recv_uldata: Arc::new(Mutex::new(vec![])),
            recv_dldata_result: Arc::new(Mutex::new(vec![])),
        }
    }
}

#[async_trait]
impl EventHandler for TestHandler {
    async fn on_status_change(&self, _mgr: &NetworkMgr, _status: MgrStatus) {
        *self.status_changed.lock().unwrap() = true;
    }

    async fn on_uldata(&self, _mgr: &NetworkMgr, data: Box<UlData>) -> Result<(), ()> {
        let count;
        let mut error = false;
        {
            let mut mutex = self.recv_uldata.lock().unwrap();
            count = mutex.len();
            // Simulate error processing.
            if data.data.as_str() == "ee01" {
                error = true;
            }
            mutex.push(data);
        }
        if error && count == 0 {
            return Err(());
        }
        Ok(())
    }

    async fn on_dldata_result(&self, _mgr: &NetworkMgr, data: Box<DlDataResult>) -> Result<(), ()> {
        let count;
        let mut error = false;
        {
            let mut mutex = self.recv_dldata_result.lock().unwrap();
            count = mutex.len();
            // Simulate error processing.
            if data.data_id.as_str() == "error" {
                error = true;
            }
            mutex.push(data);
        }
        if error && count == 0 {
            return Err(());
        }
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
        let data = match serde_json::from_slice::<NetDlData>(msg.payload()) {
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
    let status = mgr.status();
    let mq_status = mgr.mq_status();
    state.net_mgrs = Some(vec![mgr.clone()]);

    expect(mgr.unit_id()).equals("unit_id")?;
    expect(mgr.unit_code()).equals("unit_code")?;
    expect(mgr.id()).equals("id_network")?;
    expect(mgr.name()).equals("code_network")?;
    expect(status == MgrStatus::NotReady).equals(true)?;
    expect(mq_status.uldata == MqStatus::Connecting).equals(true)?;
    expect(mq_status.dldata == MqStatus::Connecting).equals(true)?;
    expect(mq_status.dldata_resp == MqStatus::Closed).equals(true)?;
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
    expect(mq_status.dldata_resp == MqStatus::Closed).equals(true)?;
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

/// Test receiving uldata.
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
        id: "id_network".to_string(),
        name: "code_network".to_string(),
        prefetch: state.amqp_prefetch,
        shared_prefix: state.mqtt_shared_prefix.clone(),
        ..Default::default()
    };
    let mgr = NetworkMgr::new(conn_pool.clone(), &host_uri, opts, handler.clone())?;
    state.net_mgrs = Some(vec![mgr.clone()]);

    let recv_uldata_count;
    let queue_send = match conn {
        Connection::Amqp(conn, _) => {
            recv_uldata_count = 3;
            let opts = QueueOptions::Amqp(
                AmqpQueueOptions {
                    name: "broker.network.unit_code.code_network.uldata".to_string(),
                    is_recv: false,
                    reliable: true,
                    broadcast: false,
                    ..Default::default()
                },
                &conn,
            );
            let mut queue_send = Queue::new(opts)?;
            if let Err(e) = queue_send.connect() {
                return Err(format!("connect uldata queue error: {}", e));
            }
            queue_send
        }
        Connection::Mqtt(conn, _) => {
            recv_uldata_count = 2;
            let opts = QueueOptions::Mqtt(
                MqttQueueOptions {
                    name: "broker.network.unit_code.code_network.uldata".to_string(),
                    is_recv: false,
                    reliable: true,
                    broadcast: false,
                    ..Default::default()
                },
                &conn,
            );
            let mut queue_send = Queue::new(opts)?;
            if let Err(e) = queue_send.connect() {
                return Err(format!("connect uldata queue error: {}", e));
            }
            queue_send
        }
    };

    runtime.block_on(async move {
        for _ in 0..WAIT_COUNT {
            if queue_send.status() == MqStatus::Connected {
                break;
            }
            time::sleep(Duration::from_millis(WAIT_TICK)).await;
        }
        if queue_send.status() != MqStatus::Connected {
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
        let send_data1 = NetUlData {
            time: "2022-01-02T03:04:05.678Z".to_string(),
            network_addr: "addr1".to_string(),
            data: "ee01".to_string(),
            extension: Some(ext),
        };
        let payload = match serde_json::to_vec(&send_data1) {
            Err(e) => return Err(format!("generate payload 1 error: {}", e)),
            Ok(data) => data,
        };
        if let Err(e) = queue_send.send_msg(payload).await {
            return Err(format!("send UlData 1 error: {}", e));
        }
        let send_data2 = NetUlData {
            time: "2023-01-02T03:04:05.678Z".to_string(),
            network_addr: "addr2".to_string(),
            data: "da02".to_string(),
            ..Default::default()
        };
        let payload = match serde_json::to_vec(&send_data2) {
            Err(e) => return Err(format!("generate payload 2 error: {}", e)),
            Ok(data) => data,
        };
        if let Err(e) = queue_send.send_msg(payload).await {
            return Err(format!("send UlData 2 error: {}", e));
        }
        for _ in 0..WAIT_COUNT {
            if handler.recv_uldata.lock().unwrap().len() < recv_uldata_count {
                time::sleep(Duration::from_millis(WAIT_TICK)).await;
                continue;
            }
        }
        if handler.recv_uldata.lock().unwrap().len() < recv_uldata_count {
            return Err(format!("receive {} uldata", {
                handler.recv_uldata.lock().unwrap().len()
            }));
        }

        let mut recv_1_count = recv_uldata_count - 1;
        let mut recv_2_count = 1;
        for _ in 0..recv_uldata_count {
            let recv_data = match { handler.recv_uldata.lock().unwrap().pop() } {
                None => return Err("receive no data".to_string()),
                Some(data) => data,
            };
            let data = recv_data.data.as_str();
            if data == "ee01" {
                if recv_1_count == 0 {
                    return Err("receive data-1 more than expect".to_string());
                }
                recv_1_count -= 1;
                expect(recv_data.time.as_str()).equals(send_data1.time.as_str())?;
                expect(recv_data.network_addr.as_str()).equals(send_data1.network_addr.as_str())?;
                expect(recv_data.extension.as_ref()).equals(send_data1.extension.as_ref())?;
            } else if data == "da02" {
                if recv_2_count == 0 {
                    return Err("receive data-2 more than expect".to_string());
                }
                recv_2_count -= 1;
                expect(recv_data.time.as_str()).equals(send_data2.time.as_str())?;
                expect(recv_data.network_addr.as_str()).equals(send_data2.network_addr.as_str())?;
                expect(recv_data.extension.as_ref()).equals(send_data2.extension.as_ref())?;
            } else {
                return Err(format!("receive wrong data correlation {}", data));
            }
        }

        if let Err(e) = mgr.close().await {
            return Err(format!("close manager error: {}", e));
        }
        Ok(())
    })?;
    Ok(())
}

/// Test uldata with wrong parameter.
pub fn uldata_wrong(context: &mut SpecContext<TestState>) -> Result<(), String> {
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
        id: "id_network".to_string(),
        name: "code_network".to_string(),
        prefetch: state.amqp_prefetch,
        shared_prefix: state.mqtt_shared_prefix.clone(),
        ..Default::default()
    };
    let mgr = NetworkMgr::new(conn_pool.clone(), &host_uri, opts, handler.clone())?;
    state.net_mgrs = Some(vec![mgr.clone()]);

    let queue_send = match conn {
        Connection::Amqp(conn, _) => {
            let opts = QueueOptions::Amqp(
                AmqpQueueOptions {
                    name: "broker.network.unit_code.code_network.uldata".to_string(),
                    is_recv: false,
                    reliable: true,
                    broadcast: false,
                    ..Default::default()
                },
                &conn,
            );
            let mut queue_send = Queue::new(opts)?;
            if let Err(e) = queue_send.connect() {
                return Err(format!("connect uldata queue error: {}", e));
            }
            queue_send
        }
        Connection::Mqtt(conn, _status) => {
            let opts = QueueOptions::Mqtt(
                MqttQueueOptions {
                    name: "broker.network.unit_code.code_network.uldata".to_string(),
                    is_recv: false,
                    reliable: true,
                    broadcast: false,
                    ..Default::default()
                },
                &conn,
            );
            let mut queue_send = Queue::new(opts)?;
            if let Err(e) = queue_send.connect() {
                return Err(format!("connect uldata queue error: {}", e));
            }
            queue_send
        }
    };

    runtime.block_on(async move {
        for _ in 0..WAIT_COUNT {
            if queue_send.status() == MqStatus::Connected {
                break;
            }
            time::sleep(Duration::from_millis(WAIT_TICK)).await;
        }
        if queue_send.status() != MqStatus::Connected {
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
            return Err(format!("send UlData 0 error: {}", e));
        }
        let send_data1 = NetUlData {
            time: "asdlj".to_string(),
            ..Default::default()
        };
        let payload = match serde_json::to_vec(&send_data1) {
            Err(e) => return Err(format!("generate payload 1 error: {}", e)),
            Ok(data) => data,
        };
        if let Err(e) = queue_send.send_msg(payload).await {
            return Err(format!("send UlData 1 error: {}", e));
        }
        let send_data2 = NetUlData {
            time: "2022-01-02T03:04:05Z".to_string(),
            network_addr: "".to_string(),
            ..Default::default()
        };
        let payload = match serde_json::to_vec(&send_data2) {
            Err(e) => return Err(format!("generate payload 2 error: {}", e)),
            Ok(data) => data,
        };
        if let Err(e) = queue_send.send_msg(payload).await {
            return Err(format!("send UlData 2 error: {}", e));
        }
        let send_data3 = NetUlData {
            time: "2022-01-02T03:04:05Z".to_string(),
            network_addr: "addr".to_string(),
            data: "zz".to_string(),
            ..Default::default()
        };
        let payload = match serde_json::to_vec(&send_data3) {
            Err(e) => return Err(format!("generate payload 3 error: {}", e)),
            Ok(data) => data,
        };
        if let Err(e) = queue_send.send_msg(payload).await {
            return Err(format!("send UlData 3 error: {}", e));
        }
        time::sleep(Duration::from_secs(1)).await;
        expect(handler.recv_uldata.lock().unwrap().len()).equals(0)?;

        if let Err(e) = mgr.close().await {
            return Err(format!("close manager error: {}", e));
        }
        Ok(())
    })?;
    Ok(())
}

/// Test generating dldata.
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
        id: "id_network".to_string(),
        name: "code_network".to_string(),
        prefetch: state.amqp_prefetch,
        shared_prefix: state.mqtt_shared_prefix.clone(),
        ..Default::default()
    };
    let mgr = NetworkMgr::new(conn_pool.clone(), &host_uri, opts, handler.clone())?;
    state.net_mgrs = Some(vec![mgr.clone()]);

    let queue_handler = Arc::new(TestDlDataHandler::new());
    let _queue_result = match conn {
        Connection::Amqp(conn, _) => {
            let opts = QueueOptions::Amqp(
                AmqpQueueOptions {
                    name: "broker.network.unit_code.code_network.dldata".to_string(),
                    is_recv: true,
                    reliable: true,
                    broadcast: false,
                    ..Default::default()
                },
                &conn,
            );
            let mut queue_result = Queue::new(opts)?;
            queue_result.set_handler(queue_handler.clone());
            if let Err(e) = queue_result.connect() {
                return Err(format!("connect dldata queue error: {}", e));
            }
            queue_result
        }
        Connection::Mqtt(conn, _) => {
            let opts = QueueOptions::Mqtt(
                MqttQueueOptions {
                    name: "broker.network.unit_code.code_network.dldata".to_string(),
                    is_recv: true,
                    reliable: true,
                    broadcast: false,
                    ..Default::default()
                },
                &conn,
            );
            let mut queue_result = Queue::new(opts)?;
            queue_result.set_handler(queue_handler.clone());
            if let Err(e) = queue_result.connect() {
                return Err(format!("connect dldata queue error: {}", e));
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

        let data1 = DlData {
            data_id: "1".to_string(),
            publish: "pub1".to_string(),
            expires_in: 3600,
            network_addr: "addr1".to_string(),
            data: "data1".to_string(),
            extension: None,
        };
        if let Err(e) = mgr.send_dldata(&data1) {
            return Err(format!("send data1 error: {}", e));
        }
        let mut ext = Map::<String, Value>::new();
        ext.insert("key".to_string(), Value::String("value".to_string()));
        let data2 = DlData {
            data_id: "2".to_string(),
            publish: "pub2".to_string(),
            expires_in: 7200,
            network_addr: "addr2".to_string(),
            data: "data2".to_string(),
            extension: Some(ext),
        };
        if let Err(e) = mgr.send_dldata(&data2) {
            return Err(format!("send data2 error: {}", e));
        }

        for _ in 0..WAIT_COUNT {
            if queue_handler.recv_dldata.lock().unwrap().len() < 2 {
                time::sleep(Duration::from_millis(WAIT_TICK)).await;
                continue;
            }
        }
        if queue_handler.recv_dldata.lock().unwrap().len() < 2 {
            return Err(format!(
                "receive {}/2 data",
                queue_handler.recv_dldata.lock().unwrap().len()
            ));
        }

        for i in 0..2 {
            let data = match { queue_handler.recv_dldata.lock().unwrap().pop() } {
                None => return Err(format!("only receive {}/2 data", i)),
                Some(data) => data,
            };
            let data_id = data.data_id.as_str();
            if data_id == "1" {
                expect(data.publish.as_str()).equals(data1.publish.as_str())?;
                expect(data.expires_in).equals(data1.expires_in)?;
                expect(data.network_addr.as_str()).equals(data1.network_addr.as_str())?;
                expect(data.data.as_str()).equals(data1.data.as_str())?;
                expect(data.extension.as_ref()).equals(data1.extension.as_ref())?;
            } else if data_id == "2" {
                expect(data.publish.as_str()).equals(data2.publish.as_str())?;
                expect(data.expires_in).equals(data2.expires_in)?;
                expect(data.network_addr.as_str()).equals(data2.network_addr.as_str())?;
                expect(data.data.as_str()).equals(data2.data.as_str())?;
                expect(data.extension.as_ref()).equals(data2.extension.as_ref())?;
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

/// Test receiving dldata-result.
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
        id: "id_network".to_string(),
        name: "code_network".to_string(),
        prefetch: state.amqp_prefetch,
        shared_prefix: state.mqtt_shared_prefix.clone(),
        ..Default::default()
    };
    let mgr = NetworkMgr::new(conn_pool.clone(), &host_uri, opts, handler.clone())?;
    state.net_mgrs = Some(vec![mgr.clone()]);

    let recv_dldata_result_count;
    let queue_send = match conn {
        Connection::Amqp(conn, _) => {
            recv_dldata_result_count = 3;
            let opts = QueueOptions::Amqp(
                AmqpQueueOptions {
                    name: "broker.network.unit_code.code_network.dldata-result".to_string(),
                    is_recv: false,
                    reliable: true,
                    broadcast: false,
                    ..Default::default()
                },
                &conn,
            );
            let mut queue_send = Queue::new(opts)?;
            if let Err(e) = queue_send.connect() {
                return Err(format!("connect dldata-result queue error: {}", e));
            }
            queue_send
        }
        Connection::Mqtt(conn, _) => {
            recv_dldata_result_count = 2;
            let opts = QueueOptions::Mqtt(
                MqttQueueOptions {
                    name: "broker.network.unit_code.code_network.dldata-result".to_string(),
                    is_recv: false,
                    reliable: true,
                    broadcast: false,
                    ..Default::default()
                },
                &conn,
            );
            let mut queue_send = Queue::new(opts)?;
            if let Err(e) = queue_send.connect() {
                return Err(format!("connect dldata-result queue error: {}", e));
            }
            queue_send
        }
    };

    runtime.block_on(async move {
        for _ in 0..WAIT_COUNT {
            if queue_send.status() == MqStatus::Connected {
                break;
            }
            time::sleep(Duration::from_millis(WAIT_TICK)).await;
        }
        if queue_send.status() != MqStatus::Connected {
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
        let send_data1 = NetDlDataResult {
            data_id: "error".to_string(),
            ..Default::default()
        };
        let payload = match serde_json::to_vec(&send_data1) {
            Err(e) => return Err(format!("generate payload 1 error: {}", e)),
            Ok(data) => data,
        };
        if let Err(e) = queue_send.send_msg(payload).await {
            return Err(format!("send DlDataResult 1 error: {}", e));
        }
        let send_data2 = NetDlDataResult {
            data_id: "2".to_string(),
            status: 1,
            message: Some("message2".to_string()),
        };
        let payload = match serde_json::to_vec(&send_data2) {
            Err(e) => return Err(format!("generate payload 2 error: {}", e)),
            Ok(data) => data,
        };
        if let Err(e) = queue_send.send_msg(payload).await {
            return Err(format!("send DlDataResult 2 error: {}", e));
        }
        for _ in 0..WAIT_COUNT {
            if handler.recv_dldata_result.lock().unwrap().len() < recv_dldata_result_count {
                time::sleep(Duration::from_millis(WAIT_TICK)).await;
                continue;
            }
        }
        if handler.recv_dldata_result.lock().unwrap().len() < recv_dldata_result_count {
            return Err(format!("receive {} dldata-result", {
                handler.recv_dldata_result.lock().unwrap().len()
            }));
        }

        let mut recv_1_count = recv_dldata_result_count - 1;
        let mut recv_2_count = 1;
        for _ in 0..recv_dldata_result_count {
            let recv_data = match { handler.recv_dldata_result.lock().unwrap().pop() } {
                None => return Err("receive no data".to_string()),
                Some(data) => data,
            };
            let data_id = recv_data.data_id.as_str();
            if data_id == "error" {
                if recv_1_count == 0 {
                    return Err("receive data-1 more than expect".to_string());
                }
                recv_1_count -= 1;
                expect(recv_data.status).equals(send_data1.status)?;
                expect(recv_data.message.as_ref()).equals(send_data1.message.as_ref())?;
            } else if data_id == "2" {
                if recv_2_count == 0 {
                    return Err("receive data-2 more than expect".to_string());
                }
                recv_2_count -= 1;
                expect(recv_data.status).equals(send_data2.status)?;
                expect(recv_data.message.as_ref()).equals(send_data2.message.as_ref())?;
            } else {
                return Err(format!("receive wrong data correlation {}", data_id));
            }
        }

        if let Err(e) = mgr.close().await {
            return Err(format!("close manager error: {}", e));
        }
        Ok(())
    })?;
    Ok(())
}

/// Test dldata-result with wrong parameter.
pub fn dldata_result_wrong(context: &mut SpecContext<TestState>) -> Result<(), String> {
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
        id: "id_network".to_string(),
        name: "code_network".to_string(),
        prefetch: state.amqp_prefetch,
        shared_prefix: state.mqtt_shared_prefix.clone(),
        ..Default::default()
    };
    let mgr = NetworkMgr::new(conn_pool.clone(), &host_uri, opts, handler.clone())?;
    state.net_mgrs = Some(vec![mgr.clone()]);

    let queue_send = match conn {
        Connection::Amqp(conn, _) => {
            let opts = QueueOptions::Amqp(
                AmqpQueueOptions {
                    name: "broker.network.unit_code.code_network.dldata-result".to_string(),
                    is_recv: false,
                    reliable: true,
                    broadcast: false,
                    ..Default::default()
                },
                &conn,
            );
            let mut queue_send = Queue::new(opts)?;
            if let Err(e) = queue_send.connect() {
                return Err(format!("connect dldata-result queue error: {}", e));
            }
            queue_send
        }
        Connection::Mqtt(conn, _) => {
            let opts = QueueOptions::Mqtt(
                MqttQueueOptions {
                    name: "broker.network.unit_code.code_network.dldata-result".to_string(),
                    is_recv: false,
                    reliable: true,
                    broadcast: false,
                    ..Default::default()
                },
                &conn,
            );
            let mut queue_send = Queue::new(opts)?;
            if let Err(e) = queue_send.connect() {
                return Err(format!("connect dldata-result queue error: {}", e));
            }
            queue_send
        }
    };

    runtime.block_on(async move {
        for _ in 0..WAIT_COUNT {
            if queue_send.status() == MqStatus::Connected {
                break;
            }
            time::sleep(Duration::from_millis(WAIT_TICK)).await;
        }
        if queue_send.status() != MqStatus::Connected {
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
            return Err(format!("send DlDataResult 0 error: {}", e));
        }
        let send_data1 = NetDlDataResult {
            data_id: "".to_string(),
            ..Default::default()
        };
        let payload = match serde_json::to_vec(&send_data1) {
            Err(e) => return Err(format!("generate payload 1 error: {}", e)),
            Ok(data) => data,
        };
        if let Err(e) = queue_send.send_msg(payload).await {
            return Err(format!("send DlDataResult 1 error: {}", e));
        }
        let send_data2 = NetDlDataResult {
            data_id: "1".to_string(),
            message: Some("".to_string()),
            ..Default::default()
        };
        let payload = match serde_json::to_vec(&send_data2) {
            Err(e) => return Err(format!("generate payload 2 error: {}", e)),
            Ok(data) => data,
        };
        if let Err(e) = queue_send.send_msg(payload).await {
            return Err(format!("send DlDataResult 2 error: {}", e));
        }
        time::sleep(Duration::from_secs(1)).await;
        expect(handler.recv_dldata_result.lock().unwrap().len()).equals(0)?;

        if let Err(e) = mgr.close().await {
            return Err(format!("close manager error: {}", e));
        }
        Ok(())
    })?;
    Ok(())
}
