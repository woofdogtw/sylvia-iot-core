use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::Duration,
};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use general_mq::{
    connection::GmqConnection,
    queue::{Event, EventHandler, GmqQueue, Message, Status},
    AmqpConnection, AmqpConnectionOptions, AmqpQueueOptions, MqttConnection, MqttConnectionOptions,
    MqttQueueOptions, Queue, QueueOptions,
};
use laboratory::{expect, SpecContext};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use tokio::time;

use sylvia_iot_broker::{
    libs::mq::{Connection, MgrStatus, Options},
    models::{
        device::{QueryCond, QueryOneCond},
        device_route::QueryCond as RouteQueryCond,
        Model,
    },
    routes::ErrReq,
};
use sylvia_iot_corelib::strings;

use super::{
    application, device, device_route, libs, network, network_route, unit, STATE, TOKEN_MANAGER,
    TOKEN_OWNER,
};
use crate::{TestState, WAIT_COUNT, WAIT_TICK};

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

/// Downlink data from application to broker.
#[derive(Debug, Default, Serialize)]
pub struct AppDlData {
    #[serde(rename = "correlationId")]
    pub correlation_id: String,
    #[serde(rename = "deviceId")]
    pub device_id: Option<String>,
    #[serde(rename = "networkCode")]
    pub network_code: Option<String>,
    #[serde(rename = "networkAddr")]
    pub network_addr: Option<String>,
    pub data: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extension: Option<Map<String, Value>>,
}

/// Downlink data response.
#[derive(Debug, Default, Deserialize)]
pub struct AppDlDataResp {
    #[serde(rename = "correlationId")]
    pub correlation_id: String,
    #[serde(rename = "dataId")]
    pub data_id: Option<String>,
    pub error: Option<String>,
    pub message: Option<String>,
}

/// Downlink data result when processing or completing data transfer to the device.
#[derive(Debug, Deserialize)]
pub struct AppDlDataResult {
    #[serde(rename = "dataId")]
    pub data_id: String,
    pub status: isize,
    pub message: Option<String>,
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

/// Downlink data result when processing or completing data transfer to the device.
#[derive(Debug, Serialize)]
pub struct NetDlDataResult {
    #[serde(rename = "dataId")]
    pub data_id: String,
    pub status: isize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

struct TestResources {
    _manager_application_uldata: Queue,
    manager_application_dldata: Queue,
    _manager_application_dldata_resp: Queue,
    _manager_application_dldata_result: Queue,
    _owner_application_uldata: Queue,
    owner_application_dldata: Queue,
    _owner_application_dldata_resp: Queue,
    _owner_application_dldata_result: Queue,
    manager_network_uldata: Queue,
    _manager_network_dldata: Queue,
    manager_network_dldata_result: Queue,
    public_network_uldata: Queue,
    _public_network_dldata: Queue,
    public_network_dldata_result: Queue,
    owner_network_uldata: Queue,
    _owner_network_dldata: Queue,
    _owner_network_dldata_result: Queue,

    manager_app_handler: TestAppHandler,
    owner_app_handler: TestAppHandler,

    manager_net_handler: TestNetHandler,
    public_net_handler: TestNetHandler,
}

#[derive(Clone)]
struct TestAppHandler {
    // Use Mutex to implement interior mutability.
    recv_uldata: Arc<Mutex<Vec<Box<AppUlData>>>>,
    recv_dldata_resp: Arc<Mutex<Vec<Box<AppDlDataResp>>>>,
    recv_dldata_result: Arc<Mutex<Vec<Box<AppDlDataResult>>>>,
}

#[derive(Clone)]
struct TestNetHandler {
    // Use Mutex to implement interior mutability.
    recv_dldata: Arc<Mutex<Vec<Box<NetDlData>>>>,
}

const OWNER_UNIT_ID: &'static str = "OWNER_UNIT_ID";
const OWNER_APP_ID: &'static str = "OWNER_APP_ID";
const OWNER_NET_ID: &'static str = "OWNER_NET_ID";
const OWNER_DEV2_ID: &'static str = "OWNER_DEV2_ID";

impl TestAppHandler {
    fn new() -> Self {
        TestAppHandler {
            recv_uldata: Arc::new(Mutex::new(vec![])),
            recv_dldata_resp: Arc::new(Mutex::new(vec![])),
            recv_dldata_result: Arc::new(Mutex::new(vec![])),
        }
    }
}

#[async_trait]
impl EventHandler for TestAppHandler {
    async fn on_event(&self, _queue: Arc<dyn GmqQueue>, _ev: Event) {}

    async fn on_message(&self, queue: Arc<dyn GmqQueue>, msg: Box<dyn Message>) {
        let name = queue.name();
        if name.ends_with(".uldata") {
            let data = match serde_json::from_slice::<AppUlData>(msg.payload()) {
                Err(_) => return,
                Ok(data) => Box::new(data),
            };
            self.recv_uldata.lock().unwrap().push(data);
        } else if name.ends_with(".dldata-resp") {
            let data = match serde_json::from_slice::<AppDlDataResp>(msg.payload()) {
                Err(_) => return,
                Ok(data) => Box::new(data),
            };
            self.recv_dldata_resp.lock().unwrap().push(data);
        } else if name.ends_with(".dldata-result") {
            let data = match serde_json::from_slice::<AppDlDataResult>(msg.payload()) {
                Err(_) => return,
                Ok(data) => Box::new(data),
            };
            self.recv_dldata_result.lock().unwrap().push(data);
        }
        let _ = msg.ack().await;
    }
}

impl TestNetHandler {
    fn new() -> Self {
        TestNetHandler {
            recv_dldata: Arc::new(Mutex::new(vec![])),
        }
    }
}

#[async_trait]
impl EventHandler for TestNetHandler {
    async fn on_event(&self, _queue: Arc<dyn GmqQueue>, _ev: Event) {}

