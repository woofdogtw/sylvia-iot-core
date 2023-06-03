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
    strings::{self, time_str},
};

use super::{
    super::{
        super::{ErrReq, State},
        lib::{check_application, check_network, check_unit, get_token_id_roles},
    },
    request, response,
};
use crate::{
    libs::{
        config::BrokerCtrl as CfgCtrl,
        mq::{self, Connection},
    },
    models::{
        network_route::{ListOptions, ListQueryCond, NetworkRoute, QueryCond, SortCond, SortKey},
        Cache,
    },
};

#[derive(Deserialize, Serialize)]
#[serde(tag = "operation")]
enum RecvCtrlMsg {
    #[serde(rename = "del-network-route")]
    DelNetworkRoute { new: CtrlDelNetworkRoute },
}

#[derive(Serialize)]
#[serde(untagged)]
enum SendCtrlMsg {
    DelNetworkRoute {
        operation: String,
        new: CtrlDelNetworkRoute,
    },
}

struct CtrlMsgOp;

#[derive(Deserialize, Serialize)]
struct CtrlDelNetworkRoute {
    #[serde(rename = "unitId")]
    unit_id: String,
    #[serde(rename = "unitCode")]
    unit_code: Option<String>,
    #[serde(rename = "networkId")]
    network_id: String,
    #[serde(rename = "networkCode")]
    network_code: String,
}

struct CtrlSenderHandler;

struct CtrlReceiverHandler {
    cache: Option<Arc<dyn Cache>>,
}

impl CtrlMsgOp {
    const DEL_NETWORK_ROUTE: &'static str = "del-network-route";
}

const LIST_LIMIT_DEFAULT: u64 = 100;
const LIST_CURSOR_MAX: u64 = 100;
const ID_RAND_LEN: usize = 12;
const CTRL_QUEUE_NAME: &'static str = "network-route";

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

    let ctrl_sender = { state.ctrl_senders.network_route.lock().unwrap().clone() };
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
        Arc::new(CtrlSenderHandler {}),
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

