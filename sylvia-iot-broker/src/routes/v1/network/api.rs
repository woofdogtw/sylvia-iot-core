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
use chrono::{DateTime, Utc};
use log::{error, info, warn};
use serde::{Deserialize, Serialize};
use serde_json::{self, Map, Value};
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
    strings::{self, time_str},
};

use super::{
    super::{
        super::{middleware::GetTokenInfoData, ErrReq, State as AppState},
        lib::{check_network, check_unit, gen_mgr_key},
    },
    request, response,
};
use crate::{
    libs::{
        config::BrokerCtrl as CfgCtrl,
        mq::{
            self,
            application::{
                ApplicationMgr, DlDataResult as ApplicationDlDataResult,
                UlData as ApplicationUlData,
            },
            network::{DlDataResult, EventHandler, NetworkMgr, UlData},
            Connection, MgrStatus, Options as MgrOptions,
        },
    },
    models::{
        device::{self, DeviceCacheItem},
        device_route, dldata_buffer,
        network::{
            ListOptions, ListQueryCond, Network, QueryCond, SortCond, SortKey, UpdateQueryCond,
            Updates,
        },
        network_route, Cache, Model,
    },
};

struct MgrHandler {
    model: Arc<dyn Model>,
    cache: Option<Arc<dyn Cache>>,
    application_mgrs: Arc<Mutex<HashMap<String, ApplicationMgr>>>,
    data_sender: Option<Queue>,
}

#[derive(Deserialize, Serialize)]
#[serde(tag = "operation")]
enum RecvCtrlMsg {
    #[serde(rename = "del-network")]
    DelNetwork { new: CtrlDelNetwork },
    #[serde(rename = "add-manager")]
    AddManager { new: CtrlAddManager },
    #[serde(rename = "del-manager")]
    DelManager { new: String },
}

/// Control channel.
#[derive(Serialize)]
#[serde(untagged)]
enum SendCtrlMsg {
    DelNetwork {
        operation: String,
        new: CtrlDelNetwork,
    },
    AddManager {
        operation: String,
        new: CtrlAddManager,
    },
    DelManager {
        operation: String,
        new: String,
    },
}

/// Data channel.
#[derive(Serialize)]
struct SendDataMsg {
    kind: String,
    data: SendDataKind,
}

#[derive(Serialize)]
#[serde(untagged)]
enum SendDataKind {
    AppUlData {
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
        #[serde(skip_serializing_if = "Option::is_none")]
        extension: Option<Map<String, Value>>,
    },
    AppDlDataResult {
        #[serde(rename = "dataId")]
        data_id: String,
        resp: String,
        status: i32,
    },
    NetUlData {
        #[serde(rename = "dataId")]
        data_id: String,
        proc: String,
        #[serde(rename = "unitCode")]
        unit_code: Option<String>,
        #[serde(rename = "networkCode")]
        network_code: String,
        #[serde(rename = "networkAddr")]
        network_addr: String,
        #[serde(rename = "unitId", skip_serializing_if = "Option::is_none")]
        unit_id: Option<String>,
        #[serde(rename = "deviceId", skip_serializing_if = "Option::is_none")]
        device_id: Option<String>,
        time: String,
        profile: Option<String>,
        data: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        extension: Option<Map<String, Value>>,
    },
    NetDlDataResult {
        #[serde(rename = "dataId")]
        data_id: String,
        resp: String,
        status: i32,
    },
}

struct CtrlMsgOp;
struct DataMsgKind;

#[derive(Deserialize, Serialize)]
struct CtrlDelNetwork {
    #[serde(rename = "unitId")]
    unit_id: Option<String>,
    #[serde(rename = "unitCode")]
    unit_code: Option<String>,
    #[serde(rename = "networkId")]
    network_id: String,
    #[serde(rename = "networkCode")]
    network_code: String,
}

#[derive(Deserialize, Serialize)]
struct CtrlAddManager {
    #[serde(rename = "hostUri")]
    host_uri: String,
    #[serde(rename = "mgrOptions")]
    mgr_options: MgrOptions,
}

struct CtrlSenderHandler {
    cache: Option<Arc<dyn Cache>>,
}

struct CtrlReceiverHandler {
    model: Arc<dyn Model>,
    cache: Option<Arc<dyn Cache>>,
    mq_conns: Arc<Mutex<HashMap<String, Connection>>>,
    application_mgrs: Arc<Mutex<HashMap<String, ApplicationMgr>>>,
    network_mgrs: Arc<Mutex<HashMap<String, NetworkMgr>>>,
    data_sender: Option<Queue>,
}

impl CtrlMsgOp {
    const DEL_NETWORK: &'static str = "del-network";
    const ADD_MANAGER: &'static str = "add-manager";
    const DEL_MANAGER: &'static str = "del-manager";
}

impl DataMsgKind {
    const APP_ULDATA: &'static str = "application-uldata";
    const APP_DLDATA_RES: &'static str = "application-dldata-result";
    const NET_ULDATA: &'static str = "network-uldata";
    const NET_DLDATA_RES: &'static str = "network-dldata-result";
}

const LIST_LIMIT_DEFAULT: u64 = 100;
const LIST_CURSOR_MAX: u64 = 100;
const ID_RAND_LEN: usize = 8;
const DATA_ID_RAND_LEN: usize = 12;
const CTRL_QUEUE_NAME: &'static str = "network";

