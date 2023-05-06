use std::{
    cmp::Ordering,
    collections::HashMap,
    error::Error as StdError,
    io::{Error as IoError, ErrorKind},
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use general_mq::{
    queue::{
        Event as QueueEvent, EventHandler as QueueEventHandler, Message, Queue,
        Status as QueueStatus,
    },
    Queue as MqQueue,
};
use hex;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use tokio::task;
use url::Url;

use crate::util::strings;

use super::{
    get_connection, new_data_queues, remove_connection, Connection, DataMqStatus, MgrStatus,
    Options,
};

/// Uplink data from network to broker.
pub struct UlData {
    pub time: DateTime<Utc>,
    pub network_addr: String,
    pub data: String,
    pub extension: Option<Map<String, Value>>,
}

/// Downlink data from broker to network.
#[derive(Clone, Deserialize, Serialize)]
pub struct DlData {
    #[serde(rename = "dataId")]
    pub data_id: String,
    #[serde(rename = "pub")]
    pub publish: DateTime<Utc>,
    #[serde(rename = "expiresIn")]
    pub expires_in: i64,
    #[serde(rename = "networkAddr")]
    pub network_addr: String,
    pub data: String,
    #[serde(skip_serializing_if = "Option::is_none")]
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

/// The manager for network queues.
#[derive(Clone)]
pub struct NetworkMgr {
    opts: Arc<Options>,

    // Information for delete connection automatically.
    conn_pool: Arc<Mutex<HashMap<String, Connection>>>,
    host_uri: String,

    uldata: Arc<Mutex<MqQueue>>,
    dldata: Arc<Mutex<MqQueue>>,
    dldata_result: Arc<Mutex<MqQueue>>,

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
}

/// The event handler for [`general_mq::queue::Queue`].
struct MgrMqEventHandler {
    mgr: NetworkMgr,
}

#[derive(Serialize)]
struct UlDataInner {
    time: String,
    #[serde(rename = "networkAddr")]
    network_addr: String,
    data: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    extension: Option<Map<String, Value>>,
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
    extension: Option<Map<String, Value>>,
}

const QUEUE_PREFIX: &'static str = "broker.network";
const ERR_PARAM_DEV: &'static str = "the `network_addr` must be a non-empty string";
const ERR_PARAM_DATA_ID: &'static str = "the `data_id` must be a non-empty string";
const ERR_PARAM_DATA: &'static str = "the `data` must be a hex string";

impl NetworkMgr {
    /// To create a manager instance.
    pub fn new(
        conn_pool: Arc<Mutex<HashMap<String, Connection>>>,
        host_uri: &Url,
        opts: Options,
        handler: Arc<dyn EventHandler>,
    ) -> Result<Self, String> {
        let conn = get_connection(&conn_pool, host_uri)?;

        let (uldata, dldata, _, dldata_result) = new_data_queues(&conn, &opts, QUEUE_PREFIX, true)?;

        let mgr = NetworkMgr {
            opts: Arc::new(opts),
            conn_pool,
            host_uri: host_uri.to_string(),
            uldata,
            dldata,
            dldata_result,
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
        if let Err(e) = q.connect() {
            return Err(e.to_string());
        }
        let mut q = { mgr.dldata_result.lock().unwrap().clone() };
        q.set_handler(mq_handler.clone());
        if let Err(e) = q.connect() {
            return Err(e.to_string());
        }
        match conn {
            Connection::Amqp(_, counter) => {
                *counter.lock().unwrap() += 3;
            }
            Connection::Mqtt(_, counter) => {
                *counter.lock().unwrap() += 3;
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
        }
    }

    /// To close the manager queues.
    /// The underlying connection will be closed when there are no queues use it.
    pub async fn close(&self) -> Result<(), Box<dyn StdError>> {
        let mut q = { self.uldata.lock().unwrap().clone() };
        q.close().await?;
        let mut q = { self.dldata.lock().unwrap().clone() };
        q.close().await?;
        let mut q = { self.dldata_result.lock().unwrap().clone() };
        q.close().await?;

        remove_connection(&self.conn_pool, &self.host_uri, 3).await
    }

    /// Send uplink data to the broker.
    pub fn send_uldata(&self, data: &UlData) -> Result<(), Box<dyn StdError>> {
        if data.network_addr.len() == 0 {
            let err = IoError::new(ErrorKind::InvalidInput, ERR_PARAM_DEV.to_string());
            return Err(Box::new(err));
        }
        if data.data.len() > 0 {
            if let Err(_) = hex::decode(data.data.as_str()) {
                let err = IoError::new(ErrorKind::InvalidInput, ERR_PARAM_DATA.to_string());
                return Err(Box::new(err));
            }
        }

        let msg_data = UlDataInner {
            time: strings::time_str(&data.time),
            network_addr: data.network_addr.clone(),
            data: data.data.clone(),
            extension: data.extension.clone(),
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
    async fn on_event(&self, _queue: Arc<dyn Queue>, _ev: QueueEvent) {
        let uldata_status = { self.mgr.uldata.lock().unwrap().status() };
        let dldata_status = { self.mgr.dldata.lock().unwrap().status() };
        let dldata_result_status = { self.mgr.dldata_result.lock().unwrap().status() };

        let status = match uldata_status == QueueStatus::Connected
            && dldata_status == QueueStatus::Connected
            && dldata_result_status == QueueStatus::Connected
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

    // Validate and decode data.
    async fn on_message(&self, queue: Arc<dyn Queue>, msg: Box<dyn Message>) {
        const _FN_NAME: &'static str = "NetworkMgr.on_message";

        let queue_name = queue.name();
        if queue_name.cmp(self.mgr.dldata.lock().unwrap().name()) == Ordering::Equal {
            let data = match serde_json::from_slice::<DlDataInner>(msg.payload()) {
                Err(_) => {
                    let _ = msg.ack().await;
                    return;
                }
                Ok(data) => {
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
                        data: data.data,
                        extension: data.extension,
                    }
                }
            };
            let handler = { self.mgr.handler.lock().unwrap().clone() };
            let _ = match handler.on_dldata(&self.mgr, Box::new(data)).await {
                Err(_) => msg.nack().await,
                Ok(_) => msg.ack().await,
            };
        }
    }
}
