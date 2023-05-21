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
use crate::models::{
    application_dldata::{
        ApplicationDlData, UpdateQueryCond as ApplicationDlDataCond,
        Updates as ApplicationDlDataUpdate,
    },
    application_uldata::ApplicationUlData,
    network_dldata::{
        NetworkDlData, UpdateQueryCond as NetworkDlDataCond, Updates as NetworkDlDataUpdate,
    },
    network_uldata::NetworkUlData,
    Model,
};

#[derive(Clone)]
struct DataHandler {
    model: Arc<dyn Model>,
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
    data_id: String,
    proc: String,
    #[serde(rename = "pub")]
    publish: String,
    #[serde(rename = "unitCode")]
    unit_code: Option<String>,
    #[serde(rename = "networkCode")]
    network_code: String,
    #[serde(rename = "networkAddr")]
    network_addr: String,
    #[serde(rename = "unitId")]
    unit_id: String,
    #[serde(rename = "deviceId")]
    device_id: String,
    time: String,
    profile: String,
    data: String,
    extension: Option<Map<String, Value>>,
}

#[derive(Deserialize)]
struct AppDlData {
    #[serde(rename = "dataId")]
    data_id: String,
    proc: String,
    status: i32,
    #[serde(rename = "unitId")]
    unit_id: String,
    #[serde(rename = "deviceId")]
    device_id: Option<String>,
    #[serde(rename = "networkCode")]
    network_code: Option<String>,
    #[serde(rename = "networkAddr")]
    network_addr: Option<String>,
    profile: String,
    data: String,
    extension: Option<Map<String, Value>>,
}

#[derive(Deserialize)]
struct AppDlDataResult {
    #[serde(rename = "dataId")]
    data_id: String,
    resp: String,
    status: i32,
}

#[derive(Deserialize)]
struct NetUlData {
    #[serde(rename = "dataId")]
    data_id: String,
    proc: String,
    #[serde(rename = "unitCode")]
    unit_code: Option<String>,
    #[serde(rename = "networkCode")]
    network_code: String,
    #[serde(rename = "networkAddr")]
    network_addr: String,
    #[serde(rename = "unitId")]
    unit_id: Option<String>,
    #[serde(rename = "deviceId")]
    device_id: Option<String>,
    time: String,
    profile: String,
    data: String,
    extension: Option<Map<String, Value>>,
}

#[derive(Deserialize)]
struct NetDlData {
    #[serde(rename = "dataId")]
    data_id: String,
    proc: String,
    #[serde(rename = "pub")]
    publish: String,
    status: i32,
    #[serde(rename = "unitId")]
    unit_id: String,
    #[serde(rename = "deviceId")]
    device_id: String,
    #[serde(rename = "networkCode")]
    network_code: String,
    #[serde(rename = "networkAddr")]
    network_addr: String,
    profile: String,
    data: String,
    extension: Option<Map<String, Value>>,
}

#[derive(Deserialize)]
struct NetDlDataResult {
    #[serde(rename = "dataId")]
    data_id: String,
    resp: String,
    status: i32,
}

const QUEUE_NAME: &'static str = "broker.data";

