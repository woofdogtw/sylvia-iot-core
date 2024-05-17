use std::{
    collections::HashMap,
    error::Error as StdError,
    io::{Error as IoError, ErrorKind},
    sync::{Arc, Mutex},
    time::Duration,
};

use async_trait::async_trait;
use axum::{
    body::{Body, Bytes},
    extract::State,
    http::{header, StatusCode},
    response::IntoResponse,
    Extension,
};
use chrono::Utc;
use log::{error, info, warn};
use serde::{Deserialize, Serialize};
use serde_json::{self, Map};
use tokio::time;
use url::Url;

use general_mq::{
    queue::{EventHandler as QueueEventHandler, GmqQueue, Message, MessageHandler, Status},
    Queue,
};
use sylvia_iot_corelib::{
    constants::ContentType,
    err::ErrResp,
    http::{Json, Path, Query},
    role::Role,
    strings::{self, hex_addr_to_u128, time_str, u128_to_addr},
};

use super::{
    super::{
        super::{middleware::GetTokenInfoData, ErrReq, State as AppState},
        lib::{check_device, check_unit, gen_mgr_key},
    },
    request, response,
};
use crate::{
    libs::{
        config::BrokerCtrl as CfgCtrl,
        mq::{self, Connection},
    },
    models::{
        device::{
            self, Device, ListOptions, ListQueryCond, QueryCond, SortCond, SortKey,
            UpdateQueryCond, Updates,
        },
        device_route, dldata_buffer,
        network::{Network, QueryCond as NetworkQueryCond},
        Cache,
    },
};

#[derive(Deserialize, Serialize)]
#[serde(tag = "operation")]
enum RecvCtrlMsg {
    #[serde(rename = "del-device")]
    DelDevice { new: CtrlDelDevice },
    #[serde(rename = "del-device-bulk")]
    DelDeviceBulk { new: CtrlDelDeviceBulk },
}

/// Control message inside broker cluster.
#[derive(Serialize)]
#[serde(untagged)]
enum SendCtrlMsg {
    DelDevice {
        operation: String,
        new: CtrlDelDevice,
    },
    DelDeviceBulk {
        operation: String,
        new: CtrlDelDeviceBulk,
    },
}

struct CtrlMsgOp;

#[derive(Deserialize, Serialize)]
struct CtrlDelDevice {
    #[serde(rename = "unitId")]
    unit_id: String,
    #[serde(rename = "unitCode")]
    unit_code: Option<String>,
    #[serde(rename = "networkId")]
    network_id: String,
    #[serde(rename = "networkCode")]
    network_code: String,
    #[serde(rename = "networkAddr")]
    network_addr: String,
    #[serde(rename = "deviceId")]
    device_id: String,
}

#[derive(Deserialize, Serialize)]
struct CtrlDelDeviceBulk {
    #[serde(rename = "unitId")]
    unit_id: String,
    #[serde(rename = "unitCode")]
    unit_code: Option<String>,
    #[serde(rename = "networkId")]
    network_id: String,
    #[serde(rename = "networkCode")]
    network_code: String,
    #[serde(rename = "networkAddrs")]
    network_addrs: Vec<String>,
    #[serde(rename = "deviceIds")]
    device_ids: Vec<String>,
}

struct CtrlSenderHandler {
    cache: Option<Arc<dyn Cache>>,
}

struct CtrlReceiverHandler {
    cache: Option<Arc<dyn Cache>>,
}

/// Control message from broker to network servers.
#[derive(Serialize)]
#[serde(untagged)]
enum SendNetCtrlMsg {
    AddDevice {
        time: String,
        operation: String,
        new: NetCtrlAddr,
    },
    AddDeviceBulk {
        time: String,
        operation: String,
        new: NetCtrlAddrs,
    },
    AddDeviceRange {
        time: String,
        operation: String,
        new: NetCtrlAddrRange,
    },
    DelDevice {
        time: String,
        operation: String,
        new: NetCtrlAddr,
    },
    DelDeviceBulk {
        time: String,
        operation: String,
        new: NetCtrlAddrs,
    },
    DelDeviceRange {
        time: String,
        operation: String,
        new: NetCtrlAddrRange,
    },
}

struct NetCtrlMsgOp;

/// Shared structure to keep simple design.
#[derive(Serialize)]
struct NetCtrlAddr {
    #[serde(rename = "networkAddr")]
    network_addr: String,
}

/// Shared structure to keep simple design.
#[derive(Serialize)]
struct NetCtrlAddrs {
    #[serde(rename = "networkAddrs")]
    network_addrs: Vec<String>,
}

/// Shared structure to keep simple design.
#[derive(Serialize)]
struct NetCtrlAddrRange {
    #[serde(rename = "startAddr")]
    pub start_addr: String,
    #[serde(rename = "endAddr")]
    pub end_addr: String,
}

impl CtrlMsgOp {
    const DEL_DEVICE: &'static str = "del-device";
    const DEL_DEVICE_BULK: &'static str = "del-device-bulk";
}

impl NetCtrlMsgOp {
    const ADD_DEVICE: &'static str = "add-device";
    const ADD_DEVICE_BULK: &'static str = "add-device-bulk";
    const ADD_DEVICE_RANGE: &'static str = "add-device-range";
    const DEL_DEVICE: &'static str = "del-device";
    const DEL_DEVICE_BULK: &'static str = "del-device-bulk";
    const DEL_DEVICE_RANGE: &'static str = "del-device-range";
}

const BULK_MAX: usize = 1024;
const LIST_LIMIT_DEFAULT: u64 = 100;
const LIST_CURSOR_MAX: u64 = 100;
const ID_RAND_LEN: usize = 8;
const CTRL_QUEUE_NAME: &'static str = "device";

/// Initialize channels.
pub async fn init(state: &AppState, ctrl_conf: &CfgCtrl) -> Result<(), Box<dyn StdError>> {
    const FN_NAME: &'static str = "init";

    let q = new_ctrl_receiver(state, ctrl_conf)?;
    {
        state
            .ctrl_receivers
            .lock()
            .unwrap()
            .insert(CTRL_QUEUE_NAME.to_string(), q.clone());
    }

    let ctrl_sender = { state.ctrl_senders.device.lock().unwrap().clone() };
    // Wait for connected.
    for _ in 0..500 {
        if ctrl_sender.status() == Status::Connected && q.status() == Status::Connected {
            break;
        }
        time::sleep(Duration::from_millis(10)).await;
    }
    if ctrl_sender.status() != Status::Connected {
        error!(
            "[{}] {} control sender not connected",
            FN_NAME, CTRL_QUEUE_NAME
        );
        return Err(Box::new(IoError::new(
            ErrorKind::NotConnected,
            format!("control sender {} not connected", CTRL_QUEUE_NAME),
        )));
    }
    if q.status() != Status::Connected {
        error!(
            "[{}] {} control receiver not connected",
            FN_NAME, CTRL_QUEUE_NAME
        );
        return Err(Box::new(IoError::new(
            ErrorKind::NotConnected,
            format!("control receiver {} not connected", CTRL_QUEUE_NAME),
        )));
    }

    Ok(())
}

/// Create control channel sender queue.
pub fn new_ctrl_sender(
    conn_pool: &Arc<Mutex<HashMap<String, Connection>>>,
    config: &CfgCtrl,
    cache: Option<Arc<dyn Cache>>,
) -> Result<Arc<Mutex<Queue>>, Box<dyn StdError>> {
    let url = match config.url.as_ref() {
        None => {
            return Err(Box::new(IoError::new(
                ErrorKind::InvalidInput,
                "empty control url",
            )))
        }
        Some(url) => match Url::parse(url.as_str()) {
            Err(e) => return Err(Box::new(e)),
            Ok(url) => url,
        },
    };

    match mq::control::new(
        conn_pool.clone(),
        &url,
        config.prefetch,
        CTRL_QUEUE_NAME,
        false,
        Arc::new(CtrlSenderHandler {
            cache: cache.clone(),
        }),
        Arc::new(CtrlSenderHandler { cache }),
    ) {
        Err(e) => Err(Box::new(IoError::new(ErrorKind::InvalidInput, e))),
        Ok(q) => Ok(Arc::new(Mutex::new(q))),
    }
}

