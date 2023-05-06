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
use serde_json::{self, Map, Value};
use tokio::task;
use url::Url;

use super::{
    get_connection, new_data_queues, remove_connection, Connection, DataMqStatus, MgrStatus,
    Options,
};

/// Uplink data from broker to application.
pub struct UlData {
    pub data_id: String,
    pub time: DateTime<Utc>,
    pub publish: DateTime<Utc>,
    pub device_id: String,
    pub network_id: String,
    pub network_code: String,
    pub network_addr: String,
    pub is_public: bool,
    pub data: String,
    pub extension: Option<Map<String, Value>>,
}

/// Downlink data from application to broker.
#[derive(Clone, Serialize)]
pub struct DlData {
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

/// Downlink data response for [`DlData`].
#[derive(Deserialize)]
pub struct DlDataResp {
    #[serde(rename = "correlationId")]
    pub correlation_id: String,
    #[serde(rename = "dataId")]
    pub data_id: Option<String>,
    pub error: Option<String>,
    pub message: Option<String>,
}

/// Downlink data result when processing or completing data transfer to the device.
#[derive(Deserialize)]
pub struct DlDataResult {
    #[serde(rename = "dataId")]
    pub data_id: String,
    pub status: i32,
    pub message: Option<String>,
}

/// The manager for application queues.
#[derive(Clone)]
pub struct ApplicationMgr {
    opts: Arc<Options>,

    // Information for delete connection automatically.
    conn_pool: Arc<Mutex<HashMap<String, Connection>>>,
    host_uri: String,

    uldata: Arc<Mutex<MqQueue>>,
    dldata: Arc<Mutex<MqQueue>>,
    dldata_resp: Arc<Mutex<MqQueue>>,
    dldata_result: Arc<Mutex<MqQueue>>,

    status: Arc<Mutex<MgrStatus>>,
    handler: Arc<Mutex<Arc<dyn EventHandler>>>,
}

/// Event handler trait for the [`ApplicationMgr`].
#[async_trait]
pub trait EventHandler: Send + Sync {
    /// Fired when one of the manager's queues encounters a state change.
    async fn on_status_change(&self, mgr: &ApplicationMgr, status: MgrStatus);

    /// Fired when a [`UlData`] data is received.
    ///
    /// Return [`Err`] will NACK the data.
    /// The data may will be received again depending on the protocol (such as AMQP).
    async fn on_uldata(&self, mgr: &ApplicationMgr, data: Box<UlData>) -> Result<(), ()>;

    /// Fired when a [`DlDataResp`] data is received.
    ///
    /// Return [`Err`] will NACK the data.
    /// The data may will be received again depending on the protocol (such as AMQP).
    async fn on_dldata_resp(&self, mgr: &ApplicationMgr, data: Box<DlDataResp>) -> Result<(), ()>;

    /// Fired when a [`DlDataResult`] data is received.
    ///
    /// Return [`Err`] will NACK the data.
    /// The data may will be received again depending on the protocol (such as AMQP).
    async fn on_dldata_result(
        &self,
        mgr: &ApplicationMgr,
        data: Box<DlDataResult>,
    ) -> Result<(), ()>;
}

/// The event handler for [`general_mq::queue::Queue`].
struct MgrMqEventHandler {
    mgr: ApplicationMgr,
}

#[derive(Deserialize)]
struct UlDataInner {
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
    extension: Option<Map<String, Value>>,
}

const QUEUE_PREFIX: &'static str = "broker.application";
const ERR_PARAM_CORR_ID: &'static str = "the `correlation_id` must be a non-empty string";
const ERR_PARAM_DEV: &'static str =
    "one of `device_id` or [`network_code`, `network_addr`] pair must be provided with non-empty string";
const ERR_PARAM_DATA: &'static str = "the `data` must be a hex string";

impl ApplicationMgr {
    /// To create a manager instance.
    pub fn new(
        conn_pool: Arc<Mutex<HashMap<String, Connection>>>,
        host_uri: &Url,
        opts: Options,
        handler: Arc<dyn EventHandler>,
    ) -> Result<Self, String> {
        if opts.unit_id.len() == 0 {
            return Err("`unit_id` cannot be empty for application".to_string());
        }

        let conn = get_connection(&conn_pool, host_uri)?;

        let (uldata, dldata, dldata_resp, dldata_result) =
            new_data_queues(&conn, &opts, QUEUE_PREFIX, false)?;

        let mgr = ApplicationMgr {
            opts: Arc::new(opts),
            conn_pool,
            host_uri: host_uri.to_string(),
            uldata,
            dldata,
            dldata_resp: dldata_resp.unwrap(),
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
        let mut q = { mgr.dldata_resp.lock().unwrap().clone() };
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
                *counter.lock().unwrap() += 4;
            }
            Connection::Mqtt(_, counter) => {
                *counter.lock().unwrap() += 4;
            }
        }
        Ok(mgr)
    }

    /// The associated unit ID of the application.
    pub fn unit_id(&self) -> &str {
        self.opts.unit_id.as_str()
    }

    /// The associated unit code of the application.
    pub fn unit_code(&self) -> &str {
        self.opts.unit_code.as_str()
    }

    /// The application ID.
    pub fn id(&self) -> &str {
        self.opts.id.as_str()
    }

    /// The application code.
    pub fn name(&self) -> &str {
        self.opts.name.as_str()
    }

    /// Manager status.
    pub fn status(&self) -> MgrStatus {
        *self.status.lock().unwrap()
    }