/// Create a receive queue to receive data from `broker.data` queue.
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
            RecvDataMsg::AppDlData { data } => {
                let data = ApplicationDlData {
                    data_id: data.data_id,
                    proc: match DateTime::parse_from_rfc3339(data.proc.as_str()) {
                        Err(e) => {
                            warn!(
                                "[{}] {} parse application_dldata proc \"{}\" error: {}",
                                FN_NAME, queue_name, data.proc, e
                            );
                            if let Err(e) = msg.ack().await {
                                error!("[{}] {} ACK error: {}", FN_NAME, queue_name, e);
                            }
                            return;
                        }
                        Ok(proc) => proc.into(),
                    },
                    resp: None,
                    status: data.status,
                    unit_id: data.unit_id,
                    device_id: data.device_id,
                    network_code: data.network_code,
                    network_addr: data.network_addr,
                    profile: data.profile,
                    data: data.data,
                    extension: data.extension,
                };
                let mut is_err = false;
                if let Err(e) = self.model.application_dldata().add(&data).await {
                    error!(
                        "[{}] {} add application_dldata error: {}",
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
            RecvDataMsg::AppDlDataResult { data } => {
                // FIXME: wait 1 second to wait for the associated dldata has been written in DB.
                time::sleep(Duration::from_secs(1)).await;

                let cond = ApplicationDlDataCond {
                    data_id: data.data_id.as_str(),
                };
                let updates = ApplicationDlDataUpdate {
                    resp: match DateTime::parse_from_rfc3339(data.resp.as_str()) {
                        Err(e) => {
                            warn!(
                                "[{}] {} parse application_dldata resp \"{}\" error: {}",
                                FN_NAME, queue_name, data.resp, e
                            );
                            if let Err(e) = msg.ack().await {
                                error!("[{}] {} ACK error: {}", FN_NAME, queue_name, e);
                            }
                            return;
                        }
                        Ok(resp) => resp.into(),
                    },
                    status: data.status,
                };
                let mut is_err = false;
                if let Err(e) = self
                    .model
                    .application_dldata()
                    .update(&cond, &updates)
                    .await
                {
                    error!(
                        "[{}] {} update application_dldata error: {}",
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
            RecvDataMsg::AppUlData { data } => {
                let data = ApplicationUlData {
                    data_id: data.data_id,
                    proc: match DateTime::parse_from_rfc3339(data.proc.as_str()) {
                        Err(e) => {
                            warn!(
                                "[{}] {} parse application_uldata proc \"{}\" error: {}",
                                FN_NAME, queue_name, data.proc, e
                            );
                            if let Err(e) = msg.ack().await {
                                error!("[{}] {} ACK error: {}", FN_NAME, queue_name, e);
                            }
                            return;
                        }
                        Ok(proc) => proc.into(),
                    },
                    publish: match DateTime::parse_from_rfc3339(data.publish.as_str()) {
                        Err(e) => {
                            warn!(
                                "[{}] {} parse application_uldata publish \"{}\" error: {}",
                                FN_NAME, queue_name, data.publish, e
                            );
                            if let Err(e) = msg.ack().await {
                                error!("[{}] {} ACK error: {}", FN_NAME, queue_name, e);
                            }
                            return;
                        }
                        Ok(publish) => publish.into(),
                    },
                    unit_code: data.unit_code,
                    network_code: data.network_code,
                    network_addr: data.network_addr,
                    unit_id: data.unit_id,
                    device_id: data.device_id,
                    time: match DateTime::parse_from_rfc3339(data.time.as_str()) {
                        Err(e) => {
                            warn!(
                                "[{}] {} parse application_uldata time \"{}\" error: {}",
                                FN_NAME, queue_name, data.time, e
                            );
                            if let Err(e) = msg.ack().await {
                                error!("[{}] {} ACK error: {}", FN_NAME, queue_name, e);
                            }
                            return;
                        }
                        Ok(time) => time.into(),
                    },
                    profile: data.profile,
                    data: data.data,
                    extension: data.extension,
                };
                let mut is_err = false;
                if let Err(e) = self.model.application_uldata().add(&data).await {
                    error!(
                        "[{}] {} add application_uldata error: {}",
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
            RecvDataMsg::NetDlData { data } => {
                let data = NetworkDlData {
                    data_id: data.data_id,
                    proc: match DateTime::parse_from_rfc3339(data.proc.as_str()) {
                        Err(e) => {
                            warn!(
                                "[{}] {} parse network_dldata proc \"{}\" error: {}",
                                FN_NAME, queue_name, data.proc, e
                            );
                            if let Err(e) = msg.ack().await {
                                error!("[{}] {} ACK error: {}", FN_NAME, queue_name, e);
                            }
                            return;
                        }
                        Ok(proc) => proc.into(),
                    },
                    publish: match DateTime::parse_from_rfc3339(data.publish.as_str()) {
                        Err(e) => {
                            warn!(
                                "[{}] {} parse network_dldata publish \"{}\" error: {}",
                                FN_NAME, queue_name, data.publish, e
                            );
                            if let Err(e) = msg.ack().await {
                                error!("[{}] {} ACK error: {}", FN_NAME, queue_name, e);
                            }
                            return;
                        }
                        Ok(publish) => publish.into(),
                    },
                    resp: None,
                    status: data.status,
                    unit_id: data.unit_id,
                    device_id: data.device_id,
                    network_code: data.network_code,
                    network_addr: data.network_addr,
                    profile: data.profile,
                    data: data.data,
                    extension: data.extension,
                };
                let mut is_err = false;
                if let Err(e) = self.model.network_dldata().add(&data).await {
                    error!(
                        "[{}] {} add network_dldata error: {}",
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
            RecvDataMsg::NetDlDataResult { data } => {
                // FIXME: wait 1 second to wait for the associated dldata has been written in DB.
                time::sleep(Duration::from_secs(1)).await;

                let cond = NetworkDlDataCond {
                    data_id: data.data_id.as_str(),
                };
                let updates = NetworkDlDataUpdate {
                    resp: match DateTime::parse_from_rfc3339(data.resp.as_str()) {
                        Err(e) => {
                            warn!(
                                "[{}] {} parse network_dldata resp \"{}\" error: {}",
                                FN_NAME, queue_name, data.resp, e
                            );
                            if let Err(e) = msg.ack().await {
                                error!("[{}] {} ACK error: {}", FN_NAME, queue_name, e);
                            }
                            return;
                        }
                        Ok(resp) => resp.into(),
                    },
                    status: data.status,
                };
                let mut is_err = false;
                if let Err(e) = self.model.network_dldata().update(&cond, &updates).await {
                    error!(
                        "[{}] {} update network_dldata error: {}",
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
            RecvDataMsg::NetUlData { data } => {
                let data = NetworkUlData {
                    data_id: data.data_id,
                    proc: match DateTime::parse_from_rfc3339(data.proc.as_str()) {
                        Err(e) => {
                            warn!(
                                "[{}] {} parse network_uldata proc \"{}\" error: {}",
                                FN_NAME, queue_name, data.proc, e
                            );
                            if let Err(e) = msg.ack().await {
                                error!("[{}] {} ACK error: {}", FN_NAME, queue_name, e);
                            }
                            return;
                        }
                        Ok(proc) => proc.into(),
                    },
                    unit_code: data.unit_code,
                    network_code: data.network_code,
                    network_addr: data.network_addr,
                    unit_id: data.unit_id,
                    device_id: data.device_id,
                    time: match DateTime::parse_from_rfc3339(data.time.as_str()) {
                        Err(e) => {
                            warn!(
                                "[{}] {} parse network_uldata time \"{}\" error: {}",
                                FN_NAME, queue_name, data.time, e
                            );
                            if let Err(e) = msg.ack().await {
                                error!("[{}] {} ACK error: {}", FN_NAME, queue_name, e);
                            }
                            return;
                        }
                        Ok(time) => time.into(),
                    },
                    profile: data.profile,
                    data: data.data,
                    extension: data.extension,
                };
                let mut is_err = false;
                if let Err(e) = self.model.network_uldata().add(&data).await {
                    error!(
                        "[{}] {} add network_uldata error: {}",
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