/// Create control channel receiver queue.
pub fn new_ctrl_receiver(state: &AppState, config: &CfgCtrl) -> Result<Queue, Box<dyn StdError>> {
    let url = match config.url.as_ref() {
        None => {
            return Err(Box::new(IoError::new(
                ErrorKind::InvalidInput,
                "empty control url",
            )))
        }
        Some(url) => match Url::parse(url.as_str()) {
            Err(e) => return Err(Box::new(e)),
            Ok(url) => url,
        },
    };
    let handler = Arc::new(CtrlReceiverHandler {
        cache: state.cache.clone(),
    });
    match mq::control::new(
        state.mq_conns.clone(),
        &url,
        config.prefetch,
        CTRL_QUEUE_NAME,
        true,
        handler.clone(),
        handler,
    ) {
        Err(e) => Err(Box::new(IoError::new(ErrorKind::InvalidInput, e))),
        Ok(q) => Ok(q),
    }
}

/// `POST /{base}/api/v1/device`
pub async fn post_device(
    State(state): State<AppState>,
    Extension(token_info): Extension<GetTokenInfoData>,
    Json(body): Json<request::PostDeviceBody>,
) -> impl IntoResponse {
    const FN_NAME: &'static str = "post_device";

    let user_id = token_info.user_id.as_str();
    let roles = &token_info.roles;

    if body.data.unit_id.len() == 0 {
        return Err(ErrResp::ErrParam(Some(
            "`unitId` must with at least one character".to_string(),
        )));
    } else if body.data.network_id.len() == 0 {
        return Err(ErrResp::ErrParam(Some(
            "`networkId` must with at least one character".to_string(),
        )));
    } else if body.data.network_addr.len() == 0 {
        return Err(ErrResp::ErrParam(Some(
            "`networkAddr` must with at least one character".to_string(),
        )));
    }
    if let Some(profile) = body.data.profile.as_ref() {
        if profile.len() > 0 && !strings::is_code(profile.as_str()) {
            return Err(ErrResp::ErrParam(Some(
                "`profile` must be [A-Za-z0-9]{1}[A-Za-z0-9-_]*".to_string(),
            )));
        }
    }
    let unit_id = body.data.unit_id.as_str();
    let unit = match check_unit(FN_NAME, user_id, roles, unit_id, true, &state).await? {
        None => {
            return Err(ErrResp::Custom(
                ErrReq::UNIT_NOT_EXIST.0,
                ErrReq::UNIT_NOT_EXIST.1,
                None,
            ));
        }
        Some(unit) => unit,
    };
    let network_id = body.data.network_id.as_str();
    let network = match check_network(FN_NAME, unit_id, network_id, roles, &state).await? {
        None => {
            return Err(ErrResp::Custom(
                ErrReq::NETWORK_NOT_EXIST.0,
                ErrReq::NETWORK_NOT_EXIST.1,
                None,
            ));
        }
        Some(network) => network,
    };
    let network_addr = body.data.network_addr.as_str();
    if check_addr(FN_NAME, network_id, network_addr, &state)
        .await?
        .is_some()
    {
        return Err(ErrResp::Custom(
            ErrReq::NETWORK_ADDR_EXIST.0,
            ErrReq::NETWORK_ADDR_EXIST.1,
            None,
        ));
    }

    let now = Utc::now();
    let device_id = strings::random_id(&now, ID_RAND_LEN);
    let device = Device {
        device_id: device_id.clone(),
        unit_id: unit.unit_id,
        unit_code: match network.unit_id.as_ref() {
            None => None,
            Some(_) => Some(unit.code),
        },
        network_id: network.network_id,
        network_code: network.code.clone(),
        network_addr: body.data.network_addr.to_lowercase(),
        created_at: now,
        modified_at: now,
        profile: match body.data.profile.as_ref() {
            None => "".to_string(),
            Some(profile) => profile.clone(),
        },
        name: match body.data.name.as_ref() {
            None => "".to_string(),
            Some(name) => name.clone(),
        },
        info: match body.data.info.as_ref() {
            None => Map::new(),
            Some(info) => info.clone(),
        },
    };
    if let Err(e) = state.model.device().add(&device).await {
        error!("[{}] add error: {}", FN_NAME, e);
        return Err(ErrResp::ErrDb(Some(e.to_string())));
    }

    // Clear the not-exist device in cache.
    if state.cache.is_some() {
        let msg = SendCtrlMsg::DelDevice {
            operation: CtrlMsgOp::DEL_DEVICE.to_string(),
            new: CtrlDelDevice {
                unit_id: device.unit_id,
                unit_code: device.unit_code,
                network_id: device.network_id,
                network_code: device.network_code,
                network_addr: device.network_addr.clone(),
                device_id: device.device_id,
            },
        };
        send_del_ctrl_message(FN_NAME, &msg, &state).await?;
    }

    // Send message to the device's network server.
    let mgr_key = match network.unit_code.as_ref() {
        None => gen_mgr_key("", network.code.as_str()),
        Some(unit_code) => gen_mgr_key(unit_code.as_str(), network.code.as_str()),
    };
    let msg_op = NetCtrlMsgOp::ADD_DEVICE;
    let msg = SendNetCtrlMsg::AddDevice {
        time: time_str(&Utc::now()),
        operation: msg_op.to_string(),
        new: NetCtrlAddr {
            network_addr: device.network_addr,
        },
    };
    let _ = send_net_ctrl_message(FN_NAME, &msg, msg_op, &state, &mgr_key).await;

    Ok(Json(response::PostDevice {
        data: response::PostDeviceData { device_id },
    }))
}

/// `POST /{base}/api/v1/device/bulk`
pub async fn post_device_bulk(
    State(state): State<AppState>,
    Extension(token_info): Extension<GetTokenInfoData>,
    Json(mut body): Json<request::PostDeviceBulkBody>,
) -> impl IntoResponse {
    const FN_NAME: &'static str = "post_device_bulk";

    let user_id = token_info.user_id.as_str();
    let roles = &token_info.roles;

    if body.data.unit_id.len() == 0 {
        return Err(ErrResp::ErrParam(Some(
            "`unitId` must with at least one character".to_string(),
        )));
    } else if body.data.network_id.len() == 0 {
        return Err(ErrResp::ErrParam(Some(
            "`networkId` must with at least one character".to_string(),
        )));
    } else if body.data.network_addrs.len() == 0 {
        return Err(ErrResp::ErrParam(Some(
            "`networkAddrs` must with at least one address".to_string(),
        )));
    } else if body.data.network_addrs.len() > BULK_MAX {
        return Err(ErrResp::ErrParam(Some(format!(
            "`networkAddrs` cannot more than {}",
            BULK_MAX
        ))));
    }
    if let Some(profile) = body.data.profile.as_ref() {
        if profile.len() > 0 && !strings::is_code(profile.as_str()) {
            return Err(ErrResp::ErrParam(Some(
                "`profile` must be [A-Za-z0-9]{1}[A-Za-z0-9-_]*".to_string(),
            )));
        }
    }
    let mut addrs = vec![];
    for addr in body.data.network_addrs.iter() {
        if addr.len() == 0 {
            return Err(ErrResp::ErrParam(Some(
                "`networkAddrs` must be non-empty address array".to_string(),
            )));
        }
        addrs.push(addr.to_lowercase());
    }
    body.data.network_addrs = addrs;
    let unit_id = body.data.unit_id.as_str();
    let unit = match check_unit(FN_NAME, user_id, roles, unit_id, true, &state).await? {
        None => {
            return Err(ErrResp::Custom(
                ErrReq::UNIT_NOT_EXIST.0,
                ErrReq::UNIT_NOT_EXIST.1,
                None,
            ));
        }
        Some(unit) => unit,
    };
    let network_id = body.data.network_id.as_str();
    let network = match check_network(FN_NAME, unit_id, network_id, roles, &state).await? {
        None => {
            return Err(ErrResp::Custom(
                ErrReq::NETWORK_NOT_EXIST.0,
                ErrReq::NETWORK_NOT_EXIST.1,
                None,
            ));
        }
        Some(network) => network,
    };

    let mut devices = vec![];
    for network_addr in body.data.network_addrs.iter() {
        let now = Utc::now();
        let device = Device {
            device_id: strings::random_id(&now, ID_RAND_LEN),
            unit_id: unit.unit_id.clone(),
            unit_code: match network.unit_id.as_ref() {
                None => None,
                Some(_) => Some(unit.code.clone()),
            },
            network_id: network.network_id.clone(),
            network_code: network.code.clone(),
            network_addr: network_addr.clone(),
            created_at: now,
            modified_at: now,
            profile: match body.data.profile.as_ref() {
                None => "".to_string(),
                Some(profile) => profile.clone(),
            },
            name: network_addr.to_lowercase(),
            info: Map::new(),
        };
        devices.push(device);
    }
    if let Err(e) = state.model.device().add_bulk(&devices).await {
        error!("[{}] add error: {}", FN_NAME, e);
        return Err(ErrResp::ErrDb(Some(e.to_string())));
    }

    if state.cache.is_some() {
        let msg = SendCtrlMsg::DelDeviceBulk {
            operation: CtrlMsgOp::DEL_DEVICE_BULK.to_string(),
            new: CtrlDelDeviceBulk {
                unit_id: unit.unit_id,
                unit_code: network.unit_code.clone(),
                network_id: network.network_id,
                network_code: network.code.clone(),
                network_addrs: body.data.network_addrs.clone(),
                device_ids: devices.iter().map(|x| x.device_id.clone()).collect(),
            },
        };
        send_del_ctrl_message(FN_NAME, &msg, &state).await?;
    }

    // Send message to the device's network server.
    let mgr_key = match network.unit_code.as_ref() {
        None => gen_mgr_key("", network.code.as_str()),
        Some(unit_code) => gen_mgr_key(unit_code.as_str(), network.code.as_str()),
    };
    let msg_op = NetCtrlMsgOp::ADD_DEVICE_BULK;
    let msg = SendNetCtrlMsg::AddDeviceBulk {
        time: time_str(&Utc::now()),
        operation: msg_op.to_string(),
        new: NetCtrlAddrs {
            network_addrs: body.data.network_addrs.clone(),
        },
    };
    let _ = send_net_ctrl_message(FN_NAME, &msg, msg_op, &state, &mgr_key).await;

    Ok(StatusCode::NO_CONTENT)
}

