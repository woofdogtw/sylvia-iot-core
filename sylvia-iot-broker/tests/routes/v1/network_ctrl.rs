use std::{
    collections::HashMap,
    error::Error as StdError,
    sync::{Arc, Mutex},
    time::Duration,
};

use async_trait::async_trait;
use chrono::{DateTime, SubsecRound, Utc};
use laboratory::{expect, SpecContext};
use serde::Deserialize;

use general_mq::{
    connection::GmqConnection,
    queue::{EventHandler, GmqQueue, Message, MessageHandler, Status},
    AmqpConnection, AmqpConnectionOptions, AmqpQueueOptions, MqttConnection, MqttConnectionOptions,
    MqttQueueOptions, Queue, QueueOptions,
};
use sylvia_iot_broker::{
    libs::mq::MgrStatus,
    models::{device::QueryCond, device_route::QueryCond as RouteQueryCond, Model},
};
use tokio::time;

use super::{device, libs, network, unit, STATE, TOKEN_MANAGER};
use crate::{TestState, TEST_AMQP_HOST_URI, TEST_MQTT_HOST_URI, WAIT_COUNT, WAIT_TICK};

#[derive(Clone, Deserialize)]
#[serde(tag = "operation")]
enum NetworkCtrlMsg {
    #[serde(rename = "add-device")]
    AddDevice { time: String, new: CtrlAddDevice },
    #[serde(rename = "add-device-bulk")]
    AddDeviceBulk {
        time: String,
        new: CtrlAddDeviceBulk,
    },
    #[serde(rename = "add-device-range")]
    AddDeviceRange {
        time: String,
        new: CtrlAddDeviceRange,
    },
    #[serde(rename = "del-device")]
    DelDevice { time: String, new: CtrlDelDevice },
    #[serde(rename = "del-device-bulk")]
    DelDeviceBulk {
        time: String,
        new: CtrlDelDeviceBulk,
    },
    #[serde(rename = "del-device-range")]
    DelDeviceRange {
        time: String,
        new: CtrlDelDeviceRange,
    },
}

#[derive(Clone, Deserialize)]
struct CtrlAddDevice {
    #[serde(rename = "networkAddr")]
    network_addr: String,
}

#[derive(Clone, Deserialize)]
struct CtrlAddDeviceBulk {
    #[serde(rename = "networkAddrs")]
    network_addrs: Vec<String>,
}

#[derive(Clone, Deserialize)]
struct CtrlAddDeviceRange {
    #[serde(rename = "startAddr")]
    start_addr: String,
    #[serde(rename = "endAddr")]
    end_addr: String,
}

#[derive(Clone, Deserialize)]
struct CtrlDelDevice {
    #[serde(rename = "networkAddr")]
    network_addr: String,
}

#[derive(Clone, Deserialize)]
struct CtrlDelDeviceBulk {
    #[serde(rename = "networkAddrs")]
    network_addrs: Vec<String>,
}

#[derive(Clone, Deserialize)]
struct CtrlDelDeviceRange {
    #[serde(rename = "startAddr")]
    start_addr: String,
    #[serde(rename = "endAddr")]
    end_addr: String,
}

/// To consume network control messages from the broker.
#[derive(Clone)]
struct NetCtrlHandler {
    recv_data: Arc<Mutex<Vec<NetworkCtrlMsg>>>,
}

const UNIT_CODE: &'static str = "manager-unit";
const NET_CODE_PRV: &'static str = "manager";
const NET_CODE_PUB: &'static str = "public";
const NET_ADDR_PRV: &'static str = "0000";
const NET_ADDR_PUB: &'static str = "0010";

impl NetCtrlHandler {
    fn new() -> Self {
        NetCtrlHandler {
            recv_data: Arc::new(Mutex::new(vec![])),
        }
    }
}

#[async_trait]
impl EventHandler for NetCtrlHandler {
    async fn on_error(&self, _queue: Arc<dyn GmqQueue>, _err: Box<dyn StdError + Send + Sync>) {}

    async fn on_status(&self, _queue: Arc<dyn GmqQueue>, _status: Status) {}
}

#[async_trait]
impl MessageHandler for NetCtrlHandler {
    async fn on_message(&self, _queue: Arc<dyn GmqQueue>, msg: Box<dyn Message>) {
        let _ = msg.ack().await;

        let data = match serde_json::from_slice::<NetworkCtrlMsg>(msg.payload()) {
            Err(_) => return,
            Ok(data) => data,
        };
        self.recv_data.lock().unwrap().push(data);
    }
}

