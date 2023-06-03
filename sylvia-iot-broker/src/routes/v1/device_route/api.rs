use std::{
    cmp::Ordering,
    collections::HashMap,
    error::Error as StdError,
    io::{Error as IoError, ErrorKind},
    sync::{Arc, Mutex},
    time::Duration,
};

use actix_web::{
    web::{self, Bytes},
    HttpRequest, HttpResponse, Responder,
};
use async_trait::async_trait;
use chrono::Utc;
use log::{error, info, warn};
use serde::{Deserialize, Serialize};
use serde_json;
use tokio::time;
use url::Url;

use general_mq::{
    queue::{Event, EventHandler as QueueEventHandler, GmqQueue, Message, Status},
    Queue,
};
use sylvia_iot_corelib::{
    err::ErrResp,
    role::Role,
    strings::{self, hex_addr_to_u128, time_str, u128_to_addr},
};

use super::{
    super::{
        super::{ErrReq, State},
        lib::{check_application, check_device, check_network, check_unit, get_token_id_roles},
    },
    request, response,
};
use crate::{
    libs::{
        config::BrokerCtrl as CfgCtrl,
        mq::{self, Connection},
    },
    models::{
        device::{ListOptions as DeviceListOptions, ListQueryCond as DeviceListQueryCond},
        device_route::{DeviceRoute, ListOptions, ListQueryCond, QueryCond, SortCond, SortKey},
        Cache,
    },
};

#[derive(Deserialize, Serialize)]
#[serde(tag = "operation")]
enum RecvCtrlMsg {
    #[serde(rename = "del-device-route")]
    DelDeviceRoute { new: CtrlDelDeviceRoute },
    #[serde(rename = "del-device-route")]
    DelDeviceRouteBulk { new: CtrlDelDeviceRouteBulk },
}

#[derive(Serialize)]
#[serde(untagged)]
enum SendCtrlMsg {
    DelDeviceRoute {
        operation: String,
        new: CtrlDelDeviceRoute,
    },
    DelDeviceRouteBulk {
        operation: String,
        new: CtrlDelDeviceRouteBulk,
    },
}

struct CtrlMsgOp;

#[derive(Deserialize, Serialize)]
struct CtrlDelDeviceRoute {
    #[serde(rename = "deviceId")]
    device_id: String,
}

#[derive(Deserialize, Serialize)]
struct CtrlDelDeviceRouteBulk {
    #[serde(rename = "deviceIds")]
    device_ids: Vec<String>,
}

struct CtrlSenderHandler {
    cache: Option<Arc<dyn Cache>>,
}

struct CtrlReceiverHandler {
    cache: Option<Arc<dyn Cache>>,
}

impl CtrlMsgOp {
    const DEL_DEVICE_ROUTE: &'static str = "del-device-route";
    const DEL_DEVICE_ROUTE_BULK: &'static str = "del-device-route-bulk";
}

const BULK_MAX: usize = 1024;
const LIST_LIMIT_DEFAULT: u64 = 100;
const LIST_CURSOR_MAX: u64 = 100;
const ID_RAND_LEN: usize = 12;
const CTRL_QUEUE_NAME: &'static str = "device-route";

/// Initialize channels.
pub async fn init(state: &State, ctrl_conf: &CfgCtrl) -> Result<(), Box<dyn StdError>> {
    const FN_NAME: &'static str = "init";

    let q = new_ctrl_receiver(state, ctrl_conf)?;
    {
        state
            .ctrl_receivers
            .lock()
            .unwrap()
            .insert(CTRL_QUEUE_NAME.to_string(), q.clone());
    }

    let ctrl_sender = { state.ctrl_senders.device_route.lock().unwrap().clone() };
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
        Arc::new(CtrlSenderHandler { cache }),
    ) {
        Err(e) => Err(Box::new(IoError::new(ErrorKind::InvalidInput, e))),
        Ok(q) => Ok(Arc::new(Mutex::new(q))),
    }
}

/// Create control channel receiver queue.
pub fn new_ctrl_receiver(state: &State, config: &CfgCtrl) -> Result<Queue, Box<dyn StdError>> {
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
        handler,
    ) {
        Err(e) => Err(Box::new(IoError::new(ErrorKind::InvalidInput, e))),
        Ok(q) => Ok(q),
    }
}