/// `POST /{base}/api/v1/device/bulk-delete`
pub async fn post_device_bulk_del(
    State(state): State<AppState>,
    Extension(token_info): Extension<GetTokenInfoData>,
    Json(mut body): Json<request::PostDeviceBulkBody>,
) -> impl IntoResponse {
    const FN_NAME: &'static str = "post_device_bulk_del";

    let user_id = token_info.user_id.as_str();
    let roles = &token_info.roles;

    if body.data.unit_id.len() == 0 {
        return Err(ErrResp::ErrParam(Some(
            "`unitId` must with at least one character".to_string(),
        )));
    } else if body.data.network_id.len() == 0 {
        return Err(ErrResp::ErrParam(Some(
            "`networkId` must with at least one character".to_string(),
        )));
    } else if body.data.network_addrs.len() == 0 {
        return Err(ErrResp::ErrParam(Some(
            "`networkAddrs` must with at least one address".to_string(),
        )));
    } else if body.data.network_addrs.len() > BULK_MAX {
        return Err(ErrResp::ErrParam(Some(format!(
            "`networkAddrs` cannot more than {}",
            BULK_MAX
        ))));
    }
    let mut addrs = vec![];
    for addr in body.data.network_addrs.iter() {
        if addr.len() == 0 {
            return Err(ErrResp::ErrParam(Some(
                "`networkAddrs` must be non-empty address array".to_string(),
            )));
        }
        addrs.push(addr.to_lowercase());
    }
    body.data.network_addrs = addrs;
    let unit_id = body.data.unit_id.as_str();
    if check_unit(FN_NAME, user_id, roles, unit_id, true, &state)
        .await?
        .is_none()
    {
        return Err(ErrResp::Custom(
            ErrReq::UNIT_NOT_EXIST.0,
            ErrReq::UNIT_NOT_EXIST.1,
            None,
        ));
    }
    let network_id = body.data.network_id.as_str();
    let network = match check_network(FN_NAME, unit_id, network_id, roles, &state).await? {
        None => {
            return Err(ErrResp::Custom(
                ErrReq::NETWORK_NOT_EXIST.0,
                ErrReq::NETWORK_NOT_EXIST.1,
                None,
            ));
        }
        Some(network) => network,
    };

    del_device_rsc_bulk(FN_NAME, &body.data, &network, &state).await?;

    // Send message to the device's network server.
    let mgr_key = match network.unit_code.as_ref() {
        None => gen_mgr_key("", network.code.as_str()),
        Some(unit_code) => gen_mgr_key(unit_code.as_str(), network.code.as_str()),
    };
    let msg_op = NetCtrlMsgOp::DEL_DEVICE_BULK;
    let msg = SendNetCtrlMsg::DelDeviceBulk {
        time: time_str(&Utc::now()),
        operation: msg_op.to_string(),
        new: NetCtrlAddrs {
            network_addrs: body.data.network_addrs.clone(),
        },
    };
    let _ = send_net_ctrl_message(FN_NAME, &msg, msg_op, &state, &mgr_key).await;

    Ok(StatusCode::NO_CONTENT)
}

/// `POST /{base}/api/v1/device/range`
pub async fn post_device_range(
    State(state): State<AppState>,
    Extension(token_info): Extension<GetTokenInfoData>,
    Json(body): Json<request::PostDeviceRangeBody>,
) -> impl IntoResponse {
    const FN_NAME: &'static str = "post_device_range";

    let user_id = token_info.user_id.as_str();
    let roles = &token_info.roles;

    if body.data.unit_id.len() == 0 {
        return Err(ErrResp::ErrParam(Some(
            "`unitId` must with at least one character".to_string(),
        )));
    } else if body.data.network_id.len() == 0 {
        return Err(ErrResp::ErrParam(Some(
            "`networkId` must with at least one character".to_string(),
        )));
    } else if body.data.start_addr.len() != body.data.end_addr.len() {
        return Err(ErrResp::ErrParam(Some(
            "`startAddr` and `endAddr` must have the same length".to_string(),
        )));
    }
    if let Some(profile) = body.data.profile.as_ref() {
        if profile.len() > 0 && !strings::is_code(profile.as_str()) {
            return Err(ErrResp::ErrParam(Some(
                "`profile` must be [A-Za-z0-9]{1}[A-Za-z0-9-_]*".to_string(),
            )));
        }
    }
    let start_addr = match hex_addr_to_u128(body.data.start_addr.as_str()) {
        Err(e) => return Err(ErrResp::ErrParam(Some(e.to_string()))),
        Ok(addr) => addr,
    };
    let mut end_addr = match hex_addr_to_u128(body.data.end_addr.as_str()) {
        Err(e) => return Err(ErrResp::ErrParam(Some(e.to_string()))),
        Ok(addr) => addr,
    };
    if start_addr > end_addr {
        return Err(ErrResp::ErrParam(Some(
            "`startAddr` cannot larger than `endAddr`".to_string(),
        )));
    } else if (end_addr - start_addr) as usize >= BULK_MAX {
        return Err(ErrResp::ErrParam(Some(format!(
            "numbers between `startAddr` and `endAddr` cannot more than {}",
            BULK_MAX
        ))));
    }

    let unit_id = body.data.unit_id.as_str();
    let unit = match check_unit(FN_NAME, user_id, roles, unit_id, true, &state).await? {
        None => {
            return Err(ErrResp::Custom(
                ErrReq::UNIT_NOT_EXIST.0,
                ErrReq::UNIT_NOT_EXIST.1,
                None,
            ));
        }
        Some(unit) => unit,
    };
    let network_id = body.data.network_id.as_str();
    let network = match check_network(FN_NAME, unit_id, network_id, roles, &state).await? {
        None => {
            return Err(ErrResp::Custom(
                ErrReq::NETWORK_NOT_EXIST.0,
                ErrReq::NETWORK_NOT_EXIST.1,
                None,
            ));
        }
        Some(network) => network,
    };

    let mut devices = vec![];
    end_addr += 1;
    let addr_len = body.data.start_addr.len();
    for addr in start_addr..end_addr {
        let now = Utc::now();
        let network_addr = u128_to_addr(addr, addr_len);
        let device = Device {
            device_id: strings::random_id(&now, ID_RAND_LEN),
            unit_id: unit.unit_id.clone(),
            unit_code: match network.unit_id.as_ref() {
                None => None,
                Some(_) => Some(unit.code.clone()),
            },
            network_id: network.network_id.clone(),
            network_code: network.code.clone(),
            network_addr: network_addr.clone(),
            created_at: now,
            modified_at: now,
            profile: match body.data.profile.as_ref() {
                None => "".to_string(),
                Some(profile) => profile.clone(),
            },
            name: network_addr,
            info: Map::new(),
        };
        devices.push(device);
    }
    if let Err(e) = state.model.device().add_bulk(&devices).await {
        error!("[{}] add error: {}", FN_NAME, e);
        return Err(ErrResp::ErrDb(Some(e.to_string())));
    }

    if state.cache.is_some() {
        let msg = SendCtrlMsg::DelDeviceBulk {
            operation: CtrlMsgOp::DEL_DEVICE_BULK.to_string(),
            new: CtrlDelDeviceBulk {
                unit_id: unit.unit_id,
                unit_code: network.unit_code.clone(),
                network_id: network.network_id,
                network_code: network.code.clone(),
                network_addrs: devices.iter().map(|x| x.network_addr.clone()).collect(),
                device_ids: devices.iter().map(|x| x.device_id.clone()).collect(),
            },
        };
        send_del_ctrl_message(FN_NAME, &msg, &state).await?;
    }

    // Send message to the device's network server.
    let mgr_key = match network.unit_code.as_ref() {
        None => gen_mgr_key("", network.code.as_str()),
        Some(unit_code) => gen_mgr_key(unit_code.as_str(), network.code.as_str()),
    };
    let msg_op = NetCtrlMsgOp::ADD_DEVICE_RANGE;
    let msg = SendNetCtrlMsg::AddDeviceRange {
        time: time_str(&Utc::now()),
        operation: msg_op.to_string(),
        new: NetCtrlAddrRange {
            start_addr: body.data.start_addr.clone(),
            end_addr: body.data.end_addr.clone(),
        },
    };
    let _ = send_net_ctrl_message(FN_NAME, &msg, msg_op, &state, &mgr_key).await;

    Ok(StatusCode::NO_CONTENT)
}