/// Create the following resources for testing data channel:
/// - 1 unit, application, public(mqtt)/private(amqp) network
/// - network side control receive queues
pub fn before_all_fn(state: &mut HashMap<&'static str, TestState>) -> () {
    let state = state.get_mut(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_mut().unwrap();
    let test_values = state.test_values.as_mut().unwrap();

    let unit = unit::request::PostUnit {
        data: unit::request::PostUnitData {
            code: UNIT_CODE.to_string(),
            owner_id: None,
            name: None,
            info: None,
        },
    };
    let unit_id = match libs::create_unit(runtime, routes_state, TOKEN_MANAGER, &unit) {
        Err(e) => {
            panic!("create unit error: {}", e);
        }
        Ok(unit_id) => unit_id,
    };
    test_values.insert(UNIT_CODE.to_string(), unit_id.clone());
    let mut network = network::request::PostNetwork {
        data: network::request::PostNetworkData {
            code: NET_CODE_PRV.to_string(),
            unit_id: Some(unit_id.clone()),
            host_uri: TEST_AMQP_HOST_URI.to_string(),
            name: None,
            info: None,
        },
    };
    let network_id = match libs::create_network(runtime, routes_state, TOKEN_MANAGER, &network) {
        Err(e) => {
            panic!("create network error: {}", e);
        }
        Ok(network_id) => network_id,
    };
    test_values.insert(NET_CODE_PRV.to_string(), network_id);
    network.data.host_uri = TEST_MQTT_HOST_URI.to_string();
    network.data.code = NET_CODE_PUB.to_string();
    network.data.unit_id = None;
    let public_network_id =
        match libs::create_network(runtime, routes_state, TOKEN_MANAGER, &network) {
            Err(e) => {
                panic!("create public network error: {}", e);
            }
            Ok(network_id) => network_id,
        };
    test_values.insert(NET_CODE_PUB.to_string(), public_network_id.clone());

    // Create connections and queues for receiving network control messages.
    let mut test_conns: Vec<Box<dyn GmqConnection>> = vec![];
    let mut amqp_conn = match AmqpConnection::new(AmqpConnectionOptions {
        uri: TEST_AMQP_HOST_URI.to_string(),
        ..Default::default()
    }) {
        Err(e) => panic!("new AMQP connection error: {}", e),
        Ok(conn) => conn,
    };
    if let Err(e) = amqp_conn.connect() {
        panic!("connect AMQP connection error: {}", e);
    }
    test_conns.push(Box::new(amqp_conn.clone()));
    let mut mqtt_conn = match MqttConnection::new(MqttConnectionOptions {
        uri: TEST_MQTT_HOST_URI.to_string(),
        ..Default::default()
    }) {
        Err(e) => panic!("new MQTT connection error: {}", e),
        Ok(conn) => conn,
    };
    if let Err(e) = mqtt_conn.connect() {
        panic!("connect MQTT connection error: {}", e);
    }
    test_conns.push(Box::new(mqtt_conn.clone()));
    state.test_conns = Some(test_conns);
    let dummy_handler = Arc::new(NetCtrlHandler::new()); // only used for receive unexpected messages.
    let opts = QueueOptions::Amqp(
        AmqpQueueOptions {
            name: format!("broker.network.{}.{}.ctrl", UNIT_CODE, NET_CODE_PRV),
            is_recv: true,
            reliable: true,
            broadcast: false,
            ..Default::default()
        },
        &amqp_conn,
    );
    let mut q = match Queue::new(opts) {
        Err(e) => panic!("new AMQP queue error: {}", e),
        Ok(q) => q,
    };
    q.set_msg_handler(dummy_handler.clone());
    if let Err(e) = q.connect() {
        panic!("AMQP queue connection error: {}", e);
    }
    state.netctrl_queue_amqp = Some(q);
    let opts = QueueOptions::Mqtt(
        MqttQueueOptions {
            name: format!("broker.network.{}.{}.ctrl", "_", NET_CODE_PUB),
            is_recv: true,
            reliable: true,
            broadcast: false,
            ..Default::default()
        },
        &mqtt_conn,
    );
    let mut q = match Queue::new(opts) {
        Err(e) => panic!("new MQTT queue error: {}", e),
        Ok(q) => q,
    };
    q.set_msg_handler(dummy_handler);
    if let Err(e) = q.connect() {
        panic!("MQTT queue connection error: {}", e);
    }
    state.netctrl_queue_mqtt = Some(q);

    let managers = routes_state.application_mgrs.lock().unwrap().clone();
    runtime.block_on(async move {
        for _ in 0..WAIT_COUNT {
            let mut ready = true;
            for (_key, mgr) in managers.iter() {
                if mgr.status() != MgrStatus::Ready {
                    ready = false;
                    break;
                }
            }
            if ready {
                break;
            }
            time::sleep(Duration::from_millis(WAIT_TICK)).await;
        }
    });
    let managers = routes_state.network_mgrs.lock().unwrap().clone();
    runtime.block_on(async move {
        for _ in 0..WAIT_COUNT {
            let mut ready = true;
            for (_key, mgr) in managers.iter() {
                if mgr.status() != MgrStatus::Ready {
                    ready = false;
                    break;
                }
            }
            if ready {
                break;
            }
            time::sleep(Duration::from_millis(WAIT_TICK)).await;
        }
    });

    let q = state.netctrl_queue_amqp.as_mut().unwrap();
    runtime.block_on(async move {
        for _ in 0..WAIT_COUNT {
            if q.status() == Status::Connected {
                break;
            }
            time::sleep(Duration::from_millis(WAIT_TICK)).await;
        }
    });
    let q = state.netctrl_queue_mqtt.as_mut().unwrap();
    runtime.block_on(async move {
        for _ in 0..WAIT_COUNT {
            if q.status() == Status::Connected {
                break;
            }
            time::sleep(Duration::from_millis(WAIT_TICK)).await;
        }
    });
}

/// Clear database and close connections.
pub fn after_all_fn(state: &mut HashMap<&'static str, TestState>) -> () {
    let state = state.get_mut(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();

    libs::clear_all_data(runtime, state);

    if let Some(mut q) = state.netctrl_queue_amqp.take() {
        runtime.block_on(async move {
            let _ = q.close().await;
        })
    }
    if let Some(mut q) = state.netctrl_queue_mqtt.take() {
        runtime.block_on(async move {
            let _ = q.close().await;
        })
    }
    if let Some(mut conns) = state.test_conns.take() {
        runtime.block_on(async move {
            loop {
                match conns.pop() {
                    None => break,
                    Some(mut c) => {
                        let _ = c.close().await;
                    }
                }
            }
        })
    }
}

/// Clear device relative data.
pub fn after_each_fn(state: &mut HashMap<&'static str, TestState>) -> () {
    let state = state.get_mut(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let mongodb_model = state.mongodb.as_ref();
    let sqlite_model = state.sqlite.as_ref();

    if let Some(cache) = state.cache.as_ref() {
        runtime.block_on(async {
            let _ = cache.device().clear().await;
            let _ = cache.device_route().clear().await;
        });
    }
    if let Some(device_id) = state.test_device_id.take() {
        runtime.block_on(async move {
            if let Some(model) = mongodb_model {
                let cond = RouteQueryCond {
                    device_id: Some(device_id.as_str()),
                    ..Default::default()
                };
                let _ = model.device_route().del(&cond).await;
                let cond = QueryCond {
                    device_id: Some(device_id.as_str()),
                    ..Default::default()
                };
                let _ = model.device().del(&cond).await;
            }
            if let Some(model) = sqlite_model {
                let cond = RouteQueryCond {
                    device_id: Some(device_id.as_str()),
                    ..Default::default()
                };
                let _ = model.device_route().del(&cond).await;
                let cond = QueryCond {
                    device_id: Some(device_id.as_str()),
                    ..Default::default()
                };
                let _ = model.device().del(&cond).await;
            }
        })
    }
}

pub fn post_device(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let mut state = context.state.borrow_mut();
    let state = state.get_mut(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();
    let test_values = state.test_values.as_mut().unwrap();

    let recv_handler_amqp = NetCtrlHandler::new();
    let q = state.netctrl_queue_amqp.as_mut().unwrap();
    q.set_handler(Arc::new(recv_handler_amqp.clone()));
    q.set_msg_handler(Arc::new(recv_handler_amqp.clone()));
    let recv_handler_mqtt = NetCtrlHandler::new();
    let q = state.netctrl_queue_mqtt.as_mut().unwrap();
    q.set_handler(Arc::new(recv_handler_mqtt.clone()));
    q.set_msg_handler(Arc::new(recv_handler_mqtt.clone()));

    let time_before = Utc::now().trunc_subsecs(3);
    let param = device::request::PostDevice {
        data: device::request::PostDeviceData {
            unit_id: test_values.get(UNIT_CODE).unwrap().clone(),
            network_id: test_values.get(NET_CODE_PRV).unwrap().clone(),
            network_addr: NET_ADDR_PRV.to_string(),
            profile: None,
            name: None,
            info: None,
        },
    };
    let device_id = libs::create_device(runtime, routes_state, TOKEN_MANAGER, &param)?;
    state.test_device_id = Some(device_id);

    // Sleep for retrieving control messages.
    runtime.block_on(async { time::sleep(Duration::from_secs(1)).await });
    let time_after = Utc::now().trunc_subsecs(3);

    let (mut amqp_data, mqtt_data) = {
        (
            recv_handler_amqp.recv_data.lock().unwrap().clone(),
            recv_handler_mqtt.recv_data.lock().unwrap().clone(),
        )
    };
    expect(amqp_data.len()).to_equal(1)?;
    expect(mqtt_data.len()).to_equal(0)?;
    let data = amqp_data.pop().unwrap();
    match data {
        NetworkCtrlMsg::AddDevice { time, new } => {
            let time = DateTime::parse_from_rfc3339(time.as_str());
            expect(time.is_ok()).to_equal(true)?;
            let time = time.unwrap();
            expect(time.ge(&time_before)).to_equal(true)?;
            expect(time.le(&time_after)).to_equal(true)?;
            expect(new.network_addr.as_str()).to_equal(NET_ADDR_PRV)?;
        }
        _ => return Err("unexpected device type".to_string()),
    }

    Ok(())
}

pub fn post_device_bulk(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let mut state = context.state.borrow_mut();
    let state = state.get_mut(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();
    let test_values = state.test_values.as_mut().unwrap();

    let recv_handler_amqp = NetCtrlHandler::new();
    let q = state.netctrl_queue_amqp.as_mut().unwrap();
    q.set_handler(Arc::new(recv_handler_amqp.clone()));
    q.set_msg_handler(Arc::new(recv_handler_amqp.clone()));
    let recv_handler_mqtt = NetCtrlHandler::new();
    let q = state.netctrl_queue_mqtt.as_mut().unwrap();
    q.set_handler(Arc::new(recv_handler_mqtt.clone()));
    q.set_msg_handler(Arc::new(recv_handler_mqtt.clone()));

    let time_before = Utc::now().trunc_subsecs(3);
    let param = device::request::PostDeviceBulk {
        data: device::request::PostDeviceBulkData {
            unit_id: test_values.get(UNIT_CODE).unwrap().clone(),
            network_id: test_values.get(NET_CODE_PRV).unwrap().clone(),
            network_addrs: vec![NET_ADDR_PRV.to_string()],
            profile: None,
        },
    };
    let mut device_ids = libs::create_device_bulk(runtime, routes_state, TOKEN_MANAGER, &param)?;
    expect(device_ids.len()).to_equal(1)?;
    state.test_device_id = Some(device_ids.pop().unwrap());

    // Sleep for retrieving control messages.
    runtime.block_on(async { time::sleep(Duration::from_secs(1)).await });
    let time_after = Utc::now().trunc_subsecs(3);

    let (mut amqp_data, mqtt_data) = {
        (
            recv_handler_amqp.recv_data.lock().unwrap().clone(),
            recv_handler_mqtt.recv_data.lock().unwrap().clone(),
        )
    };
    expect(amqp_data.len()).to_equal(1)?;
    expect(mqtt_data.len()).to_equal(0)?;
    let data = amqp_data.pop().unwrap();
    match data {
        NetworkCtrlMsg::AddDeviceBulk { time, new } => {
            let time = DateTime::parse_from_rfc3339(time.as_str());
            expect(time.is_ok()).to_equal(true)?;
            let time = time.unwrap();
            expect(time.ge(&time_before)).to_equal(true)?;
            expect(time.le(&time_after)).to_equal(true)?;
            expect(new.network_addrs.len()).to_equal(1)?;
            expect(new.network_addrs[0].as_str()).to_equal(NET_ADDR_PRV)?;
        }
        _ => return Err("unexpected device type".to_string()),
    }

    Ok(())
}

pub fn post_device_bulk_delete(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let mut state = context.state.borrow_mut();
    let state = state.get_mut(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();
    let test_values = state.test_values.as_mut().unwrap();

    let recv_handler_amqp = NetCtrlHandler::new();
    let q = state.netctrl_queue_amqp.as_mut().unwrap();
    q.set_handler(Arc::new(recv_handler_amqp.clone()));
    q.set_msg_handler(Arc::new(recv_handler_amqp.clone()));
    let recv_handler_mqtt = NetCtrlHandler::new();
    let q = state.netctrl_queue_mqtt.as_mut().unwrap();
    q.set_handler(Arc::new(recv_handler_mqtt.clone()));
    q.set_msg_handler(Arc::new(recv_handler_mqtt.clone()));

    let param = device::request::PostDeviceBulk {
        data: device::request::PostDeviceBulkData {
            unit_id: test_values.get(UNIT_CODE).unwrap().clone(),
            network_id: test_values.get(NET_CODE_PRV).unwrap().clone(),
            network_addrs: vec![NET_ADDR_PRV.to_string()],
            profile: None,
        },
    };
    let mut device_ids = libs::create_device_bulk(runtime, routes_state, TOKEN_MANAGER, &param)?;
    expect(device_ids.len()).to_equal(1)?;
    state.test_device_id = Some(device_ids.pop().unwrap());
    let time_before = Utc::now().trunc_subsecs(3);
    libs::delete_device_bulk(runtime, routes_state, TOKEN_MANAGER, &param)?;

    // Sleep for retrieving control messages.
    runtime.block_on(async { time::sleep(Duration::from_secs(1)).await });
    let time_after = Utc::now().trunc_subsecs(3);

    let (mut amqp_data, mqtt_data) = {
        (
            recv_handler_amqp.recv_data.lock().unwrap().clone(),
            recv_handler_mqtt.recv_data.lock().unwrap().clone(),
        )
    };
    expect(amqp_data.len()).to_equal(2)?;
    expect(mqtt_data.len()).to_equal(0)?;
    let data = amqp_data.pop().unwrap();
    match data {
        NetworkCtrlMsg::DelDeviceBulk { time, new } => {
            let time = DateTime::parse_from_rfc3339(time.as_str());
            expect(time.is_ok()).to_equal(true)?;
            let time = time.unwrap();
            expect(time.ge(&time_before)).to_equal(true)?;
            expect(time.le(&time_after)).to_equal(true)?;
            expect(new.network_addrs.len()).to_equal(1)?;
            expect(new.network_addrs[0].as_str()).to_equal(NET_ADDR_PRV)?;
        }
        _ => return Err("unexpected device type".to_string()),
    }

    Ok(())
}

pub fn post_device_range(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let mut state = context.state.borrow_mut();
    let state = state.get_mut(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();
    let test_values = state.test_values.as_mut().unwrap();

    let recv_handler_amqp = NetCtrlHandler::new();
    let q = state.netctrl_queue_amqp.as_mut().unwrap();
    q.set_handler(Arc::new(recv_handler_amqp.clone()));
    q.set_msg_handler(Arc::new(recv_handler_amqp.clone()));
    let recv_handler_mqtt = NetCtrlHandler::new();
    let q = state.netctrl_queue_mqtt.as_mut().unwrap();
    q.set_handler(Arc::new(recv_handler_mqtt.clone()));
    q.set_msg_handler(Arc::new(recv_handler_mqtt.clone()));

    let time_before = Utc::now().trunc_subsecs(3);
    let param = device::request::PostDeviceRange {
        data: device::request::PostDeviceRangeData {
            unit_id: test_values.get(UNIT_CODE).unwrap().clone(),
            network_id: test_values.get(NET_CODE_PRV).unwrap().clone(),
            start_addr: NET_ADDR_PRV.to_string(),
            end_addr: NET_ADDR_PRV.to_string(),
            profile: None,
        },
    };
    let mut device_ids = libs::create_device_range(runtime, routes_state, TOKEN_MANAGER, &param)?;
    expect(device_ids.len()).to_equal(1)?;
    state.test_device_id = Some(device_ids.pop().unwrap());

    // Sleep for retrieving control messages.
    runtime.block_on(async { time::sleep(Duration::from_secs(1)).await });
    let time_after = Utc::now().trunc_subsecs(3);

    let (mut amqp_data, mqtt_data) = {
        (
            recv_handler_amqp.recv_data.lock().unwrap().clone(),
            recv_handler_mqtt.recv_data.lock().unwrap().clone(),
        )
    };
    expect(amqp_data.len()).to_equal(1)?;
    expect(mqtt_data.len()).to_equal(0)?;
    let data = amqp_data.pop().unwrap();
    match data {
        NetworkCtrlMsg::AddDeviceRange { time, new } => {
            let time = DateTime::parse_from_rfc3339(time.as_str());
            expect(time.is_ok()).to_equal(true)?;
            let time = time.unwrap();
            expect(time.ge(&time_before)).to_equal(true)?;
            expect(time.le(&time_after)).to_equal(true)?;
            expect(new.start_addr.as_str()).to_equal(NET_ADDR_PRV)?;
            expect(new.end_addr.as_str()).to_equal(NET_ADDR_PRV)?;
        }
        _ => return Err("unexpected device type".to_string()),
    }

    Ok(())
}

pub fn post_device_range_delete(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let mut state = context.state.borrow_mut();
    let state = state.get_mut(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();
    let test_values = state.test_values.as_mut().unwrap();

    let recv_handler_amqp = NetCtrlHandler::new();
    let q = state.netctrl_queue_amqp.as_mut().unwrap();
    q.set_handler(Arc::new(recv_handler_amqp.clone()));
    q.set_msg_handler(Arc::new(recv_handler_amqp.clone()));
    let recv_handler_mqtt = NetCtrlHandler::new();
    let q = state.netctrl_queue_mqtt.as_mut().unwrap();
    q.set_handler(Arc::new(recv_handler_mqtt.clone()));
    q.set_msg_handler(Arc::new(recv_handler_mqtt.clone()));

    let param = device::request::PostDeviceRange {
        data: device::request::PostDeviceRangeData {
            unit_id: test_values.get(UNIT_CODE).unwrap().clone(),
            network_id: test_values.get(NET_CODE_PRV).unwrap().clone(),
            start_addr: NET_ADDR_PRV.to_string(),
            end_addr: NET_ADDR_PRV.to_string(),
            profile: None,
        },
    };
    let mut device_ids = libs::create_device_range(runtime, routes_state, TOKEN_MANAGER, &param)?;
    expect(device_ids.len()).to_equal(1)?;
    state.test_device_id = Some(device_ids.pop().unwrap());
    let time_before = Utc::now().trunc_subsecs(3);
    libs::delete_device_range(runtime, routes_state, TOKEN_MANAGER, &param)?;

    // Sleep for retrieving control messages.
    runtime.block_on(async { time::sleep(Duration::from_secs(1)).await });
    let time_after = Utc::now().trunc_subsecs(3);

    let (mut amqp_data, mqtt_data) = {
        (
            recv_handler_amqp.recv_data.lock().unwrap().clone(),
            recv_handler_mqtt.recv_data.lock().unwrap().clone(),
        )
    };
    expect(amqp_data.len()).to_equal(2)?;
    expect(mqtt_data.len()).to_equal(0)?;
    let data = amqp_data.pop().unwrap();
    match data {
        NetworkCtrlMsg::DelDeviceRange { time, new } => {
            let time = DateTime::parse_from_rfc3339(time.as_str());
            expect(time.is_ok()).to_equal(true)?;
            let time = time.unwrap();
            expect(time.ge(&time_before)).to_equal(true)?;
            expect(time.le(&time_after)).to_equal(true)?;
            expect(new.start_addr.as_str()).to_equal(NET_ADDR_PRV)?;
            expect(new.end_addr.as_str()).to_equal(NET_ADDR_PRV)?;
        }
        _ => return Err("unexpected device type".to_string()),
    }

    Ok(())
}

/// Patch device with the same network and different address.
pub fn patch_device_addr(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let mut state = context.state.borrow_mut();
    let state = state.get_mut(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();
    let test_values = state.test_values.as_mut().unwrap();

    let recv_handler_amqp = NetCtrlHandler::new();
    let q = state.netctrl_queue_amqp.as_mut().unwrap();
    q.set_handler(Arc::new(recv_handler_amqp.clone()));
    q.set_msg_handler(Arc::new(recv_handler_amqp.clone()));
    let recv_handler_mqtt = NetCtrlHandler::new();
    let q = state.netctrl_queue_mqtt.as_mut().unwrap();
    q.set_handler(Arc::new(recv_handler_mqtt.clone()));
    q.set_msg_handler(Arc::new(recv_handler_mqtt.clone()));

    let param = device::request::PostDevice {
        data: device::request::PostDeviceData {
            unit_id: test_values.get(UNIT_CODE).unwrap().clone(),
            network_id: test_values.get(NET_CODE_PRV).unwrap().clone(),
            network_addr: NET_ADDR_PRV.to_string(),
            profile: None,
            name: None,
            info: None,
        },
    };
    let device_id = libs::create_device(runtime, routes_state, TOKEN_MANAGER, &param)?;
    state.test_device_id = Some(device_id.clone());
    let time_before = Utc::now().trunc_subsecs(3);
    let param = device::request::PatchDevice {
        data: device::request::PatchDeviceData {
            network_addr: Some(NET_ADDR_PUB.to_string()),
            ..Default::default()
        },
    };
    libs::patch_device(
        runtime,
        routes_state,
        TOKEN_MANAGER,
        device_id.as_str(),
        &param,
    )?;

    // Sleep for retrieving control messages.
    runtime.block_on(async { time::sleep(Duration::from_secs(1)).await });
    let time_after = Utc::now().trunc_subsecs(3);

    let (mut amqp_data, mqtt_data) = {
        (
            recv_handler_amqp.recv_data.lock().unwrap().clone(),
            recv_handler_mqtt.recv_data.lock().unwrap().clone(),
        )
    };
    expect(amqp_data.len()).to_equal(3)?;
    expect(mqtt_data.len()).to_equal(0)?;
    let data = amqp_data.pop().unwrap();
    match data {
        NetworkCtrlMsg::AddDevice { time, new } => {
            let time = DateTime::parse_from_rfc3339(time.as_str());
            expect(time.is_ok()).to_equal(true)?;
            let time = time.unwrap();
            expect(time.ge(&time_before)).to_equal(true)?;
            expect(time.le(&time_after)).to_equal(true)?;
            expect(new.network_addr.as_str()).to_equal(NET_ADDR_PUB)?;
        }
        _ => return Err("unexpected device type add".to_string()),
    }
    let data = amqp_data.pop().unwrap();
    match data {
        NetworkCtrlMsg::DelDevice { time, new } => {
            let time = DateTime::parse_from_rfc3339(time.as_str());
            expect(time.is_ok()).to_equal(true)?;
            let time = time.unwrap();
            expect(time.ge(&time_before)).to_equal(true)?;
            expect(time.le(&time_after)).to_equal(true)?;
            expect(new.network_addr.as_str()).to_equal(NET_ADDR_PRV)?;
        }
        _ => return Err("unexpected device type del".to_string()),
    }

    Ok(())
}

/// Patch device with the different network and same address.
pub fn patch_device_network(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let mut state = context.state.borrow_mut();
    let state = state.get_mut(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();
    let test_values = state.test_values.as_mut().unwrap();

    let recv_handler_amqp = NetCtrlHandler::new();
    let q = state.netctrl_queue_amqp.as_mut().unwrap();
    q.set_handler(Arc::new(recv_handler_amqp.clone()));
    q.set_msg_handler(Arc::new(recv_handler_amqp.clone()));
    let recv_handler_mqtt = NetCtrlHandler::new();
    let q = state.netctrl_queue_mqtt.as_mut().unwrap();
    q.set_handler(Arc::new(recv_handler_mqtt.clone()));
    q.set_msg_handler(Arc::new(recv_handler_mqtt.clone()));

    let param = device::request::PostDevice {
        data: device::request::PostDeviceData {
            unit_id: test_values.get(UNIT_CODE).unwrap().clone(),
            network_id: test_values.get(NET_CODE_PRV).unwrap().clone(),
            network_addr: NET_ADDR_PRV.to_string(),
            profile: None,
            name: None,
            info: None,
        },
    };
    let device_id = libs::create_device(runtime, routes_state, TOKEN_MANAGER, &param)?;
    state.test_device_id = Some(device_id.clone());
    let time_before = Utc::now().trunc_subsecs(3);
    let param = device::request::PatchDevice {
        data: device::request::PatchDeviceData {
            network_id: Some(test_values.get(NET_CODE_PUB).unwrap().clone()),
            ..Default::default()
        },
    };
    libs::patch_device(
        runtime,
        routes_state,
        TOKEN_MANAGER,
        device_id.as_str(),
        &param,
    )?;

    // Sleep for retrieving control messages.
    runtime.block_on(async { time::sleep(Duration::from_secs(1)).await });
    let time_after = Utc::now().trunc_subsecs(3);

    let (mut amqp_data, mut mqtt_data) = {
        (
            recv_handler_amqp.recv_data.lock().unwrap().clone(),
            recv_handler_mqtt.recv_data.lock().unwrap().clone(),
        )
    };
    expect(amqp_data.len()).to_equal(2)?;
    expect(mqtt_data.len()).to_equal(1)?;
    let data = mqtt_data.pop().unwrap();
    match data {
        NetworkCtrlMsg::AddDevice { time, new } => {
            let time = DateTime::parse_from_rfc3339(time.as_str());
            expect(time.is_ok()).to_equal(true)?;
            let time = time.unwrap();
            expect(time.ge(&time_before)).to_equal(true)?;
            expect(time.le(&time_after)).to_equal(true)?;
            expect(new.network_addr.as_str()).to_equal(NET_ADDR_PRV)?;
        }
        _ => return Err("unexpected device type add".to_string()),
    }
    let data = amqp_data.pop().unwrap();
    match data {
        NetworkCtrlMsg::DelDevice { time, new } => {
            let time = DateTime::parse_from_rfc3339(time.as_str());
            expect(time.is_ok()).to_equal(true)?;
            let time = time.unwrap();
            expect(time.ge(&time_before)).to_equal(true)?;
            expect(time.le(&time_after)).to_equal(true)?;
            expect(new.network_addr.as_str()).to_equal(NET_ADDR_PRV)?;
        }
        _ => return Err("unexpected device type del".to_string()),
    }

    Ok(())
}

/// Patch device with the different network and different address.
pub fn patch_device_network_addr(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let mut state = context.state.borrow_mut();
    let state = state.get_mut(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();
    let test_values = state.test_values.as_mut().unwrap();

    let recv_handler_amqp = NetCtrlHandler::new();
    let q = state.netctrl_queue_amqp.as_mut().unwrap();
    q.set_handler(Arc::new(recv_handler_amqp.clone()));
    q.set_msg_handler(Arc::new(recv_handler_amqp.clone()));
    let recv_handler_mqtt = NetCtrlHandler::new();
    let q = state.netctrl_queue_mqtt.as_mut().unwrap();
    q.set_handler(Arc::new(recv_handler_mqtt.clone()));
    q.set_msg_handler(Arc::new(recv_handler_mqtt.clone()));

    let param = device::request::PostDevice {
        data: device::request::PostDeviceData {
            unit_id: test_values.get(UNIT_CODE).unwrap().clone(),
            network_id: test_values.get(NET_CODE_PRV).unwrap().clone(),
            network_addr: NET_ADDR_PRV.to_string(),
            profile: None,
            name: None,
            info: None,
        },
    };
    let device_id = libs::create_device(runtime, routes_state, TOKEN_MANAGER, &param)?;
    state.test_device_id = Some(device_id.clone());
    let time_before = Utc::now().trunc_subsecs(3);
    let param = device::request::PatchDevice {
        data: device::request::PatchDeviceData {
            network_id: Some(test_values.get(NET_CODE_PUB).unwrap().clone()),
            network_addr: Some(NET_ADDR_PUB.to_string()),
            ..Default::default()
        },
    };
    libs::patch_device(
        runtime,
        routes_state,
        TOKEN_MANAGER,
        device_id.as_str(),
        &param,
    )?;

    // Sleep for retrieving control messages.
    runtime.block_on(async { time::sleep(Duration::from_secs(1)).await });
    let time_after = Utc::now().trunc_subsecs(3);

    let (mut amqp_data, mut mqtt_data) = {
        (
            recv_handler_amqp.recv_data.lock().unwrap().clone(),
            recv_handler_mqtt.recv_data.lock().unwrap().clone(),
        )
    };
    expect(amqp_data.len()).to_equal(2)?;
    expect(mqtt_data.len()).to_equal(1)?;
    let data = mqtt_data.pop().unwrap();
    match data {
        NetworkCtrlMsg::AddDevice { time, new } => {
            let time = DateTime::parse_from_rfc3339(time.as_str());
            expect(time.is_ok()).to_equal(true)?;
            let time = time.unwrap();
            expect(time.ge(&time_before)).to_equal(true)?;
            expect(time.le(&time_after)).to_equal(true)?;
            expect(new.network_addr.as_str()).to_equal(NET_ADDR_PUB)?;
        }
        _ => return Err("unexpected device type add".to_string()),
    }
    let data = amqp_data.pop().unwrap();
    match data {
        NetworkCtrlMsg::DelDevice { time, new } => {
            let time = DateTime::parse_from_rfc3339(time.as_str());
            expect(time.is_ok()).to_equal(true)?;
            let time = time.unwrap();
            expect(time.ge(&time_before)).to_equal(true)?;
            expect(time.le(&time_after)).to_equal(true)?;
            expect(new.network_addr.as_str()).to_equal(NET_ADDR_PRV)?;
        }
        _ => return Err("unexpected device type del".to_string()),
    }

    Ok(())
}

/// Patch device without network and address.
pub fn patch_device_none(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let mut state = context.state.borrow_mut();
    let state = state.get_mut(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();
    let test_values = state.test_values.as_mut().unwrap();

    let recv_handler_amqp = NetCtrlHandler::new();
    let q = state.netctrl_queue_amqp.as_mut().unwrap();
    q.set_handler(Arc::new(recv_handler_amqp.clone()));
    q.set_msg_handler(Arc::new(recv_handler_amqp.clone()));
    let recv_handler_mqtt = NetCtrlHandler::new();
    let q = state.netctrl_queue_mqtt.as_mut().unwrap();
    q.set_handler(Arc::new(recv_handler_mqtt.clone()));
    q.set_msg_handler(Arc::new(recv_handler_mqtt.clone()));

    let param = device::request::PostDevice {
        data: device::request::PostDeviceData {
            unit_id: test_values.get(UNIT_CODE).unwrap().clone(),
            network_id: test_values.get(NET_CODE_PRV).unwrap().clone(),
            network_addr: NET_ADDR_PRV.to_string(),
            profile: None,
            name: None,
            info: None,
        },
    };
    let device_id = libs::create_device(runtime, routes_state, TOKEN_MANAGER, &param)?;
    state.test_device_id = Some(device_id.clone());
    let param = device::request::PatchDevice {
        data: device::request::PatchDeviceData {
            name: Some("update".to_string()),
            ..Default::default()
        },
    };
    libs::patch_device(
        runtime,
        routes_state,
        TOKEN_MANAGER,
        device_id.as_str(),
        &param,
    )?;

    // Sleep for retrieving control messages.
    runtime.block_on(async { time::sleep(Duration::from_secs(1)).await });

    let (amqp_data, mqtt_data) = {
        (
            recv_handler_amqp.recv_data.lock().unwrap().clone(),
            recv_handler_mqtt.recv_data.lock().unwrap().clone(),
        )
    };
    expect(amqp_data.len()).to_equal(1)?;
    expect(mqtt_data.len()).to_equal(0)
}

pub fn delete_device(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let mut state = context.state.borrow_mut();
    let state = state.get_mut(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();
    let test_values = state.test_values.as_mut().unwrap();

    let recv_handler_amqp = NetCtrlHandler::new();
    let q = state.netctrl_queue_amqp.as_mut().unwrap();
    q.set_handler(Arc::new(recv_handler_amqp.clone()));
    q.set_msg_handler(Arc::new(recv_handler_amqp.clone()));
    let recv_handler_mqtt = NetCtrlHandler::new();
    let q = state.netctrl_queue_mqtt.as_mut().unwrap();
    q.set_handler(Arc::new(recv_handler_mqtt.clone()));
    q.set_msg_handler(Arc::new(recv_handler_mqtt.clone()));

    let param = device::request::PostDevice {
        data: device::request::PostDeviceData {
            unit_id: test_values.get(UNIT_CODE).unwrap().clone(),
            network_id: test_values.get(NET_CODE_PRV).unwrap().clone(),
            network_addr: NET_ADDR_PRV.to_string(),
            profile: None,
            name: None,
            info: None,
        },
    };
    let device_id = libs::create_device(runtime, routes_state, TOKEN_MANAGER, &param)?;
    state.test_device_id = Some(device_id.clone());
    let time_before = Utc::now().trunc_subsecs(3);
    libs::delete_device(runtime, routes_state, TOKEN_MANAGER, device_id.as_str())?;

    // Sleep for retrieving control messages.
    runtime.block_on(async { time::sleep(Duration::from_secs(1)).await });
    let time_after = Utc::now().trunc_subsecs(3);

    let (mut amqp_data, mqtt_data) = {
        (
            recv_handler_amqp.recv_data.lock().unwrap().clone(),
            recv_handler_mqtt.recv_data.lock().unwrap().clone(),
        )
    };
    expect(amqp_data.len()).to_equal(2)?;
    expect(mqtt_data.len()).to_equal(0)?;
    let data = amqp_data.pop().unwrap();
    match data {
        NetworkCtrlMsg::DelDevice { time, new } => {
            let time = DateTime::parse_from_rfc3339(time.as_str());
            expect(time.is_ok()).to_equal(true)?;
            let time = time.unwrap();
            expect(time.ge(&time_before)).to_equal(true)?;
            expect(time.le(&time_after)).to_equal(true)?;
            expect(new.network_addr.as_str()).to_equal(NET_ADDR_PRV)?;
        }
        _ => return Err("unexpected device type".to_string()),
    }

    Ok(())
}