    /// Detail status of each message queue.
    pub fn mq_status(&self) -> DataMqStatus {
        DataMqStatus {
            uldata: { self.uldata.lock().unwrap().status() },
            dldata: { self.dldata.lock().unwrap().status() },
            dldata_resp: { self.dldata_resp.lock().unwrap().status() },
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
        let mut q = { self.dldata_resp.lock().unwrap().clone() };
        q.close().await?;
        let mut q = { self.dldata_result.lock().unwrap().clone() };
        q.close().await?;

        remove_connection(&self.conn_pool, &self.host_uri, 4).await
    }

    /// Send downlink data [`DlData`] to the broker.
    pub fn send_dldata(&self, data: &DlData) -> Result<(), Box<dyn StdError>> {
        if data.correlation_id.len() == 0 {
            let err = IoError::new(ErrorKind::InvalidInput, ERR_PARAM_CORR_ID.to_string());
            return Err(Box::new(err));
        }
        if data.device_id.is_none() {
            if data.network_code.is_none() || data.network_addr.is_none() {
                let err = IoError::new(ErrorKind::InvalidInput, ERR_PARAM_DEV.to_string());
                return Err(Box::new(err));
            } else if data.network_code.as_ref().unwrap().len() == 0
                || data.network_addr.as_ref().unwrap().len() == 0
            {
                let err = IoError::new(ErrorKind::InvalidInput, ERR_PARAM_DEV.to_string());
                return Err(Box::new(err));
            }
        } else if data.device_id.as_ref().unwrap().len() == 0 {
            let err = IoError::new(ErrorKind::InvalidInput, ERR_PARAM_DEV.to_string());
            return Err(Box::new(err));
        }
        if data.data.len() > 0 {
            if let Err(_) = hex::decode(data.data.as_str()) {
                let err = IoError::new(ErrorKind::InvalidInput, ERR_PARAM_DATA.to_string());
                return Err(Box::new(err));
            }
        }

        let payload = serde_json::to_vec(data)?;
        let queue = { (*self.dldata.lock().unwrap()).clone() };
        task::spawn(async move {
            let _ = queue.send_msg(payload).await;
        });
        Ok(())
    }
}

#[async_trait]
impl QueueEventHandler for MgrMqEventHandler {
    async fn on_event(&self, _queue: Arc<dyn Queue>, _ev: QueueEvent) {
        let status = match { self.mgr.uldata.lock().unwrap().status() } == QueueStatus::Connected
            && { self.mgr.dldata.lock().unwrap().status() } == QueueStatus::Connected
            && { self.mgr.dldata_resp.lock().unwrap().status() } == QueueStatus::Connected
            && { self.mgr.dldata_result.lock().unwrap().status() } == QueueStatus::Connected
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
        const _FN_NAME: &'static str = "ApplicationMgr.on_message";

        let queue_name = queue.name();
        if queue_name.cmp(self.mgr.uldata.lock().unwrap().name()) == Ordering::Equal {
            let data = match serde_json::from_slice::<UlDataInner>(msg.payload()) {
                Err(_) => {
                    let _ = msg.ack().await;
                    return;
                }
                Ok(data) => {
                    if data.data.len() > 0 {
                        if let Err(_) = hex::decode(data.data.as_str()) {
                            let _ = msg.ack().await;
                            return;
                        }
                    }
                    let time = match DateTime::parse_from_rfc3339(data.time.as_str()) {
                        Err(_) => {
                            let _ = msg.ack().await;
                            return;
                        }
                        Ok(time) => time.into(),
                    };
                    let publish = match DateTime::parse_from_rfc3339(data.publish.as_str()) {
                        Err(_) => {
                            let _ = msg.ack().await;
                            return;
                        }
                        Ok(publish) => publish.into(),
                    };
                    UlData {
                        data_id: data.data_id,
                        time,
                        publish,
                        device_id: data.device_id,
                        network_id: data.network_id,
                        network_code: data.network_code,
                        network_addr: data.network_addr,
                        is_public: data.is_public,
                        data: data.data,
                        extension: data.extension,
                    }
                }
            };
            let handler = { self.mgr.handler.lock().unwrap().clone() };
            let _ = match handler.on_uldata(&self.mgr, Box::new(data)).await {
                Err(_) => msg.nack().await,
                Ok(_) => msg.ack().await,
            };
        } else if queue_name.cmp(self.mgr.dldata_resp.lock().unwrap().name()) == Ordering::Equal {
            let data = match serde_json::from_slice::<DlDataResp>(msg.payload()) {
                Err(_) => {
                    let _ = msg.ack().await;
                    return;
                }
                Ok(data) => data,
            };
            let handler = { self.mgr.handler.lock().unwrap().clone() };
            let _ = match handler.on_dldata_resp(&self.mgr, Box::new(data)).await {
                Err(_) => msg.nack().await,
                Ok(_) => msg.ack().await,
            };
        } else if queue_name.cmp(self.mgr.dldata_result.lock().unwrap().name()) == Ordering::Equal {
            let data = match serde_json::from_slice::<DlDataResult>(msg.payload()) {
                Err(_) => {
                    let _ = msg.ack().await;
                    return;
                }
                Ok(data) => data,
            };
            let handler = { self.mgr.handler.lock().unwrap().clone() };
            let _ = match handler.on_dldata_result(&self.mgr, Box::new(data)).await {
                Err(_) => msg.nack().await,
                Ok(_) => msg.ack().await,
            };
        }
    }
}