/// `POST /{base}/api/v1/device-route`
pub async fn post_device_route(
    req: HttpRequest,
    body: web::Json<request::PostDeviceRouteBody>,
    state: web::Data<State>,
) -> impl Responder {
    const FN_NAME: &'static str = "post_device_route";

    let (user_id, roles) = get_token_id_roles(FN_NAME, &req)?;
    let user_id = user_id.as_str();

    if body.data.device_id.len() == 0 {
        return Err(ErrResp::ErrParam(Some(
            "`deviceId` must with at least one character".to_string(),
        )));
    } else if body.data.application_id.len() == 0 {
        return Err(ErrResp::ErrParam(Some(
            "`applicationId` must with at least one character".to_string(),
        )));
    }
    let device_id = body.data.device_id.as_str();
    let application_id = body.data.application_id.as_str();
    let device = match check_device(FN_NAME, device_id, user_id, true, &roles, &state).await? {
        None => {
            return Err(ErrResp::Custom(
                ErrReq::DEVICE_NOT_EXIST.0,
                ErrReq::DEVICE_NOT_EXIST.1,
                None,
            ))
        }
        Some(device) => device,
    };
    let application =
        match check_application(FN_NAME, application_id, user_id, true, &roles, &state).await? {
            None => {
                return Err(ErrResp::Custom(
                    ErrReq::APPLICATION_NOT_EXIST.0,
                    ErrReq::APPLICATION_NOT_EXIST.1,
                    None,
                ))
            }
            Some(application) => application,
        };
    if device.unit_id.as_str().cmp(application.unit_id.as_str()) != Ordering::Equal {
        return Err(ErrResp::Custom(
            ErrReq::UNIT_NOT_MATCH.0,
            ErrReq::UNIT_NOT_MATCH.1,
            None,
        ));
    }
    let cond = ListQueryCond {
        application_id: Some(application_id),
        device_id: Some(device_id),
        ..Default::default()
    };
    let opts = ListOptions {
        cond: &cond,
        offset: None,
        limit: Some(1),
        sort: None,
        cursor_max: None,
    };
    match state.model.device_route().list(&opts, None).await {
        Err(e) => {
            error!("[{}] get error: {}", FN_NAME, e);
            return Err(ErrResp::ErrDb(Some(e.to_string())));
        }
        Ok((list, _)) => match list.len() {
            0 => (),
            _ => {
                return Err(ErrResp::Custom(
                    ErrReq::ROUTE_EXIST.0,
                    ErrReq::ROUTE_EXIST.1,
                    None,
                ))
            }
        },
    }

    let now = Utc::now();
    let route_id = strings::random_id(&now, ID_RAND_LEN);
    let route = DeviceRoute {
        route_id: route_id.clone(),
        unit_id: application.unit_id,
        unit_code: application.unit_code,
        application_id: application.application_id,
        application_code: application.code,
        device_id: device.device_id,
        network_id: device.network_id,
        network_code: device.network_code,
        network_addr: device.network_addr,
        profile: device.profile,
        created_at: now,
        modified_at: now,
    };
    if let Err(e) = state.model.device_route().add(&route).await {
        error!("[{}] add error: {}", FN_NAME, e);
        return Err(ErrResp::ErrDb(Some(e.to_string())));
    }

    // Clear the denied route in cache.
    if state.cache.is_some() {
        let msg = SendCtrlMsg::DelDeviceRoute {
            operation: CtrlMsgOp::DEL_DEVICE_ROUTE.to_string(),
            new: CtrlDelDeviceRoute {
                device_id: route.device_id,
            },
        };
        send_del_ctrl_message(FN_NAME, &msg, &state).await?;
    }

    Ok(HttpResponse::Ok().json(response::PostDeviceRoute {
        data: response::PostDeviceRouteData { route_id },
    }))
}