/// `POST /{base}/api/v1/device/range-delete`
pub async fn post_device_range_del(
    State(state): State<AppState>,
    Extension(token_info): Extension<GetTokenInfoData>,
    Json(body): Json<request::PostDeviceRangeBody>,
) -> impl IntoResponse {
    const FN_NAME: &'static str = "post_device_range_del";

    let user_id = token_info.user_id.as_str();
    let roles = &token_info.roles;

    if body.data.unit_id.len() == 0 {
        return Err(ErrResp::ErrParam(Some(
            "`unitId` must with at least one character".to_string(),
        )));
    } else if body.data.network_id.len() == 0 {
        return Err(ErrResp::ErrParam(Some(
            "`networkId` must with at least one character".to_string(),
        )));
    } else if body.data.start_addr.len() != body.data.end_addr.len() {
        return Err(ErrResp::ErrParam(Some(
            "`startAddr` and `endAddr` must have the same length".to_string(),
        )));
    }
    let start_addr = match hex_addr_to_u128(body.data.start_addr.as_str()) {
        Err(e) => return Err(ErrResp::ErrParam(Some(e.to_string()))),
        Ok(addr) => addr,
    };
    let mut end_addr = match hex_addr_to_u128(body.data.end_addr.as_str()) {
        Err(e) => return Err(ErrResp::ErrParam(Some(e.to_string()))),
        Ok(addr) => addr,
    };
    if start_addr > end_addr {
        return Err(ErrResp::ErrParam(Some(
            "`startAddr` cannot larger than `endAddr`".to_string(),
        )));
    } else if (end_addr - start_addr) as usize >= BULK_MAX {
        return Err(ErrResp::ErrParam(Some(format!(
            "numbers between `startAddr` and `endAddr` cannot more than {}",
            BULK_MAX
        ))));
    }

    let unit_id = body.data.unit_id.as_str();
    if check_unit(FN_NAME, user_id, roles, unit_id, true, &state)
        .await?
        .is_none()
    {
        return Err(ErrResp::Custom(
            ErrReq::UNIT_NOT_EXIST.0,
            ErrReq::UNIT_NOT_EXIST.1,
            None,
        ));
    }
    let network_id = body.data.network_id.as_str();
    let network = match check_network(FN_NAME, unit_id, network_id, roles, &state).await? {
        None => {
            return Err(ErrResp::Custom(
                ErrReq::NETWORK_NOT_EXIST.0,
                ErrReq::NETWORK_NOT_EXIST.1,
                None,
            ));
        }
        Some(network) => network,
    };

    let mut network_addrs = vec![];
    end_addr += 1;
    let addr_len = body.data.start_addr.len();
    for addr in start_addr..end_addr {
        network_addrs.push(u128_to_addr(addr, addr_len));
    }
    let rm_cond = request::PostDeviceBulkData {
        unit_id: body.data.unit_id.clone(),
        network_id: body.data.network_id.clone(),
        network_addrs,
        profile: None,
    };

    del_device_rsc_bulk(FN_NAME, &rm_cond, &network, &state).await?;

    // Send message to the device's network server.
    let mgr_key = match network.unit_code.as_ref() {
        None => gen_mgr_key("", network.code.as_str()),
        Some(unit_code) => gen_mgr_key(unit_code.as_str(), network.code.as_str()),
    };
    let msg_op = NetCtrlMsgOp::DEL_DEVICE_RANGE;
    let msg = SendNetCtrlMsg::DelDeviceRange {
        time: time_str(&Utc::now()),
        operation: msg_op.to_string(),
        new: NetCtrlAddrRange {
            start_addr: body.data.start_addr.clone(),
            end_addr: body.data.end_addr.clone(),
        },
    };
    let _ = send_net_ctrl_message(FN_NAME, &msg, msg_op, &state, &mgr_key).await;

    Ok(StatusCode::NO_CONTENT)
}

/// `GET /{base}/api/v1/device/count`
pub async fn get_device_count(
    State(state): State<AppState>,
    Extension(token_info): Extension<GetTokenInfoData>,
    Query(query): Query<request::GetDeviceCountQuery>,
) -> impl IntoResponse {
    const FN_NAME: &'static str = "get_device_count";

    let user_id = token_info.user_id.as_str();
    let roles = &token_info.roles;

    if !Role::is_role(roles, Role::ADMIN) && !Role::is_role(roles, Role::MANAGER) {
        match query.unit.as_ref() {
            None => return Err(ErrResp::ErrParam(Some("missing `unit`".to_string()))),
            Some(unit_id) => {
                if unit_id.len() == 0 {
                    return Err(ErrResp::ErrParam(Some("missing `unit`".to_string())));
                }
            }
        }
    }
    let unit_cond = match query.unit.as_ref() {
        None => None,
        Some(unit_id) => match unit_id.len() {
            0 => None,
            _ => {
                match check_unit(FN_NAME, user_id, roles, unit_id.as_str(), false, &state).await? {
                    None => {
                        return Err(ErrResp::Custom(
                            ErrReq::UNIT_NOT_EXIST.0,
                            ErrReq::UNIT_NOT_EXIST.1,
                            None,
                        ))
                    }
                    Some(_) => Some(unit_id.as_str()),
                }
            }
        },
    };
    let mut name_contains_cond = None;
    if let Some(contains) = query.contains.as_ref() {
        if contains.len() > 0 {
            name_contains_cond = Some(contains.as_str());
        }
    }
    let cond = ListQueryCond {
        unit_id: unit_cond,
        network_code: match query.network.as_ref() {
            None => None,
            Some(network) => match network.len() {
                0 => None,
                _ => Some(network.as_ref()),
            },
        },
        network_addr: match query.addr.as_ref() {
            None => None,
            Some(addr) => match addr.len() {
                0 => None,
                _ => Some(addr.as_ref()),
            },
        },
        profile: match query.profile.as_ref() {
            None => None,
            Some(profile) => match profile.len() {
                0 => None,
                _ => Some(profile.as_str()),
            },
        },
        name_contains: name_contains_cond,
        ..Default::default()
    };
    match state.model.device().count(&cond).await {
        Err(e) => {
            error!("[{}] count error: {}", FN_NAME, e);
            Err(ErrResp::ErrDb(Some(e.to_string())))
        }
        Ok(count) => Ok(Json(response::GetDeviceCount {
            data: response::GetCountData { count },
        })),
    }
}

