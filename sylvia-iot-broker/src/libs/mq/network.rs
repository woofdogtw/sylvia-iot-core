use std::{
    cmp::Ordering,
    collections::HashMap,
    error::Error as StdError,
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use chrono::DateTime;
use hex;
use log::{error, warn};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use tokio::task;
use url::Url;

use general_mq::{
    Queue,
    queue::{
        EventHandler as QueueEventHandler, GmqQueue, Message, MessageHandler, Status as QueueStatus,
    },
};
use sylvia_iot_corelib::strings;

use super::{
    Connection, MgrMqStatus, MgrStatus, Options, get_connection, new_ctrl_queues, new_data_queues,
    remove_connection,
};

/// Uplink data from network to broker.
#[derive(Deserialize)]
pub struct UlData {
    pub time: String,
    #[serde(rename = "networkAddr")]
    pub network_addr: String,
    pub data: String,
    pub extension: Option<Map<String, Value>>,
}

/// Downlink data from broker to network.
#[derive(Serialize)]
pub struct DlData {
    #[serde(rename = "dataId")]
    pub data_id: String,
    #[serde(rename = "pub")]
    pub publish: String,
    #[serde(rename = "expiresIn")]
    pub expires_in: i64,
    #[serde(rename = "networkAddr")]
    pub network_addr: String,
    pub data: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extension: Option<Map<String, Value>>,
}

/// Downlink data result when processing or completing data transfer to the device.
#[derive(Deserialize)]
pub struct DlDataResult {
    #[serde(rename = "dataId")]
    pub data_id: String,
    pub status: i32,
    pub message: Option<String>,
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
    async fn on_status_change(&self, mgr: &NetworkMgr, status: MgrStatus);

    async fn on_uldata(&self, mgr: &NetworkMgr, data: Box<UlData>) -> Result<(), ()>;
    async fn on_dldata_result(&self, mgr: &NetworkMgr, data: Box<DlDataResult>) -> Result<(), ()>;
}

/// The event handler for [`general_mq::queue::GmqQueue`].
struct MgrMqEventHandler {
    mgr: NetworkMgr,
}

const QUEUE_PREFIX: &'static str = "broker.network";

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
        let ctrl = new_ctrl_queues(&conn, &opts, QUEUE_PREFIX)?;

        let mgr = NetworkMgr {
            opts: Arc::new(opts),
            conn_pool,
            host_uri: host_uri.to_string(),
            uldata,
            dldata,
            dldata_result,
            ctrl,
            status: Arc::new(Mutex::new(MgrStatus::NotReady)),
            handler: Arc::new(Mutex::new(handler)),
        };
        let mq_handler = Arc::new(MgrMqEventHandler { mgr: mgr.clone() });
        let mut q = { mgr.uldata.lock().unwrap().clone() };
        q.set_handler(mq_handler.clone());
        q.set_msg_handler(mq_handler.clone());
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
        q.set_msg_handler(mq_handler.clone());
        if let Err(e) = q.connect() {
            return Err(e.to_string());
        }
        let mut q = { mgr.ctrl.lock().unwrap().clone() };
        q.set_handler(mq_handler.clone());
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
    pub fn mq_status(&self) -> MgrMqStatus {
        MgrMqStatus {
            uldata: { self.uldata.lock().unwrap().status() },
            dldata: { self.dldata.lock().unwrap().status() },
            dldata_resp: QueueStatus::Closed,
            dldata_result: { self.dldata_result.lock().unwrap().status() },
            ctrl: { self.ctrl.lock().unwrap().status() },
        }
    }

    /// To close the manager queues.
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

    /// Send downlink data to the network.
    pub fn send_dldata(&self, data: &DlData) -> Result<(), Box<dyn StdError>> {
        let payload = serde_json::to_vec(data)?;
        let queue = { (*self.dldata.lock().unwrap()).clone() };
        task::spawn(async move {
            let _ = queue.send_msg(payload).await;
        });
        Ok(())
    }

    /// Send control data to the network.
    pub async fn send_ctrl(&self, payload: Vec<u8>) -> Result<(), Box<dyn StdError + Send + Sync>> {
        let queue = { (*self.ctrl.lock().unwrap()).clone() };
        queue.send_msg(payload).await
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
        const FN_NAME: &'static str = "NetworkMgr.on_message";

        let queue_name = queue.name();
        if queue_name.cmp(self.mgr.uldata.lock().unwrap().name()) == Ordering::Equal {
            let data = match serde_json::from_slice::<UlData>(msg.payload()) {
                Err(_) => {
                    warn!("[{}] invalid format from {}", FN_NAME, queue_name);
                    if let Err(e) = msg.ack().await {
                        error!("[{}] ACK message error: {}", FN_NAME, e);
                    }
                    return;
                }
                Ok(mut data) => {
                    let time = match DateTime::parse_from_rfc3339(data.time.as_str()) {
                        Err(e) => {
                            warn!(
                                "[{}] invalid time format from {}: {}",
                                FN_NAME, queue_name, e
                            );
                            if let Err(e) = msg.ack().await {
                                error!("[{}] ACK message error: {}", FN_NAME, e);
                            }
                            return;
                        }
                        Ok(time) => time.into(),
                    };
                    data.time = strings::time_str(&time);
                    if data.network_addr.len() == 0 {
                        warn!(
                            "[{}] invalid network_addr format from {}",
                            FN_NAME, queue_name,
                        );
                        if let Err(e) = msg.ack().await {
                            error!("[{}] ACK message error: {}", FN_NAME, e);
                        }
                        return;
                    }
                    data.network_addr = data.network_addr.to_lowercase();
                    if data.data.len() > 0 {
                        if let Err(_) = hex::decode(data.data.as_str()) {
                            warn!("[{}] invalid data format from {}", FN_NAME, queue_name);
                            if let Err(e) = msg.ack().await {
                                error!("[{}] ACK message error: {}", FN_NAME, e);
                            }
                            return;
                        }
                        data.data = data.data.to_lowercase();
                    }
                    data
                }
            };
            let handler = { self.mgr.handler.lock().unwrap().clone() };
            match handler.on_uldata(&self.mgr, Box::new(data)).await {
                Err(_) => {
                    if let Err(e) = msg.nack().await {
                        error!("[{}] NACK message error: {}", FN_NAME, e);
                    }
                }
                Ok(_) => {
                    if let Err(e) = msg.ack().await {
                        error!("[{}] ACK message error: {}", FN_NAME, e);
                    }
                }
            }
        } else if queue_name.cmp(self.mgr.dldata_result.lock().unwrap().name()) == Ordering::Equal {
            let data = match serde_json::from_slice::<DlDataResult>(msg.payload()) {
                Err(_) => {
                    warn!("[{}] invalid format from {}", FN_NAME, queue_name);
                    if let Err(e) = msg.ack().await {
                        error!("[{}] ACK message error: {}", FN_NAME, e);
                    }
                    return;
                }
                Ok(data) => {
                    if data.data_id.len() == 0 {
                        warn!("[{}] invalid data_id format from {}", FN_NAME, queue_name);
                        if let Err(e) = msg.ack().await {
                            error!("[{}] ACK message error: {}", FN_NAME, e);
                        }
                        return;
                    }
                    if let Some(message) = data.message.as_ref() {
                        if message.len() == 0 {
                            warn!("[{}] invalid message format from {}", FN_NAME, queue_name);
                            if let Err(e) = msg.ack().await {
                                error!("[{}] ACK message error: {}", FN_NAME, e);
                            }
                            return;
                        }
                    }
                    data
                }
            };
            let handler = { self.mgr.handler.lock().unwrap().clone() };
            match handler.on_dldata_result(&self.mgr, Box::new(data)).await {
                Err(_) => {
                    if let Err(e) = msg.nack().await {
                        error!("[{}] NACK message error: {}", FN_NAME, e);
                    }
                }
                Ok(_) => {
                    if let Err(e) = msg.ack().await {
                        error!("[{}] ACK message error: {}", FN_NAME, e);
                    }
                }
            }
        }
    }
}
