use std::{
    cmp::Ordering,
    collections::HashMap,
    error::Error as StdError,
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use hex;
use log::{error, warn};
use serde::{Deserialize, Serialize};
use serde_json::{self, Map, Value};
use tokio::task;
use url::Url;

use general_mq::{
    queue::{
        EventHandler as QueueEventHandler, GmqQueue, Message, MessageHandler, Status as QueueStatus,
    },
    Queue,
};
use sylvia_iot_corelib::{err, strings};

use super::{
    get_connection, new_data_queues, remove_connection, Connection, MgrMqStatus, MgrStatus, Options,
};

/// Uplink data from broker to application.
#[derive(Serialize)]
pub struct UlData {
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extension: Option<Map<String, Value>>,
}

/// Downlink data from application to broker.
#[derive(Deserialize)]
pub struct DlData {
    #[serde(rename = "correlationId")]
    pub correlation_id: String,
    #[serde(rename = "deviceId")]
    pub device_id: Option<String>,
    #[serde(rename = "networkCode")]
    pub network_code: Option<String>,
    #[serde(rename = "networkAddr")]
    pub network_addr: Option<String>,
    pub data: String,
    pub extension: Option<Map<String, Value>>,
}

/// Downlink data response for [`DlData`].
#[derive(Default, Serialize)]
pub struct DlDataResp {
    #[serde(rename = "correlationId")]
    pub correlation_id: String,
    #[serde(rename = "dataId", skip_serializing_if = "Option::is_none")]
    pub data_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// Downlink data result when processing or completing data transfer to the device.
#[derive(Serialize)]
pub struct DlDataResult {
    #[serde(rename = "dataId")]
    pub data_id: String,
    pub status: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// The manager for application queues.
#[derive(Clone)]
pub struct ApplicationMgr {
    opts: Arc<Options>,

    // Information for delete connection automatically.
    conn_pool: Arc<Mutex<HashMap<String, Connection>>>,
    host_uri: String,

    uldata: Arc<Mutex<Queue>>,
    dldata: Arc<Mutex<Queue>>,
    dldata_resp: Arc<Mutex<Queue>>,
    dldata_result: Arc<Mutex<Queue>>,

    status: Arc<Mutex<MgrStatus>>,
    handler: Arc<Mutex<Arc<dyn EventHandler>>>,
}

/// Event handler trait for the [`ApplicationMgr`].
#[async_trait]
pub trait EventHandler: Send + Sync {
    async fn on_status_change(&self, mgr: &ApplicationMgr, status: MgrStatus);

    async fn on_dldata(
        &self,
        mgr: &ApplicationMgr,
        data: Box<DlData>,
    ) -> Result<Box<DlDataResp>, ()>;
}

/// The event handler for [`general_mq::queue::GmqQueue`].
struct MgrMqEventHandler {
    mgr: ApplicationMgr,
}

const QUEUE_PREFIX: &'static str = "broker.application";

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
        q.set_msg_handler(mq_handler.clone());
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
    pub fn mq_status(&self) -> MgrMqStatus {
        MgrMqStatus {
            uldata: { self.uldata.lock().unwrap().status() },
            dldata: { self.dldata.lock().unwrap().status() },
            dldata_resp: { self.dldata_resp.lock().unwrap().status() },
            dldata_result: { self.dldata_result.lock().unwrap().status() },
            ctrl: QueueStatus::Closed,
        }
    }

    /// To close the manager queues.
    pub async fn close(&self) -> Result<(), Box<dyn StdError + Send + Sync>> {
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

    /// Send uplink data to the application.
    pub fn send_uldata(&self, data: &UlData) -> Result<(), Box<dyn StdError>> {
        let payload = serde_json::to_vec(data)?;
        let queue = { (*self.uldata.lock().unwrap()).clone() };
        task::spawn(async move {
            let _ = queue.send_msg(payload).await;
        });
        Ok(())
    }

    /// Send downlink response for a downlink data to the application.
    pub async fn send_dldata_resp(
        &self,
        data: &DlDataResp,
    ) -> Result<(), Box<dyn StdError + Send + Sync>> {
        let payload = serde_json::to_vec(data)?;
        let queue = { (*self.dldata_resp.lock().unwrap()).clone() };
        queue.send_msg(payload).await
    }

    /// Send the downlink data process result to the application.
    pub async fn send_dldata_result(
        &self,
        data: &DlDataResult,
    ) -> Result<(), Box<dyn StdError + Send + Sync>> {
        let payload = serde_json::to_vec(data)?;
        let queue = { (*self.dldata_result.lock().unwrap()).clone() };
        queue.send_msg(payload).await
    }
}

#[async_trait]
impl QueueEventHandler for MgrMqEventHandler {
    async fn on_error(&self, _queue: Arc<dyn GmqQueue>, _err: Box<dyn StdError + Send + Sync>) {}

    async fn on_status(&self, _queue: Arc<dyn GmqQueue>, _status: QueueStatus) {
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
}

#[async_trait]
impl MessageHandler for MgrMqEventHandler {
    // Validate and decode data.
    async fn on_message(&self, queue: Arc<dyn GmqQueue>, msg: Box<dyn Message>) {
        const FN_NAME: &'static str = "ApplicationMgr.on_message";

        let queue_name = queue.name();
        if queue_name.cmp(self.mgr.dldata.lock().unwrap().name()) == Ordering::Equal {
            let data = match parse_dldata_msg(msg.payload()) {
                Err(resp) => {
                    warn!("[{}] invalid format from {}", FN_NAME, queue_name);
                    if let Err(e) = msg.ack().await {
                        error!("[{}] ACK message error: {}", FN_NAME, e);
                    }
                    if let Err(e) = self.mgr.send_dldata_resp(&resp).await {
                        error!("[{}] send response error: {}", FN_NAME, e);
                    }
                    return;
                }
                Ok(data) => data,
            };
            let handler = { self.mgr.handler.lock().unwrap().clone() };
            match handler.on_dldata(&self.mgr, Box::new(data)).await {
                Err(_) => {
                    if let Err(e) = msg.nack().await {
                        error!("[{}] NACK message error: {}", FN_NAME, e);
                    }
                }
                Ok(resp) => {
                    if let Err(e) = msg.ack().await {
                        error!("[{}] ACK message error: {}", FN_NAME, e);
                    }
                    if let Err(e) = self.mgr.send_dldata_resp(resp.as_ref()).await {
                        error!("[{}] send response error: {}", FN_NAME, e);
                    }
                }
            }
        }
    }
}

/// Parses downlink data from the application and responds error for wrong format data.
fn parse_dldata_msg(msg: &[u8]) -> Result<DlData, DlDataResp> {
    let mut data = match serde_json::from_slice::<DlData>(msg) {
        Err(_) => {
            return Err(DlDataResp {
                correlation_id: "".to_string(),
                error: Some(err::E_PARAM.to_string()),
                message: Some("invalid format".to_string()),
                ..Default::default()
            });
        }
        Ok(data) => data,
    };

    if data.correlation_id.len() == 0 {
        return Err(DlDataResp {
            correlation_id: data.correlation_id.clone(),
            error: Some(err::E_PARAM.to_string()),
            message: Some("invalid `correlationId`".to_string()),
            ..Default::default()
        });
    }
    match data.device_id.as_ref() {
        None => {
            match data.network_code.as_ref() {
                None => {
                    return Err(DlDataResp {
                        correlation_id: data.correlation_id.clone(),
                        error: Some(err::E_PARAM.to_string()),
                        message: Some("missing `networkCode`".to_string()),
                        ..Default::default()
                    });
                }
                Some(code) => {
                    let code = code.to_lowercase();
                    match strings::is_code(code.as_str()) {
                        false => {
                            return Err(DlDataResp {
                                correlation_id: data.correlation_id.clone(),
                                error: Some(err::E_PARAM.to_string()),
                                message: Some("invalid `networkCode`".to_string()),
                                ..Default::default()
                            });
                        }
                        true => {
                            data.network_code = Some(code);
                            ()
                        }
                    }
                }
            }
            match data.network_addr.as_ref() {
                None => {
                    return Err(DlDataResp {
                        correlation_id: data.correlation_id.clone(),
                        error: Some(err::E_PARAM.to_string()),
                        message: Some("missing `networkAddr`".to_string()),
                        ..Default::default()
                    });
                }
                Some(addr) => match addr.len() {
                    0 => {
                        return Err(DlDataResp {
                            correlation_id: data.correlation_id.clone(),
                            error: Some(err::E_PARAM.to_string()),
                            message: Some("invalid `networkAddr`".to_string()),
                            ..Default::default()
                        });
                    }
                    _ => {
                        data.network_addr = Some(addr.to_lowercase());
                        ()
                    }
                },
            }
        }
        Some(device_id) => match device_id.len() {
            0 => {
                return Err(DlDataResp {
                    correlation_id: data.correlation_id.clone(),
                    error: Some(err::E_PARAM.to_string()),
                    message: Some("invalid `deviceId`".to_string()),
                    ..Default::default()
                });
            }
            _ => (),
        },
    }
    if data.data.len() > 0 {
        if let Err(_) = hex::decode(data.data.as_str()) {
            return Err(DlDataResp {
                correlation_id: data.correlation_id.clone(),
                error: Some(err::E_PARAM.to_string()),
                message: Some("invalid `data`".to_string()),
                ..Default::default()
            });
        }
        data.data = data.data.to_lowercase();
    }
    Ok(data)
}
