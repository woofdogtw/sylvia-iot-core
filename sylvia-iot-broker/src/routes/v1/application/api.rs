use std::{
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
use chrono::{DateTime, TimeZone, Utc};
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
    err::{self, ErrResp},
    role::Role,
    strings::{self, time_str},
};

use super::{
    super::{
        super::{ErrReq, State},
        lib::{check_application, check_unit, gen_mgr_key, get_token_id_roles},
    },
    request, response,
};
use crate::{
    libs::{
        config::BrokerCtrl as CfgCtrl,
        mq::{
            self,
            application::{ApplicationMgr, DlData, DlDataResp, EventHandler},
            network::{DlData as NetworkDlData, NetworkMgr},
            Connection, MgrStatus, Options as MgrOptions,
        },
    },
    models::{
        application::{
            Application, ListOptions, ListQueryCond, QueryCond, SortCond, SortKey, UpdateQueryCond,
            Updates,
        },
        device, device_route, dldata_buffer, network_route, Cache, Model,
    },
};

struct MgrHandler {
    model: Arc<dyn Model>,
    cache: Option<Arc<dyn Cache>>,
    network_mgrs: Arc<Mutex<HashMap<String, NetworkMgr>>>,
    data_sender: Option<Queue>,
}

#[derive(Deserialize)]
#[serde(tag = "operation")]
enum RecvCtrlMsg {
    #[serde(rename = "del-application")]
    DelApplication { new: CtrlDelApplication },
    #[serde(rename = "add-manager")]
    AddManager { new: CtrlAddManager },
    #[serde(rename = "del-manager")]
    DelManager { new: String },
}

/// Control channel.
#[derive(Serialize)]
#[serde(untagged)]
enum SendCtrlMsg {
    DelApplication {
        operation: String,
        new: CtrlDelApplication,
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
    AppDlData {
        #[serde(rename = "dataId")]
        data_id: String,
        proc: String,
        status: i32,
        #[serde(rename = "unitId")]
        unit_id: String,
        #[serde(rename = "deviceId", skip_serializing_if = "Option::is_none")]
        device_id: Option<String>,
        #[serde(rename = "networkCode", skip_serializing_if = "Option::is_none")]
        network_code: Option<String>,
        #[serde(rename = "networkAddr", skip_serializing_if = "Option::is_none")]
        network_addr: Option<String>,
        profile: String,
        data: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        extension: Option<Map<String, Value>>,
    },
    NetDlData {
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
        #[serde(skip_serializing_if = "Option::is_none")]
        extension: Option<Map<String, Value>>,
    },
}

struct CtrlMsgOp;
struct DataMsgKind;

#[derive(Deserialize, Serialize)]
struct CtrlDelApplication {
    #[serde(rename = "unitId")]
    unit_id: String,
    #[serde(rename = "unitCode")]
    unit_code: String,
    #[serde(rename = "applicationId")]
    application_id: String,
    #[serde(rename = "applicationCode")]
    application_code: String,
}

#[derive(Deserialize, Serialize)]
struct CtrlAddManager {
    #[serde(rename = "hostUri")]
    host_uri: String,
    #[serde(rename = "mgrOptions")]
    mgr_options: MgrOptions,
}

struct CtrlSenderHandler;

struct CtrlReceiverHandler {
    model: Arc<dyn Model>,
    cache: Option<Arc<dyn Cache>>,
    mq_conns: Arc<Mutex<HashMap<String, Connection>>>,
    application_mgrs: Arc<Mutex<HashMap<String, ApplicationMgr>>>,
    network_mgrs: Arc<Mutex<HashMap<String, NetworkMgr>>>,
    data_sender: Option<Queue>,
}

impl CtrlMsgOp {
    const DEL_APPLICATION: &'static str = "del-application";
    const ADD_MANAGER: &'static str = "add-manager";
    const DEL_MANAGER: &'static str = "del-manager";
}

impl DataMsgKind {
    const APP_DLDATA: &'static str = "application-dldata";
    const NET_DLDATA: &'static str = "network-dldata";
}

const LIST_LIMIT_DEFAULT: u64 = 100;
const LIST_CURSOR_MAX: u64 = 100;
const ID_RAND_LEN: usize = 8;
const DATA_ID_RAND_LEN: usize = 12;
const DATA_EXPIRES_IN: i64 = 86400; // in seconds
const CTRL_QUEUE_NAME: &'static str = "application";
const DEF_DLDATA_STATUS: i32 = -2;

/// Initialize application managers and channels.
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