    async fn on_message(&self, _queue: Arc<dyn GmqQueue>, msg: Box<dyn Message>) {
        let data = match serde_json::from_slice::<NetDlData>(msg.payload()) {
            Err(_) => return,
            Ok(data) => Box::new(data),
        };
        self.recv_dldata.lock().unwrap().push(data);
        let _ = msg.ack().await;
    }
}

/// Create the following resources for testing device/network routing:
/// - 2 units: manager, owner
/// - 3 networks: 1 amqp-manager, 1 amqp-public, 1 mqtt-owner
/// - 2 applications: 1 amqp-manager, 1 mqtt-owner
/// - 5 devices: 1 manager, 1 public-manager, 1 public-owner, 2 owner
/// - 4 routes: amqp-manager->manager, public->manager, public->owner, owner1->owner
pub fn before_all_fn(state: &mut HashMap<&'static str, TestState>) -> () {
    let state = state.get_mut(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();
    let routing_values = state.routing_values.as_mut().unwrap();

    let mut unit = unit::request::PostUnit {
        data: unit::request::PostUnitData {
            code: "manager".to_string(),
            owner_id: None,
            name: None,
            info: None,
        },
    };
    let manager_unit_id = match libs::create_unit(runtime, routes_state, TOKEN_MANAGER, &unit) {
        Err(e) => {
            panic!("create manager unit error: {}", e);
        }
        Ok(unit_id) => unit_id,
    };
    unit.data.code = "owner".to_string();
    let owner_unit_id = match libs::create_unit(runtime, routes_state, TOKEN_OWNER, &unit) {
        Err(e) => {
            panic!("create owner unit error: {}", e);
        }
        Ok(unit_id) => unit_id,
    };
    routing_values.insert(OWNER_UNIT_ID.to_string(), owner_unit_id.clone());
    let mut application = application::request::PostApplication {
        data: application::request::PostApplicationData {
            code: "manager".to_string(),
            unit_id: manager_unit_id.clone(),
            host_uri: "amqp://localhost".to_string(),
            name: None,
            info: None,
        },
    };
    let manager_application_id =
        match libs::create_application(runtime, routes_state, TOKEN_MANAGER, &application) {
            Err(e) => {
                panic!("create manager application error: {}", e);
            }
            Ok(application_id) => application_id,
        };
    application.data.code = "owner".to_string();
    application.data.unit_id = owner_unit_id.clone();
    application.data.host_uri = "mqtt://localhost".to_string();
    let owner_application_id =
        match libs::create_application(runtime, routes_state, TOKEN_OWNER, &application) {
            Err(e) => {
                panic!("create owner application error: {}", e);
            }
            Ok(application_id) => application_id,
        };
    routing_values.insert(OWNER_APP_ID.to_string(), owner_application_id.clone());
    let mut network = network::request::PostNetwork {
        data: network::request::PostNetworkData {
            code: "manager".to_string(),
            unit_id: Some(manager_unit_id.clone()),
            host_uri: "amqp://localhost".to_string(),
            name: None,
            info: None,
        },
    };
    let manager_network_id =
        match libs::create_network(runtime, routes_state, TOKEN_MANAGER, &network) {
            Err(e) => {
                panic!("create manager network error: {}", e);
            }
            Ok(network_id) => network_id,
        };
    network.data.code = "public".to_string();
    network.data.unit_id = None;
    let public_network_id =
        match libs::create_network(runtime, routes_state, TOKEN_MANAGER, &network) {
            Err(e) => {
                panic!("create public network error: {}", e);
            }
            Ok(network_id) => network_id,
        };
    network.data.code = "owner".to_string();
    network.data.unit_id = Some(owner_unit_id.clone());
    network.data.host_uri = "mqtt://localhost".to_string();
    let owner_network_id = match libs::create_network(runtime, routes_state, TOKEN_OWNER, &network)
    {
        Err(e) => {
            panic!("create owner network error: {}", e);
        }
        Ok(network_id) => network_id,
    };
    routing_values.insert(OWNER_NET_ID.to_string(), owner_network_id.clone());
    let mut device = device::request::PostDevice {
        data: device::request::PostDeviceData {
            unit_id: manager_unit_id.clone(),
            network_id: manager_network_id.clone(),
            network_addr: "manager".to_string(),
            profile: None,
            name: None,
            info: None,
        },
    };
    let _manager_device_id =
        match libs::create_device(runtime, routes_state, TOKEN_MANAGER, &device) {
            Err(e) => {
                panic!("create manager device error: {}", e);
            }
            Ok(device_id) => device_id,
        };
    device.data.unit_id = manager_unit_id.clone();
    device.data.network_id = public_network_id.clone();
    device.data.network_addr = "public-manager".to_string();
    let public_manager_device_id =
        match libs::create_device(runtime, routes_state, TOKEN_MANAGER, &device) {
            Err(e) => {
                panic!("create public manager device error: {}", e);
            }
            Ok(device_id) => device_id,
        };
    device.data.unit_id = owner_unit_id.clone();
    device.data.network_addr = "public-owner".to_string();
    let public_owner_device_id =
        match libs::create_device(runtime, routes_state, TOKEN_MANAGER, &device) {
            Err(e) => {
                panic!("create public owner device error: {}", e);
            }
            Ok(device_id) => device_id,
        };
    device.data.network_id = owner_network_id.clone();
    device.data.network_addr = "owner1".to_string();
    let owner_device1_id = match libs::create_device(runtime, routes_state, TOKEN_OWNER, &device) {
        Err(e) => {
            panic!("create owner device 1 error: {}", e);
        }
        Ok(device_id) => device_id,
    };
    device.data.network_addr = "owner2".to_string();
    let owner_device2_id = match libs::create_device(runtime, routes_state, TOKEN_OWNER, &device) {
        Err(e) => {
            panic!("create owner device 2 error: {}", e);
        }
        Ok(device_id) => device_id,
    };
    routing_values.insert(OWNER_DEV2_ID.to_string(), owner_device2_id);
    let route = network_route::request::PostNetworkRoute {
        data: network_route::request::PostNetworkRouteData {
            network_id: manager_network_id.clone(),
            application_id: manager_application_id.clone(),
        },
    };
    if let Err(e) = libs::create_network_route(runtime, routes_state, TOKEN_MANAGER, &route) {
        panic!("create manager-manager network route error: {}", e);
    }
    let mut route = device_route::request::PostDeviceRoute {
        data: device_route::request::PostDeviceRouteData {
            device_id: public_manager_device_id.clone(),
            application_id: manager_application_id.clone(),
        },
    };
    if let Err(e) = libs::create_device_route(runtime, routes_state, TOKEN_MANAGER, &route) {
        panic!("create public-manager device route error: {}", e);
    }
    route.data.device_id = public_owner_device_id.clone();
    route.data.application_id = owner_application_id.clone();
    if let Err(e) = libs::create_device_route(runtime, routes_state, TOKEN_OWNER, &route) {
        panic!("create public-owner device route error: {}", e);
    }
    route.data.device_id = owner_device1_id.clone();
    if let Err(e) = libs::create_device_route(runtime, routes_state, TOKEN_OWNER, &route) {
        panic!("create owner1-owner device route error: {}", e);
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
}

/// Clear general-mq relative connections and queues.
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
    if let Some(device_id) = state.routing_device_id.take() {
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

/// Test the following cases:
/// - manager: should receive
/// - public-manager: should receive
/// - public-owner: should receive
/// - owner1-owner: should receive
/// - owner2-owner: should not receive
pub fn uplink(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let mut state = context.state.borrow_mut();
    let state = state.get_mut(STATE).unwrap();

    let resources = create_connections(state)?;
    let rsc = &resources;
    let runtime = state.runtime.as_ref().unwrap();

    runtime.block_on(async move {
        const CASE: &'static str = "case manager";
        let mut ext = Map::<String, Value>::new();
        ext.insert("key".to_string(), Value::String("value".to_string()));
        let data = NetUlData {
            time: strings::time_str(&Utc::now()),
            network_addr: "manager".to_string(),
            data: "01".to_string(),
            extension: Some(ext),
        };
        let payload = match serde_json::to_vec(&data) {
            Err(e) => return Err(format!("{} generate payload error: {}", CASE, e)),
            Ok(payload) => payload,
        };
        if let Err(e) = rsc.manager_network_uldata.send_msg(payload).await {
            return Err(format!("{} send error: {}", CASE, e));
        }
        let mut found = false;
        for _ in 0..WAIT_COUNT {
            if let Some(d) = { rsc.manager_app_handler.recv_uldata.lock().unwrap().pop() } {
                expect(d.time).equals(data.time)?;
                expect(d.network_addr).equals(data.network_addr)?;
                expect(d.extension.as_ref()).equals(data.extension.as_ref())?;
                found = true;
                break;
            }
            time::sleep(Duration::from_millis(WAIT_TICK)).await;
        }
        if !found {
            return Err(format!("{} not receive data", CASE));
        }
        Ok(())
    })?;

    runtime.block_on(async move {
        const CASE: &'static str = "case public-manager";
        let data = NetUlData {
            time: strings::time_str(&Utc::now()),
            network_addr: "public-manager".to_string(),
            data: "01".to_string(),
            extension: None,
        };
        let payload = match serde_json::to_vec(&data) {
            Err(e) => return Err(format!("{} generate payload error: {}", CASE, e)),
            Ok(payload) => payload,
        };
        if let Err(e) = rsc.public_network_uldata.send_msg(payload).await {
            return Err(format!("{} send error: {}", CASE, e));
        }
        let mut found = false;
        for _ in 0..WAIT_COUNT {
            if let Some(d) = { rsc.manager_app_handler.recv_uldata.lock().unwrap().pop() } {
                expect(d.time).equals(data.time)?;
                expect(d.network_addr).equals(data.network_addr)?;
                expect(d.extension.as_ref()).equals(data.extension.as_ref())?;
                found = true;
                break;
            }
            time::sleep(Duration::from_millis(WAIT_TICK)).await;
        }
        if !found {
            return Err(format!("{} not receive data", CASE));
        }
        Ok(())
    })?;

    runtime.block_on(async move {
        const CASE: &'static str = "case public-owner";
        let data = NetUlData {
            time: strings::time_str(&Utc::now()),
            network_addr: "public-owner".to_string(),
            data: "01".to_string(),
            extension: None,
        };
        let payload = match serde_json::to_vec(&data) {
            Err(e) => return Err(format!("{} generate payload error: {}", CASE, e)),
            Ok(payload) => payload,
        };
        if let Err(e) = rsc.public_network_uldata.send_msg(payload).await {
            return Err(format!("{} send error: {}", CASE, e));
        }
        let mut found = false;
        for _ in 0..WAIT_COUNT {
            if let Some(d) = { rsc.owner_app_handler.recv_uldata.lock().unwrap().pop() } {
                expect(d.time).equals(data.time)?;
                expect(d.network_addr).equals(data.network_addr)?;
                expect(d.extension.as_ref()).equals(data.extension.as_ref())?;
                found = true;
                break;
            }
            time::sleep(Duration::from_millis(WAIT_TICK)).await;
        }
        if !found {
            return Err(format!("{} not receive data", CASE));
        }
        Ok(())
    })?;

    runtime.block_on(async move {
        const CASE: &'static str = "case owner1";
        let data = NetUlData {
            time: strings::time_str(&Utc::now()),
            network_addr: "owner1".to_string(),
            data: "01".to_string(),
            extension: None,
        };
        let payload = match serde_json::to_vec(&data) {
            Err(e) => return Err(format!("{} generate payload error: {}", CASE, e)),
            Ok(payload) => payload,
        };
        if let Err(e) = rsc.owner_network_uldata.send_msg(payload).await {
            return Err(format!("{} send error: {}", CASE, e));
        }
        let mut found = false;
        for _ in 0..WAIT_COUNT {
            if let Some(d) = { rsc.owner_app_handler.recv_uldata.lock().unwrap().pop() } {
                expect(d.time).equals(data.time)?;
                expect(d.network_addr).equals(data.network_addr)?;
                expect(d.extension.as_ref()).equals(data.extension.as_ref())?;
                found = true;
                break;
            }
            time::sleep(Duration::from_millis(WAIT_TICK)).await;
        }
        if !found {
            return Err(format!("{} not receive data", CASE));
        }
        Ok(())
    })?;

    runtime.block_on(async move {
        const CASE: &'static str = "case owner2";
        let data = NetUlData {
            time: strings::time_str(&Utc::now()),
            network_addr: "owner2".to_string(),
            data: "01".to_string(),
            extension: None,
        };
        let payload = match serde_json::to_vec(&data) {
            Err(e) => return Err(format!("{} generate payload error: {}", CASE, e)),
            Ok(payload) => payload,
        };
        if let Err(e) = rsc.owner_network_uldata.send_msg(payload).await {
            return Err(format!("{} send error: {}", CASE, e));
        }
        let mut found = false;
        for _ in 0..WAIT_COUNT {
            if let Some(_) = { rsc.owner_app_handler.recv_uldata.lock().unwrap().pop() } {
                found = true;
                break;
            }
            time::sleep(Duration::from_millis(WAIT_TICK)).await;
        }
        if found {
            return Err(format!("{} should not receive data", CASE));
        }
        Ok(())
    })?;

    runtime.block_on(async move {
        const CASE: &'static str = "case not-exist";
        let data = NetUlData {
            time: strings::time_str(&Utc::now()),
            network_addr: "not-exist".to_string(),
            data: "01".to_string(),
            extension: None,
        };
        let payload = match serde_json::to_vec(&data) {
            Err(e) => return Err(format!("{} generate payload error: {}", CASE, e)),
            Ok(payload) => payload,
        };
        if let Err(e) = rsc.manager_network_uldata.send_msg(payload).await {
            return Err(format!("{} send error: {}", CASE, e));
        }
        let mut found = false;
        for _ in 0..WAIT_COUNT {
            if let Some(_) = { rsc.manager_app_handler.recv_uldata.lock().unwrap().pop() } {
                found = true;
                break;
            }
            time::sleep(Duration::from_millis(WAIT_TICK)).await;
        }
        if found {
            return Err(format!("{} should not receive data", CASE));
        }
        Ok(())
    })?;
    Ok(())
}

/// Test the following cases:
/// - manager
/// - public-owner
pub fn downlink(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let mut state = context.state.borrow_mut();
    let state = state.get_mut(STATE).unwrap();

    let resources = create_connections(state)?;
    let rsc = &resources;
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    // 1. app send dldata
    // 2. app should receive resp, net should receive dldata
    // 3. net send result status < 0, app should receive result
    // 4. net send result status == 0, app should receive result
    runtime.block_on(async move {
        const CASE: &'static str = "case manager";

        let start_time = Utc::now().timestamp_millis();

        // Step 1.
        let mut ext = Map::<String, Value>::new();
        ext.insert("key".to_string(), Value::String("value".to_string()));
        let data = AppDlData {
            correlation_id: "1".to_string(),
            network_code: Some("manager".to_string()),
            network_addr: Some("manager".to_string()),
            data: "01".to_string(),
            extension: Some(ext),
            ..Default::default()
        };
        let payload = match serde_json::to_vec(&data) {
            Err(e) => return Err(format!("{}.1 generate payload error: {}", CASE, e)),
            Ok(payload) => payload,
        };
        if let Err(e) = rsc.manager_application_dldata.send_msg(payload).await {
            return Err(format!("{}.1 send error: {}", CASE, e));
        }

        // Step 2.
        let mut recv_dldata = None;
        let mut recv_dldata_resp = None;
        let mut end_time = 0;
        for _ in 0..WAIT_COUNT {
            if recv_dldata_resp.is_none() {
                let result = {
                    let mut mutex = rsc.manager_app_handler.recv_dldata_resp.lock().unwrap();
                    mutex.pop()
                };
                if let Some(data) = result {
                    recv_dldata_resp = Some(*data);
                }
            }
            if recv_dldata.is_none() {
                let result = {
                    let mut mutex = rsc.manager_net_handler.recv_dldata.lock().unwrap();
                    mutex.pop()
                };
                if let Some(data) = result {
                    end_time = Utc::now().timestamp_millis();
                    recv_dldata = Some(*data);
                }
            }
            if recv_dldata.is_some() && recv_dldata_resp.is_some() {
                break;
            }
            time::sleep(Duration::from_millis(WAIT_TICK)).await;
        }
        if recv_dldata.is_none() || recv_dldata_resp.is_none() {
            return Err(format!("{}.2 should receive data", CASE));
        }
        let recv_dldata = recv_dldata.unwrap();
        let recv_dldata_resp = recv_dldata_resp.unwrap();
        expect(recv_dldata_resp.correlation_id.as_str()).equals(data.correlation_id.as_str())?;
        expect(recv_dldata_resp.data_id.is_some()).equals(true)?;
        expect(recv_dldata_resp.error.is_none()).equals(true)?;
        expect(recv_dldata_resp.message.is_none()).equals(true)?;
        expect(recv_dldata.data_id.as_str())
            .equals(recv_dldata_resp.data_id.as_ref().unwrap().as_str())?;
        let pub_time = match DateTime::parse_from_rfc3339(recv_dldata.publish.as_str()) {
            Err(e) => return Err(format!("{} downlink time error: {}", CASE, e)),
            Ok(t) => t.timestamp_millis(),
        };
        expect(pub_time >= start_time).equals(true)?;
        expect(pub_time <= end_time).equals(true)?;
        expect(recv_dldata.network_addr.as_str())
            .equals(data.network_addr.as_ref().unwrap().as_str())?;
        expect(recv_dldata.extension.as_ref()).equals(data.extension.as_ref())?;

        // Step 3.
        let mut recv_dldata_result = None;
        let result_data1 = NetDlDataResult {
            data_id: recv_dldata.data_id.clone(),
            status: -1,
            message: None,
        };
        let payload = match serde_json::to_vec(&result_data1) {
            Err(e) => return Err(format!("{}.3 generate payload error: {}", CASE, e)),
            Ok(payload) => payload,
        };
        if let Err(e) = rsc.manager_network_dldata_result.send_msg(payload).await {
            return Err(format!("{}.3 send error: {}", CASE, e));
        }
        for _ in 0..WAIT_COUNT {
            let result = {
                let mut mutex = rsc.manager_app_handler.recv_dldata_result.lock().unwrap();
                mutex.pop()
            };
            if let Some(data) = result {
                recv_dldata_result = Some(*data);
                break;
            }
            time::sleep(Duration::from_millis(WAIT_TICK)).await;
        }
        if recv_dldata_result.is_none() {
            return Err(format!("{}.3 should receive data", CASE));
        }
        let recv_dldata_result = recv_dldata_result.unwrap();
        expect(recv_dldata_result.data_id.as_str())
            .equals(recv_dldata_resp.data_id.as_ref().unwrap().as_str())?;
        expect(recv_dldata_result.status).equals(result_data1.status)?;

        // Step 4.
        let mut recv_dldata_result = None;
        let result_data2 = NetDlDataResult {
            data_id: recv_dldata.data_id.clone(),
            status: 0,
            message: None,
        };
        let payload = match serde_json::to_vec(&result_data2) {
            Err(e) => return Err(format!("{}.4 generate payload error: {}", CASE, e)),
            Ok(payload) => payload,
        };
        if let Err(e) = rsc.manager_network_dldata_result.send_msg(payload).await {
            return Err(format!("{}.4 send error: {}", CASE, e));
        }
        for _ in 0..WAIT_COUNT {
            let result = {
                let mut mutex = rsc.manager_app_handler.recv_dldata_result.lock().unwrap();
                mutex.pop()
            };
            if let Some(data) = result {
                recv_dldata_result = Some(*data);
                break;
            }
            time::sleep(Duration::from_millis(WAIT_TICK)).await;
        }
        if recv_dldata_result.is_none() {
            return Err(format!("{}.4 should receive data", CASE));
        }
        let recv_dldata_result = recv_dldata_result.unwrap();
        expect(recv_dldata_result.data_id.as_str())
            .equals(recv_dldata_resp.data_id.as_ref().unwrap().as_str())?;
        expect(recv_dldata_result.status).equals(result_data2.status)?;

        Ok(())
    })?;

    // 1. app send dldata
    // 2. app should receive resp, net should receive dldata
    // 3. net send result status == 0, app should receive result
    runtime.block_on(async move {
        const CASE: &'static str = "case public";

        // Get device ID.
        let cond = QueryCond {
            device: Some(QueryOneCond {
                unit_code: None,
                network_code: "public",
                network_addr: "public-owner",
            }),
            ..Default::default()
        };
        let (device_id, network_addr) = match routes_state.model.device().get(&cond).await {
            Err(e) => return Err(format!("{}.0 get device error: {}", CASE, e)),
            Ok(device) => match device {
                None => return Err(format!("{}.0 get no device", CASE)),
                Some(device) => (device.device_id, device.network_addr),
            },
        };

        let start_time = Utc::now().timestamp_millis();

        // Step 1.
        let mut ext = Map::<String, Value>::new();
        ext.insert("key".to_string(), Value::String("value".to_string()));
        let data = AppDlData {
            correlation_id: "2".to_string(),
            device_id: Some(device_id.clone()),
            data: "02".to_string(),
            extension: Some(ext),
            ..Default::default()
        };
        let payload = match serde_json::to_vec(&data) {
            Err(e) => return Err(format!("{}.1 generate payload error: {}", CASE, e)),
            Ok(payload) => payload,
        };
        if let Err(e) = rsc.owner_application_dldata.send_msg(payload).await {
            return Err(format!("{}.1 send error: {}", CASE, e));
        }

        // Step 2.
        let mut recv_dldata = None;
        let mut recv_dldata_resp = None;
        let mut end_time = 0;
        for _ in 0..WAIT_COUNT {
            if recv_dldata_resp.is_none() {
                let result = {
                    let mut mutex = rsc.owner_app_handler.recv_dldata_resp.lock().unwrap();
                    mutex.pop()
                };
                if let Some(data) = result {
                    recv_dldata_resp = Some(*data);
                }
            }
            if recv_dldata.is_none() {
                let result = {
                    let mut mutex = rsc.public_net_handler.recv_dldata.lock().unwrap();
                    mutex.pop()
                };
                if let Some(data) = result {
                    end_time = Utc::now().timestamp_millis();
                    recv_dldata = Some(*data);
                }
            }
            if recv_dldata.is_some() && recv_dldata_resp.is_some() {
                break;
            }
            time::sleep(Duration::from_millis(WAIT_TICK)).await;
        }
        if recv_dldata.is_none() || recv_dldata_resp.is_none() {
            return Err(format!("{}.2 should receive data", CASE));
        }
        let recv_dldata = recv_dldata.unwrap();
        let recv_dldata_resp = recv_dldata_resp.unwrap();
        expect(recv_dldata_resp.correlation_id.as_str()).equals(data.correlation_id.as_str())?;
        expect(recv_dldata_resp.data_id.is_some()).equals(true)?;
        expect(recv_dldata_resp.error.is_none()).equals(true)?;
        expect(recv_dldata_resp.message.is_none()).equals(true)?;
        expect(recv_dldata.data_id.as_str())
            .equals(recv_dldata_resp.data_id.as_ref().unwrap().as_str())?;
        let pub_time = match DateTime::parse_from_rfc3339(recv_dldata.publish.as_str()) {
            Err(e) => return Err(format!("{} downlink time error: {}", CASE, e)),
            Ok(t) => t.timestamp_millis(),
        };
        expect(pub_time >= start_time).equals(true)?;
        expect(pub_time <= end_time).equals(true)?;
        expect(recv_dldata.network_addr.as_str()).equals(network_addr.as_str())?;
        expect(recv_dldata.extension.as_ref()).equals(data.extension.as_ref())?;

        // Step 3.
        let mut recv_dldata_result = None;
        let result_data1 = NetDlDataResult {
            data_id: recv_dldata.data_id.clone(),
            status: -1,
            message: None,
        };
        let payload = match serde_json::to_vec(&result_data1) {
            Err(e) => return Err(format!("{}.3 generate payload error: {}", CASE, e)),
            Ok(payload) => payload,
        };
        if let Err(e) = rsc.public_network_dldata_result.send_msg(payload).await {
            return Err(format!("{}.3 send error: {}", CASE, e));
        }
        for _ in 0..WAIT_COUNT {
            let result = {
                let mut mutex = rsc.owner_app_handler.recv_dldata_result.lock().unwrap();
                mutex.pop()
            };
            if let Some(data) = result {
                recv_dldata_result = Some(*data);
                break;
            }
            time::sleep(Duration::from_millis(WAIT_TICK)).await;
        }
        if recv_dldata_result.is_none() {
            return Err(format!("{}.3 should receive data", CASE));
        }
        let recv_dldata_result = recv_dldata_result.unwrap();
        expect(recv_dldata_result.data_id.as_str())
            .equals(recv_dldata_resp.data_id.as_ref().unwrap().as_str())?;
        expect(recv_dldata_result.status).equals(result_data1.status)?;

        // Step 4.
        let mut recv_dldata_result = None;
        let result_data2 = NetDlDataResult {
            data_id: recv_dldata.data_id.clone(),
            status: 0,
            message: None,
        };
        let payload = match serde_json::to_vec(&result_data2) {
            Err(e) => return Err(format!("{}.4 generate payload error: {}", CASE, e)),
            Ok(payload) => payload,
        };
        if let Err(e) = rsc.public_network_dldata_result.send_msg(payload).await {
            return Err(format!("{}.4 send error: {}", CASE, e));
        }
        for _ in 0..WAIT_COUNT {
            let result = {
                let mut mutex = rsc.owner_app_handler.recv_dldata_result.lock().unwrap();
                mutex.pop()
            };
            if let Some(data) = result {
                recv_dldata_result = Some(*data);
                break;
            }
            time::sleep(Duration::from_millis(WAIT_TICK)).await;
        }
        if recv_dldata_result.is_none() {
            return Err(format!("{}.4 should receive data", CASE));
        }
        let recv_dldata_result = recv_dldata_result.unwrap();
        expect(recv_dldata_result.data_id.as_str())
            .equals(recv_dldata_resp.data_id.as_ref().unwrap().as_str())?;
        expect(recv_dldata_result.status).equals(result_data2.status)?;

        Ok(())
    })?;

    Ok(())
}

/// Test the following cases:
/// - device not exists
pub fn downlink_not_exist(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let mut state = context.state.borrow_mut();
    let state = state.get_mut(STATE).unwrap();

    let resources = create_connections(state)?;
    let rsc = &resources;
    let runtime = state.runtime.as_ref().unwrap();

    runtime.block_on(async move {
        const CASE: &'static str = "case not-exist";

        // Step 1.
        let mut ext = Map::<String, Value>::new();
        ext.insert("key".to_string(), Value::String("value".to_string()));
        let data = AppDlData {
            correlation_id: "1".to_string(),
            network_code: Some("owner".to_string()),
            network_addr: Some("owner1".to_string()),
            data: "01".to_string(),
            extension: Some(ext),
            ..Default::default()
        };
        let payload = match serde_json::to_vec(&data) {
            Err(e) => return Err(format!("{}.1 generate payload error: {}", CASE, e)),
            Ok(payload) => payload,
        };
        if let Err(e) = rsc.manager_application_dldata.send_msg(payload).await {
            return Err(format!("{}.1 send error: {}", CASE, e));
        }

        // Step 2.
        let mut recv_dldata_resp = None;
        for _ in 0..WAIT_COUNT {
            if recv_dldata_resp.is_none() {
                let result = {
                    let mut mutex = rsc.manager_app_handler.recv_dldata_resp.lock().unwrap();
                    mutex.pop()
                };
                if let Some(data) = result {
                    recv_dldata_resp = Some(*data);
                }
            }
            if recv_dldata_resp.is_some() {
                break;
            }
            time::sleep(Duration::from_millis(WAIT_TICK)).await;
        }
        if recv_dldata_resp.is_none() {
            return Err(format!("{}.2 should receive data", CASE));
        }
        let recv_dldata_resp = recv_dldata_resp.unwrap();
        expect(recv_dldata_resp.correlation_id.as_str()).equals(data.correlation_id.as_str())?;
        expect(recv_dldata_resp.data_id.is_none()).equals(true)?;
        expect(recv_dldata_resp.error.is_some()).equals(true)?;
        expect(recv_dldata_resp.error.as_ref().unwrap().as_str()).equals(ErrReq::DEVICE_NOT_EXIST.1)
    })?;

    Ok(())
}

/// Run the following steps:
/// - confirm no route and clear cache.
/// - send data twice and check that data will not be received.
/// - add routes (use the device "owner2", see [`before_all_fn`] description).
/// - send data twice and check that data will be received.
/// - delete routes.
/// - send data twice and check that data will not be received.
pub fn uplink_route_on_off(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let mut state = context.state.borrow_mut();
    let state = state.get_mut(STATE).unwrap();

    let resources = create_connections(state)?;
    let rsc = &resources;
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();
    let routing_values = state.routing_values.as_ref().unwrap();

    // Send with no routes.
    runtime.block_on(async move {
        const CASE: &'static str = "case no route";
        let mut data = NetUlData {
            time: strings::time_str(&Utc::now()),
            network_addr: "owner2".to_string(),
            data: "01".to_string(),
            extension: None,
        };
        let payload = match serde_json::to_vec(&data) {
            Err(e) => return Err(format!("{} generate payload 01 error: {}", CASE, e)),
            Ok(payload) => payload,
        };
        if let Err(e) = rsc.owner_network_uldata.send_msg(payload).await {
            return Err(format!("{} send error: {}", CASE, e));
        }
        time::sleep(Duration::from_secs(1)).await;
        data.data = "02".to_string();
        let payload = match serde_json::to_vec(&data) {
            Err(e) => return Err(format!("{} generate payload 02 error: {}", CASE, e)),
            Ok(payload) => payload,
        };
        if let Err(e) = rsc.owner_network_uldata.send_msg(payload).await {
            return Err(format!("{} send twice error: {}", CASE, e));
        }
        for _ in 0..WAIT_COUNT {
            if let Some(_) = { rsc.manager_app_handler.recv_uldata.lock().unwrap().pop() } {
                return Err(format!("{} should not receive data", CASE));
            }
            time::sleep(Duration::from_millis(WAIT_TICK)).await;
        }
        Ok(())
    })?;

    // Add route and send data.
    let route = device_route::request::PostDeviceRoute {
        data: device_route::request::PostDeviceRouteData {
            device_id: routing_values.get(OWNER_DEV2_ID).unwrap().clone(),
            application_id: routing_values.get(OWNER_APP_ID).unwrap().clone(),
        },
    };
    let route_id = libs::create_device_route(runtime, routes_state, TOKEN_MANAGER, &route)?;
    runtime.block_on(async move {
        const CASE: &'static str = "case route";

        let mut data = NetUlData {
            time: strings::time_str(&Utc::now()),
            network_addr: "owner2".to_string(),
            data: "03".to_string(),
            extension: None,
        };
        let payload = match serde_json::to_vec(&data) {
            Err(e) => return Err(format!("{} generate payload 03 error: {}", CASE, e)),
            Ok(payload) => payload,
        };
        if let Err(e) = rsc.owner_network_uldata.send_msg(payload).await {
            return Err(format!("{} send error: {}", CASE, e));
        }
        let mut found = false;
        for _ in 0..WAIT_COUNT {
            if let Some(d) = { rsc.owner_app_handler.recv_uldata.lock().unwrap().pop() } {
                expect(d.data.as_str()).to_equal("03")?;
                found = true;
                break;
            }
            time::sleep(Duration::from_millis(WAIT_TICK)).await;
        }
        if !found {
            return Err(format!("{} should receive data 03", CASE));
        }
        data.data = "04".to_string();
        let payload = match serde_json::to_vec(&data) {
            Err(e) => return Err(format!("{} generate payload 04 error: {}", CASE, e)),
            Ok(payload) => payload,
        };
        if let Err(e) = rsc.owner_network_uldata.send_msg(payload).await {
            return Err(format!("{} send twice error: {}", CASE, e));
        }
        found = false;
        for _ in 0..WAIT_COUNT {
            if let Some(d) = { rsc.owner_app_handler.recv_uldata.lock().unwrap().pop() } {
                expect(d.data.as_str()).to_equal("04")?;
                found = true;
                break;
            }
            time::sleep(Duration::from_millis(WAIT_TICK)).await;
        }
        if !found {
            return Err(format!("{} should receive data 04", CASE));
        }
        Ok(())
    })?;

    // Delete route and send data.
    libs::delete_device_route(runtime, routes_state, TOKEN_MANAGER, route_id.as_str())?;
    runtime.block_on(async move {
        const CASE: &'static str = "case delete route";

        let mut data = NetUlData {
            time: strings::time_str(&Utc::now()),
            network_addr: "owner2".to_string(),
            data: "05".to_string(),
            extension: None,
        };
        let payload = match serde_json::to_vec(&data) {
            Err(e) => return Err(format!("{} generate payload 05 error: {}", CASE, e)),
            Ok(payload) => payload,
        };
        if let Err(e) = rsc.owner_network_uldata.send_msg(payload).await {
            return Err(format!("{} send error: {}", CASE, e));
        }
        time::sleep(Duration::from_secs(1)).await;
        data.data = "06".to_string();
        let payload = match serde_json::to_vec(&data) {
            Err(e) => return Err(format!("{} generate payload 06 error: {}", CASE, e)),
            Ok(payload) => payload,
        };
        if let Err(e) = rsc.owner_network_uldata.send_msg(payload).await {
            return Err(format!("{} send twice error: {}", CASE, e));
        }
        for _ in 0..WAIT_COUNT {
            if let Some(_) = { rsc.manager_app_handler.recv_uldata.lock().unwrap().pop() } {
                return Err(format!("{} should not receive data", CASE));
            }
            time::sleep(Duration::from_millis(WAIT_TICK)).await;
        }
        Ok(())
    })?;

    Ok(())
}

/// Run the following steps:
/// - add a device and routes.
/// - send data and check the profile in data.
/// - change device profile.
/// - send data and check where the profile is changed.
pub fn uplink_route_profile(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let mut state = context.state.borrow_mut();
    let state = state.get_mut(STATE).unwrap();
    const ADDR: &'static str = "routing-addr";

    let resources = create_connections(state)?;
    let rsc = &resources;
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();
    let routing_values = state.routing_values.as_mut().unwrap();

    // Add a device for testing changing the device profile.
    let device = device::request::PostDevice {
        data: device::request::PostDeviceData {
            unit_id: routing_values.get(OWNER_UNIT_ID).unwrap().clone(),
            network_id: routing_values.get(OWNER_NET_ID).unwrap().clone(),
            network_addr: ADDR.to_string(),
            profile: None,
            name: None,
            info: None,
        },
    };
    let device_id = libs::create_device(runtime, routes_state, TOKEN_OWNER, &device)?;
    state.routing_device_id = Some(device_id.clone());

    // Add route and send data.
    let route = device_route::request::PostDeviceRoute {
        data: device_route::request::PostDeviceRouteData {
            device_id: device_id.clone(),
            application_id: routing_values.get(OWNER_APP_ID).unwrap().clone(),
        },
    };
    let _ = libs::create_device_route(runtime, routes_state, TOKEN_MANAGER, &route)?;
    runtime.block_on(async move {
        const CASE: &'static str = "case route";

        let data = NetUlData {
            time: strings::time_str(&Utc::now()),
            network_addr: ADDR.to_string(),
            data: "05".to_string(),
            extension: None,
        };
        let payload = match serde_json::to_vec(&data) {
            Err(e) => return Err(format!("{} generate payload 05 error: {}", CASE, e)),
            Ok(payload) => payload,
        };
        if let Err(e) = rsc.owner_network_uldata.send_msg(payload).await {
            return Err(format!("{} send error: {}", CASE, e));
        }
        let mut found = false;
        for _ in 0..WAIT_COUNT {
            if let Some(d) = { rsc.owner_app_handler.recv_uldata.lock().unwrap().pop() } {
                expect(d.profile.as_str()).to_equal("")?;
                expect(d.data.as_str()).to_equal("05")?;
                found = true;
                break;
            }
            time::sleep(Duration::from_millis(WAIT_TICK)).await;
        }
        if !found {
            return Err(format!("{} should receive data 05", CASE));
        }
        Ok(())
    })?;

    // Patch device profile and send another data.
    let updates = device::request::PatchDevice {
        data: device::request::PatchDeviceData {
            profile: Some("profile-update".to_string()),
            ..Default::default()
        },
    };
    let device_id = device_id.as_str();
    let _ = libs::patch_device(runtime, routes_state, TOKEN_MANAGER, device_id, &updates)?;
    runtime.block_on(async move {
        const CASE: &'static str = "case route";

        // Wait for control channel updating cache.
        time::sleep(Duration::from_secs(1)).await;

        let data = NetUlData {
            time: strings::time_str(&Utc::now()),
            network_addr: ADDR.to_string(),
            data: "06".to_string(),
            extension: None,
        };
        let payload = match serde_json::to_vec(&data) {
            Err(e) => return Err(format!("{} generate payload 06 error: {}", CASE, e)),
            Ok(payload) => payload,
        };
        if let Err(e) = rsc.owner_network_uldata.send_msg(payload).await {
            return Err(format!("{} send error: {}", CASE, e));
        }
        let mut found = false;
        for _ in 0..WAIT_COUNT {
            if let Some(d) = { rsc.owner_app_handler.recv_uldata.lock().unwrap().pop() } {
                expect(d.profile.as_str()).to_equal("profile-update")?;
                expect(d.data.as_str()).to_equal("06")?;
                found = true;
                break;
            }
            time::sleep(Duration::from_millis(WAIT_TICK)).await;
        }
        if !found {
            return Err(format!("{} should receive data 06", CASE));
        }
        Ok(())
    })?;

    Ok(())
}

/// Run the following steps:
/// - add a device and routes.
/// - send data and confirm the data is received.
/// - change device address.
/// - send data with originial address and check that data should not be received.
/// - send data with new address and check that data can be received.
pub fn uplink_route_change_addr(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let mut state = context.state.borrow_mut();
    let state = state.get_mut(STATE).unwrap();
    const ADDR: &'static str = "routing-addr";
    const ADDR2: &'static str = "routing-addr2";

    let resources = create_connections(state)?;
    let rsc = &resources;
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();
    let routing_values = state.routing_values.as_mut().unwrap();

    // Add a device for testing changing the device profile.
    let device = device::request::PostDevice {
        data: device::request::PostDeviceData {
            unit_id: routing_values.get(OWNER_UNIT_ID).unwrap().clone(),
            network_id: routing_values.get(OWNER_NET_ID).unwrap().clone(),
            network_addr: ADDR.to_string(),
            profile: None,
            name: None,
            info: None,
        },
    };
    let device_id = libs::create_device(runtime, routes_state, TOKEN_OWNER, &device)?;
    state.routing_device_id = Some(device_id.clone());

    // Add route and send data.
    let route = device_route::request::PostDeviceRoute {
        data: device_route::request::PostDeviceRouteData {
            device_id: device_id.clone(),
            application_id: routing_values.get(OWNER_APP_ID).unwrap().clone(),
        },
    };
    let _ = libs::create_device_route(runtime, routes_state, TOKEN_MANAGER, &route)?;
    runtime.block_on(async move {
        const CASE: &'static str = "case route";

        let data = NetUlData {
            time: strings::time_str(&Utc::now()),
            network_addr: ADDR.to_string(),
            data: "07".to_string(),
            extension: None,
        };
        let payload = match serde_json::to_vec(&data) {
            Err(e) => return Err(format!("{} generate payload 07 error: {}", CASE, e)),
            Ok(payload) => payload,
        };
        if let Err(e) = rsc.owner_network_uldata.send_msg(payload).await {
            return Err(format!("{} send error: {}", CASE, e));
        }
        let mut found = false;
        for _ in 0..WAIT_COUNT {
            if let Some(d) = { rsc.owner_app_handler.recv_uldata.lock().unwrap().pop() } {
                expect(d.data.as_str()).to_equal("07")?;
                found = true;
                break;
            }
            time::sleep(Duration::from_millis(WAIT_TICK)).await;
        }
        if !found {
            return Err(format!("{} should receive data 07", CASE));
        }
        Ok(())
    })?;

    // Patch device address and send another data.
    let updates = device::request::PatchDevice {
        data: device::request::PatchDeviceData {
            network_addr: Some(ADDR2.to_string()),
            ..Default::default()
        },
    };
    let device_id = device_id.as_str();
    let _ = libs::patch_device(runtime, routes_state, TOKEN_MANAGER, device_id, &updates)?;
    runtime.block_on(async move {
        const CASE: &'static str = "case route";

        // Wait for control channel updating cache.
        time::sleep(Duration::from_secs(1)).await;

        let data = NetUlData {
            time: strings::time_str(&Utc::now()),
            network_addr: ADDR.to_string(),
            data: "08".to_string(),
            extension: None,
        };
        let payload = match serde_json::to_vec(&data) {
            Err(e) => return Err(format!("{} generate payload 08 error: {}", CASE, e)),
            Ok(payload) => payload,
        };
        if let Err(e) = rsc.owner_network_uldata.send_msg(payload).await {
            return Err(format!("{} send error: {}", CASE, e));
        }
        let mut found = false;
        for _ in 0..WAIT_COUNT {
            if let Some(d) = { rsc.owner_app_handler.recv_uldata.lock().unwrap().pop() } {
                expect(d.data.as_str()).to_equal("08")?;
                found = true;
                break;
            }
            time::sleep(Duration::from_millis(WAIT_TICK)).await;
        }
        if found {
            return Err(format!("{} should not receive data 08", CASE));
        }

        let data = NetUlData {
            time: strings::time_str(&Utc::now()),
            network_addr: ADDR2.to_string(),
            data: "09".to_string(),
            extension: None,
        };
        let payload = match serde_json::to_vec(&data) {
            Err(e) => return Err(format!("{} generate payload 09 error: {}", CASE, e)),
            Ok(payload) => payload,
        };
        if let Err(e) = rsc.owner_network_uldata.send_msg(payload).await {
            return Err(format!("{} send error: {}", CASE, e));
        }
        let mut found = false;
        for _ in 0..WAIT_COUNT {
            if let Some(d) = { rsc.owner_app_handler.recv_uldata.lock().unwrap().pop() } {
                expect(d.network_addr.as_str()).to_equal(ADDR2)?;
                expect(d.data.as_str()).to_equal("09")?;
                found = true;
                break;
            }
            time::sleep(Duration::from_millis(WAIT_TICK)).await;
        }
        if !found {
            return Err(format!("{} should receive data 09", CASE));
        }
        Ok(())
    })?;

    Ok(())
}

/// Create connections, queues, handlers for routing test cases.
fn create_connections(state: &mut TestState) -> Result<TestResources, String> {
    state.routing_conns = Some(vec![]);
    let mut amqp_conn = AmqpConnection::new(AmqpConnectionOptions {
        uri: "amqp://localhost".to_string(),
        ..Default::default()
    })?;
    if let Err(e) = amqp_conn.connect() {
        return Err(format!("new AMQP connection error: {}", e));
    }
    state
        .routing_conns
        .as_mut()
        .unwrap()
        .push(Box::new(amqp_conn.clone()));
    let mut mqtt_conn = MqttConnection::new(MqttConnectionOptions {
        uri: "mqtt://localhost".to_string(),
        ..Default::default()
    })?;
    if let Err(e) = mqtt_conn.connect() {
        return Err(format!("new MQTT connection error: {}", e));
    }
    state
        .routing_conns
        .as_mut()
        .unwrap()
        .push(Box::new(mqtt_conn.clone()));

    let amqp_conn = Connection::Amqp(amqp_conn, Arc::new(Mutex::new(0)));
    let mqtt_conn = Connection::Mqtt(mqtt_conn, Arc::new(Mutex::new(0)));

    state.routing_queues = Some(vec![]);

    let opts = Options {
        unit_id: "manager".to_string(),
        unit_code: "manager".to_string(),
        id: "manager".to_string(),
        name: "manager".to_string(),
        shared_prefix: Some("$share/sylvia-iot-broker/".to_string()),
        ..Default::default()
    };
    let (
        mut manager_application_uldata,
        mut manager_application_dldata,
        manager_application_dldata_resp,
        mut manager_application_dldata_result,
    ) = new_data_queues(state, &amqp_conn, &opts, "broker.application", false)?;
    let mut manager_application_dldata_resp = manager_application_dldata_resp.unwrap();
    let manager_app_handler = TestAppHandler::new();
    manager_application_uldata.set_handler(Arc::new(manager_app_handler.clone()));
    manager_application_dldata_resp.set_handler(Arc::new(manager_app_handler.clone()));
    manager_application_dldata_result.set_handler(Arc::new(manager_app_handler.clone()));
    let _ = manager_application_uldata.connect();
    let _ = manager_application_dldata.connect();
    let _ = manager_application_dldata_resp.connect();
    let _ = manager_application_dldata_result.connect();

    let (
        mut manager_network_uldata,
        mut manager_network_dldata,
        _,
        mut manager_network_dldata_result,
    ) = new_data_queues(state, &amqp_conn, &opts, "broker.network", true)?;
    let manager_net_handler = TestNetHandler::new();
    manager_network_dldata.set_handler(Arc::new(manager_net_handler.clone()));
    let _ = manager_network_uldata.connect();
    let _ = manager_network_dldata.connect();
    let _ = manager_network_dldata_result.connect();

    let opts = Options {
        unit_id: "".to_string(),
        unit_code: "".to_string(),
        id: "public".to_string(),
        name: "public".to_string(),
        shared_prefix: Some("$share/sylvia-iot-broker/".to_string()),
        ..Default::default()
    };
    let (mut public_network_uldata, mut public_network_dldata, _, mut public_network_dldata_result) =
        new_data_queues(state, &amqp_conn, &opts, "broker.network", true)?;
    let public_net_handler = TestNetHandler::new();
    public_network_dldata.set_handler(Arc::new(public_net_handler.clone()));
    let _ = public_network_uldata.connect();
    let _ = public_network_dldata.connect();
    let _ = public_network_dldata_result.connect();

    let opts = Options {
        unit_id: "owner".to_string(),
        unit_code: "owner".to_string(),
        id: "owner".to_string(),
        name: "owner".to_string(),
        shared_prefix: Some("$share/sylvia-iot-broker/".to_string()),
        ..Default::default()
    };
    let (
        mut owner_application_uldata,
        mut owner_application_dldata,
        owner_application_dldata_resp,
        mut owner_application_dldata_result,
    ) = new_data_queues(state, &mqtt_conn, &opts, "broker.application", false)?;
    let mut owner_application_dldata_resp = owner_application_dldata_resp.unwrap();
    let owner_app_handler = TestAppHandler::new();
    owner_application_uldata.set_handler(Arc::new(owner_app_handler.clone()));
    owner_application_dldata_resp.set_handler(Arc::new(owner_app_handler.clone()));
    owner_application_dldata_result.set_handler(Arc::new(owner_app_handler.clone()));
    let _ = owner_application_uldata.connect();
    let _ = owner_application_dldata.connect();
    let _ = owner_application_dldata_resp.connect();
    let _ = owner_application_dldata_result.connect();

    let (mut owner_network_uldata, mut owner_network_dldata, _, mut owner_network_dldata_result) =
        new_data_queues(state, &mqtt_conn, &opts, "broker.network", true)?;
    let owner_net_handler = TestNetHandler::new();
    owner_network_dldata.set_handler(Arc::new(owner_net_handler.clone()));
    let _ = owner_network_uldata.connect();
    let _ = owner_network_dldata.connect();
    let _ = owner_network_dldata_result.connect();

    let mut connected = false;
    let runtime = state.runtime.as_ref().unwrap();
    runtime.block_on(async {
        for _ in 0..WAIT_COUNT {
            time::sleep(Duration::from_millis(WAIT_TICK)).await;
            if manager_application_uldata.status() == Status::Connected
                && manager_application_dldata.status() == Status::Connected
                && manager_application_dldata_resp.status() == Status::Connected
                && manager_application_dldata_result.status() == Status::Connected
                && owner_application_uldata.status() == Status::Connected
                && owner_application_dldata.status() == Status::Connected
                && owner_application_dldata_resp.status() == Status::Connected
                && owner_application_dldata_result.status() == Status::Connected
                && manager_network_uldata.status() == Status::Connected
                && manager_network_dldata.status() == Status::Connected
                && manager_network_dldata_result.status() == Status::Connected
                && public_network_uldata.status() == Status::Connected
                && public_network_dldata.status() == Status::Connected
                && public_network_dldata_result.status() == Status::Connected
                && owner_network_uldata.status() == Status::Connected
                && owner_network_dldata.status() == Status::Connected
                && owner_network_dldata_result.status() == Status::Connected
            {
                connected = true;
                break;
            }
        }
    });
    if !connected {
        return Err("one or more queues are not connected".to_string());
    }
    Ok(TestResources {
        _manager_application_uldata: manager_application_uldata,
        manager_application_dldata,
        _manager_application_dldata_resp: manager_application_dldata_resp,
        _manager_application_dldata_result: manager_application_dldata_result,
        _owner_application_uldata: owner_application_uldata,
        owner_application_dldata,
        _owner_application_dldata_resp: owner_application_dldata_resp,
        _owner_application_dldata_result: owner_application_dldata_result,
        manager_network_uldata,
        _manager_network_dldata: manager_network_dldata,
        manager_network_dldata_result,
        public_network_uldata,
        _public_network_dldata: public_network_dldata,
        public_network_dldata_result,
        owner_network_uldata,
        _owner_network_dldata: owner_network_dldata,
        _owner_network_dldata_result: owner_network_dldata_result,
        manager_app_handler,
        owner_app_handler,
        manager_net_handler,
        public_net_handler,
    })
}

fn new_data_queues(
    state: &mut TestState,
    conn: &Connection,
    opts: &Options,
    prefix: &str,
    is_network: bool,
) -> Result<(Queue, Queue, Option<Queue>, Queue), String> {
    let uldata: Queue;
    let dldata: Queue;
    let dldata_resp: Option<Queue>;
    let dldata_result: Queue;

    if opts.unit_id.len() == 0 {
        if opts.unit_code.len() != 0 {
            return Err("unit_id and unit_code must both empty or non-empty".to_string());
        }
    } else {
        if opts.unit_code.len() == 0 {
            return Err("unit_id and unit_code must both empty or non-empty".to_string());
        }
    }
    if opts.id.len() == 0 {
        return Err("`id` cannot be empty".to_string());
    }
    if opts.name.len() == 0 {
        return Err("`name` cannot be empty".to_string());
    }

    let unit = match opts.unit_code.len() {
        0 => "_",
        _ => opts.unit_code.as_str(),
    };

    match conn {
        Connection::Amqp(conn, _) => {
            let prefetch = match opts.prefetch {
                None => 100,
                Some(prefetch) => match prefetch {
                    0 => 100,
                    _ => prefetch,
                },
            };

            let uldata_opts = QueueOptions::Amqp(
                AmqpQueueOptions {
                    name: format!("{}.{}.{}.uldata", prefix, unit, opts.name.as_str()),
                    is_recv: !is_network,
                    reliable: true,
                    broadcast: false,
                    prefetch,
                    ..Default::default()
                },
                conn,
            );
            let dldata_opts = QueueOptions::Amqp(
                AmqpQueueOptions {
                    name: format!("{}.{}.{}.dldata", prefix, unit, opts.name.as_str()),
                    is_recv: is_network,
                    reliable: true,
                    broadcast: false,
                    prefetch,
                    ..Default::default()
                },
                conn,
            );
            let dldata_resp_opts = QueueOptions::Amqp(
                AmqpQueueOptions {
                    name: format!("{}.{}.{}.dldata-resp", prefix, unit, opts.name.as_str()),
                    is_recv: !is_network,
                    reliable: true,
                    broadcast: false,
                    prefetch,
                    ..Default::default()
                },
                conn,
            );
            let dldata_result_opts = QueueOptions::Amqp(
                AmqpQueueOptions {
                    name: format!("{}.{}.{}.dldata-result", prefix, unit, opts.name.as_str()),
                    is_recv: !is_network,
                    reliable: true,
                    broadcast: false,
                    prefetch,
                    ..Default::default()
                },
                conn,
            );
            uldata = Queue::new(uldata_opts)?;
            dldata = Queue::new(dldata_opts)?;
            dldata_resp = match is_network {
                false => Some(Queue::new(dldata_resp_opts)?),
                true => None,
            };
            dldata_result = Queue::new(dldata_result_opts)?;
        }
        Connection::Mqtt(conn, _) => {
            let uldata_opts = QueueOptions::Mqtt(
                MqttQueueOptions {
                    name: format!("{}.{}.{}.uldata", prefix, unit, opts.name.as_str()),
                    is_recv: !is_network,
                    reliable: true,
                    broadcast: false,
                    shared_prefix: opts.shared_prefix.clone(),
                    ..Default::default()
                },
                conn,
            );
            let dldata_opts = QueueOptions::Mqtt(
                MqttQueueOptions {
                    name: format!("{}.{}.{}.dldata", prefix, unit, opts.name.as_str()),
                    is_recv: is_network,
                    reliable: true,
                    broadcast: false,
                    shared_prefix: opts.shared_prefix.clone(),
                    ..Default::default()
                },
                conn,
            );
            let dldata_resp_opts = QueueOptions::Mqtt(
                MqttQueueOptions {
                    name: format!("{}.{}.{}.dldata-resp", prefix, unit, opts.name.as_str()),
                    is_recv: !is_network,
                    reliable: true,
                    broadcast: false,
                    shared_prefix: opts.shared_prefix.clone(),
                    ..Default::default()
                },
                conn,
            );
            let dldata_result_opts = QueueOptions::Mqtt(
                MqttQueueOptions {
                    name: format!("{}.{}.{}.dldata-result", prefix, unit, opts.name.as_str()),
                    is_recv: !is_network,
                    reliable: true,
                    broadcast: false,
                    shared_prefix: opts.shared_prefix.clone(),
                    ..Default::default()
                },
                conn,
            );
            uldata = Queue::new(uldata_opts)?;
            dldata = Queue::new(dldata_opts)?;
            dldata_resp = match is_network {
                false => Some(Queue::new(dldata_resp_opts)?),
                true => None,
            };
            dldata_result = Queue::new(dldata_result_opts)?;
        }
    }

    let routing_queues = state.routing_queues.as_mut().unwrap();
    routing_queues.push(Box::new(uldata.clone()));
    routing_queues.push(Box::new(dldata.clone()));
    if let Some(q) = dldata_resp.as_ref() {
        routing_queues.push(Box::new(q.clone()));
    }
    routing_queues.push(Box::new(dldata_result.clone()));

    Ok((uldata, dldata, dldata_resp, dldata_result))
}