/// `GET /{base}/api/v1/device/list`
pub async fn get_device_list(
    State(state): State<AppState>,
    Extension(token_info): Extension<GetTokenInfoData>,
    Query(query): Query<request::GetDeviceListQuery>,
) -> impl IntoResponse {
    const FN_NAME: &'static str = "get_device_list";

    let user_id = token_info.user_id.as_str();
    let roles = &token_info.roles;

    if !Role::is_role(roles, Role::ADMIN) && !Role::is_role(roles, Role::MANAGER) {
        match query.unit.as_ref() {
            None => return Err(ErrResp::ErrParam(Some("missing `unit`".to_string()))),
            Some(unit_id) => {
                if unit_id.len() == 0 {
                    return Err(ErrResp::ErrParam(Some("missing `unit`".to_string())));
                }
            }
        }
    }
    let unit_cond = match query.unit.as_ref() {
        None => None,
        Some(unit_id) => match unit_id.len() {
            0 => None,
            _ => {
                match check_unit(FN_NAME, user_id, roles, unit_id.as_str(), false, &state).await? {
                    None => {
                        return Err(ErrResp::Custom(
                            ErrReq::UNIT_NOT_EXIST.0,
                            ErrReq::UNIT_NOT_EXIST.1,
                            None,
                        ))
                    }
                    Some(_) => Some(unit_id.as_str()),
                }
            }
        },
    };
    let mut name_contains_cond = None;
    if let Some(contains) = query.contains.as_ref() {
        if contains.len() > 0 {
            name_contains_cond = Some(contains.as_str());
        }
    }
    let cond = ListQueryCond {
        unit_id: unit_cond,
        network_code: match query.network.as_ref() {
            None => None,
            Some(network) => match network.len() {
                0 => None,
                _ => Some(network.as_ref()),
            },
        },
        network_addr: match query.addr.as_ref() {
            None => None,
            Some(addr) => match addr.len() {
                0 => None,
                _ => Some(addr.as_ref()),
            },
        },
        profile: match query.profile.as_ref() {
            None => None,
            Some(profile) => match profile.len() {
                0 => None,
                _ => Some(profile.as_str()),
            },
        },
        name_contains: name_contains_cond,
        ..Default::default()
    };
    let sort_cond = get_sort_cond(&query.sort)?;
    let opts = ListOptions {
        cond: &cond,
        offset: query.offset,
        limit: match query.limit {
            None => Some(LIST_LIMIT_DEFAULT),
            Some(limit) => match limit {
                0 => None,
                _ => Some(limit),
            },
        },
        sort: Some(sort_cond.as_slice()),
        cursor_max: Some(LIST_CURSOR_MAX),
    };

    let (list, cursor) = match state.model.device().list(&opts, None).await {
        Err(e) => {
            error!("[{}] list error: {}", FN_NAME, e);
            return Err(ErrResp::ErrDb(Some(e.to_string())));
        }
        Ok((list, cursor)) => match cursor {
            None => match query.format {
                Some(request::ListFormat::Array) => {
                    return Ok(Json(device_list_transform(&list)).into_response())
                }
                _ => {
                    return Ok(Json(response::GetDeviceList {
                        data: device_list_transform(&list),
                    })
                    .into_response())
                }
            },
            Some(_) => (list, cursor),
        },
    };

    let body = Body::from_stream(async_stream::stream! {
        let unit_cond = match query.unit.as_ref() {
            None => None,
            Some(unit_id) => match unit_id.len() {
                0 => None,
                _ => Some(unit_id.as_str()),
            },
        };
        let mut name_contains_cond = None;
        if let Some(contains) = query.contains.as_ref() {
            if contains.len() > 0 {
                name_contains_cond = Some(contains.as_str());
            }
        }
        let cond = ListQueryCond {
            unit_id: unit_cond,
            network_code: match query.network.as_ref() {
                None => None,
                Some(network) => match network.len() {
                    0 => None,
                    _ => Some(network.as_ref())
                },
            },
            network_addr: match query.addr.as_ref() {
                None => None,
                Some(addr) => match addr.len() {
                    0 => None,
                    _ => Some(addr.as_ref())
                },
            },
            name_contains: name_contains_cond,
            ..Default::default()
        };
        let opts = ListOptions {
            cond: &cond,
            offset: query.offset,
            limit: match query.limit {
                None => Some(LIST_LIMIT_DEFAULT),
                Some(limit) => match limit {
                    0 => None,
                    _ => Some(limit),
                },
            },
            sort: Some(sort_cond.as_slice()),
            cursor_max: Some(LIST_CURSOR_MAX),
        };

        let mut list = list;
        let mut cursor = cursor;
        let mut is_first = true;
        loop {
            yield device_list_transform_bytes(&list, is_first, cursor.is_none(), query.format.as_ref());
            is_first = false;
            if cursor.is_none() {
                break;
            }
            let (_list, _cursor) = match state.model.device().list(&opts, cursor).await {
                Err(_) => break,
                Ok((list, cursor)) => (list, cursor),
            };
            list = _list;
            cursor = _cursor;
        }
    });
    Ok(([(header::CONTENT_TYPE, ContentType::JSON)], body).into_response())
}

/// `GET /{base}/api/v1/device/{deviceId}`
pub async fn get_device(
    State(state): State<AppState>,
    Extension(token_info): Extension<GetTokenInfoData>,
    Path(param): Path<request::DeviceIdPath>,
) -> impl IntoResponse {
    const FN_NAME: &'static str = "get_device";

    let user_id = token_info.user_id.as_str();
    let roles = &token_info.roles;
    let device_id = param.device_id.as_str();

    match check_device(FN_NAME, device_id, user_id, false, roles, &state).await? {
        None => Err(ErrResp::ErrNotFound(None)),
        Some(device) => Ok(Json(response::GetDevice {
            data: device_transform(&device),
        })),
    }
}

