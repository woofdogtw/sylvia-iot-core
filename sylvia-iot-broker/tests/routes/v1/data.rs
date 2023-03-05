use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::Duration,
};

use async_trait::async_trait;
use chrono::Utc;
use general_mq::{
    connection::Connection as MqConnection,
    queue::{Event, EventHandler, Message, Queue},
    AmqpConnection, AmqpConnectionOptions, AmqpQueueOptions, MqttConnection, MqttConnectionOptions,
    MqttQueueOptions, Queue as MqQueue, QueueOptions as MqQueueOptions,
};
use hex;
use laboratory::SpecContext;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use tokio::time;
use url::Url;

use sylvia_iot_broker::libs::mq::{data, Connection, MgrStatus};
use sylvia_iot_corelib::strings::time_str;

use super::{application, device, device_route, libs, network, unit, STATE, TOKEN_MANAGER};
use crate::{TestState, WAIT_COUNT, WAIT_TICK};

/// Uplink data from network to broker.
#[derive(Serialize)]
struct QueueNetUlData {
    time: String,
    #[serde(rename = "networkAddr")]
    network_addr: String,
    data: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    extension: Option<Map<String, Value>>,
}

/// Downlink data from application to broker.
#[derive(Serialize)]
struct QueueAppDlData {
    #[serde(rename = "correlationId")]
    correlation_id: String,
    #[serde(rename = "deviceId")]
    device_id: Option<String>,
    #[serde(rename = "networkCode")]
    network_code: Option<String>,
    #[serde(rename = "networkAddr")]
    network_addr: Option<String>,
    data: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    extension: Option<Map<String, Value>>,
}

#[derive(Deserialize)]
struct QueueNetDlData {
    #[serde(rename = "dataId")]
    data_id: String,
    #[serde(rename = "pub")]
    _publish: String,
    #[serde(rename = "expiresIn")]
    _expires_in: isize,
    #[serde(rename = "networkAddr")]
    _network_addr: String,
    #[serde(rename = "data")]
    _data: String,
    #[serde(rename = "extension")]
    _extension: Option<Map<String, Value>>,
}

#[derive(Serialize)]
struct QueueNetDlDataResult {
    #[serde(rename = "dataId")]
    data_id: String,
    status: isize,
}

#[derive(Deserialize)]
#[serde(tag = "kind")]
enum RecvDataMsg {
    #[serde(rename = "application-uldata")]
    AppUlData { data: AppUlData },
    #[serde(rename = "application-dldata")]
    AppDlData { data: AppDlData },
    #[serde(rename = "application-dldata-result")]
    AppDlDataResult { data: AppDlDataResult },
    #[serde(rename = "network-uldata")]
    NetUlData { data: NetUlData },
    #[serde(rename = "network-dldata")]
    NetDlData { data: NetDlData },
    #[serde(rename = "network-dldata-result")]
    NetDlDataResult { data: NetDlDataResult },
}

#[derive(Deserialize)]
struct AppUlData {
    #[serde(rename = "dataId")]
    _data_id: String,
    #[serde(rename = "proc")]
    _proc: String,
    #[serde(rename = "pub")]
    _publish: String,
    #[serde(rename = "unitCode")]
    _unit_code: Option<String>,
    #[serde(rename = "networkCode")]
    _network_code: String,
    #[serde(rename = "networkAddr")]
    _network_addr: String,
    #[serde(rename = "unitId")]
    _unit_id: String,
    #[serde(rename = "deviceId")]
    _device_id: String,
    time: String,
    data: String,
    #[serde(rename = "extension")]
    _extension: Option<Map<String, Value>>,
}

#[derive(Deserialize)]
struct AppDlData {
    #[serde(rename = "dataId")]
    _data_id: String,
    #[serde(rename = "proc")]
    _proc: String,
    #[serde(rename = "status")]
    _status: isize,
    #[serde(rename = "unitId")]
    _unit_id: String,
    #[serde(rename = "deviceId")]
    device_id: Option<String>,
    #[serde(rename = "networkCode")]
    network_code: Option<String>,
    #[serde(rename = "networkAddr")]
    network_addr: Option<String>,
    data: String,
    extension: Option<Map<String, Value>>,
}

#[derive(Deserialize)]
struct AppDlDataResult {
    #[serde(rename = "dataId")]
    _data_id: String,
    #[serde(rename = "resp")]
    _resp: String,
    #[serde(rename = "status")]
    _status: isize,
}

#[derive(Deserialize)]
struct NetUlData {
    #[serde(rename = "dataId")]
    _data_id: String,
    #[serde(rename = "proc")]
    _proc: String,
    #[serde(rename = "unitCode")]
    _unit_code: Option<String>,
    #[serde(rename = "networkCode")]
    _network_code: String,
    #[serde(rename = "networkAddr")]
    _network_addr: String,
    #[serde(rename = "unitId")]
    _unit_id: Option<String>,
    #[serde(rename = "deviceId")]
    _device_id: Option<String>,
    time: String,
    data: String,
    #[serde(rename = "extension")]
    _extension: Option<Map<String, Value>>,
}