/// `POST /{base}/api/v1/device-route/bulk`
pub async fn post_device_route_bulk(
    req: HttpRequest,
    mut body: web::Json<request::PostDeviceRouteBulkBody>,
    state: web::Data<State>,
) -> impl Responder {
    const FN_NAME: &'static str = "post_device_route_bulk";

    let (user_id, roles) = get_token_id_roles(FN_NAME, &req)?;
    let user_id = user_id.as_str();

    if body.data.application_id.len() == 0 {
        return Err(ErrResp::ErrParam(Some(
            "`applicationId` must with at least one character".to_string(),
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
    let application_id = body.data.application_id.as_str();
    let application =
        match check_application(FN_NAME, application_id, user_id, true, &roles, &state).await? {
            None => {
                return Err(ErrResp::Custom(
                    ErrReq::APPLICATION_NOT_EXIST.0,
                    ErrReq::APPLICATION_NOT_EXIST.1,
                    None,
                ))
            }
            Some(application) => application,
        };
    let network_id = body.data.network_id.as_str();
    let network = match check_network(FN_NAME, network_id, user_id, true, &roles, &state).await? {
        None => {
            return Err(ErrResp::Custom(
                ErrReq::NETWORK_NOT_EXIST.0,
                ErrReq::NETWORK_NOT_EXIST.1,
                None,
            ));
        }
        Some(network) => network,
    };

    let addrs: Vec<&str> = body.data.network_addrs.iter().map(|x| x.as_str()).collect();
    let cond = DeviceListQueryCond {
        unit_id: match network.unit_id.as_ref() {
            None => None,
            Some(unit_id) => Some(unit_id.as_str()),
        },
        network_id: Some(network_id),
        network_addrs: Some(&addrs),
        ..Default::default()
    };
    let opts = DeviceListOptions {
        cond: &cond,
        offset: None,
        limit: None,
        sort: None,
        cursor_max: None,
    };
    let devices = match state.model.device().list(&opts, None).await {
        Err(e) => {
            error!("[{}] list device error: {}", FN_NAME, e);
            return Err(ErrResp::ErrDb(Some(e.to_string())));
        }
        Ok((list, _)) => list,
    };
    if devices.len() == 0 {
        return Err(ErrResp::Custom(
            ErrReq::DEVICE_NOT_EXIST.0,
            ErrReq::DEVICE_NOT_EXIST.1,
            None,
        ));
    }

    let mut routes = vec![];
    for device in devices.iter() {
        let now = Utc::now();
        let route = DeviceRoute {
            route_id: strings::random_id(&now, ID_RAND_LEN),
            unit_id: application.unit_id.clone(),
            unit_code: application.unit_code.clone(),
            application_id: application.application_id.clone(),
            application_code: application.code.clone(),
            network_id: network.network_id.clone(),
            network_code: network.code.clone(),
            network_addr: device.network_addr.clone(),
            device_id: device.device_id.clone(),
            profile: device.profile.clone(),
            created_at: now,
            modified_at: now,
        };
        routes.push(route);
    }
    if let Err(e) = state.model.device_route().add_bulk(&routes).await {
        error!("[{}] add error: {}", FN_NAME, e);
        return Err(ErrResp::ErrDb(Some(e.to_string())));
    }

    if state.cache.is_some() {
        let msg = SendCtrlMsg::DelDeviceRouteBulk {
            operation: CtrlMsgOp::DEL_DEVICE_ROUTE_BULK.to_string(),
            new: CtrlDelDeviceRouteBulk {
                device_ids: devices.iter().map(|x| x.device_id.clone()).collect(),
            },
        };
        send_del_ctrl_message(FN_NAME, &msg, &state).await?;
    }

    Ok(HttpResponse::NoContent().finish())
}

/// `POST /{base}/api/v1/device-route/bulk-delete`
pub async fn post_device_route_bulk_del(
    req: HttpRequest,
    mut body: web::Json<request::PostDeviceRouteBulkBody>,
    state: web::Data<State>,
) -> impl Responder {
    const FN_NAME: &'static str = "post_device_route_bulk_del";

    let (user_id, roles) = get_token_id_roles(FN_NAME, &req)?;
    let user_id = user_id.as_str();

    if body.data.application_id.len() == 0 {
        return Err(ErrResp::ErrParam(Some(
            "`applicationId` must with at least one character".to_string(),
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
    let application_id = body.data.application_id.as_str();
    if check_application(FN_NAME, application_id, user_id, true, &roles, &state)
        .await?
        .is_none()
    {
        return Err(ErrResp::Custom(
            ErrReq::APPLICATION_NOT_EXIST.0,
            ErrReq::APPLICATION_NOT_EXIST.1,
            None,
        ));
    }
    let network_id = body.data.network_id.as_str();
    let network = match check_network(FN_NAME, network_id, user_id, true, &roles, &state).await? {
        None => {
            return Err(ErrResp::Custom(
                ErrReq::NETWORK_NOT_EXIST.0,
                ErrReq::NETWORK_NOT_EXIST.1,
                None,
            ));
        }
        Some(network) => network,
    };

    let addrs: Vec<&str> = body.data.network_addrs.iter().map(|x| x.as_str()).collect();
    let cond = QueryCond {
        application_id: Some(body.data.application_id.as_str()),
        network_id: Some(body.data.network_id.as_str()),
        network_addrs: Some(&addrs),
        ..Default::default()
    };
    if let Err(e) = state.model.device_route().del(&cond).await {
        error!("[{}] del error: {}", FN_NAME, e);
        return Err(ErrResp::ErrDb(Some(e.to_string())));
    }

    if state.cache.is_some() {
        let cond = DeviceListQueryCond {
            unit_id: match network.unit_id.as_ref() {
                None => None,
                Some(unit_id) => Some(unit_id.as_str()),
            },
            network_id: Some(network_id),
            network_addrs: Some(&addrs),
            ..Default::default()
        };
        let opts = DeviceListOptions {
            cond: &cond,
            offset: None,
            limit: None,
            sort: None,
            cursor_max: None,
        };
        let devices = match state.model.device().list(&opts, None).await {
            Err(e) => {
                error!("[{}] list device error: {}", FN_NAME, e);
                return Err(ErrResp::ErrDb(Some(e.to_string())));
            }
            Ok((list, _)) => list,
        };
        let msg = SendCtrlMsg::DelDeviceRouteBulk {
            operation: CtrlMsgOp::DEL_DEVICE_ROUTE_BULK.to_string(),
            new: CtrlDelDeviceRouteBulk {
                device_ids: devices.iter().map(|x| x.device_id.clone()).collect(),
            },
        };
        send_del_ctrl_message(FN_NAME, &msg, &state).await?;
    }

    Ok(HttpResponse::NoContent().finish())
}

/// `POST /{base}/api/v1/device-route/range`
pub async fn post_device_route_range(
    req: HttpRequest,
    body: web::Json<request::PostDeviceRouteRangeBody>,
    state: web::Data<State>,
) -> impl Responder {
    const FN_NAME: &'static str = "post_device_route_range";

    let (user_id, roles) = get_token_id_roles(FN_NAME, &req)?;
    let user_id = user_id.as_str();

    if body.data.application_id.len() == 0 {
        return Err(ErrResp::ErrParam(Some(
            "`applicationId` must with at least one character".to_string(),
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

    let application_id = body.data.application_id.as_str();
    let application =
        match check_application(FN_NAME, application_id, user_id, true, &roles, &state).await? {
            None => {
                return Err(ErrResp::Custom(
                    ErrReq::APPLICATION_NOT_EXIST.0,
                    ErrReq::APPLICATION_NOT_EXIST.1,
                    None,
                ))
            }
            Some(application) => application,
        };
    let network_id = body.data.network_id.as_str();
    let network = match check_network(FN_NAME, network_id, user_id, true, &roles, &state).await? {
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
    let addrs: Vec<&str> = network_addrs.iter().map(|x| x.as_str()).collect();
    let cond = DeviceListQueryCond {
        unit_id: match network.unit_id.as_ref() {
            None => None,
            Some(unit_id) => Some(unit_id.as_str()),
        },
        network_id: Some(network_id),
        network_addrs: Some(&addrs),
        ..Default::default()
    };
    let opts = DeviceListOptions {
        cond: &cond,
        offset: None,
        limit: None,
        sort: None,
        cursor_max: None,
    };
    let devices = match state.model.device().list(&opts, None).await {
        Err(e) => {
            error!("[{}] list device error: {}", FN_NAME, e);
            return Err(ErrResp::ErrDb(Some(e.to_string())));
        }
        Ok((list, _)) => list,
    };
    if devices.len() == 0 {
        return Err(ErrResp::Custom(
            ErrReq::DEVICE_NOT_EXIST.0,
            ErrReq::DEVICE_NOT_EXIST.1,
            None,
        ));
    }

    let mut routes = vec![];
    for device in devices.iter() {
        let now = Utc::now();
        let route = DeviceRoute {
            route_id: strings::random_id(&now, ID_RAND_LEN),
            unit_id: application.unit_id.clone(),
            unit_code: application.unit_code.clone(),
            application_id: application.application_id.clone(),
            application_code: application.code.clone(),
            network_id: network.network_id.clone(),
            network_code: network.code.clone(),
            network_addr: device.network_addr.clone(),
            device_id: device.device_id.clone(),
            profile: device.profile.clone(),
            created_at: now,
            modified_at: now,
        };
        routes.push(route);
    }
    if let Err(e) = state.model.device_route().add_bulk(&routes).await {
        error!("[{}] add error: {}", FN_NAME, e);
        return Err(ErrResp::ErrDb(Some(e.to_string())));
    }

    if state.cache.is_some() {
        let msg = SendCtrlMsg::DelDeviceRouteBulk {
            operation: CtrlMsgOp::DEL_DEVICE_ROUTE_BULK.to_string(),
            new: CtrlDelDeviceRouteBulk {
                device_ids: devices.iter().map(|x| x.device_id.clone()).collect(),
            },
        };
        send_del_ctrl_message(FN_NAME, &msg, &state).await?;
    }

    Ok(HttpResponse::NoContent().finish())
}

/// `POST /{base}/api/v1/device-route/range-delete`
pub async fn post_device_route_range_del(
    req: HttpRequest,
    body: web::Json<request::PostDeviceRouteRangeBody>,
    state: web::Data<State>,
) -> impl Responder {
    const FN_NAME: &'static str = "post_device_route_range_del";

    let (user_id, roles) = get_token_id_roles(FN_NAME, &req)?;
    let user_id = user_id.as_str();

    if body.data.application_id.len() == 0 {
        return Err(ErrResp::ErrParam(Some(
            "`applicationId` must with at least one character".to_string(),
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

    let application_id = body.data.application_id.as_str();
    if check_application(FN_NAME, application_id, user_id, true, &roles, &state)
        .await?
        .is_none()
    {
        return Err(ErrResp::Custom(
            ErrReq::APPLICATION_NOT_EXIST.0,
            ErrReq::APPLICATION_NOT_EXIST.1,
            None,
        ));
    }
    let network_id = body.data.network_id.as_str();
    let network = match check_network(FN_NAME, network_id, user_id, true, &roles, &state).await? {
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

    let addrs: Vec<&str> = network_addrs.iter().map(|x| x.as_str()).collect();
    let cond = QueryCond {
        application_id: Some(body.data.application_id.as_str()),
        network_id: Some(body.data.network_id.as_str()),
        network_addrs: Some(&addrs),
        ..Default::default()
    };
    if let Err(e) = state.model.device_route().del(&cond).await {
        error!("[{}] del error: {}", FN_NAME, e);
        return Err(ErrResp::ErrDb(Some(e.to_string())));
    }

    if state.cache.is_some() {
        let cond = DeviceListQueryCond {
            unit_id: match network.unit_id.as_ref() {
                None => None,
                Some(unit_id) => Some(unit_id.as_str()),
            },
            network_id: Some(network_id),
            network_addrs: Some(&addrs),
            ..Default::default()
        };
        let opts = DeviceListOptions {
            cond: &cond,
            offset: None,
            limit: None,
            sort: None,
            cursor_max: None,
        };
        let devices = match state.model.device().list(&opts, None).await {
            Err(e) => {
                error!("[{}] list device error: {}", FN_NAME, e);
                return Err(ErrResp::ErrDb(Some(e.to_string())));
            }
            Ok((list, _)) => list,
        };
        let msg = SendCtrlMsg::DelDeviceRouteBulk {
            operation: CtrlMsgOp::DEL_DEVICE_ROUTE_BULK.to_string(),
            new: CtrlDelDeviceRouteBulk {
                device_ids: devices.iter().map(|x| x.device_id.clone()).collect(),
            },
        };
        send_del_ctrl_message(FN_NAME, &msg, &state).await?;
    }

    Ok(HttpResponse::NoContent().finish())
}

/// `GET /{base}/api/v1/device-route/count`
pub async fn get_device_route_count(
    req: HttpRequest,
    query: web::Query<request::GetDeviceRouteCountQuery>,
    state: web::Data<State>,
) -> impl Responder {
    const FN_NAME: &'static str = "get_device_route_count";

    let (user_id, roles) = get_token_id_roles(FN_NAME, &req)?;
    let user_id = user_id.as_str();

    if !Role::is_role(&roles, Role::ADMIN) && !Role::is_role(&roles, Role::MANAGER) {
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
            _ => match check_unit(FN_NAME, user_id, &roles, unit_id.as_str(), false, &state).await?
            {
                None => {
                    return Err(ErrResp::Custom(
                        ErrReq::UNIT_NOT_EXIST.0,
                        ErrReq::UNIT_NOT_EXIST.1,
                        None,
                    ))
                }
                Some(_) => Some(unit_id.as_str()),
            },
        },
    };
    let cond = ListQueryCond {
        unit_id: unit_cond,
        application_id: match query.application.as_ref() {
            None => None,
            Some(application) => match application.len() {
                0 => None,
                _ => Some(application.as_ref()),
            },
        },
        network_id: match query.network.as_ref() {
            None => None,
            Some(network_id) => match network_id.len() {
                0 => None,
                _ => Some(network_id.as_ref()),
            },
        },
        device_id: match query.device.as_ref() {
            None => None,
            Some(device_id) => match device_id.len() {
                0 => None,
                _ => Some(device_id.as_ref()),
            },
        },
        ..Default::default()
    };
    match state.model.device_route().count(&cond).await {
        Err(e) => {
            error!("[{}] count error: {}", FN_NAME, e);
            Err(ErrResp::ErrDb(Some(e.to_string())))
        }
        Ok(count) => Ok(HttpResponse::Ok().json(response::GetDeviceRouteCount {
            data: response::GetCountData { count },
        })),
    }
}

/// `GET /{base}/api/v1/device-route/list`
pub async fn get_device_route_list(
    req: HttpRequest,
    query: web::Query<request::GetDeviceRouteListQuery>,
    state: web::Data<State>,
) -> impl Responder {
    const FN_NAME: &'static str = "get_device_route_list";

    let (user_id, roles) = get_token_id_roles(FN_NAME, &req)?;
    let user_id = user_id.as_str();

    if !Role::is_role(&roles, Role::ADMIN) && !Role::is_role(&roles, Role::MANAGER) {
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
            _ => match check_unit(FN_NAME, user_id, &roles, unit_id.as_str(), false, &state).await?
            {
                None => {
                    return Err(ErrResp::Custom(
                        ErrReq::UNIT_NOT_EXIST.0,
                        ErrReq::UNIT_NOT_EXIST.1,
                        None,
                    ))
                }
                Some(_) => Some(unit_id.as_str()),
            },
        },
    };
    let cond = ListQueryCond {
        unit_id: unit_cond,
        application_id: match query.application.as_ref() {
            None => None,
            Some(application) => match application.len() {
                0 => None,
                _ => Some(application.as_ref()),
            },
        },
        network_id: match query.network.as_ref() {
            None => None,
            Some(network_id) => match network_id.len() {
                0 => None,
                _ => Some(network_id.as_ref()),
            },
        },
        device_id: match query.device.as_ref() {
            None => None,
            Some(device_id) => match device_id.len() {
                0 => None,
                _ => Some(device_id.as_ref()),
            },
        },
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

    let (list, cursor) = match state.model.device_route().list(&opts, None).await {
        Err(e) => {
            error!("[{}] list error: {}", FN_NAME, e);
            return Err(ErrResp::ErrDb(Some(e.to_string())));
        }
        Ok((list, cursor)) => match cursor {
            None => match query.format {
                Some(request::ListFormat::Array) => {
                    return Ok(HttpResponse::Ok().json(route_list_transform(&list)))
                }
                _ => {
                    return Ok(HttpResponse::Ok().json(response::GetDeviceRouteList {
                        data: route_list_transform(&list),
                    }))
                }
            },
            Some(_) => (list, cursor),
        },
    };

    // TODO: detect client disconnect
    let stream = async_stream::stream! {
        let query = query.0.clone();
        let unit_cond = match query.unit.as_ref() {
            None => None,
            Some(unit_id) => match unit_id.len() {
                0 => None,
                _ => Some(unit_id.as_str()),
            },
        };
        let cond = ListQueryCond {
            unit_id: unit_cond,
            application_id: match query.application.as_ref() {
                None => None,
                Some(application) => match application.len() {
                    0 => None,
                    _ => Some(application.as_ref())
                },
            },
            network_id: match query.network.as_ref() {
                None => None,
                Some(network_id) => match network_id.len() {
                    0 => None,
                    _ => Some(network_id.as_ref())
                },
            },
            device_id: match query.device.as_ref() {
                None => None,
                Some(device_id) => match device_id.len() {
                    0 => None,
                    _ => Some(device_id.as_ref())
                },
            },
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
            yield route_list_transform_bytes(&list, is_first, cursor.is_none(), query.format.as_ref());
            is_first = false;
            if cursor.is_none() {
                break;
            }
            let (_list, _cursor) = match state.model.device_route().list(&opts, cursor).await {
                Err(_) => break,
                Ok((list, cursor)) => (list, cursor),
            };
            list = _list;
            cursor = _cursor;
        }
    };
    Ok(HttpResponse::Ok().streaming(stream))
}

/// `DELETE /{base}/api/v1/device-route/{routeId}`
pub async fn delete_device_route(
    req: HttpRequest,
    param: web::Path<request::RouteIdPath>,
    state: web::Data<State>,
) -> impl Responder {
    const FN_NAME: &'static str = "delete_device_route";

    let (user_id, roles) = get_token_id_roles(FN_NAME, &req)?;
    let user_id = user_id.as_str();
    let route_id = param.route_id.as_str();

    // To check if the device route is for the user.
    let route = match check_route(FN_NAME, route_id, user_id, true, &roles, &state).await {
        Err(e) => return Err(e), // XXX: not use "?" to solve E0282 error.
        Ok(route) => match route {
            None => return Ok(HttpResponse::NoContent().finish()),
            Some(route) => route,
        },
    };

    let cond = QueryCond {
        route_id: Some(route_id),
        ..Default::default()
    };
    if let Err(e) = state.model.device_route().del(&cond).await {
        error!("[{}] del error: {}", FN_NAME, e);
        return Err(ErrResp::ErrDb(Some(e.to_string())));
    }

    if state.cache.is_some() {
        let msg = SendCtrlMsg::DelDeviceRoute {
            operation: CtrlMsgOp::DEL_DEVICE_ROUTE.to_string(),
            new: CtrlDelDeviceRoute {
                device_id: route.device_id,
            },
        };
        send_del_ctrl_message(FN_NAME, &msg, &state).await?;
    }

    Ok(HttpResponse::NoContent().finish())
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
            SortCond {
                key: SortKey::CreatedAt,
                asc: false,
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
                        "application" => SortKey::ApplicationCode,
                        "network" => SortKey::NetworkCode,
                        "addr" => SortKey::NetworkAddr,
                        "created" => SortKey::CreatedAt,
                        "modified" => SortKey::ModifiedAt,
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

/// To check if the user ID can access the device route. Choose `only_owner` to check if the user
/// is the unit owner or one of unit members.
///
/// # Errors
///
/// Returns OK if the device route is found or not. Otherwise errors will be returned.
async fn check_route(
    fn_name: &str,
    route_id: &str,
    user_id: &str,
    only_owner: bool, // to check if this `user_id` is the owner.
    roles: &HashMap<String, bool>,
    state: &web::Data<State>,
) -> Result<Option<DeviceRoute>, ErrResp> {
    let route = match state.model.device_route().get(route_id).await {
        Err(e) => {
            error!("[{}] get error: {}", fn_name, e);
            return Err(ErrResp::ErrDb(Some(e.to_string())));
        }
        Ok(route) => match route {
            None => return Ok(None),
            Some(route) => route,
        },
    };
    let unit_id = route.unit_id.as_str();
    match check_unit(fn_name, user_id, roles, unit_id, only_owner, state).await? {
        None => Ok(None),
        Some(_) => Ok(Some(route)),
    }
}

fn route_list_transform(list: &Vec<DeviceRoute>) -> Vec<response::GetDeviceRouteData> {
    let mut ret = vec![];
    for route in list.iter() {
        ret.push(route_transform(&route));
    }
    ret
}

fn route_list_transform_bytes(
    list: &Vec<DeviceRoute>,
    with_start: bool,
    with_end: bool,
    format: Option<&request::ListFormat>,
) -> Result<Bytes, Box<dyn StdError>> {
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
        let json_str = match serde_json::to_string(&route_transform(item)) {
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

fn route_transform(route: &DeviceRoute) -> response::GetDeviceRouteData {
    response::GetDeviceRouteData {
        route_id: route.route_id.clone(),
        unit_id: route.unit_id.clone(),
        application_id: route.application_id.clone(),
        application_code: route.application_code.clone(),
        device_id: route.device_id.clone(),
        network_id: route.network_id.clone(),
        network_code: route.network_code.clone(),
        network_addr: route.network_addr.clone(),
        profile: route.profile.clone(),
        created_at: time_str(&route.created_at),
        modified_at: time_str(&route.modified_at),
    }
}

/// Send delete control message.
async fn send_del_ctrl_message(
    fn_name: &str,
    msg: &SendCtrlMsg,
    state: &web::Data<State>,
) -> Result<(), ErrResp> {
    let payload = match serde_json::to_vec(&msg) {
        Err(e) => {
            error!(
                "[{}] marshal JSON for {} error: {}",
                fn_name,
                CtrlMsgOp::DEL_DEVICE_ROUTE,
                e
            );
            return Err(ErrResp::ErrRsc(Some(format!(
                "marshal control message error: {}",
                e
            ))));
        }
        Ok(payload) => payload,
    };
    let ctrl_sender = { state.ctrl_senders.device_route.lock().unwrap().clone() };
    if let Err(e) = ctrl_sender.send_msg(payload).await {
        error!(
            "[{}] send control message for {} error: {}",
            fn_name,
            CtrlMsgOp::DEL_DEVICE_ROUTE,
            e
        );
        return Err(ErrResp::ErrIntMsg(Some(format!(
            "send control message error: {}",
            e
        ))));
    }

    Ok(())
}

#[async_trait]
impl QueueEventHandler for CtrlSenderHandler {
    async fn on_event(&self, queue: Arc<dyn GmqQueue>, ev: Event) {
        const FN_NAME: &'static str = "CtrlSenderHandler::on_event";
        let queue_name = queue.name();

        // Clear cache to avoid missing update cache content during queue status changing.
        if let Some(cache) = self.cache.as_ref() {
            if let Err(e) = cache.device_route().clear().await {
                error!(
                    "[{}] {} clear device route cache error: {}",
                    FN_NAME, queue_name, e
                );
            }
        }

        match ev {
            Event::Error(e) => error!("[{}] {} error: {}", FN_NAME, queue_name, e),
            Event::Status(status) => match status {
                Status::Connected => info!("[{}] {} connected", queue_name, FN_NAME),
                _ => warn!("[{}] {} status to {:?}", FN_NAME, queue_name, status),
            },
        }
    }

    async fn on_message(&self, _queue: Arc<dyn GmqQueue>, _msg: Box<dyn Message>) {}
}

#[async_trait]
impl QueueEventHandler for CtrlReceiverHandler {
    async fn on_event(&self, queue: Arc<dyn GmqQueue>, ev: Event) {
        const FN_NAME: &'static str = "CtrlReceiverHandler::on_event";
        let queue_name = queue.name();

        // Clear cache to avoid missing update cache content during queue status changing.
        if let Some(cache) = self.cache.as_ref() {
            if let Err(e) = cache.device_route().clear().await {
                error!(
                    "[{}] {} clear device route cache error: {}",
                    FN_NAME, queue_name, e
                );
            }
        }

        match ev {
            Event::Error(e) => error!("[{}] {} error: {}", FN_NAME, queue_name, e),
            Event::Status(status) => match status {
                Status::Connected => info!("[{}] {} connected", queue_name, FN_NAME),
                _ => warn!("[{}] {} status to {:?}", FN_NAME, queue_name, status),
            },
        }
    }

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
            RecvCtrlMsg::DelDeviceRoute { new } => {
                if let Some(cache) = self.cache.as_ref() {
                    if let Err(e) = cache
                        .device_route()
                        .del_uldata(new.device_id.as_str())
                        .await
                    {
                        error!(
                            "[{}] {} delete device route cache {} error: {}",
                            FN_NAME, queue_name, new.device_id, e
                        );
                    }
                }
            }
            RecvCtrlMsg::DelDeviceRouteBulk { new } => {
                if let Some(cache) = self.cache.as_ref() {
                    for device_id in new.device_ids.iter() {
                        if let Err(e) = cache.device_route().del_uldata(device_id.as_str()).await {
                            error!(
                                "[{}] {} delete device route cache {} error: {}",
                                FN_NAME, queue_name, device_id, e
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