/// `PATCH /{base}/api/v1/device/{deviceId}`
pub async fn patch_device(
    State(state): State<AppState>,
    Extension(token_info): Extension<GetTokenInfoData>,
    Path(param): Path<request::DeviceIdPath>,
    Json(mut body): Json<request::PatchDeviceBody>,
) -> impl IntoResponse {
    const FN_NAME: &'static str = "patch_device";

    let user_id = token_info.user_id.as_str();
    let roles = &token_info.roles;
    let device_id = param.device_id.as_str();

    if let Some(network_id) = body.data.network_id.as_ref() {
        if network_id.len() == 0 {
            return Err(ErrResp::ErrParam(Some(
                "`networkId` must with at least one character".to_string(),
            )));
        }
    }
    if let Some(network_addr) = body.data.network_addr.as_ref() {
        if network_addr.len() == 0 {
            return Err(ErrResp::ErrParam(Some(
                "`networkAddr` must with at least one character".to_string(),
            )));
        }
    }
    if let Some(profile) = body.data.profile.as_ref() {
        if profile.len() > 0 && !strings::is_code(profile.as_str()) {
            return Err(ErrResp::ErrParam(Some(
                "`profile` must be [A-Za-z0-9]{1}[A-Za-z0-9-_]*".to_string(),
            )));
        }
    }

    let mut updates = get_updates(&mut body.data).await?;

    // To check if the device is for the user.
    let device = match check_device(FN_NAME, device_id, user_id, true, roles, &state).await? {
        None => return Err(ErrResp::ErrNotFound(None)),
        Some(device) => device,
    };
    let unit_id = device.unit_id.as_str();
    let network = match updates.network.as_ref() {
        None => None,
        Some((network_id, _)) => {
            match check_network(FN_NAME, unit_id, network_id, roles, &state).await? {
                None => {
                    return Err(ErrResp::Custom(
                        ErrReq::NETWORK_NOT_EXIST.0,
                        ErrReq::NETWORK_NOT_EXIST.1,
                        None,
                    ));
                }
                Some(network) => Some(network),
            }
        }
    };
    if let Some(network) = network.as_ref() {
        updates.network = Some((network.network_id.as_str(), network.code.as_str()));
    }
    if let Some(network_addr) = updates.network_addr {
        let network_id = match updates.network {
            None => device.network_id.as_str(),
            Some((network_id, _)) => network_id,
        };
        if check_addr(FN_NAME, network_id, network_addr, &state)
            .await?
            .is_some()
        {
            return Err(ErrResp::Custom(
                ErrReq::NETWORK_ADDR_EXIST.0,
                ErrReq::NETWORK_ADDR_EXIST.1,
                None,
            ));
        }
    }

    let cond = UpdateQueryCond { device_id };
    if let Err(e) = state.model.device().update(&cond, &updates).await {
        error!("[{}] update error: {}", FN_NAME, e);
        return Err(ErrResp::ErrDb(Some(e.to_string())));
    }
    if let Some(profile) = updates.profile {
        let cond = device_route::UpdateQueryCond { device_id };
        let updates = device_route::Updates {
            profile: Some(profile),
            modified_at: Some(Utc::now()),
            ..Default::default()
        };
        if let Err(e) = state.model.device_route().update(&cond, &updates).await {
            error!("[{}] update device route error: {}", FN_NAME, e);
            return Err(ErrResp::ErrDb(Some(e.to_string())));
        }
    }

    // Delete device cache to update profile.
    if state.cache.is_some() {
        let msg = SendCtrlMsg::DelDevice {
            operation: CtrlMsgOp::DEL_DEVICE.to_string(),
            new: CtrlDelDevice {
                unit_id: device.unit_id,
                unit_code: device.unit_code.clone(),
                network_id: device.network_id,
                network_code: device.network_code.clone(),
                network_addr: device.network_addr.clone(),
                device_id: device.device_id,
            },
        };
        send_del_ctrl_message(FN_NAME, &msg, &state).await?;
    }

    // Send message to the device's network server if device network or address is changed.
    if updates.network.is_some() || updates.network_addr.is_some() {
        let now = Utc::now();

        let msg_op = NetCtrlMsgOp::DEL_DEVICE;
        let msg = SendNetCtrlMsg::DelDevice {
            time: time_str(&now),
            operation: msg_op.to_string(),
            new: NetCtrlAddr {
                network_addr: device.network_addr.clone(),
            },
        };
        let mgr_key = match device.unit_code.as_ref() {
            None => gen_mgr_key("", device.network_code.as_str()),
            Some(code) => gen_mgr_key(code.as_str(), device.network_code.as_str()),
        };
        let _ = send_net_ctrl_message(FN_NAME, &msg, msg_op, &state, &mgr_key).await;

        let msg_op = NetCtrlMsgOp::ADD_DEVICE;
        let msg = SendNetCtrlMsg::AddDevice {
            time: time_str(&now),
            operation: msg_op.to_string(),
            new: NetCtrlAddr {
                network_addr: match updates.network_addr {
                    None => device.network_addr,
                    Some(addr) => addr.to_string(),
                },
            },
        };
        let mgr_key = match network.as_ref() {
            None => mgr_key,
            Some(network) => match network.unit_code.as_ref() {
                None => gen_mgr_key("", network.code.as_str()),
                Some(code) => gen_mgr_key(code.as_str(), network.code.as_str()),
            },
        };
        let _ = send_net_ctrl_message(FN_NAME, &msg, msg_op, &state, &mgr_key).await;
    }

    Ok(StatusCode::NO_CONTENT)
}

/// `DELETE /{base}/api/v1/device/{deviceId}`
pub async fn delete_device(
    State(state): State<AppState>,
    Extension(token_info): Extension<GetTokenInfoData>,
    Path(param): Path<request::DeviceIdPath>,
) -> impl IntoResponse {
    const FN_NAME: &'static str = "delete_device";

    let user_id = token_info.user_id.as_str();
    let roles = &token_info.roles;
    let device_id = param.device_id.as_str();

    // To check if the device is for the user.
    let device = match check_device(FN_NAME, device_id, user_id, true, roles, &state).await {
        Err(e) => return Err(e), // XXX: not use "?" to solve E0282 error.
        Ok(device) => match device {
            None => return Ok(StatusCode::NO_CONTENT),
            Some(device) => device,
        },
    };

    del_device_rsc(FN_NAME, device_id, &state).await?;

    if state.cache.is_some() {
        let msg = SendCtrlMsg::DelDevice {
            operation: CtrlMsgOp::DEL_DEVICE.to_string(),
            new: CtrlDelDevice {
                unit_id: device.unit_id,
                unit_code: device.unit_code.clone(),
                network_id: device.network_id,
                network_code: device.network_code.clone(),
                network_addr: device.network_addr.clone(),
                device_id: device.device_id,
            },
        };
        send_del_ctrl_message(FN_NAME, &msg, &state).await?;
    }

    // Send message to the device's network server.
    let mgr_key = match device.unit_code.as_ref() {
        None => gen_mgr_key("", device.network_code.as_str()),
        Some(unit_code) => gen_mgr_key(unit_code.as_str(), device.network_code.as_str()),
    };
    let msg_op = NetCtrlMsgOp::DEL_DEVICE;
    let msg = SendNetCtrlMsg::DelDevice {
        time: time_str(&Utc::now()),
        operation: msg_op.to_string(),
        new: NetCtrlAddr {
            network_addr: device.network_addr,
        },
    };
    let _ = send_net_ctrl_message(FN_NAME, &msg, msg_op, &state, &mgr_key).await;

    Ok(StatusCode::NO_CONTENT)
}

fn get_sort_cond(sort_args: &Option<String>) -> Result<Vec<SortCond>, ErrResp> {
    match sort_args.as_ref() {
        None => Ok(vec![
            SortCond {
                key: SortKey::NetworkCode,
                asc: true,
            },
            SortCond {
                key: SortKey::NetworkAddr,
                asc: true,
            },
        ]),
        Some(args) => {
            let mut args = args.split(",");
            let mut sort_cond = vec![];
            while let Some(arg) = args.next() {
                let mut cond = arg.split(":");
                let key = match cond.next() {
                    None => return Err(ErrResp::ErrParam(Some("wrong sort argument".to_string()))),
                    Some(field) => match field {
                        "network" => SortKey::NetworkCode,
                        "addr" => SortKey::NetworkAddr,
                        "created" => SortKey::CreatedAt,
                        "modified" => SortKey::ModifiedAt,
                        "profile" => SortKey::Profile,
                        "name" => SortKey::Name,
                        _ => {
                            return Err(ErrResp::ErrParam(Some(format!(
                                "invalid sort key {}",
                                field
                            ))))
                        }
                    },
                };
                let asc = match cond.next() {
                    None => return Err(ErrResp::ErrParam(Some("wrong sort argument".to_string()))),
                    Some(asc) => match asc {
                        "asc" => true,
                        "desc" => false,
                        _ => {
                            return Err(ErrResp::ErrParam(Some(format!(
                                "invalid sort asc {}",
                                asc
                            ))))
                        }
                    },
                };
                if cond.next().is_some() {
                    return Err(ErrResp::ErrParam(Some(
                        "invalid sort condition".to_string(),
                    )));
                }
                sort_cond.push(SortCond { key, asc });
            }
            Ok(sort_cond)
        }
    }
}

async fn get_updates<'a>(body: &'a mut request::PatchDeviceData) -> Result<Updates<'a>, ErrResp> {
    let mut updates = Updates {
        ..Default::default()
    };
    let mut count = 0;
    if let Some(network_id) = body.network_id.as_ref() {
        updates.network = Some((network_id.as_str(), ""));
        count += 1;
    }
    if let Some(network_addr) = body.network_addr.as_ref() {
        updates.network_addr = Some(network_addr.as_str());
        count += 1;
    }
    if let Some(profile) = body.profile.as_ref() {
        updates.profile = Some(profile.as_str());
        count += 1;
    }
    if let Some(name) = body.name.as_ref() {
        updates.name = Some(name.as_str());
        count += 1;
    }
    if let Some(info) = body.info.as_ref() {
        for (k, _) in info.iter() {
            if k.len() == 0 {
                return Err(ErrResp::ErrParam(Some(
                    "`info` key must not be empty".to_string(),
                )));
            }
        }
        updates.info = Some(info);
        count += 1;
    }

    if count == 0 {
        return Err(ErrResp::ErrParam(Some(
            "at least one parameter".to_string(),
        )));
    }
    updates.modified_at = Some(Utc::now());
    Ok(updates)
}