#[derive(Deserialize)]
struct NetDlData {
    #[serde(rename = "dataId")]
    _data_id: String,
    #[serde(rename = "proc")]
    _proc: String,
    #[serde(rename = "pub")]
    _publish: String,
    #[serde(rename = "status")]
    _status: isize,
    #[serde(rename = "unitId")]
    _unit_id: String,
    #[serde(rename = "deviceId")]
    _device_id: String,
    #[serde(rename = "networkCode")]
    network_code: String,
    #[serde(rename = "networkAddr")]
    network_addr: String,
    data: String,
    extension: Option<Map<String, Value>>,
}

#[derive(Deserialize)]
struct NetDlDataResult {
    #[serde(rename = "dataId")]
    _data_id: String,
    #[serde(rename = "resp")]
    _resp: String,
    #[serde(rename = "status")]
    _status: isize,
}

struct TestResources {
    app_dldata: MqQueue,
    net_prv_uldata: MqQueue,
    net_pub_uldata: MqQueue,
    data_recv_handler: TestHandler,
}

#[derive(Clone)]
struct TestHandler {
    recv_data: Arc<Mutex<Vec<RecvDataMsg>>>,
}

/// To consume routed data from applications or networks.
struct AppNetConsumerHandler {
    result_queue: Option<MqQueue>, // for "broker.network.*.*.dldata" queues.
}

const UNIT_CODE: &'static str = "manager";
const APP_CODE: &'static str = "manager";
const NET_CODE_PRV: &'static str = "manager";
const NET_CODE_PUB: &'static str = "manager";
const NET_ADDR_PRV: &'static str = "manager";
const NET_ADDR_PRV_NOT_ROUTE: &'static str = "manager-not-route";
const NET_ADDR_PUB: &'static str = "public";
const NET_ADDR_PUB_NOT_ROUTE: &'static str = "public-not-route";

impl TestHandler {
    fn new() -> Self {
        TestHandler {
            recv_data: Arc::new(Mutex::new(vec![])),
        }
    }
}

#[async_trait]
impl EventHandler for TestHandler {
    async fn on_event(&self, _queue: Arc<dyn Queue>, _ev: Event) {}

    async fn on_message(&self, _queue: Arc<dyn Queue>, msg: Box<dyn Message>) {
        let _ = msg.ack().await;

        let data = match serde_json::from_slice::<RecvDataMsg>(msg.payload()) {
            Err(e) => {
                println!("unmarshal error: {}", e);
                return;
            }
            Ok(data) => data,
        };
        {
            self.recv_data.lock().unwrap().push(data);
        }
    }
}

#[async_trait]
impl EventHandler for AppNetConsumerHandler {
    async fn on_event(&self, _queue: Arc<dyn Queue>, _ev: Event) {}

    async fn on_message(&self, _queue: Arc<dyn Queue>, msg: Box<dyn Message>) {
        let _ = msg.ack().await;

        let q = match self.result_queue.as_ref() {
            None => return,
            Some(q) => q,
        };
        let dldata = match serde_json::from_slice::<QueueNetDlData>(msg.payload()) {
            Err(_) => return,
            Ok(data) => data,
        };
        let resp = QueueNetDlDataResult {
            data_id: dldata.data_id,
            status: 1,
        };
        let payload = match serde_json::to_vec(&resp) {
            Err(_) => return,
            Ok(payload) => payload,
        };
        let _ = q.send_msg(payload).await;
    }
}

