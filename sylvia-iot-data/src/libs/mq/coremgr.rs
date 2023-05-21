use std::{
    collections::HashMap,
    error::Error as StdError,
    io::{Error as IoError, ErrorKind},
    sync::Arc,
    time::Duration,
};

use async_trait::async_trait;
use chrono::DateTime;
use general_mq::{
    queue::{Event, EventHandler, GmqQueue, Message, Status},
    Queue,
};
use log::{error, info, warn};
use serde::Deserialize;
use serde_json::{Map, Value};
use tokio::time;

use super::{super::config::DataData as DataMqConfig, new_data_queue, Connection};
use crate::models::{coremgr_opdata::CoremgrOpData, Model};

#[derive(Clone)]
struct DataHandler {
    model: Arc<dyn Model>,
}

#[derive(Deserialize)]
#[serde(tag = "kind")]
enum RecvDataMsg {
    #[serde(rename = "operation")]
    Operation { data: CmOpData },
}

#[derive(Deserialize)]
struct CmOpData {
    #[serde(rename = "dataId")]
    data_id: String,
    #[serde(rename = "reqTime")]
    req_time: String,
    #[serde(rename = "resTime")]
    res_time: String,
    #[serde(rename = "latencyMs")]
    latency_ms: i64,
    status: i32,
    #[serde(rename = "sourceIp")]
    source_ip: String,
    method: String,
    path: String,
    body: Option<Map<String, Value>>,
    #[serde(rename = "userId")]
    user_id: String,
    #[serde(rename = "clientId")]
    client_id: String,
    #[serde(rename = "errCode")]
    err_code: Option<String>,
    #[serde(rename = "errMessage")]
    err_message: Option<String>,
}

const QUEUE_NAME: &'static str = "coremgr.data";

/// Create a receive queue to receive data from `coremgr.data` queue.
pub fn new(
    model: Arc<dyn Model>,
    mq_conns: &mut HashMap<String, Connection>,
    config: &DataMqConfig,
) -> Result<Queue, Box<dyn StdError>> {
    let handler = Arc::new(DataHandler { model });
    match new_data_queue(mq_conns, config, QUEUE_NAME, handler) {
        Err(e) => Err(Box::new(IoError::new(ErrorKind::Other, e))),
        Ok(q) => Ok(q),
    }
}

#[async_trait]
impl EventHandler for DataHandler {
    async fn on_event(&self, queue: Arc<dyn GmqQueue>, ev: Event) {
        const FN_NAME: &'static str = "DataHandler::on_event";
        let queue_name = queue.name();

        match ev {
            Event::Error(e) => error!("[{}] {} error: {}", FN_NAME, queue_name, e),
            Event::Status(status) => match status {
                Status::Connected => info!("[{}] {} connected", queue_name, FN_NAME),
                _ => warn!("[{}] {} status to {:?}", FN_NAME, queue_name, status),
            },
        }
    }

    async fn on_message(&self, queue: Arc<dyn GmqQueue>, msg: Box<dyn Message>) {
        const FN_NAME: &'static str = "DataHandler::on_message";
        let queue_name = queue.name();

        let data_msg = match serde_json::from_slice::<RecvDataMsg>(msg.payload()) {
            Err(e) => {
                let src_str: String = String::from_utf8_lossy(msg.payload()).into();
                warn!(
                    "[{}] {} parse JSON error: {}, src: {}",
                    FN_NAME, queue_name, e, src_str
                );
                if let Err(e) = msg.ack().await {
                    error!("[{}] {} ACK error: {}", FN_NAME, queue_name, e);
                }
                return;
            }
            Ok(msg) => msg,
        };
        match data_msg {
            RecvDataMsg::Operation { data } => {
                let data = CoremgrOpData {
                    data_id: data.data_id,
                    req_time: match DateTime::parse_from_rfc3339(data.req_time.as_str()) {
                        Err(e) => {
                            warn!(
                                "[{}] {} parse coremgr_opdata req_time \"{}\" error: {}",
                                FN_NAME, queue_name, data.req_time, e
                            );
                            if let Err(e) = msg.ack().await {
                                error!("[{}] {} ACK error: {}", FN_NAME, queue_name, e);
                            }
                            return;
                        }
                        Ok(req_time) => req_time.into(),
                    },
                    res_time: match DateTime::parse_from_rfc3339(data.res_time.as_str()) {
                        Err(e) => {
                            warn!(
                                "[{}] {} parse coremgr_opdata res_time \"{}\" error: {}",
                                FN_NAME, queue_name, data.res_time, e
                            );
                            if let Err(e) = msg.ack().await {
                                error!("[{}] {} ACK error: {}", FN_NAME, queue_name, e);
                            }
                            return;
                        }
                        Ok(res_time) => res_time.into(),
                    },
                    latency_ms: data.latency_ms,
                    status: data.status,
                    source_ip: data.source_ip,
                    method: data.method,
                    path: data.path,
                    body: data.body,
                    user_id: data.user_id,
                    client_id: data.client_id,
                    err_code: data.err_code,
                    err_message: data.err_message,
                };
                let mut is_err = false;
                if let Err(e) = self.model.coremgr_opdata().add(&data).await {
                    error!(
                        "[{}] {} add coremgr_opdata error: {}",
                        FN_NAME, queue_name, e
                    );
                    is_err = true;
                }
                if is_err {
                    time::sleep(Duration::from_secs(1)).await;
                    if let Err(e) = msg.nack().await {
                        error!("[{}] {} NACK error: {}", FN_NAME, queue_name, e);
                    }
                    return;
                }
            }
        }
        if let Err(e) = msg.ack().await {
            error!("[{}] {} ACK error: {}", FN_NAME, queue_name, e);
        }
    }
}