/// To check if the network is exists for the unit. Public network can be matched for admin or
/// manager roles.
///
/// # Errors
///
/// Returns OK if the network is found or not. Otherwise errors will be returned.
async fn check_network(
    fn_name: &str,
    unit_id: &str,
    network_id: &str,
    roles: &HashMap<String, bool>,
    state: &AppState,
) -> Result<Option<Network>, ErrResp> {
    let cond = NetworkQueryCond {
        network_id: Some(network_id),
        ..Default::default()
    };
    let network = match state.model.network().get(&cond).await {
        Err(e) => {
            error!("[{}] get error: {}", fn_name, e);
            return Err(ErrResp::ErrDb(Some(e.to_string())));
        }
        Ok(network) => match network {
            None => return Ok(None),
            Some(network) => network,
        },
    };
    match network.unit_id.as_ref() {
        None => match Role::is_role(roles, Role::ADMIN) || Role::is_role(roles, Role::MANAGER) {
            false => Ok(None),
            true => Ok(Some(network)),
        },
        Some(id) => match id.as_str() == unit_id {
            false => Ok(None),
            true => Ok(Some(network)),
        },
    }
}

/// To check if the address is exists for the network.
///
/// # Errors
///
/// Returns OK if the device is found or not. Otherwise errors will be returned.
async fn check_addr(
    fn_name: &str,
    network_id: &str,
    network_addr: &str,
    state: &AppState,
) -> Result<Option<Device>, ErrResp> {
    let cond = ListQueryCond {
        network_id: Some(network_id),
        network_addr: Some(network_addr),
        ..Default::default()
    };
    let opts = ListOptions {
        cond: &cond,
        offset: None,
        limit: None,
        sort: None,
        cursor_max: None,
    };
    match state.model.device().list(&opts, None).await {
        Err(e) => {
            error!("[{}] get error: {}", fn_name, e);
            Err(ErrResp::ErrDb(Some(e.to_string())))
        }
        Ok((mut list, _)) => Ok(list.pop()),
    }
}

fn device_list_transform(list: &Vec<Device>) -> Vec<response::GetDeviceData> {
    let mut ret = vec![];
    for device in list.iter() {
        ret.push(device_transform(&device));
    }
    ret
}

fn device_list_transform_bytes(
    list: &Vec<Device>,
    with_start: bool,
    with_end: bool,
    format: Option<&request::ListFormat>,
) -> Result<Bytes, Box<dyn StdError + Send + Sync>> {
    let mut build_str = match with_start {
        false => "".to_string(),
        true => match format {
            Some(request::ListFormat::Array) => "[".to_string(),
            _ => "{\"data\":[".to_string(),
        },
    };
    let mut is_first = with_start;

    for item in list {
        if is_first {
            is_first = false;
        } else {
            build_str.push(',');
        }
        let json_str = match serde_json::to_string(&device_transform(item)) {
            Err(e) => return Err(Box::new(e)),
            Ok(str) => str,
        };
        build_str += json_str.as_str();
    }

    if with_end {
        build_str += match format {
            Some(request::ListFormat::Array) => "]",
            _ => "]}",
        }
    }
    Ok(Bytes::copy_from_slice(build_str.as_str().as_bytes()))
}

fn device_transform(device: &Device) -> response::GetDeviceData {
    response::GetDeviceData {
        device_id: device.device_id.clone(),
        unit_id: device.unit_id.clone(),
        unit_code: device.unit_code.clone(),
        network_id: device.network_id.clone(),
        network_code: device.network_code.clone(),
        network_addr: device.network_addr.clone(),
        created_at: time_str(&device.created_at),
        modified_at: time_str(&device.modified_at),
        profile: device.profile.clone(),
        name: device.name.clone(),
        info: device.info.clone(),
    }
}

async fn del_device_rsc(fn_name: &str, device_id: &str, state: &AppState) -> Result<(), ErrResp> {
    let cond = device_route::QueryCond {
        device_id: Some(device_id),
        ..Default::default()
    };
    if let Err(e) = state.model.device_route().del(&cond).await {
        error!("[{}] del device_route error: {}", fn_name, e);
        return Err(ErrResp::ErrDb(Some(e.to_string())));
    }

    let cond = dldata_buffer::QueryCond {
        device_id: Some(device_id),
        ..Default::default()
    };
    if let Err(e) = state.model.dldata_buffer().del(&cond).await {
        error!("[{}] del dldata_buffer error: {}", fn_name, e);
        return Err(ErrResp::ErrDb(Some(e.to_string())));
    }

    let cond = QueryCond {
        device_id: Some(device_id),
        ..Default::default()
    };
    if let Err(e) = state.model.device().del(&cond).await {
        error!("[{}] del device error: {}", fn_name, e);
        return Err(ErrResp::ErrDb(Some(e.to_string())));
    }

    Ok(())
}

async fn del_device_rsc_bulk(
    fn_name: &str,
    rm_cond: &request::PostDeviceBulkData,
    network: &Network,
    state: &AppState,
) -> Result<(), ErrResp> {
    let addrs: Vec<&str> = rm_cond.network_addrs.iter().map(|x| x.as_str()).collect();

    let ctrl_msg = match state.cache.as_ref() {
        None => None,
        Some(_) => {
            let cond = ListQueryCond {
                unit_id: Some(rm_cond.unit_id.as_str()),
                network_id: Some(rm_cond.network_id.as_str()),
                network_addrs: Some(&addrs),
                ..Default::default()
            };
            let opts = ListOptions {
                cond: &cond,
                offset: None,
                limit: None,
                sort: None,
                cursor_max: None,
            };
            let devices = match state.model.device().list(&opts, None).await {
                Err(e) => {
                    error!("[{}] list device for cache error: {}", fn_name, e);
                    return Err(ErrResp::ErrDb(Some(e.to_string())));
                }
                Ok((list, _)) => list,
            };
            Some(SendCtrlMsg::DelDeviceBulk {
                operation: CtrlMsgOp::DEL_DEVICE_BULK.to_string(),
                new: CtrlDelDeviceBulk {
                    unit_id: rm_cond.unit_id.clone(),
                    unit_code: match network.unit_code.as_ref() {
                        None => None,
                        Some(code) => Some(code.clone()),
                    },
                    network_id: network.network_id.clone(),
                    network_code: network.code.clone(),
                    network_addrs: rm_cond.network_addrs.clone(),
                    device_ids: devices.iter().map(|x| x.device_id.clone()).collect(),
                },
            })
        }
    };

    let cond = device_route::QueryCond {
        unit_id: Some(rm_cond.unit_id.as_str()),
        network_id: Some(rm_cond.network_id.as_str()),
        network_addrs: Some(&addrs),
        ..Default::default()
    };
    if let Err(e) = state.model.device_route().del(&cond).await {
        error!("[{}] del device_route error: {}", fn_name, e);
        return Err(ErrResp::ErrDb(Some(e.to_string())));
    }

    let cond = dldata_buffer::QueryCond {
        unit_id: Some(rm_cond.unit_id.as_str()),
        network_id: Some(rm_cond.network_id.as_str()),
        network_addrs: Some(&addrs),
        ..Default::default()
    };
    if let Err(e) = state.model.dldata_buffer().del(&cond).await {
        error!("[{}] del dldata_buffer error: {}", fn_name, e);
        return Err(ErrResp::ErrDb(Some(e.to_string())));
    }

    let cond = QueryCond {
        unit_id: Some(rm_cond.unit_id.as_str()),
        network_id: Some(rm_cond.network_id.as_str()),
        network_addrs: Some(&addrs),
        ..Default::default()
    };
    if let Err(e) = state.model.device().del(&cond).await {
        error!("[{}] del device error: {}", fn_name, e);
        return Err(ErrResp::ErrDb(Some(e.to_string())));
    }

    if let Some(msg) = ctrl_msg.as_ref() {
        send_del_ctrl_message(fn_name, msg, state).await?;
    }

    Ok(())
}

/// Send delete control message.
async fn send_del_ctrl_message(
    fn_name: &str,
    msg: &SendCtrlMsg,
    state: &AppState,
) -> Result<(), ErrResp> {
    let payload = match serde_json::to_vec(&msg) {
        Err(e) => {
            error!(
                "[{}] marshal JSON for {} error: {}",
                fn_name,
                CtrlMsgOp::DEL_DEVICE,
                e
            );
            return Err(ErrResp::ErrRsc(Some(format!(
                "marshal control message error: {}",
                e
            ))));
        }
        Ok(payload) => payload,
    };
    let ctrl_sender = { state.ctrl_senders.device.lock().unwrap().clone() };
    if let Err(e) = ctrl_sender.send_msg(payload).await {
        error!(
            "[{}] send control message for {} error: {}",
            fn_name,
            CtrlMsgOp::DEL_DEVICE,
            e
        );
        return Err(ErrResp::ErrIntMsg(Some(format!(
            "send control message error: {}",
            e
        ))));
    }

    Ok(())
}