/// Initialize network managers and channels.
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

    let ctrl_sender = { state.ctrl_senders.network.lock().unwrap().clone() };
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

    let cond = ListQueryCond {
        ..Default::default()
    };
    let opts = ListOptions {
        cond: &cond,
        offset: None,
        limit: None,
        sort: None,
        cursor_max: Some(LIST_CURSOR_MAX),
    };
    let mut list;
    let mut cursor = None;
    loop {
        (list, cursor) = state.model.network().list(&opts, cursor).await?;
        for item in list.iter() {
            let url = Url::parse(item.host_uri.as_str())?;
            let unit_id = match item.unit_id.as_ref() {
                None => "",
                Some(unit_id) => unit_id.as_str(),
            };
            let unit_code = match item.unit_code.as_ref() {
                None => "",
                Some(unit_code) => unit_code.as_str(),
            };
            let key = gen_mgr_key(unit_code, item.code.as_str());
            let opts = MgrOptions {
                unit_id: unit_id.to_string(),
                unit_code: unit_code.to_string(),
                id: item.network_id.clone(),
                name: item.code.clone(),
                prefetch: Some(state.amqp_prefetch),
                persistent: state.amqp_persistent,
                shared_prefix: Some(state.mqtt_shared_prefix.clone()),
            };
            let handler = MgrHandler {
                model: state.model.clone(),
                cache: state.cache.clone(),
                application_mgrs: state.application_mgrs.clone(),
                data_sender: state.data_sender.clone(),
            };
            let mgr = match NetworkMgr::new(state.mq_conns.clone(), &url, opts, Arc::new(handler)) {
                Err(e) => {
                    error!("[{}] new manager for {} error: {}", FN_NAME, key, e);
                    return Err(Box::new(ErrResp::ErrRsc(Some(e))));
                }
                Ok(mgr) => mgr,
            };
            {
                state.network_mgrs.lock().unwrap().insert(key.clone(), mgr);
            }
        }
        if cursor.is_none() {
            break;
        }
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
        model: state.model.clone(),
        cache: state.cache.clone(),
        mq_conns: state.mq_conns.clone(),
        application_mgrs: state.application_mgrs.clone(),
        network_mgrs: state.network_mgrs.clone(),
        data_sender: state.data_sender.clone(),
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

/// `POST /{base}/api/v1/network`
pub async fn post_network(
    State(state): State<AppState>,
    Extension(token_info): Extension<GetTokenInfoData>,
    Json(body): Json<request::PostNetworkBody>,
) -> impl IntoResponse {
    const FN_NAME: &'static str = "post_network";

    let user_id = token_info.user_id.as_str();
    let roles = &token_info.roles;

    let code = body.data.code.to_lowercase();
    let host_uri = body.data.host_uri.as_str();
    if !strings::is_code(code.as_str()) {
        return Err(ErrResp::ErrParam(Some(
            "`code` must be [A-Za-z0-9]{1}[A-Za-z0-9-_]*".to_string(),
        )));
    }
    let host_uri = match Url::parse(host_uri) {
        Err(_) => return Err(ErrResp::ErrParam(Some("invalid `hostUri`".to_string()))),
        Ok(uri) => match mq::SUPPORT_SCHEMES.contains(&uri.scheme()) {
            false => {
                return Err(ErrResp::ErrParam(Some(
                    "unsupport `hostUri` scheme".to_string(),
                )))
            }
            true => uri,
        },
    };
    if let Some(info) = body.data.info.as_ref() {
        for (k, _) in info.iter() {
            if k.len() == 0 {
                return Err(ErrResp::ErrParam(Some(
                    "`info` key must not be empty".to_string(),
                )));
            }
        }
    }
    let mut unit_code: Option<String> = None;
    let unit_id = match body.data.unit_id.as_ref() {
        None => {
            if !Role::is_role(roles, Role::ADMIN) && !Role::is_role(roles, Role::MANAGER) {
                return Err(ErrResp::ErrParam(Some("missing `unitId`".to_string())));
            }
            None
        }
        Some(unit_id) => {
            if unit_id.len() == 0 {
                return Err(ErrResp::ErrParam(Some(
                    "`unitId` must with at least one character".to_string(),
                )));
            }
            match check_unit(FN_NAME, user_id, roles, unit_id.as_str(), true, &state).await? {
                None => {
                    return Err(ErrResp::Custom(
                        ErrReq::UNIT_NOT_EXIST.0,
                        ErrReq::UNIT_NOT_EXIST.1,
                        None,
                    ))
                }
                Some(unit) => {
                    unit_code = Some(unit.code);
                }
            }
            Some(unit_id.clone())
        }
    };
    if check_code(FN_NAME, unit_id.as_ref(), code.as_str(), &state).await? {
        return Err(ErrResp::Custom(
            ErrReq::NETWORK_EXIST.0,
            ErrReq::NETWORK_EXIST.1,
            None,
        ));
    }

    let now = Utc::now();
    let network = Network {
        network_id: strings::random_id(&now, ID_RAND_LEN),
        code: code.clone(),
        unit_id: unit_id.clone(),
        unit_code: unit_code.clone(),
        created_at: now,
        modified_at: now,
        host_uri: host_uri.to_string(),
        name: match body.data.name.as_ref() {
            None => "".to_string(),
            Some(name) => name.clone(),
        },
        info: match body.data.info.as_ref() {
            None => Map::new(),
            Some(info) => info.clone(),
        },
    };
    if let Err(e) = state.model.network().add(&network).await {
        error!("[{}] add error: {}", FN_NAME, e);
        return Err(ErrResp::ErrDb(Some(e.to_string())));
    }
    let unit_id = match unit_id.as_ref() {
        None => "",
        Some(id) => id.as_str(),
    };
    let unit_code = match unit_code.as_ref() {
        None => "",
        Some(code) => code.as_str(),
    };
    add_manager(
        FN_NAME,
        &state,
        &host_uri,
        unit_id,
        unit_code,
        network.network_id.as_str(),
        code.as_str(),
    )
    .await?;
    Ok(Json(response::PostNetwork {
        data: response::PostNetworkData {
            network_id: network.network_id,
        },
    }))
}

/// `GET /{base}/api/v1/network/count`
pub async fn get_network_count(
    State(state): State<AppState>,
    Extension(token_info): Extension<GetTokenInfoData>,
    Query(query): Query<request::GetNetworkCountQuery>,
) -> impl IntoResponse {
    const FN_NAME: &'static str = "get_network_count";

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
            0 => Some(None),
            _ => {
                match check_unit(FN_NAME, user_id, roles, unit_id.as_str(), false, &state).await? {
                    None => {
                        return Err(ErrResp::Custom(
                            ErrReq::UNIT_NOT_EXIST.0,
                            ErrReq::UNIT_NOT_EXIST.1,
                            None,
                        ))
                    }
                    Some(_) => Some(Some(unit_id.as_str())),
                }
            }
        },
    };
    let mut code_cond = None;
    let mut code_contains_cond = None;
    if let Some(code) = query.code.as_ref() {
        if code.len() > 0 {
            code_cond = Some(code.as_str());
        }
    }
    if code_cond.is_none() {
        if let Some(contains) = query.contains.as_ref() {
            if contains.len() > 0 {
                code_contains_cond = Some(contains.as_str());
            }
        }
    }
    let cond = ListQueryCond {
        unit_id: unit_cond,
        code: code_cond,
        code_contains: code_contains_cond,
        ..Default::default()
    };
    match state.model.network().count(&cond).await {
        Err(e) => {
            error!("[{}] count error: {}", FN_NAME, e);
            Err(ErrResp::ErrDb(Some(e.to_string())))
        }
        Ok(count) => Ok(Json(response::GetNetworkCount {
            data: response::GetCountData { count },
        })),
    }
}

