use std::{
    cmp::Ordering,
    collections::HashMap,
    error::Error as StdError,
    io::{Error as IoError, ErrorKind},
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use hex;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use tokio::task;
use url::Url;

use general_mq::{
    queue::{
        EventHandler as QueueEventHandler, GmqQueue, Message, MessageHandler, Status as QueueStatus,
    },
    Queue,
};

use crate::util::strings;

use super::{
    get_connection, new_data_queues, remove_connection, Connection, DataMqStatus, MgrStatus,
    Options,
};

/// Uplink data from network to broker.
pub struct UlData {
    pub time: DateTime<Utc>,
    pub network_addr: String,
    pub data: Vec<u8>,
    pub extension: Option<Map<String, Value>>,
}

/// Downlink data from broker to network.
pub struct DlData {
    pub data_id: String,
    pub publish: DateTime<Utc>,
    pub expires_in: i64,
    pub network_addr: String,
    pub data: Vec<u8>,
    pub extension: Option<Map<String, Value>>,
}

/// Downlink data result when processing or completing data transfer to the device.
#[derive(Clone, Serialize)]
pub struct DlDataResult {
    #[serde(rename = "dataId")]
    pub data_id: String,
    pub status: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// Network control message from broker to network.
#[derive(Clone, Deserialize)]
#[serde(tag = "operation")]
pub enum NetworkCtrlMsg {
    #[serde(rename = "add-device")]
    AddDevice {
        time: DateTime<Utc>,
        new: CtrlAddDevice,
    },
    #[serde(rename = "add-device-bulk")]
    AddDeviceBulk {
        time: DateTime<Utc>,
        new: CtrlAddDeviceBulk,
    },
    #[serde(rename = "add-device-range")]
    AddDeviceRange {
        time: DateTime<Utc>,
        new: CtrlAddDeviceRange,
    },
    #[serde(rename = "del-device")]
    DelDevice {
        time: DateTime<Utc>,
        new: CtrlDelDevice,
    },
    #[serde(rename = "del-device-bulk")]
    DelDeviceBulk {
        time: DateTime<Utc>,
        new: CtrlDelDeviceBulk,
    },
    #[serde(rename = "del-device-range")]
    DelDeviceRange {
        time: DateTime<Utc>,
        new: CtrlDelDeviceRange,
    },
}

#[derive(Clone, Deserialize)]
pub struct CtrlAddDevice {
    #[serde(rename = "networkAddr")]
    pub network_addr: String,
}

#[derive(Clone, Deserialize)]
pub struct CtrlAddDeviceBulk {
    #[serde(rename = "networkAddrs")]
    pub network_addrs: Vec<String>,
}

#[derive(Clone, Deserialize)]
pub struct CtrlAddDeviceRange {
    #[serde(rename = "startAddr")]
    pub start_addr: String,
    #[serde(rename = "endAddr")]
    pub end_addr: String,
}

#[derive(Clone, Deserialize)]
pub struct CtrlDelDevice {
    #[serde(rename = "networkAddr")]
    pub network_addr: String,
}

#[derive(Clone, Deserialize)]
pub struct CtrlDelDeviceBulk {
    #[serde(rename = "networkAddrs")]
    pub network_addrs: Vec<String>,
}

#[derive(Clone, Deserialize)]
pub struct CtrlDelDeviceRange {
    #[serde(rename = "startAddr")]
    pub start_addr: String,
    #[serde(rename = "endAddr")]
    pub end_addr: String,
}

/// The manager for network queues.
#[derive(Clone)]
pub struct NetworkMgr {
    opts: Arc<Options>,

    // Information for delete connection automatically.
    conn_pool: Arc<Mutex<HashMap<String, Connection>>>,
    host_uri: String,

    uldata: Arc<Mutex<Queue>>,
    dldata: Arc<Mutex<Queue>>,
    dldata_result: Arc<Mutex<Queue>>,
    ctrl: Arc<Mutex<Queue>>,

    status: Arc<Mutex<MgrStatus>>,
    handler: Arc<Mutex<Arc<dyn EventHandler>>>,
}

/// Event handler trait for the [`NetworkMgr`].
#[async_trait]
pub trait EventHandler: Send + Sync {
    /// Fired when one of the manager's queues encounters a state change.
    async fn on_status_change(&self, mgr: &NetworkMgr, status: MgrStatus);

    /// Fired when a [`DlData`] data is received.
    ///
    /// Return [`Err`] will NACK the data.
    /// The data may will be received again depending on the protocol (such as AMQP).
    async fn on_dldata(&self, mgr: &NetworkMgr, data: Box<DlData>) -> Result<(), ()>;

    /// Fired when a [`NetworkCtrlMsg`] data is received.
    ///
    /// Return [`Err`] will NACK the data.
    /// The data may will be received again depending on the protocol (such as AMQP).
    async fn on_ctrl(&self, mgr: &NetworkMgr, data: Box<NetworkCtrlMsg>) -> Result<(), ()>;
}

/// The event handler for [`general_mq::queue::GmqQueue`].
struct MgrMqEventHandler {
    mgr: NetworkMgr,
}

#[derive(Serialize)]
struct UlDataInner<'a> {
    time: String,
    #[serde(rename = "networkAddr")]
    network_addr: &'a String,
    data: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    extension: &'a Option<Map<String, Value>>,
}

/// Downlink data from broker to network.
#[derive(Deserialize)]
struct DlDataInner {
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

const QUEUE_PREFIX: &'static str = "broker.network";
const ERR_PARAM_DEV: &'static str = "the `network_addr` must be a non-empty string";
const ERR_PARAM_DATA_ID: &'static str = "the `data_id` must be a non-empty string";

impl NetworkMgr {
    /// To create a manager instance.
    pub fn new(
        conn_pool: Arc<Mutex<HashMap<String, Connection>>>,
        host_uri: &Url,
        opts: Options,
        handler: Arc<dyn EventHandler>,
    ) -> Result<Self, String> {
        let conn = get_connection(&conn_pool, host_uri)?;

        let (uldata, dldata, _, dldata_result, ctrl) =
            new_data_queues(&conn, &opts, QUEUE_PREFIX, true)?;

        let mgr = NetworkMgr {
            opts: Arc::new(opts),
            conn_pool,
            host_uri: host_uri.to_string(),
            uldata,
            dldata,
            dldata_result,
            ctrl: ctrl.unwrap(),
            status: Arc::new(Mutex::new(MgrStatus::NotReady)),
            handler: Arc::new(Mutex::new(handler)),
        };
        let mq_handler = Arc::new(MgrMqEventHandler { mgr: mgr.clone() });
        let mut q = { mgr.uldata.lock().unwrap().clone() };
        q.set_handler(mq_handler.clone());
        if let Err(e) = q.connect() {
            return Err(e.to_string());
        }
        let mut q = { mgr.dldata.lock().unwrap().clone() };
        q.set_handler(mq_handler.clone());
        q.set_msg_handler(mq_handler.clone());
        if let Err(e) = q.connect() {
            return Err(e.to_string());
        }
        let mut q = { mgr.dldata_result.lock().unwrap().clone() };
        q.set_handler(mq_handler.clone());
        if let Err(e) = q.connect() {
            return Err(e.to_string());
        }
        let mut q = { mgr.ctrl.lock().unwrap().clone() };
        q.set_handler(mq_handler.clone());
        q.set_msg_handler(mq_handler.clone());
        if let Err(e) = q.connect() {
            return Err(e.to_string());
        }
        match conn {
            Connection::Amqp(_, counter) => {
                *counter.lock().unwrap() += 4;
            }
            Connection::Mqtt(_, counter) => {
                *counter.lock().unwrap() += 4;
            }
        }
        Ok(mgr)
    }

    /// The associated unit ID of the network.
    pub fn unit_id(&self) -> &str {
        self.opts.unit_id.as_str()
    }

    /// The associated unit code of the network.
    pub fn unit_code(&self) -> &str {
        self.opts.unit_code.as_str()
    }

    /// The network ID.
    pub fn id(&self) -> &str {
        self.opts.id.as_str()
    }

    /// The network code.
    pub fn name(&self) -> &str {
        self.opts.name.as_str()
    }

    /// Manager status.
    pub fn status(&self) -> MgrStatus {
        *self.status.lock().unwrap()
    }

    /// Detail status of each message queue. Please ignore `dldata_resp`.
    pub fn mq_status(&self) -> DataMqStatus {
        DataMqStatus {
            uldata: { self.uldata.lock().unwrap().status() },
            dldata: { self.dldata.lock().unwrap().status() },
            dldata_resp: QueueStatus::Closed,
            dldata_result: { self.dldata_result.lock().unwrap().status() },
            ctrl: { self.ctrl.lock().unwrap().status() },
        }
    }

    /// To close the manager queues.
    /// The underlying connection will be closed when there are no queues use it.
    pub async fn close(&self) -> Result<(), Box<dyn StdError + Send + Sync>> {
        let mut q = { self.uldata.lock().unwrap().clone() };
        q.close().await?;
        let mut q = { self.dldata.lock().unwrap().clone() };
        q.close().await?;
        let mut q = { self.dldata_result.lock().unwrap().clone() };
        q.close().await?;
        let mut q = { self.ctrl.lock().unwrap().clone() };
        q.close().await?;

        remove_connection(&self.conn_pool, &self.host_uri, 4).await
    }

    /// Send uplink data to the broker.
    pub fn send_uldata(&self, data: &UlData) -> Result<(), Box<dyn StdError>> {
        if data.network_addr.len() == 0 {
            let err = IoError::new(ErrorKind::InvalidInput, ERR_PARAM_DEV.to_string());
            return Err(Box::new(err));
        }

        let msg_data = UlDataInner {
            time: strings::time_str(&data.time),
            network_addr: &data.network_addr,
            data: hex::encode(&data.data),
            extension: &data.extension,
        };
        let payload = serde_json::to_vec(&msg_data)?;
        let queue = { (*self.uldata.lock().unwrap()).clone() };
        task::spawn(async move {
            let _ = queue.send_msg(payload).await;
        });
        Ok(())
    }

    /// Send downlink result data to the broker.
    pub fn send_dldata_result(&self, data: &DlDataResult) -> Result<(), Box<dyn StdError>> {
        if data.data_id.len() == 0 {
            let err = IoError::new(ErrorKind::InvalidInput, ERR_PARAM_DATA_ID.to_string());
            return Err(Box::new(err));
        }

        let payload = serde_json::to_vec(&data)?;
        let queue = { (*self.dldata_result.lock().unwrap()).clone() };
        task::spawn(async move {
            let _ = queue.send_msg(payload).await;
        });
        Ok(())
    }
}

#[async_trait]
impl QueueEventHandler for MgrMqEventHandler {
    async fn on_error(&self, _queue: Arc<dyn GmqQueue>, _err: Box<dyn StdError + Send + Sync>) {}

    async fn on_status(&self, _queue: Arc<dyn GmqQueue>, _status: QueueStatus) {
        let uldata_status = { self.mgr.uldata.lock().unwrap().status() };
        let dldata_status = { self.mgr.dldata.lock().unwrap().status() };
        let dldata_result_status = { self.mgr.dldata_result.lock().unwrap().status() };
        let ctrl_status = { self.mgr.ctrl.lock().unwrap().status() };

        let status = match uldata_status == QueueStatus::Connected
            && dldata_status == QueueStatus::Connected
            && dldata_result_status == QueueStatus::Connected
            && ctrl_status == QueueStatus::Connected
        {
            false => MgrStatus::NotReady,
            true => MgrStatus::Ready,
        };

        let mut changed = false;
        {
            let mut mutex = self.mgr.status.lock().unwrap();
            if *mutex != status {
                *mutex = status;
                changed = true;
            }
        }
        if changed {
            let handler = { self.mgr.handler.lock().unwrap().clone() };
            handler.on_status_change(&self.mgr, status).await;
        }
    }
}

#[async_trait]
impl MessageHandler for MgrMqEventHandler {
    // Validate and decode data.
    async fn on_message(&self, queue: Arc<dyn GmqQueue>, msg: Box<dyn Message>) {
        const _FN_NAME: &'static str = "NetworkMgr.on_message";

        let queue_name = queue.name();
        if queue_name.cmp(self.mgr.dldata.lock().unwrap().name()) == Ordering::Equal {
            let data = match serde_json::from_slice::<DlDataInner>(msg.payload()) {
                Err(_) => {
                    let _ = msg.ack().await;
                    return;
                }
                Ok(data) => {
                    let data_bytes = match data.data.len() {
                        0 => vec![],
                        _ => match hex::decode(data.data.as_str()) {
                            Err(_) => {
                                let _ = msg.ack().await;
                                return;
                            }
                            Ok(data) => data,
                        },
                    };
                    let publish = match DateTime::parse_from_rfc3339(data.publish.as_str()) {
                        Err(_) => {
                            let _ = msg.ack().await;
                            return;
                        }
                        Ok(publish) => publish.into(),
                    };
                    DlData {
                        data_id: data.data_id,
                        publish,
                        expires_in: data.expires_in,
                        network_addr: data.network_addr,
                        data: data_bytes,
                        extension: data.extension,
                    }
                }
            };
            let handler = { self.mgr.handler.lock().unwrap().clone() };
            let _ = match handler.on_dldata(&self.mgr, Box::new(data)).await {
                Err(_) => msg.nack().await,
                Ok(_) => msg.ack().await,
            };
        } else if queue_name.cmp(self.mgr.ctrl.lock().unwrap().name()) == Ordering::Equal {
            let data = match serde_json::from_slice::<NetworkCtrlMsg>(msg.payload()) {
                Err(_) => {
                    let _ = msg.ack().await;
                    return;
                }
                Ok(data) => data,
            };
            let handler = { self.mgr.handler.lock().unwrap().clone() };
            let _ = match handler.on_ctrl(&self.mgr, Box::new(data)).await {
                Err(_) => msg.nack().await,
                Ok(_) => msg.ack().await,
            };
        }
    }
}