/// Send network control message.
async fn send_net_ctrl_message(
    fn_name: &str,
    msg: &SendNetCtrlMsg,
    msg_op: &str,
    state: &AppState,
    mgr_key: &str,
) -> Result<(), ErrResp> {
    let mgr = {
        match state.network_mgrs.lock().unwrap().get_mut(mgr_key) {
            None => return Ok(()),
            Some(mgr) => mgr.clone(),
        }
    };
    match serde_json::to_vec(msg) {
        Err(e) => warn!("[{}] marshal {} error: {}", fn_name, msg_op, e),
        Ok(payload) => match mgr.send_ctrl(payload).await {
            Err(e) => warn!("[{}] send {} error: {}", fn_name, msg_op, e),
            Ok(_) => (),
        },
    }
    Ok(())
}

/// Clear the device and relative cache.
async fn clear_cache(fn_name: &str, queue_name: &str, cache: &Arc<dyn Cache>) {
    if let Err(e) = cache.device().clear().await {
        error!(
            "[{}] {} clear device cache error: {}",
            fn_name, queue_name, e
        );
    }
    if let Err(e) = cache.device_route().clear().await {
        error!(
            "[{}] {} clear device route cache error: {}",
            fn_name, queue_name, e
        );
    }
}

#[async_trait]
impl QueueEventHandler for CtrlSenderHandler {
    async fn on_error(&self, queue: Arc<dyn GmqQueue>, err: Box<dyn StdError + Send + Sync>) {
        const FN_NAME: &'static str = "CtrlSenderHandler::on_error";
        let queue_name = queue.name();

        // Clear cache to avoid missing update cache content during queue status changing.
        if let Some(cache) = self.cache.as_ref() {
            clear_cache(FN_NAME, queue_name, cache).await;
        }

        error!("[{}] {} error: {}", FN_NAME, queue_name, err);
    }

    async fn on_status(&self, queue: Arc<dyn GmqQueue>, status: Status) {
        const FN_NAME: &'static str = "CtrlSenderHandler::on_status";
        let queue_name = queue.name();

        // Clear cache to avoid missing update cache content during queue status changing.
        if let Some(cache) = self.cache.as_ref() {
            clear_cache(FN_NAME, queue_name, cache).await;
        }

        match status {
            Status::Connected => info!("[{}] {} connected", queue_name, FN_NAME),
            _ => warn!("[{}] {} status to {:?}", FN_NAME, queue_name, status),
        }
    }
}

#[async_trait]
impl MessageHandler for CtrlSenderHandler {
    async fn on_message(&self, _queue: Arc<dyn GmqQueue>, _msg: Box<dyn Message>) {}
}

#[async_trait]
impl QueueEventHandler for CtrlReceiverHandler {
    async fn on_error(&self, queue: Arc<dyn GmqQueue>, err: Box<dyn StdError + Send + Sync>) {
        const FN_NAME: &'static str = "CtrlReceiverHandler::on_error";
        let queue_name = queue.name();

        // Clear cache to avoid missing update cache content during queue status changing.
        if let Some(cache) = self.cache.as_ref() {
            clear_cache(FN_NAME, queue_name, cache).await;
        }

        error!("[{}] {} error: {}", FN_NAME, queue_name, err);
    }

    async fn on_status(&self, queue: Arc<dyn GmqQueue>, status: Status) {
        const FN_NAME: &'static str = "CtrlReceiverHandler::on_status";
        let queue_name = queue.name();

        // Clear cache to avoid missing update cache content during queue status changing.
        if let Some(cache) = self.cache.as_ref() {
            clear_cache(FN_NAME, queue_name, cache).await;
        }

        match status {
            Status::Connected => info!("[{}] {} connected", queue_name, FN_NAME),
            _ => warn!("[{}] {} status to {:?}", FN_NAME, queue_name, status),
        }
    }
}

#[async_trait]
impl MessageHandler for CtrlReceiverHandler {
    async fn on_message(&self, queue: Arc<dyn GmqQueue>, msg: Box<dyn Message>) {
        const FN_NAME: &'static str = "CtrlReceiverHandler::on_message";
        let queue_name = queue.name();

        let ctrl_msg = match serde_json::from_slice::<RecvCtrlMsg>(msg.payload()) {
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
        match ctrl_msg {
            RecvCtrlMsg::DelDevice { new } => {
                if let Some(cache) = self.cache.as_ref() {
                    let unit_code = match new.unit_code.as_ref() {
                        None => "",
                        Some(unit_code) => unit_code.as_str(),
                    };
                    let network_code = new.network_code.as_str();
                    let network_addr = new.network_addr.as_str();
                    let cond = device::DelCacheQueryCond {
                        unit_code,
                        network_code: Some(network_code),
                        network_addr: Some(network_addr),
                    };
                    if let Err(e) = cache.device().del(&cond).await {
                        error!(
                            "[{}] {} delete device cache {}.{}.{} error: {}",
                            FN_NAME, queue_name, unit_code, network_code, network_addr, e
                        );
                    }
                    let device_id = new.device_id.as_str();
                    if let Err(e) = cache.device_route().del_uldata(device_id).await {
                        error!(
                            "[{}] {} delete device route cache uldata {} error: {}",
                            FN_NAME, queue_name, device_id, e
                        );
                    }
                    let cond = device_route::DelCacheQueryCond {
                        unit_code,
                        network_code: Some(network_code),
                        network_addr: Some(network_addr),
                    };
                    if let Err(e) = cache.device_route().del_dldata(&cond).await {
                        error!(
                            "[{}] {} delete device route cache dldata {}.{}.{} error: {}",
                            FN_NAME, queue_name, unit_code, network_code, network_addr, e
                        );
                    }
                    let unit_id = new.unit_id.as_str();
                    let cond = device_route::DelCachePubQueryCond {
                        unit_id,
                        device_id: Some(device_id),
                    };
                    if let Err(e) = cache.device_route().del_dldata_pub(&cond).await {
                        error!(
                            "[{}] {} delete device route cache dldata_pub {}.{} error: {}",
                            FN_NAME, queue_name, unit_id, device_id, e
                        );
                    }
                }
            }
            RecvCtrlMsg::DelDeviceBulk { new } => {
                if let Some(cache) = self.cache.as_ref() {
                    for device_id in new.device_ids.iter() {
                        let device_id = device_id.as_str();
                        if let Err(e) = cache.device_route().del_uldata(device_id).await {
                            error!(
                                "[{}] {} delete bulk device route cache uldata {} error: {}",
                                FN_NAME, queue_name, device_id, e
                            );
                        }
                        let unit_id = new.unit_id.as_str();
                        let cond = device_route::DelCachePubQueryCond {
                            unit_id,
                            device_id: Some(device_id),
                        };
                        if let Err(e) = cache.device_route().del_dldata_pub(&cond).await {
                            error!(
                                "[{}] {} delete bulk device route cache dldata_pub {}.{} error: {}",
                                FN_NAME, queue_name, unit_id, device_id, e
                            );
                        }
                    }
                    let unit_code = match new.unit_code.as_ref() {
                        None => "",
                        Some(unit_code) => unit_code.as_str(),
                    };
                    let network_code = new.network_code.as_str();
                    for network_addr in new.network_addrs.iter() {
                        let network_addr = network_addr.as_str();
                        let cond = device::DelCacheQueryCond {
                            unit_code,
                            network_code: Some(network_code),
                            network_addr: Some(network_addr),
                        };
                        if let Err(e) = cache.device().del(&cond).await {
                            error!(
                                "[{}] {} delete bulk device cache {}.{}.{} error: {}",
                                FN_NAME, queue_name, unit_code, network_code, network_addr, e
                            );
                        }
                        let cond = device_route::DelCacheQueryCond {
                            unit_code,
                            network_code: Some(network_code),
                            network_addr: Some(network_addr),
                        };
                        if let Err(e) = cache.device_route().del_dldata(&cond).await {
                            error!(
                                "[{}] {} delete device route cache dldata {}.{}.{} error: {}",
                                FN_NAME, queue_name, unit_code, network_code, network_addr, e
                            );
                        }
                    }
                }
            }
        }

        if let Err(e) = msg.ack().await {
            error!("[{}] {} ACK error: {}", FN_NAME, queue_name, e);
        }
    }
}