/// `GET /{base}/api/v1/network/list`
pub async fn get_network_list(
    State(state): State<AppState>,
    Extension(token_info): Extension<GetTokenInfoData>,
    Query(query): Query<request::GetNetworkListQuery>,
) -> impl IntoResponse {
    const FN_NAME: &'static str = "get_network_list";

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
            0 => Some(None),
            _ => {
                match check_unit(FN_NAME, user_id, roles, unit_id.as_str(), false, &state).await? {
                    None => {
                        return Err(ErrResp::Custom(
                            ErrReq::UNIT_NOT_EXIST.0,
                            ErrReq::UNIT_NOT_EXIST.1,
                            None,
                        ))
                    }
                    Some(_) => Some(Some(unit_id.as_str())),
                }
            }
        },
    };
    let mut code_cond = None;
    let mut code_contains_cond = None;
    if let Some(code) = query.code.as_ref() {
        if code.len() > 0 {
            code_cond = Some(code.as_str());
        }
    }
    if code_cond.is_none() {
        if let Some(contains) = query.contains.as_ref() {
            if contains.len() > 0 {
                code_contains_cond = Some(contains.as_str());
            }
        }
    }
    let cond = ListQueryCond {
        unit_id: unit_cond,
        code: code_cond,
        code_contains: code_contains_cond,
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

    let (list, cursor) = match state.model.network().list(&opts, None).await {
        Err(e) => {
            error!("[{}] list error: {}", FN_NAME, e);
            return Err(ErrResp::ErrDb(Some(e.to_string())));
        }
        Ok((list, cursor)) => match cursor {
            None => match query.format {
                Some(request::ListFormat::Array) => {
                    return Ok(Json(network_list_transform(&list)).into_response())
                }
                _ => {
                    return Ok(Json(response::GetNetworkList {
                        data: network_list_transform(&list),
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
                0 => Some(None),
                _ => Some(Some(unit_id.as_str())),
            },
        };
        let mut code_contains_cond = None;
        if let Some(contains) = query.contains.as_ref() {
            if contains.len() > 0 {
                code_contains_cond = Some(contains.as_str());
            }
        }
        let cond = ListQueryCond {
            unit_id: unit_cond,
            code_contains: code_contains_cond,
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
            yield network_list_transform_bytes(&list, is_first, cursor.is_none(), query.format.as_ref());
            is_first = false;
            if cursor.is_none() {
                break;
            }
            let (_list, _cursor) = match state.model.network().list(&opts, cursor).await {
                Err(_) => break,
                Ok((list, cursor)) => (list, cursor),
            };
            list = _list;
            cursor = _cursor;
        }
    });
    Ok(([(header::CONTENT_TYPE, ContentType::JSON)], body).into_response())
}

/// `GET /{base}/api/v1/network/{networkId}`
pub async fn get_network(
    State(state): State<AppState>,
    Extension(token_info): Extension<GetTokenInfoData>,
    Path(param): Path<request::NetworkIdPath>,
) -> impl IntoResponse {
    const FN_NAME: &'static str = "get_network";

    let user_id = token_info.user_id.as_str();
    let roles = &token_info.roles;
    let network_id = param.network_id.as_str();

    match check_network(FN_NAME, network_id, user_id, false, roles, &state).await? {
        None => Err(ErrResp::ErrNotFound(None)),
        Some(network) => Ok(Json(response::GetNetwork {
            data: network_transform(&network),
        })),
    }
}

/// `PATCH /{base}/api/v1/network/{networkId}`
pub async fn patch_network(
    State(state): State<AppState>,
    Extension(token_info): Extension<GetTokenInfoData>,
    Path(param): Path<request::NetworkIdPath>,
    Json(mut body): Json<request::PatchNetworkBody>,
) -> impl IntoResponse {
    const FN_NAME: &'static str = "patch_network";

    let user_id = token_info.user_id.as_str();
    let roles = &token_info.roles;
    let network_id = param.network_id.as_str();

    // To check if the network is for the user.
    let network = match check_network(FN_NAME, network_id, user_id, true, roles, &state).await? {
        None => return Err(ErrResp::ErrNotFound(None)),
        Some(network) => network,
    };

    let updates = get_updates(&mut body.data).await?;
    let mut should_add_mgr = false;

    // Remove old manager.
    if let Some(host_uri) = updates.host_uri {
        let uri = Url::parse(host_uri).unwrap();
        if !uri.as_str().eq(network.host_uri.as_str()) {
            delete_manager(FN_NAME, &state, &network).await?;
            should_add_mgr = true;
        }
    }

    // Update database.
    let cond = UpdateQueryCond { network_id };
    if let Err(e) = state.model.network().update(&cond, &updates).await {
        error!("[{}] update error: {}", FN_NAME, e);
        return Err(ErrResp::ErrDb(Some(e.to_string())));
    }

    // Add new manager.
    if should_add_mgr {
        if let Some(host_uri) = updates.host_uri {
            let uri = Url::parse(host_uri).unwrap();
            let unit_id = match network.unit_id.as_ref() {
                None => "",
                Some(id) => id.as_str(),
            };
            let unit_code = match network.unit_code.as_ref() {
                None => "",
                Some(code) => code.as_str(),
            };
            add_manager(
                FN_NAME,
                &state,
                &uri,
                unit_id,
                unit_code,
                network.network_id.as_str(),
                network.code.as_str(),
            )
            .await?;
        }
    }
    Ok(StatusCode::NO_CONTENT)
}

/// `DELETE /{base}/api/v1/network/{networkId}`
pub async fn delete_network(
    State(state): State<AppState>,
    Extension(token_info): Extension<GetTokenInfoData>,
    Path(param): Path<request::NetworkIdPath>,
) -> impl IntoResponse {
    const FN_NAME: &'static str = "delete_network";

    let user_id = token_info.user_id.as_str();
    let roles = &token_info.roles;
    let network_id = param.network_id.as_str();

    // To check if the network is for the user.
    let network = match check_network(FN_NAME, network_id, user_id, true, roles, &state).await {
        Err(e) => return Err(e), // XXX: not use "?" to solve E0282 error.
        Ok(network) => match network {
            None => return Ok(StatusCode::NO_CONTENT),
            Some(network) => network,
        },
    };

    delete_manager(FN_NAME, &state, &network).await?;
    del_network_rsc(FN_NAME, network_id, &state).await?;
    send_del_ctrl_message(FN_NAME, network, &state).await?;

    Ok(StatusCode::NO_CONTENT)
}

fn get_sort_cond(sort_args: &Option<String>) -> Result<Vec<SortCond>, ErrResp> {
    match sort_args.as_ref() {
        None => Ok(vec![SortCond {
            key: SortKey::Code,
            asc: true,
        }]),
        Some(args) => {
            let mut args = args.split(",");
            let mut sort_cond = vec![];
            while let Some(arg) = args.next() {
                let mut cond = arg.split(":");
                let key = match cond.next() {
                    None => return Err(ErrResp::ErrParam(Some("wrong sort argument".to_string()))),
                    Some(field) => match field {
                        "code" => SortKey::Code,
                        "created" => SortKey::CreatedAt,
                        "modified" => SortKey::ModifiedAt,
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

async fn get_updates<'a>(body: &'a mut request::PatchNetworkData) -> Result<Updates<'a>, ErrResp> {
    let mut updates = Updates {
        ..Default::default()
    };
    let mut count = 0;
    if let Some(host_uri) = body.host_uri.as_ref() {
        match Url::parse(host_uri) {
            Err(_) => return Err(ErrResp::ErrParam(Some("invalid `hostUri`".to_string()))),
            Ok(uri) => {
                if !mq::SUPPORT_SCHEMES.contains(&uri.scheme()) {
                    return Err(ErrResp::ErrParam(Some(
                        "unsupport `hostUri` scheme".to_string(),
                    )));
                }
                body.host_uri = Some(uri.to_string()); // change host name case.
            }
        }
    }
    if let Some(host_uri) = body.host_uri.as_ref() {
        updates.host_uri = Some(host_uri.as_str());
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

/// To check if the network code is used by the unit.
///
/// # Errors
///
/// Returns OK if the code is found or not. Otherwise errors will be returned.
async fn check_code(
    fn_name: &str,
    unit_id: Option<&String>,
    code: &str,
    state: &AppState,
) -> Result<bool, ErrResp> {
    let cond = QueryCond {
        unit_id: match unit_id {
            None => Some(None),
            Some(unit_id) => Some(Some(unit_id.as_str())),
        },
        code: Some(code),
        ..Default::default()
    };
    match state.model.network().get(&cond).await {
        Err(e) => {
            error!("[{}] check code error: {}", fn_name, e);
            return Err(ErrResp::ErrDb(Some(format!("check code error: {}", e))));
        }
        Ok(network) => match network {
            None => Ok(false),
            Some(_) => Ok(true),
        },
    }
}

fn network_list_transform(list: &Vec<Network>) -> Vec<response::GetNetworkData> {
    let mut ret = vec![];
    for network in list.iter() {
        ret.push(network_transform(&network));
    }
    ret
}

fn network_list_transform_bytes(
    list: &Vec<Network>,
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
        let json_str = match serde_json::to_string(&network_transform(item)) {
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

fn network_transform(network: &Network) -> response::GetNetworkData {
    response::GetNetworkData {
        network_id: network.network_id.clone(),
        code: network.code.clone(),
        unit_id: network.unit_id.clone(),
        unit_code: network.unit_code.clone(),
        created_at: time_str(&network.created_at),
        modified_at: time_str(&network.modified_at),
        host_uri: network.host_uri.clone(),
        name: network.name.clone(),
        info: network.info.clone(),
    }
}

async fn del_network_rsc(fn_name: &str, network_id: &str, state: &AppState) -> Result<(), ErrResp> {
    let cond = network_route::QueryCond {
        network_id: Some(network_id),
        ..Default::default()
    };
    if let Err(e) = state.model.network_route().del(&cond).await {
        error!("[{}] del network_route error: {}", fn_name, e);
        return Err(ErrResp::ErrDb(Some(e.to_string())));
    }

    let cond = device_route::QueryCond {
        network_id: Some(network_id),
        ..Default::default()
    };
    if let Err(e) = state.model.device_route().del(&cond).await {
        error!("[{}] del device_route error: {}", fn_name, e);
        return Err(ErrResp::ErrDb(Some(e.to_string())));
    }

    let cond = dldata_buffer::QueryCond {
        network_id: Some(network_id),
        ..Default::default()
    };
    if let Err(e) = state.model.dldata_buffer().del(&cond).await {
        error!("[{}] del dldata_buffer error: {}", fn_name, e);
        return Err(ErrResp::ErrDb(Some(e.to_string())));
    }

    let cond = device::QueryCond {
        network_id: Some(network_id),
        ..Default::default()
    };
    if let Err(e) = state.model.device().del(&cond).await {
        error!("[{}] del device error: {}", fn_name, e);
        return Err(ErrResp::ErrDb(Some(e.to_string())));
    }

    let cond = QueryCond {
        network_id: Some(network_id),
        ..Default::default()
    };
    if let Err(e) = state.model.network().del(&cond).await {
        error!("[{}] del network error: {}", fn_name, e);
        return Err(ErrResp::ErrDb(Some(e.to_string())));
    }

    Ok(())
}

/// Send delete control message.
async fn send_del_ctrl_message(
    fn_name: &str,
    network: Network,
    state: &AppState,
) -> Result<(), ErrResp> {
    if state.cache.is_some() {
        let msg = SendCtrlMsg::DelNetwork {
            operation: CtrlMsgOp::DEL_NETWORK.to_string(),
            new: CtrlDelNetwork {
                unit_id: network.unit_id,
                unit_code: network.unit_code,
                network_id: network.network_id,
                network_code: network.code,
            },
        };
        let payload = match serde_json::to_vec(&msg) {
            Err(e) => {
                error!(
                    "[{}] marshal JSON for {} error: {}",
                    fn_name,
                    CtrlMsgOp::DEL_NETWORK,
                    e
                );
                return Err(ErrResp::ErrRsc(Some(format!(
                    "marshal control message error: {}",
                    e
                ))));
            }
            Ok(payload) => payload,
        };
        let ctrl_sender = { state.ctrl_senders.network.lock().unwrap().clone() };
        if let Err(e) = ctrl_sender.send_msg(payload).await {
            error!(
                "[{}] send control message for {} error: {}",
                fn_name,
                CtrlMsgOp::DEL_NETWORK,
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

/// To create a manager by:
/// - get a connection from the pool.
/// - register manager handlers.
async fn add_manager(
    fn_name: &str,
    state: &AppState,
    host_uri: &Url,
    unit_id: &str,
    unit_code: &str,
    id: &str,
    name: &str,
) -> Result<(), ErrResp> {
    let opts = MgrOptions {
        unit_id: unit_id.to_string(),
        unit_code: unit_code.to_string(),
        id: id.to_string(),
        name: name.to_string(),
        prefetch: Some(state.amqp_prefetch),
        persistent: state.amqp_persistent,
        shared_prefix: Some(state.mqtt_shared_prefix.clone()),
    };
    let msg = SendCtrlMsg::AddManager {
        operation: CtrlMsgOp::ADD_MANAGER.to_string(),
        new: CtrlAddManager {
            host_uri: host_uri.to_string(),
            mgr_options: opts,
        },
    };
    let payload = match serde_json::to_vec(&msg) {
        Err(e) => {
            error!("[{}] marshal JSON for {} error: {}", fn_name, name, e);
            return Err(ErrResp::ErrRsc(Some(format!("new manager error: {}", e))));
        }
        Ok(payload) => payload,
    };
    let ctrl_sender = { state.ctrl_senders.network.lock().unwrap().clone() };
    if let Err(e) = ctrl_sender.send_msg(payload).await {
        error!(
            "[{}] send control message for {} error: {}",
            fn_name, name, e
        );
        return Err(ErrResp::ErrIntMsg(Some(format!(
            "new manager error: {}",
            e
        ))));
    }
    Ok(())
}

/// To delete a network manager.
async fn delete_manager(fn_name: &str, state: &AppState, network: &Network) -> Result<(), ErrResp> {
    let key = match network.unit_code.as_ref() {
        None => gen_mgr_key("", network.code.as_str()),
        Some(unit_code) => gen_mgr_key(unit_code.as_str(), network.code.as_str()),
    };
    let msg = SendCtrlMsg::DelManager {
        operation: CtrlMsgOp::DEL_MANAGER.to_string(),
        new: key.clone(),
    };
    let payload = match serde_json::to_vec(&msg) {
        Err(e) => {
            error!("[{}] marshal JSON for {} error: {}", fn_name, key, e);
            return Err(ErrResp::ErrRsc(Some(format!(
                "delete manager error: {}",
                e
            ))));
        }
        Ok(payload) => payload,
    };
    let ctrl_sender = { state.ctrl_senders.network.lock().unwrap().clone() };
    if let Err(e) = ctrl_sender.send_msg(payload).await {
        error!(
            "[{}] send control message for {} error: {}",
            fn_name, key, e
        );
        return Err(ErrResp::ErrIntMsg(Some(format!(
            "delete manager error: {}",
            e
        ))));
    }
    Ok(())
}

impl MgrHandler {
    /// To validate the device and return device ID and profile.
    async fn validate_device(
        &self,
        mgr: &NetworkMgr,
        data: &Box<UlData>,
    ) -> Result<Option<DeviceCacheItem>, Box<dyn StdError>> {
        const FN_NAME: &'static str = "validate_device";

        let dev_cond = device::QueryOneCond {
            unit_code: match mgr.unit_code().len() {
                0 => None,
                _ => Some(mgr.unit_code()),
            },
            network_code: mgr.name(),
            network_addr: data.network_addr.as_str(),
        };
        match self.cache.as_ref() {
            None => {
                let cond = device::QueryCond {
                    device: Some(dev_cond.clone()),
                    ..Default::default()
                };
                match self.model.device().get(&cond).await {
                    Err(e) => {
                        error!("[{}] get device with error: {}", FN_NAME, e);
                        return Err(e);
                    }
                    Ok(device) => match device {
                        None => {
                            warn!(
                                "[{}] no device for {:?}.{}.{}",
                                FN_NAME,
                                dev_cond.unit_code,
                                dev_cond.network_code,
                                dev_cond.network_addr
                            );
                            Ok(None)
                        }
                        Some(device) => Ok(Some(DeviceCacheItem {
                            device_id: device.device_id,
                            profile: device.profile,
                        })),
                    },
                }
            }
            Some(cache) => {
                let cond = device::GetCacheQueryCond::CodeAddr(dev_cond.clone());
                match cache.device().get(&cond).await {
                    Err(e) => {
                        error!("[{}] get device cache with error: {}", FN_NAME, e);
                        Err(e)
                    }
                    Ok(device) => match device {
                        None => {
                            warn!(
                                "[{}] no device cache for {:?}.{}.{}",
                                FN_NAME,
                                dev_cond.unit_code,
                                dev_cond.network_code,
                                dev_cond.network_addr
                            );
                            Ok(None)
                        }
                        Some(device) => Ok(Some(device)),
                    },
                }
            }
        }
    }

    async fn send_by_device_route(
        &self,
        netmgr_unit_code: Option<String>,
        app_data: &mut ApplicationUlData,
        sent_mgrs: &mut Vec<String>,
    ) -> Result<(), ()> {
        const FN_NAME: &'static str = "send_by_device_route";

        if let Some(cache) = self.cache.as_ref() {
            let route = match cache
                .device_route()
                .get_uldata(&app_data.device_id.as_str())
                .await
            {
                Err(e) => {
                    error!("[{}] get device route error: {}", FN_NAME, e);
                    return Err(());
                }
                Ok(route) => match route {
                    None => return Ok(()),
                    Some(route) => route,
                },
            };
            for key in route.app_mgr_keys.iter() {
                if sent_mgrs.contains(key) {
                    continue;
                }
                let mgr = {
                    match self.application_mgrs.lock().unwrap().get(key) {
                        None => continue,
                        Some(mgr) => mgr.clone(),
                    }
                };
                let now = Utc::now();
                app_data.data_id = strings::random_id(&now, DATA_ID_RAND_LEN);
                if let Err(e) = mgr.send_uldata(&app_data) {
                    // TODO: retry internally because one or more routes may sent successfully.
                    error!("[{}] send data to {} error: {}", FN_NAME, key, e);
                    continue;
                }
                self.send_application_uldata_msg(
                    &now,
                    netmgr_unit_code.clone(),
                    mgr.unit_id().to_string(),
                    &app_data,
                )
                .await?;
                sent_mgrs.push(key.clone());
            }

            return Ok(());
        }

        let cond = device_route::ListQueryCond {
            device_id: Some(app_data.device_id.as_str()),
            ..Default::default()
        };
        let opts = device_route::ListOptions {
            cond: &cond,
            offset: None,
            limit: None,
            sort: None,
            cursor_max: Some(LIST_CURSOR_MAX),
        };
        let mut cursor: Option<Box<dyn device_route::Cursor>> = None;
        loop {
            let (list, _cursor) = match self.model.device_route().list(&opts, cursor).await {
                Err(e) => {
                    error!("[{}] get device route error: {}", FN_NAME, e);
                    return Err(());
                }
                Ok((list, cursor)) => (list, cursor),
            };
            for route in list.iter() {
                let key = gen_mgr_key(route.unit_code.as_str(), route.application_code.as_str());
                if sent_mgrs.contains(&key) {
                    continue;
                }
                let mgr = {
                    match self.application_mgrs.lock().unwrap().get(&key) {
                        None => continue,
                        Some(mgr) => mgr.clone(),
                    }
                };
                let now = Utc::now();
                app_data.data_id = strings::random_id(&now, DATA_ID_RAND_LEN);
                if let Err(e) = mgr.send_uldata(&app_data) {
                    // TODO: retry internally because one or more routes may sent successfully.
                    error!("[{}] send data to {} error: {}", FN_NAME, key, e);
                    continue;
                }
                self.send_application_uldata_msg(
                    &now,
                    netmgr_unit_code.clone(),
                    mgr.unit_id().to_string(),
                    &app_data,
                )
                .await?;
                sent_mgrs.push(key);
            }
            if _cursor.is_none() {
                break;
            }
            cursor = _cursor;
        }
        Ok(())
    }

    async fn send_by_network_route(
        &self,
        netmgr_unit_id: String,
        netmgr_unit_code: Option<String>,
        app_data: &mut ApplicationUlData,
        sent_mgrs: &mut Vec<String>,
    ) -> Result<(), ()> {
        const FN_NAME: &'static str = "send_by_network_route";

        if let Some(cache) = self.cache.as_ref() {
            let route = match cache
                .network_route()
                .get_uldata(&app_data.network_id.as_str())
                .await
            {
                Err(e) => {
                    error!("[{}] get network route error: {}", FN_NAME, e);
                    return Err(());
                }
                Ok(route) => match route {
                    None => return Ok(()),
                    Some(route) => route,
                },
            };
            for key in route.app_mgr_keys.iter() {
                if sent_mgrs.contains(key) {
                    continue;
                }
                let mgr = {
                    match self.application_mgrs.lock().unwrap().get(key) {
                        None => continue,
                        Some(mgr) => mgr.clone(),
                    }
                };
                let now = Utc::now();
                app_data.data_id = strings::random_id(&now, DATA_ID_RAND_LEN);
                if let Err(e) = mgr.send_uldata(&app_data) {
                    // TODO: retry internally because one or more routes may sent successfully.
                    error!("[{}] send data to {} error: {}", FN_NAME, key, e);
                    continue;
                }
                self.send_application_uldata_msg(
                    &now,
                    netmgr_unit_code.clone(),
                    mgr.unit_id().to_string(),
                    &app_data,
                )
                .await?;
                sent_mgrs.push(key.clone());
            }

            return Ok(());
        }

        let cond = network_route::ListQueryCond {
            network_id: Some(netmgr_unit_id.as_str()),
            ..Default::default()
        };
        let opts = network_route::ListOptions {
            cond: &cond,
            offset: None,
            limit: None,
            sort: None,
            cursor_max: Some(LIST_CURSOR_MAX),
        };
        let mut cursor: Option<Box<dyn network_route::Cursor>> = None;
        loop {
            let (list, _cursor) = match self.model.network_route().list(&opts, cursor).await {
                Err(e) => {
                    error!("[{}] get network route error: {}", FN_NAME, e);
                    return Err(());
                }
                Ok((list, cursor)) => (list, cursor),
            };
            for route in list.iter() {
                let key = gen_mgr_key(route.unit_code.as_str(), route.application_code.as_str());
                if sent_mgrs.contains(&key) {
                    continue;
                }
                let mgr = {
                    match self.application_mgrs.lock().unwrap().get(&key) {
                        None => continue,
                        Some(mgr) => mgr.clone(),
                    }
                };
                let now = Utc::now();
                app_data.data_id = strings::random_id(&now, DATA_ID_RAND_LEN);
                if let Err(e) = mgr.send_uldata(&app_data) {
                    // TODO: retry internally because one or more routes may sent successfully.
                    error!("[{}] send data to {} error: {}", FN_NAME, key, e);
                    continue;
                }
                self.send_application_uldata_msg(
                    &now,
                    netmgr_unit_code.clone(),
                    mgr.unit_id().to_string(),
                    &app_data,
                )
                .await?;
                sent_mgrs.push(key);
            }
            if _cursor.is_none() {
                break;
            }
            cursor = _cursor;
        }
        Ok(())
    }

    async fn send_application_uldata_msg(
        &self,
        proc: &DateTime<Utc>,
        netmgr_unit_code: Option<String>,
        app_unit_id: String,
        app_data: &ApplicationUlData,
    ) -> Result<(), ()> {
        const FN_NAME: &'static str = "send_application_uldata_msg";

        if let Some(sender) = self.data_sender.as_ref() {
            let msg = SendDataMsg {
                kind: DataMsgKind::APP_ULDATA.to_string(),
                data: SendDataKind::AppUlData {
                    data_id: app_data.data_id.clone(),
                    proc: time_str(proc),
                    publish: app_data.publish.clone(),
                    unit_code: netmgr_unit_code,
                    network_code: app_data.network_code.clone(),
                    network_addr: app_data.network_addr.clone(),
                    unit_id: app_unit_id,
                    device_id: app_data.device_id.clone(),
                    time: app_data.time.clone(),
                    profile: app_data.profile.clone(),
                    data: app_data.data.clone(),
                    extension: app_data.extension.clone(),
                },
            };
            let payload = match serde_json::to_vec(&msg) {
                Err(e) => {
                    error!("[{}] marshal JSON error: {}", FN_NAME, e);
                    return Err(());
                }
                Ok(payload) => payload,
            };
            if let Err(e) = sender.send_msg(payload).await {
                error!("[{}] send data to {} error: {}", FN_NAME, sender.name(), e);
                return Err(());
            }
        }
        Ok(())
    }

    async fn send_application_dldata_result_msg(
        &self,
        proc: &DateTime<Utc>,
        data: &Box<DlDataResult>,
    ) -> Result<(), ()> {
        const FN_NAME: &'static str = "send_application_dldata_result_msg";

        if let Some(sender) = self.data_sender.as_ref() {
            let msg = SendDataMsg {
                kind: DataMsgKind::APP_DLDATA_RES.to_string(),
                data: SendDataKind::AppDlDataResult {
                    data_id: data.data_id.clone(),
                    resp: time_str(proc),
                    status: data.status,
                },
            };
            let payload = match serde_json::to_vec(&msg) {
                Err(e) => {
                    error!("[{}] marshal JSON error: {}", FN_NAME, e);
                    return Err(());
                }
                Ok(payload) => payload,
            };
            if let Err(e) = sender.send_msg(payload).await {
                error!("[{}] send data to {} error: {}", FN_NAME, sender.name(), e);
                return Err(());
            }
        }
        Ok(())
    }

    async fn send_network_uldata_msg(
        &self,
        mgr: &NetworkMgr,
        proc: &DateTime<Utc>,
        data: &Box<UlData>,
        device: Option<&DeviceCacheItem>,
    ) -> Result<(), ()> {
        const FN_NAME: &'static str = "send_network_uldata_msg";

        if let Some(sender) = self.data_sender.as_ref() {
            let msg = SendDataMsg {
                kind: DataMsgKind::NET_ULDATA.to_string(),
                data: SendDataKind::NetUlData {
                    data_id: strings::random_id(&proc, DATA_ID_RAND_LEN),
                    proc: time_str(proc),
                    unit_code: match mgr.unit_code().len() {
                        0 => None,
                        _ => Some(mgr.unit_code().to_string()),
                    },
                    network_code: mgr.name().to_string(),
                    network_addr: data.network_addr.clone(),
                    unit_id: match mgr.unit_id().len() {
                        0 => None,
                        _ => Some(mgr.unit_id().to_string()),
                    },
                    device_id: match device {
                        None => None,
                        Some(device) => Some(device.device_id.clone()),
                    },
                    profile: match device {
                        None => None,
                        Some(device) => Some(device.profile.clone()),
                    },
                    time: data.time.clone(),
                    data: data.data.clone(),
                    extension: data.extension.clone(),
                },
            };
            let payload = match serde_json::to_vec(&msg) {
                Err(e) => {
                    error!("[{}] marshal JSON error: {}", FN_NAME, e);
                    return Err(());
                }
                Ok(payload) => payload,
            };
            if let Err(e) = sender.send_msg(payload).await {
                error!("[{}] send data to {} error: {}", FN_NAME, sender.name(), e);
                return Err(());
            }
        }
        Ok(())
    }

    async fn send_network_dldata_result_msg(
        &self,
        proc: &DateTime<Utc>,
        data: &Box<DlDataResult>,
    ) -> Result<(), ()> {
        const FN_NAME: &'static str = "send_network_dldata_result_msg";

        if let Some(sender) = self.data_sender.as_ref() {
            let msg = SendDataMsg {
                kind: DataMsgKind::NET_DLDATA_RES.to_string(),
                data: SendDataKind::NetDlDataResult {
                    data_id: data.data_id.clone(),
                    resp: time_str(proc),
                    status: data.status,
                },
            };
            let payload = match serde_json::to_vec(&msg) {
                Err(e) => {
                    error!("[{}] marshal JSON error: {}", FN_NAME, e);
                    return Err(());
                }
                Ok(payload) => payload,
            };
            if let Err(e) = sender.send_msg(payload).await {
                error!("[{}] send data to {} error: {}", FN_NAME, sender.name(), e);
                return Err(());
            }
        }
        Ok(())
    }
}

#[async_trait]
impl EventHandler for MgrHandler {
    async fn on_status_change(&self, mgr: &NetworkMgr, status: MgrStatus) {
        // Clear cache when manager status changed.
        if let Some(cache) = self.cache.as_ref() {
            if let Err(e) = cache.device().clear().await {
                error!(
                    "[on_status_change] {}.{} clear device cache error: {}",
                    mgr.unit_code(),
                    mgr.name(),
                    e
                );
            }
            if let Err(e) = cache.device_route().clear().await {
                error!(
                    "[on_status_change] {}.{} clear device_route cache error: {}",
                    mgr.unit_code(),
                    mgr.name(),
                    e
                );
            }
            if let Err(e) = cache.network_route().clear().await {
                error!(
                    "[on_status_change] {}.{} clear network_route cache error: {}",
                    mgr.unit_code(),
                    mgr.name(),
                    e
                );
            }
        }

        match status {
            MgrStatus::NotReady => {
                error!(
                    "[on_status_change] {}.{} to NotReady",
                    mgr.unit_code(),
                    mgr.name()
                );
            }
            MgrStatus::Ready => {
                info!(
                    "[on_status_change] {}.{} to Ready",
                    mgr.unit_code(),
                    mgr.name()
                );
            }
        }
    }

    // Do the following jobs:
    // - check if the source device is valid for the unit.
    // - lookup device route to send the data.
    // - lookup network route to send the data.
    // The manager does not send duplicate data to one application if the data matches both routes.
    async fn on_uldata(&self, mgr: &NetworkMgr, data: Box<UlData>) -> Result<(), ()> {
        let proc = Utc::now();
        // To validate the device and get the device ID for generating data for applications.
        let device = match self.validate_device(mgr, &data).await {
            Err(_) => return Err(()),
            Ok(device) => device,
        };
        self.send_network_uldata_msg(mgr, &proc, &data, device.as_ref())
            .await?;
        let device = match device {
            None => return Ok(()),
            Some(device) => device,
        };

        let mut app_data = {
            let now = Utc::now();
            ApplicationUlData {
                data_id: strings::random_id(&proc, DATA_ID_RAND_LEN),
                time: data.time,
                publish: time_str(&now),
                device_id: device.device_id,
                network_id: mgr.id().to_string(),
                network_code: mgr.name().to_string(),
                network_addr: data.network_addr,
                is_public: mgr.unit_id().len() > 0,
                profile: device.profile,
                data: data.data,
                extension: data.extension,
            }
        };

        let mut sent_mgrs = vec![];
        let unit_code = match mgr.unit_code().len() {
            0 => None,
            _ => Some(mgr.unit_code().to_string()),
        };

        // Get device routes to pass data.
        self.send_by_device_route(unit_code.clone(), &mut app_data, &mut sent_mgrs)
            .await?;

        // Get network routes to pass data.
        self.send_by_network_route(
            mgr.id().to_string(),
            unit_code,
            &mut app_data,
            &mut sent_mgrs,
        )
        .await
    }

    // Do the following jobs:
    // - check if the associated dldata buffer exists.
    // - send the result to the source application.
    async fn on_dldata_result(&self, _mgr: &NetworkMgr, data: Box<DlDataResult>) -> Result<(), ()> {
        const FN_NAME: &'static str = "on_dldata_result";

        let now = Utc::now();
        self.send_network_dldata_result_msg(&now, &data).await?;

        let dldata = match self.model.dldata_buffer().get(data.data_id.as_str()).await {
            Err(e) => {
                error!(
                    "[{}] get dldata buffer for {} error: {}",
                    FN_NAME, data.data_id, e
                );
                return Err(());
            }
            Ok(dldata) => match dldata {
                None => {
                    warn!("[{}] no data ID {}", FN_NAME, data.data_id);
                    return Ok(());
                }
                Some(dldata) => dldata,
            },
        };

        let key = gen_mgr_key(dldata.unit_code.as_str(), dldata.application_code.as_str());
        let mgr = {
            match self.application_mgrs.lock().unwrap().get(&key) {
                None => None,
                Some(mgr) => Some(mgr.clone()),
            }
        };
        if let Some(mgr) = mgr {
            let result_data = ApplicationDlDataResult {
                data_id: dldata.data_id,
                status: data.status,
                message: data.message.clone(),
            };
            if let Err(e) = mgr.send_dldata_result(&result_data).await {
                error!("[{}] send data to {} error: {}", FN_NAME, key, e);
                return Err(());
            }
            self.send_application_dldata_result_msg(&now, &data).await?;
        }
        if data.status < 0 {
            return Ok(());
        }

        // Remove dldata buffer after completing data processing.
        let cond = dldata_buffer::QueryCond {
            data_id: Some(data.data_id.as_str()),
            ..Default::default()
        };
        if let Err(e) = self.model.dldata_buffer().del(&cond).await {
            // TODO: retry delete internally.
            error!("[{}] delete dldata {} error: {}", FN_NAME, data.data_id, e);
            return Err(());
        }
        Ok(())
    }
}

/// Clear the network relative cache.
async fn clear_cache(fn_name: &str, queue_name: &str, cache: &Arc<dyn Cache>) {
    if let Err(e) = cache.device().clear().await {
        error!(
            "[{}] {} clear device cache error: {}",
            fn_name, queue_name, e
        );
    }
    if let Err(e) = cache.network_route().clear().await {
        error!(
            "[{}] {} clear network route cache error: {}",
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
            RecvCtrlMsg::DelNetwork { new } => {
                if let Some(cache) = self.cache.as_ref() {
                    let cond = device::DelCacheQueryCond {
                        unit_code: match new.unit_code.as_ref() {
                            None => "",
                            Some(code) => code.as_str(),
                        },
                        network_code: Some(new.network_code.as_str()),
                        network_addr: None,
                    };
                    if let Err(e) = cache.device().del(&cond).await {
                        error!(
                            "[{}] {} delete device cache error: {}",
                            FN_NAME, queue_name, e
                        );
                    }
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
            RecvCtrlMsg::AddManager { new } => {
                let host_uri = match Url::parse(new.host_uri.as_str()) {
                    Err(e) => {
                        warn!("[{}] {} hostUri error: {}", FN_NAME, queue_name, e);
                        if let Err(e) = msg.ack().await {
                            error!("[{}] {} ACK error: {}", FN_NAME, queue_name, e);
                        }
                        return;
                    }
                    Ok(uri) => uri,
                };
                let handler = MgrHandler {
                    model: self.model.clone(),
                    cache: self.cache.clone(),
                    application_mgrs: self.application_mgrs.clone(),
                    data_sender: self.data_sender.clone(),
                };
                let unit_code = new.mgr_options.unit_code.clone();
                let name = new.mgr_options.name.clone();
                let mgr = match NetworkMgr::new(
                    self.mq_conns.clone(),
                    &host_uri,
                    new.mgr_options,
                    Arc::new(handler),
                ) {
                    Err(e) => {
                        error!("[{}] {} new manager error: {}", FN_NAME, queue_name, e);
                        if let Err(e) = msg.ack().await {
                            error!("[{}] {} ACK error: {}", FN_NAME, queue_name, e);
                        }
                        return;
                    }
                    Ok(mgr) => mgr,
                };
                let key = gen_mgr_key(unit_code.as_str(), name.as_str());
                let old_mgr = { self.network_mgrs.lock().unwrap().insert(key.clone(), mgr) };
                if let Some(mgr) = old_mgr {
                    if let Err(e) = mgr.close().await {
                        error!(
                            "[{}] {} close old manager {} error: {}",
                            FN_NAME, queue_name, key, e
                        );
                    }
                }
                if let Err(e) = msg.ack().await {
                    error!("[{}] {} ACK error: {}", FN_NAME, queue_name, e);
                }
                info!("[{}] {} manager {} added", FN_NAME, queue_name, key);
            }
            RecvCtrlMsg::DelManager { new } => {
                let old_mgr = { self.network_mgrs.lock().unwrap().remove(&new) };
                match old_mgr {
                    None => {
                        error!("[{}] {} get no manager {}", FN_NAME, queue_name, new);
                        if let Err(e) = msg.ack().await {
                            error!("[{}] {} ACK error: {}", FN_NAME, queue_name, e);
                        }
                        return;
                    }
                    Some(mgr) => {
                        if let Err(e) = mgr.close().await {
                            error!(
                                "[{}] {} close old manager {} error: {}",
                                FN_NAME, queue_name, new, e
                            );
                        }
                    }
                }
                if let Err(e) = msg.ack().await {
                    error!("[{}] {} ACK error: {}", FN_NAME, queue_name, e);
                }
                info!("[{}] {} manager {} deleted", FN_NAME, queue_name, new);
            }
        }
    }
}