    let ctrl_sender = { state.ctrl_senders.application.lock().unwrap().clone() };
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
        (list, cursor) = state.model.application().list(&opts, cursor).await?;
        for item in list.iter() {
            let url = Url::parse(item.host_uri.as_str())?;
            let key = gen_mgr_key(item.unit_code.as_str(), item.code.as_str());
            let opts = MgrOptions {
                unit_id: item.unit_id.clone(),
                unit_code: item.unit_code.clone(),
                id: item.application_id.clone(),
                name: item.code.clone(),
                prefetch: Some(state.amqp_prefetch),
                persistent: state.amqp_persistent,
                shared_prefix: Some(state.mqtt_shared_prefix.clone()),
            };
            let handler = MgrHandler {
                model: state.model.clone(),
                cache: state.cache.clone(),
                network_mgrs: state.network_mgrs.clone(),
                data_sender: state.data_sender.clone(),
            };
            let mgr =
                match ApplicationMgr::new(state.mq_conns.clone(), &url, opts, Arc::new(handler)) {
                    Err(e) => {
                        error!("[{}] new manager for {} error: {}", FN_NAME, key, e);
                        return Err(Box::new(ErrResp::ErrRsc(Some(e))));
                    }
                    Ok(mgr) => mgr,
                };
            {
                state
                    .application_mgrs
                    .lock()
                    .unwrap()
                    .insert(key.clone(), mgr);
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

/// `POST /{base}/api/v1/application`
pub async fn post_application(
    req: HttpRequest,
    body: web::Json<request::PostApplicationBody>,
    state: web::Data<State>,
) -> impl Responder {
    const FN_NAME: &'static str = "post_application";

    let (user_id, roles) = get_token_id_roles(FN_NAME, &req)?;
    let user_id = user_id.as_str();

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
    let unit_id = body.data.unit_id.as_str();
    if unit_id.len() == 0 {
        return Err(ErrResp::ErrParam(Some(
            "`unitId` must with at least one character".to_string(),
        )));
    }
    let unit_code = match check_unit(FN_NAME, user_id, &roles, unit_id, true, &state).await? {
        None => {
            return Err(ErrResp::Custom(
                ErrReq::UNIT_NOT_EXIST.0,
                ErrReq::UNIT_NOT_EXIST.1,
                None,
            ))
        }
        Some(unit) => unit.code,
    };
    if check_code(FN_NAME, unit_id, code.as_str(), &state).await? {
        return Err(ErrResp::Custom(
            ErrReq::APPLICATION_EXIST.0,
            ErrReq::APPLICATION_EXIST.1,
            None,
        ));
    }

    let now = Utc::now();
    let application = Application {
        application_id: strings::random_id(&now, ID_RAND_LEN),
        code,
        unit_id: unit_id.to_string(),
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
    if let Err(e) = state.model.application().add(&application).await {
        error!("[{}] add error: {}", FN_NAME, e);
        return Err(ErrResp::ErrDb(Some(e.to_string())));
    }
    add_manager(
        FN_NAME,
        &state,
        &host_uri,
        unit_id,
        unit_code.as_str(),
        application.application_id.as_str(),
        application.code.as_str(),
    )
    .await?;
    Ok(HttpResponse::Ok().json(response::PostApplication {
        data: response::PostApplicationData {
            application_id: application.application_id,
        },
    }))
}

/// `GET /{base}/api/v1/application/count`
pub async fn get_application_count(
    req: HttpRequest,
    query: web::Query<request::GetApplicationCountQuery>,
    state: web::Data<State>,
) -> impl Responder {
    const FN_NAME: &'static str = "get_application_count";

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
    match state.model.application().count(&cond).await {
        Err(e) => {
            error!("[{}] count error: {}", FN_NAME, e);
            Err(ErrResp::ErrDb(Some(e.to_string())))
        }
        Ok(count) => Ok(HttpResponse::Ok().json(response::GetApplicationCount {
            data: response::GetCountData { count },
        })),
    }
}

/// `GET /{base}/api/v1/application/list`
pub async fn get_application_list(
    req: HttpRequest,
    query: web::Query<request::GetApplicationListQuery>,
    state: web::Data<State>,
) -> impl Responder {
    const FN_NAME: &'static str = "get_application_list";

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

    let (list, cursor) = match state.model.application().list(&opts, None).await {
        Err(e) => {
            error!("[{}] list error: {}", FN_NAME, e);
            return Err(ErrResp::ErrDb(Some(e.to_string())));
        }
        Ok((list, cursor)) => match cursor {
            None => match query.format {
                Some(request::ListFormat::Array) => {
                    return Ok(HttpResponse::Ok().json(application_list_transform(&list)))
                }
                _ => {
                    return Ok(HttpResponse::Ok().json(response::GetApplicationList {
                        data: application_list_transform(&list),
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
            yield application_list_transform_bytes(&list, is_first, cursor.is_none(), query.format.as_ref());
            is_first = false;
            if cursor.is_none() {
                break;
            }
            let (_list, _cursor) = match state.model.application().list(&opts, cursor).await {
                Err(_) => break,
                Ok((list, cursor)) => (list, cursor),
            };
            list = _list;
            cursor = _cursor;
        }
    };
    Ok(HttpResponse::Ok().streaming(stream))
}

/// `GET /{base}/api/v1/application/{applicationId}`
pub async fn get_application(
    req: HttpRequest,
    param: web::Path<request::ApplicationIdPath>,
    state: web::Data<State>,
) -> impl Responder {
    const FN_NAME: &'static str = "get_application";

    let (user_id, roles) = get_token_id_roles(FN_NAME, &req)?;
    let user_id = user_id.as_str();
    let application_id = param.application_id.as_str();

    match check_application(FN_NAME, application_id, user_id, false, &roles, &state).await? {
        None => Err(ErrResp::ErrNotFound(None)),
        Some(application) => Ok(HttpResponse::Ok().json(response::GetApplication {
            data: application_transform(&application),
        })),
    }
}

/// `PATCH /{base}/api/v1/application/{applicationId}`
pub async fn patch_application(
    req: HttpRequest,
    param: web::Path<request::ApplicationIdPath>,
    mut body: web::Json<request::PatchApplicationBody>,
    state: web::Data<State>,
) -> impl Responder {
    const FN_NAME: &'static str = "patch_application";

    let (user_id, roles) = get_token_id_roles(FN_NAME, &req)?;
    let user_id = user_id.as_str();
    let application_id = param.application_id.as_str();

    // To check if the application is for the user.
    let application =
        match check_application(FN_NAME, application_id, user_id, true, &roles, &state).await? {
            None => return Err(ErrResp::ErrNotFound(None)),
            Some(application) => application,
        };

    let updates = get_updates(&mut body.data).await?;
    let mut should_add_mgr = false;

    // Remove old manager.
    if let Some(host_uri) = updates.host_uri {
        let uri = Url::parse(host_uri).unwrap();
        if !uri.as_str().eq(application.host_uri.as_str()) {
            delete_manager(FN_NAME, &state, &application).await?;
            should_add_mgr = true;
        }
    }

    // Update database.
    let cond = UpdateQueryCond { application_id };
    if let Err(e) = state.model.application().update(&cond, &updates).await {
        error!("[{}] update error: {}", FN_NAME, e);
        return Err(ErrResp::ErrDb(Some(e.to_string())));
    }

    // Add new manager.
    if should_add_mgr {
        if let Some(host_uri) = updates.host_uri {
            let uri = Url::parse(host_uri).unwrap();
            add_manager(
                FN_NAME,
                &state,
                &uri,
                application.unit_id.as_str(),
                application.unit_code.as_str(),
                application.application_id.as_str(),
                application.code.as_str(),
            )
            .await?;
        }
    }
    Ok(HttpResponse::NoContent().finish())
}

/// `DELETE /{base}/api/v1/application/{applicationId}`
pub async fn delete_application(
    req: HttpRequest,
    param: web::Path<request::ApplicationIdPath>,
    state: web::Data<State>,
) -> impl Responder {
    const FN_NAME: &'static str = "delete_application";

    let (user_id, roles) = get_token_id_roles(FN_NAME, &req)?;
    let user_id = user_id.as_str();
    let application_id = param.application_id.as_str();

    // To check if the application is for the user.
    let application =
        match check_application(FN_NAME, application_id, user_id, true, &roles, &state).await {
            Err(e) => return Err(e), // XXX: not use "?" to solve E0282 error.
            Ok(application) => match application {
                None => return Ok(HttpResponse::NoContent().finish()),
                Some(application) => application,
            },
        };

    delete_manager(FN_NAME, &state, &application).await?;
    del_application_rsc(FN_NAME, application_id, &state).await?;
    send_del_ctrl_message(FN_NAME, application, &state).await?;

    Ok(HttpResponse::NoContent().finish())
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

async fn get_updates<'a>(
    body: &'a mut request::PatchApplicationData,
) -> Result<Updates<'a>, ErrResp> {
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

/// To check if the application code is used by the unit.
///
/// # Errors
///
/// Returns OK if the code is found or not. Otherwise errors will be returned.
async fn check_code(
    fn_name: &str,
    unit_id: &str,
    code: &str,
    state: &web::Data<State>,
) -> Result<bool, ErrResp> {
    let cond = QueryCond {
        unit_id: Some(unit_id),
        code: Some(code),
        ..Default::default()
    };
    match state.model.application().get(&cond).await {
        Err(e) => {
            error!("[{}] check code error: {}", fn_name, e);
            return Err(ErrResp::ErrDb(Some(format!("check code error: {}", e))));
        }
        Ok(application) => match application {
            None => Ok(false),
            Some(_) => Ok(true),
        },
    }
}

fn application_list_transform(list: &Vec<Application>) -> Vec<response::GetApplicationData> {
    let mut ret = vec![];
    for application in list.iter() {
        ret.push(application_transform(&application));
    }
    ret
}

fn application_list_transform_bytes(
    list: &Vec<Application>,
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
        let json_str = match serde_json::to_string(&application_transform(item)) {
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

fn application_transform(application: &Application) -> response::GetApplicationData {
    response::GetApplicationData {
        application_id: application.application_id.clone(),
        code: application.code.clone(),
        unit_id: application.unit_id.clone(),
        unit_code: application.unit_code.clone(),
        created_at: time_str(&application.created_at),
        modified_at: time_str(&application.modified_at),
        host_uri: application.host_uri.clone(),
        name: application.name.clone(),
        info: application.info.clone(),
    }
}

async fn del_application_rsc(
    fn_name: &str,
    application_id: &str,
    state: &web::Data<State>,
) -> Result<(), ErrResp> {
    let cond = network_route::QueryCond {
        application_id: Some(application_id),
        ..Default::default()
    };
    if let Err(e) = state.model.network_route().del(&cond).await {
        error!("[{}] del network_route error: {}", fn_name, e);
        return Err(ErrResp::ErrDb(Some(e.to_string())));
    }

    let cond = device_route::QueryCond {
        application_id: Some(application_id),
        ..Default::default()
    };
    if let Err(e) = state.model.device_route().del(&cond).await {
        error!("[{}] del device_route error: {}", fn_name, e);
        return Err(ErrResp::ErrDb(Some(e.to_string())));
    }

    let cond = dldata_buffer::QueryCond {
        application_id: Some(application_id),
        ..Default::default()
    };
    if let Err(e) = state.model.dldata_buffer().del(&cond).await {
        error!("[{}] del dldata_buffer error: {}", fn_name, e);
        return Err(ErrResp::ErrDb(Some(e.to_string())));
    }

    let cond = QueryCond {
        application_id: Some(application_id),
        ..Default::default()
    };
    if let Err(e) = state.model.application().del(&cond).await {
        error!("[{}] del application error: {}", fn_name, e);
        return Err(ErrResp::ErrDb(Some(e.to_string())));
    }

    Ok(())
}

/// Send delete control message.
async fn send_del_ctrl_message(
    fn_name: &str,
    application: Application,
    state: &web::Data<State>,
) -> Result<(), ErrResp> {
    if state.cache.is_some() {
        let msg = SendCtrlMsg::DelApplication {
            operation: CtrlMsgOp::DEL_APPLICATION.to_string(),
            new: CtrlDelApplication {
                unit_id: application.unit_id,
                unit_code: application.unit_code,
                application_id: application.application_id,
                application_code: application.code,
            },
        };
        let payload = match serde_json::to_vec(&msg) {
            Err(e) => {
                error!(
                    "[{}] marshal JSON for {} error: {}",
                    fn_name,
                    CtrlMsgOp::DEL_APPLICATION,
                    e
                );
                return Err(ErrResp::ErrRsc(Some(format!(
                    "marshal control message error: {}",
                    e
                ))));
            }
            Ok(payload) => payload,
        };
        let ctrl_sender = { state.ctrl_senders.application.lock().unwrap().clone() };
        if let Err(e) = ctrl_sender.send_msg(payload).await {
            error!(
                "[{}] send control message for {} error: {}",
                fn_name,
                CtrlMsgOp::DEL_APPLICATION,
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
    state: &State,
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
            return Err(ErrResp::ErrRsc(Some(format!("new manager error:{}", e))));
        }
        Ok(payload) => payload,
    };
    let ctrl_sender = { state.ctrl_senders.application.lock().unwrap().clone() };
    if let Err(e) = ctrl_sender.send_msg(payload).await {
        error!(
            "[{}] send control message for {} error: {}",
            fn_name, name, e
        );
        return Err(ErrResp::ErrIntMsg(Some(format!("new manager error:{}", e))));
    }
    Ok(())
}

/// To delete an application manager.
async fn delete_manager(
    fn_name: &str,
    state: &State,
    application: &Application,
) -> Result<(), ErrResp> {
    let key = gen_mgr_key(application.unit_code.as_str(), application.code.as_str());
    let msg = SendCtrlMsg::DelManager {
        operation: CtrlMsgOp::DEL_MANAGER.to_string(),
        new: key.clone(),
    };
    let payload = match serde_json::to_vec(&msg) {
        Err(e) => {
            error!("[{}] marshal JSON for {} error: {}", fn_name, key, e);
            return Err(ErrResp::ErrRsc(Some(format!("delete manager error:{}", e))));
        }
        Ok(payload) => payload,
    };
    let ctrl_sender = { state.ctrl_senders.application.lock().unwrap().clone() };
    if let Err(e) = ctrl_sender.send_msg(payload).await {
        error!(
            "[{}] send control message for {} error: {}",
            fn_name, key, e
        );
        return Err(ErrResp::ErrIntMsg(Some(format!(
            "delete manager error:{}",
            e
        ))));
    }
    Ok(())
}

impl MgrHandler {
    /// Get device route information from cache or database. This function handles two cases:
    /// - with `network_code` and `network_addr` for private network devices.
    /// - with `device_id` for both private and public network devices.
    async fn get_device_route(
        &self,
        mgr: &ApplicationMgr,
        data: &Box<DlData>,
    ) -> Result<device_route::DeviceRouteCacheDlData, Box<DlDataResp>> {
        const FN_NAME: &'static str = "get_device_route";

        if let Some(cache) = self.cache.as_ref() {
            match data.device_id.as_ref() {
                None => {
                    let cond = device_route::GetCacheQueryCond {
                        unit_code: mgr.unit_code(),
                        network_code: data.network_code.as_ref().unwrap().as_str(),
                        network_addr: data.network_addr.as_ref().unwrap().as_str(),
                    };
                    match cache.device_route().get_dldata(&cond).await {
                        Err(e) => {
                            error!("[{}] get device with error: {}", FN_NAME, e);
                            return Err(Box::new(DlDataResp {
                                correlation_id: data.correlation_id.clone(),
                                error: Some(err::E_DB.to_string()),
                                message: Some(format!("{}", e)),
                                ..Default::default()
                            }));
                        }
                        Ok(route) => match route {
                            None => {
                                warn!(
                                    "[{}] no device for {}.{}.{:?}",
                                    FN_NAME,
                                    mgr.unit_code(),
                                    mgr.name(),
                                    data.network_addr.as_ref()
                                );
                                return Err(Box::new(DlDataResp {
                                    correlation_id: data.correlation_id.clone(),
                                    error: Some(ErrReq::DEVICE_NOT_EXIST.1.to_string()),
                                    ..Default::default()
                                }));
                            }
                            Some(route) => return Ok(route),
                        },
                    }
                }
                Some(device_id) => {
                    let cond = device_route::GetCachePubQueryCond {
                        unit_id: mgr.unit_id(),
                        device_id: device_id.as_str(),
                    };
                    match cache.device_route().get_dldata_pub(&cond).await {
                        Err(e) => {
                            error!("[{}] get device with error: {}", FN_NAME, e);
                            return Err(Box::new(DlDataResp {
                                correlation_id: data.correlation_id.clone(),
                                error: Some(err::E_DB.to_string()),
                                message: Some(format!("{}", e)),
                                ..Default::default()
                            }));
                        }
                        Ok(route) => match route {
                            None => {
                                warn!(
                                    "[{}] no device for {}.{:?}",
                                    FN_NAME,
                                    mgr.unit_id(),
                                    data.device_id.as_ref(),
                                );
                                return Err(Box::new(DlDataResp {
                                    correlation_id: data.correlation_id.clone(),
                                    error: Some(ErrReq::DEVICE_NOT_EXIST.1.to_string()),
                                    ..Default::default()
                                }));
                            }
                            Some(route) => return Ok(route),
                        },
                    }
                }
            }
        }

        // Get information from database.
        let cond = match data.device_id.as_ref() {
            None => device::QueryCond {
                device: Some(device::QueryOneCond {
                    unit_code: Some(mgr.unit_code()),
                    network_code: data.network_code.as_ref().unwrap().as_str(),
                    network_addr: data.network_addr.as_ref().unwrap().as_str(),
                }),
                ..Default::default()
            },
            Some(device_id) => device::QueryCond {
                unit_id: Some(mgr.unit_id()),
                device_id: Some(device_id.as_str()),
                ..Default::default()
            },
        };
        let device = match self.model.device().get(&cond).await {
            Err(e) => {
                error!("[{}] get device with error: {}", FN_NAME, e);
                return Err(Box::new(DlDataResp {
                    correlation_id: data.correlation_id.clone(),
                    error: Some(err::E_DB.to_string()),
                    message: Some(format!("{}", e)),
                    ..Default::default()
                }));
            }
            Ok(device) => match device {
                None => {
                    warn!(
                        "[{}] no device for {}.{:?} or {}.{}.{:?}",
                        FN_NAME,
                        mgr.unit_id(),
                        data.device_id.as_ref(),
                        mgr.unit_code(),
                        mgr.name(),
                        data.network_addr.as_ref()
                    );
                    return Err(Box::new(DlDataResp {
                        correlation_id: data.correlation_id.clone(),
                        error: Some(ErrReq::DEVICE_NOT_EXIST.1.to_string()),
                        ..Default::default()
                    }));
                }
                Some(device) => device,
            },
        };
        let unit_code = match device.unit_code.as_ref() {
            None => "",
            Some(_) => mgr.unit_code(),
        };
        Ok(device_route::DeviceRouteCacheDlData {
            net_mgr_key: gen_mgr_key(unit_code, device.network_code.as_str()),
            network_id: device.network_id,
            network_addr: device.network_addr,
            device_id: device.device_id,
            profile: device.profile,
        })
    }

    async fn send_application_dldata_msg(
        &self,
        proc: &DateTime<Utc>,
        data_id: &str,
        unit_id: &str,
        profile: &str,
        data: &Box<DlData>,
    ) -> Result<(), ()> {
        const FN_NAME: &'static str = "send_application_dldata_msg";

        if let Some(sender) = self.data_sender.as_ref() {
            let msg = SendDataMsg {
                kind: DataMsgKind::APP_DLDATA.to_string(),
                data: SendDataKind::AppDlData {
                    data_id: data_id.to_string(),
                    proc: time_str(proc),
                    status: DEF_DLDATA_STATUS,
                    unit_id: unit_id.to_string(),
                    device_id: data.device_id.clone(),
                    network_code: data.network_code.clone(),
                    network_addr: data.network_addr.clone(),
                    profile: profile.to_string(),
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

    async fn send_network_dldata_msg(
        &self,
        proc: &DateTime<Utc>,
        netmgr_code: &str,
        dldata_buff: &dldata_buffer::DlDataBuffer,
        profile: &str,
        net_data: &NetworkDlData,
    ) -> Result<(), ()> {
        const FN_NAME: &'static str = "send_network_dldata_msg";

        if let Some(sender) = self.data_sender.as_ref() {
            let msg = SendDataMsg {
                kind: DataMsgKind::NET_DLDATA.to_string(),
                data: SendDataKind::NetDlData {
                    data_id: dldata_buff.data_id.clone(),
                    proc: time_str(proc),
                    publish: net_data.publish.clone(),
                    status: DEF_DLDATA_STATUS,
                    unit_id: dldata_buff.unit_id.clone(),
                    device_id: dldata_buff.device_id.clone(),
                    network_code: netmgr_code.to_string(),
                    network_addr: dldata_buff.network_addr.clone(),
                    profile: profile.to_string(),
                    data: net_data.data.clone(),
                    extension: net_data.extension.clone(),
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
    async fn on_status_change(&self, mgr: &ApplicationMgr, status: MgrStatus) {
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
    // - check if the destination device is valid for the unit.
    // - generate dldata buffer to trace data processing.
    // - send to the network manager.
    async fn on_dldata(
        &self,
        mgr: &ApplicationMgr,
        data: Box<DlData>,
    ) -> Result<Box<DlDataResp>, ()> {
        const FN_NAME: &'static str = "on_dldata";

        // Check if the device is valid.
        let dldata_route = match self.get_device_route(mgr, &data).await {
            Err(e) => return Ok(e),
            Ok(route) => route,
        };

        let now = Utc::now();
        let data_id = strings::random_id(&now, DATA_ID_RAND_LEN);

        self.send_application_dldata_msg(
            &now,
            data_id.as_str(),
            mgr.unit_id(),
            &dldata_route.profile,
            &data,
        )
        .await?;

        // Check if the network exists.
        let network_mgr = {
            match self
                .network_mgrs
                .lock()
                .unwrap()
                .get(&dldata_route.net_mgr_key)
            {
                None => {
                    return Ok(Box::new(DlDataResp {
                        correlation_id: data.correlation_id,
                        error: Some(ErrReq::NETWORK_NOT_EXIST.1.to_string()),
                        ..Default::default()
                    }));
                }
                Some(mgr) => mgr.clone(),
            }
        };

        let expired_at =
            Utc.timestamp_nanos(now.timestamp_nanos() + DATA_EXPIRES_IN * 1_000_000_000);
        let dldata = dldata_buffer::DlDataBuffer {
            data_id: data_id.clone(),
            unit_id: mgr.unit_id().to_string(),
            unit_code: mgr.unit_code().to_string(),
            application_id: mgr.id().to_string(),
            application_code: mgr.name().to_string(),
            network_id: dldata_route.network_id,
            network_addr: dldata_route.network_addr.clone(),
            device_id: dldata_route.device_id,
            created_at: now,
            expired_at,
        };
        match self.model.dldata_buffer().add(&dldata).await {
            Err(e) => {
                error!("[{}] add data buffer with error: {}", FN_NAME, e);
                return Ok(Box::new(DlDataResp {
                    correlation_id: data.correlation_id,
                    error: Some(err::E_DB.to_string()),
                    message: Some(format!("{}", e)),
                    ..Default::default()
                }));
            }
            Ok(_) => (),
        }

        let net_data = NetworkDlData {
            data_id,
            publish: time_str(&now),
            expires_in: DATA_EXPIRES_IN,
            network_addr: dldata_route.network_addr,
            data: data.data,
            extension: data.extension,
        };
        self.send_network_dldata_msg(
            &now,
            network_mgr.name(),
            &dldata,
            &dldata_route.profile,
            &net_data,
        )
        .await?;
        if let Err(e) = network_mgr.send_dldata(&net_data) {
            error!("[{}] send dldata to network with error: {}", FN_NAME, e);
            return Ok(Box::new(DlDataResp {
                correlation_id: data.correlation_id,
                error: Some(err::E_INT_MSG.to_string()),
                message: Some(format!("send data with error: {}", e)),
                ..Default::default()
            }));
        }

        Ok(Box::new(DlDataResp {
            correlation_id: data.correlation_id,
            data_id: Some(net_data.data_id),
            ..Default::default()
        }))
    }
}

#[async_trait]
impl QueueEventHandler for CtrlSenderHandler {
    async fn on_error(&self, queue: Arc<dyn GmqQueue>, err: Box<dyn StdError + Send + Sync>) {
        const FN_NAME: &'static str = "CtrlSenderHandler::on_error";
        let queue_name = queue.name();
        error!("[{}] {} error: {}", FN_NAME, queue_name, err);
    }

    async fn on_status(&self, queue: Arc<dyn GmqQueue>, status: Status) {
        const FN_NAME: &'static str = "CtrlSenderHandler::on_status";
        let queue_name = queue.name();
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
        error!("[{}] {} error: {}", FN_NAME, queue_name, err);
    }

    async fn on_status(&self, queue: Arc<dyn GmqQueue>, status: Status) {
        const FN_NAME: &'static str = "CtrlReceiverHandler::on_status";
        let queue_name = queue.name();
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
            RecvCtrlMsg::DelApplication { new: _new } => {}
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
                    network_mgrs: self.network_mgrs.clone(),
                    data_sender: self.data_sender.clone(),
                };
                let unit_code = new.mgr_options.unit_code.clone();
                let name = new.mgr_options.name.clone();
                let mgr = match ApplicationMgr::new(
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
                let old_mgr = {
                    self.application_mgrs
                        .lock()
                        .unwrap()
                        .insert(key.clone(), mgr)
                };
                if let Some(mgr) = old_mgr {
                    if let Err(e) = mgr.close().await {
                        error!(
                            "[{}] {} close old manager {} error: {}",
                            FN_NAME, queue_name, key, e
                        );
                    }
                }
                info!("[{}] {} manager {} added", FN_NAME, queue_name, key);
            }
            RecvCtrlMsg::DelManager { new } => {
                let old_mgr = { self.application_mgrs.lock().unwrap().remove(&new) };
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
                info!("[{}] {} manager {} deleted", FN_NAME, queue_name, new);
            }
        }
        if let Err(e) = msg.ack().await {
            error!("[{}] {} ACK error: {}", FN_NAME, queue_name, e);
        }
    }
}