/// `POST /{base}/api/v1/network-route`
pub async fn post_network_route(
    req: HttpRequest,
    body: web::Json<request::PostNetworkRouteBody>,
    state: web::Data<State>,
) -> impl Responder {
    const FN_NAME: &'static str = "post_network_route";

    let (user_id, roles) = get_token_id_roles(FN_NAME, &req)?;
    let user_id = user_id.as_str();

    if body.data.network_id.len() == 0 {
        return Err(ErrResp::ErrParam(Some(
            "`networkId` must with at least one character".to_string(),
        )));
    } else if body.data.application_id.len() == 0 {
        return Err(ErrResp::ErrParam(Some(
            "`applicationId` must with at least one character".to_string(),
        )));
    }
    let network_id = body.data.network_id.as_str();
    let application_id = body.data.application_id.as_str();
    let network = match check_network(FN_NAME, network_id, user_id, true, &roles, &state).await? {
        None => {
            return Err(ErrResp::Custom(
                ErrReq::NETWORK_NOT_EXIST.0,
                ErrReq::NETWORK_NOT_EXIST.1,
                None,
            ))
        }
        Some(network) => network,
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
    if let Some(unit_id) = network.unit_id.as_ref() {
        if unit_id.as_str().cmp(application.unit_id.as_str()) != Ordering::Equal {
            return Err(ErrResp::Custom(
                ErrReq::UNIT_NOT_MATCH.0,
                ErrReq::UNIT_NOT_MATCH.1,
                None,
            ));
        }
    }
    let cond = ListQueryCond {
        application_id: Some(application_id),
        network_id: Some(network_id),
        ..Default::default()
    };
    let opts = ListOptions {
        cond: &cond,
        offset: None,
        limit: Some(1),
        sort: None,
        cursor_max: None,
    };
    match state.model.network_route().list(&opts, None).await {
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
    let route = NetworkRoute {
        route_id: route_id.clone(),
        unit_id: application.unit_id,
        unit_code: application.unit_code,
        application_id: application.application_id,
        application_code: application.code,
        network_id: network.network_id,
        network_code: network.code,
        created_at: now,
    };
    if let Err(e) = state.model.network_route().add(&route).await {
        error!("[{}] add error: {}", FN_NAME, e);
        return Err(ErrResp::ErrDb(Some(e.to_string())));
    }
    send_del_ctrl_message(FN_NAME, route, &state).await?;

    Ok(HttpResponse::Ok().json(response::PostNetworkRoute {
        data: response::PostNetworkRouteData { route_id },
    }))
}

/// `GET /{base}/api/v1/network-route/count`
pub async fn get_network_route_count(
    req: HttpRequest,
    query: web::Query<request::GetNetworkRouteCountQuery>,
    state: web::Data<State>,
) -> impl Responder {
    const FN_NAME: &'static str = "get_network_route_count";

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
        ..Default::default()
    };
    match state.model.network_route().count(&cond).await {
        Err(e) => {
            error!("[{}] count error: {}", FN_NAME, e);
            Err(ErrResp::ErrDb(Some(e.to_string())))
        }
        Ok(count) => Ok(HttpResponse::Ok().json(response::GetNetworkRouteCount {
            data: response::GetCountData { count },
        })),
    }
}

/// `GET /{base}/api/v1/network-route/list`
pub async fn get_network_route_list(
    req: HttpRequest,
    query: web::Query<request::GetNetworkRouteListQuery>,
    state: web::Data<State>,
) -> impl Responder {
    const FN_NAME: &'static str = "get_network_route_list";

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

    let (list, cursor) = match state.model.network_route().list(&opts, None).await {
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
                    return Ok(HttpResponse::Ok().json(response::GetNetworkRouteList {
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
            let (_list, _cursor) = match state.model.network_route().list(&opts, cursor).await {
                Err(_) => break,
                Ok((list, cursor)) => (list, cursor),
            };
            list = _list;
            cursor = _cursor;
        }
    };
    Ok(HttpResponse::Ok().streaming(stream))
}

/// `DELETE /{base}/api/v1/network-route/{routeId}`
pub async fn delete_network_route(
    req: HttpRequest,
    param: web::Path<request::RouteIdPath>,
    state: web::Data<State>,
) -> impl Responder {
    const FN_NAME: &'static str = "delete_network_route";

    let (user_id, roles) = get_token_id_roles(FN_NAME, &req)?;
    let user_id = user_id.as_str();
    let route_id = param.route_id.as_str();

    // To check if the network route is for the user.
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
    if let Err(e) = state.model.network_route().del(&cond).await {
        error!("[{}] del error: {}", FN_NAME, e);
        return Err(ErrResp::ErrDb(Some(e.to_string())));
    }
    send_del_ctrl_message(FN_NAME, route, &state).await?;

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
                        "created" => SortKey::CreatedAt,
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

/// To check if the user ID can access the network route. Choose `only_owner` to check if the user
/// is the unit owner or one of unit members.
///
/// # Errors
///
/// Returns OK if the network route is found or not. Otherwise errors will be returned.
async fn check_route(
    fn_name: &str,
    route_id: &str,
    user_id: &str,
    only_owner: bool, // to check if this `user_id` is the owner.
    roles: &HashMap<String, bool>,
    state: &web::Data<State>,
) -> Result<Option<NetworkRoute>, ErrResp> {
    let route = match state.model.network_route().get(route_id).await {
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

fn route_list_transform(list: &Vec<NetworkRoute>) -> Vec<response::GetNetworkRouteData> {
    let mut ret = vec![];
    for route in list.iter() {
        ret.push(route_transform(&route));
    }
    ret
}

fn route_list_transform_bytes(
    list: &Vec<NetworkRoute>,
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

fn route_transform(route: &NetworkRoute) -> response::GetNetworkRouteData {
    response::GetNetworkRouteData {
        route_id: route.route_id.clone(),
        unit_id: route.unit_id.clone(),
        application_id: route.application_id.clone(),
        application_code: route.application_code.clone(),
        network_id: route.network_id.clone(),
        network_code: route.network_code.clone(),
        created_at: time_str(&route.created_at),
    }
}

/// Send delete control message.
async fn send_del_ctrl_message(
    fn_name: &str,
    route: NetworkRoute,
    state: &web::Data<State>,
) -> Result<(), ErrResp> {
    if state.cache.is_some() {
        let msg = SendCtrlMsg::DelNetworkRoute {
            operation: CtrlMsgOp::DEL_NETWORK_ROUTE.to_string(),
            new: CtrlDelNetworkRoute {
                unit_id: route.unit_id,
                unit_code: Some(route.unit_code),
                network_id: route.network_id,
                network_code: route.network_code,
            },
        };
        let payload = match serde_json::to_vec(&msg) {
            Err(e) => {
                error!(
                    "[{}] marshal JSON for {} error: {}",
                    fn_name,
                    CtrlMsgOp::DEL_NETWORK_ROUTE,
                    e
                );
                return Err(ErrResp::ErrRsc(Some(format!(
                    "marshal control message error: {}",
                    e
                ))));
            }
            Ok(payload) => payload,
        };
        let ctrl_sender = { state.ctrl_senders.network_route.lock().unwrap().clone() };
        if let Err(e) = ctrl_sender.send_msg(payload).await {
            error!(
                "[{}] send control message for {} error: {}",
                fn_name,
                CtrlMsgOp::DEL_NETWORK_ROUTE,
                e
            );
            return Err(ErrResp::ErrIntMsg(Some(format!(
                "send control message error: {}",
                e
            ))));
        }
    }

    Ok(())
}

#[async_trait]
impl QueueEventHandler for CtrlSenderHandler {
    async fn on_event(&self, queue: Arc<dyn GmqQueue>, ev: Event) {
        const FN_NAME: &'static str = "CtrlSenderHandler::on_event";
        let queue_name = queue.name();

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
            RecvCtrlMsg::DelNetworkRoute { new } => {
                if let Some(cache) = self.cache.as_ref() {
                    if let Err(e) = cache
                        .network_route()
                        .del_uldata(new.network_id.as_str())
                        .await
                    {
                        error!(
                            "[{}] {} delete network route cache error: {}",
                            FN_NAME, queue_name, e
                        );
                    }
                }
                if let Err(e) = msg.ack().await {
                    error!("[{}] {} ACK error: {}", FN_NAME, queue_name, e);
                }
            }
        }
    }
}