/// Create the following resources for testing data channel:
/// - 1 unit, application, public/private network, public/private/not-route device, 2 device route
/// - application side queues
/// - network side queues
/// - data channel receive queue
pub fn before_all_fn(state: &mut HashMap<&'static str, TestState>) -> () {
    let state = state.get_mut(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_mut().unwrap();
    let routing_values = state.routing_values.as_mut().unwrap();
    let data_ch_host = state.data_ch_host.as_ref().unwrap();

    // Setup data channel send queue.
    // Do this before creating application/network because the message handlers will copy the
    // data_sender during creating ApplicationMgr/NetworkMgr.
    let url = Url::parse(data_ch_host).unwrap();
    let handler = Arc::new(TestHandler::new());
    routes_state.data_sender = Some(data::new(&routes_state.mq_conns, &url, handler).unwrap());

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
    let application = application::request::PostApplication {
        data: application::request::PostApplicationData {
            code: APP_CODE.to_string(),
            unit_id: unit_id.clone(),
            host_uri: data_ch_host.clone(),
            name: None,
            info: None,
        },
    };
    let application_id =
        match libs::create_application(runtime, routes_state, TOKEN_MANAGER, &application) {
            Err(e) => {
                panic!("create application error: {}", e);
            }
            Ok(application_id) => application_id,
        };
    let mut network = network::request::PostNetwork {
        data: network::request::PostNetworkData {
            code: NET_CODE_PRV.to_string(),
            unit_id: Some(unit_id.clone()),
            host_uri: data_ch_host.clone(),
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
    network.data.code = NET_CODE_PUB.to_string();
    network.data.unit_id = None;
    let public_network_id =
        match libs::create_network(runtime, routes_state, TOKEN_MANAGER, &network) {
            Err(e) => {
                panic!("create public network error: {}", e);
            }
            Ok(network_id) => network_id,
        };
    let mut device = device::request::PostDevice {
        data: device::request::PostDeviceData {
            unit_id: unit_id.clone(),
            network_id: network_id.clone(),
            network_addr: NET_ADDR_PRV.to_string(),
            name: None,
            info: None,
        },
    };
    let device_id = match libs::create_device(runtime, routes_state, TOKEN_MANAGER, &device) {
        Err(e) => {
            panic!("create device error: {}", e);
        }
        Ok(device_id) => device_id,
    };
    device.data.network_addr = NET_ADDR_PRV_NOT_ROUTE.to_string();
    let _not_route_device_id =
        match libs::create_device(runtime, routes_state, TOKEN_MANAGER, &device) {
            Err(e) => {
                panic!("create not-route device error: {}", e);
            }
            Ok(device_id) => device_id,
        };
    device.data.network_id = public_network_id.clone();
    device.data.network_addr = NET_ADDR_PUB.to_string();
    let public_device_id = match libs::create_device(runtime, routes_state, TOKEN_MANAGER, &device)
    {
        Err(e) => {
            panic!("create public device error: {}", e);
        }
        Ok(device_id) => device_id,
    };
    routing_values.insert(NET_ADDR_PUB.to_string(), public_device_id.clone());
    device.data.network_id = public_network_id.clone();
    device.data.network_addr = NET_ADDR_PUB_NOT_ROUTE.to_string();
    let _public_device_id = match libs::create_device(runtime, routes_state, TOKEN_MANAGER, &device)
    {
        Err(e) => {
            panic!("create public not-route device error: {}", e);
        }
        Ok(device_id) => device_id,
    };
    let mut route = device_route::request::PostDeviceRoute {
        data: device_route::request::PostDeviceRouteData {
            device_id: device_id.clone(),
            application_id: application_id.clone(),
        },
    };
    if let Err(e) = libs::create_device_route(runtime, routes_state, TOKEN_MANAGER, &route) {
        panic!("create device route error: {}", e);
    }
    route.data.device_id = public_device_id.clone();
    if let Err(e) = libs::create_device_route(runtime, routes_state, TOKEN_MANAGER, &route) {
        panic!("create public device route error: {}", e);
    }
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
}

/// Clear database and close connections.
pub fn after_all_fn(state: &mut HashMap<&'static str, TestState>) -> () {
    let state = state.get_mut(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();

    libs::clear_all_data(runtime, state);

    if let Some(state) = state.routes_state.as_mut() {
        if let Some(mut q) = state.data_sender.take() {
            runtime.block_on(async {
                if let Err(e) = q.close().await {
                    println!("close data channel {} error: {}", q.name(), e);
                }
            });
        }
    }

    if let Some(conn) = state.mq_conn.take() {
        runtime.block_on(async {
            match conn {
                Connection::Amqp(mut conn, _) => {
                    let _ = conn.close().await;
                }
                Connection::Mqtt(mut conn, _) => {
                    let _ = conn.close().await;
                }
            }
        })
    }
}

/// Clear application/network/data side connection and queues.
pub fn after_each_fn(state: &mut HashMap<&'static str, TestState>) -> () {
    let state = state.get_mut(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();

    if let Some(cache) = state.cache.as_ref() {
        runtime.block_on(async {
            let _ = cache.device().clear().await;
            let _ = cache.device_route().clear().await;
        });
    }
    if let Some(mut queues) = state.routing_queues.take() {
        runtime.block_on(async move {
            loop {
                match queues.pop() {
                    None => break,
                    Some(mut q) => {
                        let _ = q.close().await;
                    }
                }
            }
        })
    }
    if let Some(mut conns) = state.routing_conns.take() {
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
    if let Some(mut queue) = state.data_queue.take() {
        runtime.block_on(async {
            let _ = queue.close().await;
        })
    }
}

/// Test the following cases:
/// - send uplink data from 4 devices.
/// - check data channel, only two routed device data should be received.
pub fn uplink(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let mut state = context.state.borrow_mut();
    let state = state.get_mut(STATE).unwrap();

    let rsc = create_connections(state)?;
    let runtime = state.runtime.as_ref().unwrap();

    // Test routed private device data.
    let now = Utc::now();
    let payload_hex = hex::encode(NET_ADDR_PRV);
    let data = QueueNetUlData {
        time: time_str(&now),
        network_addr: NET_ADDR_PRV.to_string(),
        data: payload_hex.clone(),
        extension: None,
    };
    runtime.block_on(async {
        let payload = match serde_json::to_vec(&data) {
            Err(e) => return Err(format!("marshal private data error: {}", e)),
            Ok(payload) => payload,
        };
        if let Err(e) = rsc.net_prv_uldata.send_msg(payload).await {
            return Err(format!("send private data error: {}", e));
        }

        let mut is_app_recv = false;
        let mut is_net_recv = false;
        for _ in 0..WAIT_COUNT {
            if let Some(data) = { rsc.data_recv_handler.recv_data.lock().unwrap().pop() } {
                match data {
                    RecvDataMsg::NetUlData { data } => {
                        if data.data.as_str().eq(payload_hex.as_str())
                            && data.time.as_str().eq(time_str(&now).as_str())
                        {
                            is_net_recv = true;
                        }
                    }
                    RecvDataMsg::AppUlData { data } => {
                        if data.data.as_str().eq(payload_hex.as_str())
                            && data.time.as_str().eq(time_str(&now).as_str())
                        {
                            is_app_recv = true;
                        }
                    }
                    _ => (),
                }
                if is_app_recv && is_net_recv {
                    break;
                }
            }
            time::sleep(Duration::from_millis(WAIT_TICK)).await;
        }
        if !is_app_recv || !is_net_recv {
            return Err(format!(
                "recv private data error. app: {}, net: {}",
                is_app_recv, is_net_recv
            ));
        }
        Ok(())
    })?;

    // Test routed public device data.
    let now = Utc::now();
    let payload_hex = hex::encode(NET_ADDR_PUB);
    let data = QueueNetUlData {
        time: time_str(&now),
        network_addr: NET_ADDR_PUB.to_string(),
        data: payload_hex.clone(),
        extension: None,
    };
    runtime.block_on(async {
        let payload = match serde_json::to_vec(&data) {
            Err(e) => return Err(format!("marshal public data error: {}", e)),
            Ok(payload) => payload,
        };
        if let Err(e) = rsc.net_pub_uldata.send_msg(payload).await {
            return Err(format!("send public data error: {}", e));
        }

        let mut is_app_recv = false;
        let mut is_net_recv = false;
        for _ in 0..WAIT_COUNT {
            if let Some(data) = { rsc.data_recv_handler.recv_data.lock().unwrap().pop() } {
                match data {
                    RecvDataMsg::NetUlData { data } => {
                        if data.data.as_str().eq(payload_hex.as_str())
                            && data.time.as_str().eq(time_str(&now).as_str())
                        {
                            is_net_recv = true;
                        }
                    }
                    RecvDataMsg::AppUlData { data } => {
                        if data.data.as_str().eq(payload_hex.as_str())
                            && data.time.as_str().eq(time_str(&now).as_str())
                        {
                            is_app_recv = true;
                        }
                    }
                    _ => (),
                }
                if is_app_recv && is_net_recv {
                    break;
                }
            }
            time::sleep(Duration::from_millis(WAIT_TICK)).await;
        }
        if !is_app_recv || !is_net_recv {
            return Err(format!(
                "recv public data error. app: {}, net: {}",
                is_app_recv, is_net_recv
            ));
        }
        Ok(())
    })?;

    // Test not-routed private device data.
    let now = Utc::now();
    let payload_hex = hex::encode(NET_ADDR_PRV_NOT_ROUTE);
    let data = QueueNetUlData {
        time: time_str(&now),
        network_addr: NET_ADDR_PRV_NOT_ROUTE.to_string(),
        data: payload_hex.clone(),
        extension: None,
    };
    runtime.block_on(async {
        let payload = match serde_json::to_vec(&data) {
            Err(e) => return Err(format!("marshal not-route private data error: {}", e)),
            Ok(payload) => payload,
        };
        if let Err(e) = rsc.net_prv_uldata.send_msg(payload).await {
            return Err(format!("send not-route private data error: {}", e));
        }

        let mut is_app_recv = false;
        let mut is_net_recv = false;
        for _ in 0..WAIT_COUNT {
            if let Some(data) = { rsc.data_recv_handler.recv_data.lock().unwrap().pop() } {
                match data {
                    RecvDataMsg::NetUlData { data } => {
                        if data.data.as_str().eq(payload_hex.as_str())
                            && data.time.as_str().eq(time_str(&now).as_str())
                        {
                            is_net_recv = true;
                        }
                    }
                    RecvDataMsg::AppUlData { data } => {
                        if data.data.as_str().eq(payload_hex.as_str())
                            && data.time.as_str().eq(time_str(&now).as_str())
                        {
                            is_app_recv = true;
                        }
                    }
                    _ => (),
                }
                if is_app_recv && is_net_recv {
                    break;
                }
            }
            time::sleep(Duration::from_millis(WAIT_TICK)).await;
        }
        if is_app_recv || !is_net_recv {
            return Err(format!(
                "recv not-route private data error. app: {}, net: {}",
                is_app_recv, is_net_recv
            ));
        }
        Ok(())
    })?;

    // Test routed public device data.
    let now = Utc::now();
    let payload_hex = hex::encode(NET_ADDR_PUB_NOT_ROUTE);
    let data = QueueNetUlData {
        time: time_str(&now),
        network_addr: NET_ADDR_PUB_NOT_ROUTE.to_string(),
        data: payload_hex.clone(),
        extension: None,
    };
    runtime.block_on(async {
        let payload = match serde_json::to_vec(&data) {
            Err(e) => return Err(format!("marshal not-route public data error: {}", e)),
            Ok(payload) => payload,
        };
        if let Err(e) = rsc.net_pub_uldata.send_msg(payload).await {
            return Err(format!("send not-route public data error: {}", e));
        }

        let mut is_app_recv = false;
        let mut is_net_recv = false;
        for _ in 0..WAIT_COUNT {
            if let Some(data) = { rsc.data_recv_handler.recv_data.lock().unwrap().pop() } {
                match data {
                    RecvDataMsg::NetUlData { data } => {
                        if data.data.as_str().eq(payload_hex.as_str())
                            && data.time.as_str().eq(time_str(&now).as_str())
                        {
                            is_net_recv = true;
                        }
                    }
                    RecvDataMsg::AppUlData { data } => {
                        if data.data.as_str().eq(payload_hex.as_str())
                            && data.time.as_str().eq(time_str(&now).as_str())
                        {
                            is_app_recv = true;
                        }
                    }
                    _ => (),
                }
                if is_app_recv && is_net_recv {
                    break;
                }
            }
            time::sleep(Duration::from_millis(WAIT_TICK)).await;
        }
        if is_app_recv || !is_net_recv {
            return Err(format!(
                "recv not-route public data error. app: {}, net: {}",
                is_app_recv, is_net_recv
            ));
        }
        Ok(())
    })?;

    Ok(())
}

/// Test the following cases:
/// - send data to 2 devices.
/// - check data channel, only two routed device data should be received.
pub fn downlink(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let mut state = context.state.borrow_mut();
    let state = state.get_mut(STATE).unwrap();

    let rsc = create_connections(state)?;
    let runtime = state.runtime.as_ref().unwrap();
    let routing_values = state.routing_values.as_ref().unwrap();

    // Send to private device by network address.
    let payload_hex = hex::encode(NET_ADDR_PRV);
    let data = QueueAppDlData {
        correlation_id: NET_ADDR_PRV.to_string(),
        device_id: None,
        network_code: Some(NET_CODE_PRV.to_string()),
        network_addr: Some(NET_ADDR_PRV.to_string()),
        data: payload_hex.clone(),
        extension: None,
    };
    runtime.block_on(async {
        let payload = match serde_json::to_vec(&data) {
            Err(e) => return Err(format!("marshal private data error: {}", e)),
            Ok(payload) => payload,
        };
        if let Err(e) = rsc.app_dldata.send_msg(payload).await {
            return Err(format!("send private data error: {}", e));
        }

        let mut is_app_recv = false;
        let mut is_app_result_recv = false;
        let mut is_net_recv = false;
        let mut is_net_result_recv = false;
        for _ in 0..WAIT_COUNT {
            if let Some(data) = { rsc.data_recv_handler.recv_data.lock().unwrap().pop() } {
                match data {
                    RecvDataMsg::NetDlData { data } => {
                        if data.data.as_str().eq(payload_hex.as_str())
                            && data.extension.is_none()
                            && data.network_code.as_str().eq(NET_CODE_PRV)
                            && data.network_addr.as_str().eq(NET_ADDR_PRV)
                        {
                            is_net_recv = true;
                        }
                    }
                    RecvDataMsg::NetDlDataResult { data } => {
                        let _ = data;
                        is_net_result_recv = true;
                    }
                    RecvDataMsg::AppDlData { data } => {
                        if data.data.as_str().eq(payload_hex.as_str())
                            && data.extension.is_none()
                            && data.device_id.is_none()
                            && data.network_code.is_some()
                            && data.network_addr.is_some()
                        {
                            is_app_recv = true;
                        }
                    }
                    RecvDataMsg::AppDlDataResult { data } => {
                        let _ = data;
                        is_app_result_recv = true;
                    }
                    _ => (),
                }
                if is_app_recv && is_app_result_recv && is_net_recv && is_net_result_recv {
                    break;
                }
            }
            time::sleep(Duration::from_millis(WAIT_TICK)).await;
        }
        if !is_app_recv || !is_app_result_recv || !is_net_recv || !is_net_result_recv {
            return Err(format!(
                "recv private data error. app: {}/{}, net: {}/{}",
                is_app_recv, is_app_result_recv, is_net_recv, is_net_result_recv
            ));
        }
        Ok(())
    })?;

    // Send to public device by device ID.
    let payload_hex = hex::encode(NET_ADDR_PUB);
    let data = QueueAppDlData {
        correlation_id: NET_ADDR_PUB.to_string(),
        device_id: Some(routing_values.get(NET_ADDR_PUB).unwrap().clone()),
        network_code: None,
        network_addr: None,
        data: payload_hex.clone(),
        extension: None,
    };
    runtime.block_on(async {
        let payload = match serde_json::to_vec(&data) {
            Err(e) => return Err(format!("marshal private data error: {}", e)),
            Ok(payload) => payload,
        };
        if let Err(e) = rsc.app_dldata.send_msg(payload).await {
            return Err(format!("send private data error: {}", e));
        }

        let mut is_app_recv = false;
        let mut is_app_result_recv = false;
        let mut is_net_recv = false;
        let mut is_net_result_recv = false;
        for _ in 0..WAIT_COUNT {
            if let Some(data) = { rsc.data_recv_handler.recv_data.lock().unwrap().pop() } {
                match data {
                    RecvDataMsg::NetDlData { data } => {
                        if data.data.as_str().eq(payload_hex.as_str())
                            && data.extension.is_none()
                            && data.network_code.as_str().eq(NET_CODE_PUB)
                            && data.network_addr.as_str().eq(NET_ADDR_PUB)
                        {
                            is_net_recv = true;
                        }
                    }
                    RecvDataMsg::NetDlDataResult { data } => {
                        let _ = data;
                        is_net_result_recv = true;
                    }
                    RecvDataMsg::AppDlData { data } => {
                        if data.data.as_str().eq(payload_hex.as_str())
                            && data.extension.is_none()
                            && data.device_id.is_some()
                            && data.network_code.is_none()
                            && data.network_addr.is_none()
                        {
                            is_app_recv = true;
                        }
                    }
                    RecvDataMsg::AppDlDataResult { data } => {
                        let _ = data;
                        is_app_result_recv = true;
                    }
                    _ => (),
                }
                if is_app_recv && is_app_result_recv && is_net_recv && is_net_result_recv {
                    break;
                }
            }
            time::sleep(Duration::from_millis(WAIT_TICK)).await;
        }
        if !is_app_recv || !is_app_result_recv || !is_net_recv || !is_net_result_recv {
            return Err(format!(
                "recv private data error. app: {}/{}, net: {}/{}",
                is_app_recv, is_app_result_recv, is_net_recv, is_net_result_recv
            ));
        }
        Ok(())
    })?;

    Ok(())
}

/// Create application/network/data side connection and queues.
fn create_connections(state: &mut TestState) -> Result<TestResources, String> {
    let host = state.data_ch_host.as_ref().unwrap();
    if host.starts_with("mqtt") {
        // Create connection.
        let mut conn = MqttConnection::new(MqttConnectionOptions {
            uri: host.to_string(),
            ..Default::default()
        })?;
        if let Err(e) = conn.connect() {
            return Err(format!("new connection error: {}", e));
        }
        state.routing_conns = Some(vec![Box::new(conn.clone())]);

        // Create data channel receive queue.
        let opts = MqQueueOptions::Mqtt(
            MqttQueueOptions {
                name: "broker.data".to_string(),
                is_recv: true,
                reliable: true,
                broadcast: false,
                ..Default::default()
            },
            &conn,
        );
        let data_recv_handler = TestHandler::new();
        let mut q = MqQueue::new(opts)?;
        q.set_handler(Arc::new(data_recv_handler.clone()));
        if let Err(e) = q.connect() {
            return Err(format!("data queue connection error: {}", e));
        }
        state.data_queue = Some(q);

        // Create application/network side send queues.
        let mut routing_queues: Vec<Box<dyn Queue>> = vec![];
        let mut opts = MqttQueueOptions {
            name: format!(
                "broker.network.{}.{}.dldata-result",
                UNIT_CODE, NET_CODE_PRV
            ),
            is_recv: false,
            reliable: true,
            broadcast: false,
            ..Default::default()
        };
        let mut net_dldata_result = MqQueue::new(MqQueueOptions::Mqtt(opts.clone(), &conn))?;
        net_dldata_result.set_handler(Arc::new(AppNetConsumerHandler { result_queue: None }));
        if let Err(e) = net_dldata_result.connect() {
            return Err(format!("net dldata-result queue connection error: {}", e));
        }
        routing_queues.push(Box::new(net_dldata_result.clone()));
        opts.name = format!("broker.network._.{}.dldata-result", NET_CODE_PUB);
        let mut pubnet_dldata_result = MqQueue::new(MqQueueOptions::Mqtt(opts.clone(), &conn))?;
        pubnet_dldata_result.set_handler(Arc::new(AppNetConsumerHandler { result_queue: None }));
        if let Err(e) = pubnet_dldata_result.connect() {
            return Err(format!(
                "net pub dldata-result queue connection error: {}",
                e
            ));
        }
        routing_queues.push(Box::new(pubnet_dldata_result.clone()));
        opts.name = format!("broker.application.{}.{}.dldata", UNIT_CODE, APP_CODE);
        let mut app_dldata = MqQueue::new(MqQueueOptions::Mqtt(opts.clone(), &conn))?;
        app_dldata.set_handler(Arc::new(AppNetConsumerHandler { result_queue: None }));
        if let Err(e) = app_dldata.connect() {
            return Err(format!("app dldata queue connection error: {}", e));
        }
        routing_queues.push(Box::new(app_dldata.clone()));
        opts.name = format!("broker.network.{}.{}.uldata", UNIT_CODE, NET_CODE_PRV);
        let mut net_prv_uldata = MqQueue::new(MqQueueOptions::Mqtt(opts.clone(), &conn))?;
        net_prv_uldata.set_handler(Arc::new(AppNetConsumerHandler { result_queue: None }));
        if let Err(e) = net_prv_uldata.connect() {
            return Err(format!("net uldata queue connection error: {}", e));
        }
        routing_queues.push(Box::new(net_prv_uldata.clone()));
        opts.name = format!("broker.network._.{}.uldata", NET_CODE_PUB);
        let mut net_pub_uldata = MqQueue::new(MqQueueOptions::Mqtt(opts.clone(), &conn))?;
        net_pub_uldata.set_handler(Arc::new(AppNetConsumerHandler { result_queue: None }));
        if let Err(e) = net_pub_uldata.connect() {
            return Err(format!("net pub uldata queue connection error: {}", e));
        }
        routing_queues.push(Box::new(net_pub_uldata.clone()));

        // Create application/network side received queues.
        opts.is_recv = true;
        opts.name = format!("broker.application.{}.{}.uldata", UNIT_CODE, APP_CODE);
        let mut q = MqQueue::new(MqQueueOptions::Mqtt(opts.clone(), &conn))?;
        q.set_handler(Arc::new(AppNetConsumerHandler { result_queue: None }));
        if let Err(e) = q.connect() {
            return Err(format!("app uldata queue connection error: {}", e));
        }
        routing_queues.push(Box::new(q));
        opts.name = format!("broker.application.{}.{}.dldata-resp", UNIT_CODE, APP_CODE);
        let mut q = MqQueue::new(MqQueueOptions::Mqtt(opts.clone(), &conn))?;
        q.set_handler(Arc::new(AppNetConsumerHandler { result_queue: None }));
        if let Err(e) = q.connect() {
            return Err(format!("app dldata-resp queue connection error: {}", e));
        }
        routing_queues.push(Box::new(q));
        opts.name = format!(
            "broker.application.{}.{}.dldata-result",
            UNIT_CODE, APP_CODE
        );
        let mut q = MqQueue::new(MqQueueOptions::Mqtt(opts.clone(), &conn))?;
        q.set_handler(Arc::new(AppNetConsumerHandler { result_queue: None }));
        if let Err(e) = q.connect() {
            return Err(format!("app dldata-result queue connection error: {}", e));
        }
        routing_queues.push(Box::new(q));
        opts.name = format!("broker.network.{}.{}.dldata", UNIT_CODE, NET_CODE_PRV);
        let mut q = MqQueue::new(MqQueueOptions::Mqtt(opts.clone(), &conn))?;
        q.set_handler(Arc::new(AppNetConsumerHandler {
            result_queue: Some(net_dldata_result),
        }));
        if let Err(e) = q.connect() {
            return Err(format!("net dldata queue connection error: {}", e));
        }
        routing_queues.push(Box::new(q));
        opts.name = format!("broker.network._.{}.dldata", NET_CODE_PUB);
        let mut q = MqQueue::new(MqQueueOptions::Mqtt(opts.clone(), &conn))?;
        q.set_handler(Arc::new(AppNetConsumerHandler {
            result_queue: Some(pubnet_dldata_result),
        }));
        if let Err(e) = q.connect() {
            return Err(format!("net pub dldata queue connection error: {}", e));
        }
        routing_queues.push(Box::new(q));

        state.routing_queues = Some(routing_queues);

        // Wait for queues connected.
        let runtime = state.runtime.as_ref().unwrap();
        runtime.block_on(async { time::sleep(Duration::from_secs(2)).await });

        Ok(TestResources {
            app_dldata,
            net_prv_uldata,
            net_pub_uldata,
            data_recv_handler,
        })
    } else {
        // Create connection.
        let mut conn = AmqpConnection::new(AmqpConnectionOptions {
            uri: host.to_string(),
            ..Default::default()
        })?;
        if let Err(e) = conn.connect() {
            return Err(format!("new connection error: {}", e));
        }
        state.routing_conns = Some(vec![Box::new(conn.clone())]);

        // Create data channel receive queue.
        let opts = MqQueueOptions::Amqp(
            AmqpQueueOptions {
                name: "broker.data".to_string(),
                is_recv: true,
                reliable: true,
                broadcast: false,
                ..Default::default()
            },
            &conn,
        );
        let data_recv_handler = TestHandler::new();
        let mut q = MqQueue::new(opts)?;
        q.set_handler(Arc::new(data_recv_handler.clone()));
        if let Err(e) = q.connect() {
            return Err(format!("data queue connection error: {}", e));
        }
        state.data_queue = Some(q);

        // Create application/network side send queues.
        let mut routing_queues: Vec<Box<dyn Queue>> = vec![];
        let mut opts = AmqpQueueOptions {
            name: format!(
                "broker.network.{}.{}.dldata-result",
                UNIT_CODE, NET_CODE_PRV
            ),
            is_recv: false,
            reliable: true,
            broadcast: false,
            ..Default::default()
        };
        let mut net_dldata_result = MqQueue::new(MqQueueOptions::Amqp(opts.clone(), &conn))?;
        net_dldata_result.set_handler(Arc::new(AppNetConsumerHandler { result_queue: None }));
        if let Err(e) = net_dldata_result.connect() {
            return Err(format!("net dldata-result queue connection error: {}", e));
        }
        routing_queues.push(Box::new(net_dldata_result.clone()));
        opts.name = format!("broker.network._.{}.dldata-result", NET_CODE_PUB);
        let mut pubnet_dldata_result = MqQueue::new(MqQueueOptions::Amqp(opts.clone(), &conn))?;
        pubnet_dldata_result.set_handler(Arc::new(AppNetConsumerHandler { result_queue: None }));
        if let Err(e) = pubnet_dldata_result.connect() {
            return Err(format!(
                "net pub dldata-result queue connection error: {}",
                e
            ));
        }
        routing_queues.push(Box::new(pubnet_dldata_result.clone()));
        opts.name = format!("broker.application.{}.{}.dldata", UNIT_CODE, APP_CODE);
        let mut app_dldata = MqQueue::new(MqQueueOptions::Amqp(opts.clone(), &conn))?;
        app_dldata.set_handler(Arc::new(AppNetConsumerHandler { result_queue: None }));
        if let Err(e) = app_dldata.connect() {
            return Err(format!("app dldata queue connection error: {}", e));
        }
        routing_queues.push(Box::new(app_dldata.clone()));
        opts.name = format!("broker.network.{}.{}.uldata", UNIT_CODE, NET_CODE_PRV);
        let mut net_prv_uldata = MqQueue::new(MqQueueOptions::Amqp(opts.clone(), &conn))?;
        net_prv_uldata.set_handler(Arc::new(AppNetConsumerHandler { result_queue: None }));
        if let Err(e) = net_prv_uldata.connect() {
            return Err(format!("net uldata queue connection error: {}", e));
        }
        routing_queues.push(Box::new(net_prv_uldata.clone()));
        opts.name = format!("broker.network._.{}.uldata", NET_CODE_PUB);
        let mut net_pub_uldata = MqQueue::new(MqQueueOptions::Amqp(opts.clone(), &conn))?;
        net_pub_uldata.set_handler(Arc::new(AppNetConsumerHandler { result_queue: None }));
        if let Err(e) = net_pub_uldata.connect() {
            return Err(format!("net pub uldata queue connection error: {}", e));
        }
        routing_queues.push(Box::new(net_pub_uldata.clone()));

        // Create application/network side received queues.
        opts.is_recv = true;
        opts.name = format!("broker.application.{}.{}.uldata", UNIT_CODE, APP_CODE);
        let mut q = MqQueue::new(MqQueueOptions::Amqp(opts.clone(), &conn))?;
        q.set_handler(Arc::new(AppNetConsumerHandler { result_queue: None }));
        if let Err(e) = q.connect() {
            return Err(format!("app uldata queue connection error: {}", e));
        }
        routing_queues.push(Box::new(q));
        opts.name = format!("broker.application.{}.{}.dldata-resp", UNIT_CODE, APP_CODE);
        let mut q = MqQueue::new(MqQueueOptions::Amqp(opts.clone(), &conn))?;
        q.set_handler(Arc::new(AppNetConsumerHandler { result_queue: None }));
        if let Err(e) = q.connect() {
            return Err(format!("app dldata-resp queue connection error: {}", e));
        }
        routing_queues.push(Box::new(q));
        opts.name = format!(
            "broker.application.{}.{}.dldata-result",
            UNIT_CODE, APP_CODE
        );
        let mut q = MqQueue::new(MqQueueOptions::Amqp(opts.clone(), &conn))?;
        q.set_handler(Arc::new(AppNetConsumerHandler { result_queue: None }));
        if let Err(e) = q.connect() {
            return Err(format!("app dldata-result queue connection error: {}", e));
        }
        routing_queues.push(Box::new(q));
        opts.name = format!("broker.network.{}.{}.dldata", UNIT_CODE, NET_CODE_PRV);
        let mut q = MqQueue::new(MqQueueOptions::Amqp(opts.clone(), &conn))?;
        q.set_handler(Arc::new(AppNetConsumerHandler {
            result_queue: Some(net_dldata_result),
        }));
        if let Err(e) = q.connect() {
            return Err(format!("net dldata queue connection error: {}", e));
        }
        routing_queues.push(Box::new(q));
        opts.name = format!("broker.network._.{}.dldata", NET_CODE_PUB);
        let mut q = MqQueue::new(MqQueueOptions::Amqp(opts.clone(), &conn))?;
        q.set_handler(Arc::new(AppNetConsumerHandler {
            result_queue: Some(pubnet_dldata_result),
        }));
        if let Err(e) = q.connect() {
            return Err(format!("net pub dldata queue connection error: {}", e));
        }
        routing_queues.push(Box::new(q));

        state.routing_queues = Some(routing_queues);

        // Wait for queues connected.
        let runtime = state.runtime.as_ref().unwrap();
        runtime.block_on(async { time::sleep(Duration::from_secs(2)).await });

        Ok(TestResources {
            app_dldata,
            net_prv_uldata,
            net_pub_uldata,
            data_recv_handler,
        })
    }
}
