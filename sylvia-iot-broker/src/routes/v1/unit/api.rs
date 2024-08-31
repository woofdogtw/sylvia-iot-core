use std::{
    cmp::Ordering,
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
use log::{debug, error, info, warn};
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
    strings::{self, time_str},
};

use super::{
    super::{
        super::{middleware::GetTokenInfoData, ErrReq, State as AppState},
        lib::{check_unit, gen_mgr_key},
    },
    request, response,
};
use crate::{
    libs::{
        config::BrokerCtrl as CfgCtrl,
        mq::{self, Connection},
    },
    models::{
        application::{
            self, Cursor as ApplicationCursor, ListOptions as ApplicationListOpts,
            ListQueryCond as ApplicationCond,
        },
        device, device_route, dldata_buffer,
        network::{
            self, Cursor as NetworkCursor, ListOptions as NetworkListOpts,
            ListQueryCond as NetworkCond,
        },
        network_route,
        unit::{
            Cursor, ListOptions, ListQueryCond, QueryCond, SortCond, SortKey, Unit,
            UpdateQueryCond, Updates,
        },
        Cache,
    },
};

#[derive(Deserialize, Serialize)]
#[serde(tag = "operation")]
enum RecvCtrlMsg {
    #[serde(rename = "del-unit")]
    DelUnit { new: CtrlDelUnit },
}

#[derive(Serialize)]
#[serde(untagged)]
enum SendCtrlMsg {
    DelUnit { operation: String, new: CtrlDelUnit },
}

struct CtrlMsgOp;

#[derive(Deserialize, Serialize)]
struct CtrlDelUnit {
    #[serde(rename = "unitId")]
    unit_id: String,
    #[serde(rename = "unitCode")]
    unit_code: String,
}

struct CtrlSenderHandler;

struct CtrlReceiverHandler {
    cache: Option<Arc<dyn Cache>>,
}

impl CtrlMsgOp {
    const DEL_UNIT: &'static str = "del-unit";
}

const LIST_LIMIT_DEFAULT: u64 = 100;
const LIST_CURSOR_MAX: u64 = 100;
const ID_RAND_LEN: usize = 8;
const CTRL_QUEUE_NAME: &'static str = "unit";

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

    let ctrl_sender = { state.ctrl_senders.unit.lock().unwrap().clone() };
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
        Arc::new(CtrlSenderHandler {}),
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

/// `POST /{base}/api/v1/unit`
pub async fn post_unit(
    State(state): State<AppState>,
    Extension(token_info): Extension<GetTokenInfoData>,
    Json(body): Json<request::PostUnitBody>,
) -> impl IntoResponse {
    const FN_NAME: &'static str = "post_unit";

    let user_id = token_info.user_id.as_str();
    let roles = &token_info.roles;
    let token = token_info.token.as_str();

    let code = body.data.code.to_lowercase();
    if !strings::is_code(code.as_str()) {
        return Err(ErrResp::ErrParam(Some(
            "`code` must be [A-Za-z0-9]{1}[A-Za-z0-9-_]*".to_string(),
        )));
    }
    let owner_id = match body.data.owner_id.as_ref() {
        None => user_id,
        Some(owner_id) => {
            if Role::is_role(roles, Role::ADMIN) || Role::is_role(roles, Role::MANAGER) {
                if owner_id.len() == 0 {
                    return Err(ErrResp::ErrParam(Some(
                        "`ownerId` must with at least one character".to_string(),
                    )));
                }
                match check_user(FN_NAME, token, owner_id.as_str(), &state).await {
                    Err(e) => return Err(e),
                    Ok(result) => match result {
                        false => {
                            return Err(ErrResp::Custom(
                                ErrReq::OWNER_NOT_EXIST.0,
                                ErrReq::OWNER_NOT_EXIST.1,
                                None,
                            ))
                        }
                        true => owner_id.as_str(),
                    },
                }
            } else {
                user_id
            }
        }
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

    let cond = QueryCond {
        code: Some(code.as_str()),
        ..Default::default()
    };
    match state.model.unit().get(&cond).await {
        Err(e) => {
            error!("[{}] get error: {}", FN_NAME, e);
            return Err(ErrResp::ErrDb(Some(e.to_string())));
        }
        Ok(unit) => match unit {
            None => (),
            Some(_) => {
                return Err(ErrResp::Custom(
                    ErrReq::UNIT_EXIST.0,
                    ErrReq::UNIT_EXIST.1,
                    None,
                ))
            }
        },
    }

    let now = Utc::now();
    let unit = Unit {
        unit_id: strings::random_id(&now, ID_RAND_LEN),
        code,
        created_at: now,
        modified_at: now,
        owner_id: owner_id.to_string(),
        member_ids: vec![owner_id.to_string()],
        name: match body.data.name.as_ref() {
            None => "".to_string(),
            Some(name) => name.clone(),
        },
        info: match body.data.info.as_ref() {
            None => Map::new(),
            Some(info) => info.clone(),
        },
    };
    if let Err(e) = state.model.unit().add(&unit).await {
        error!("[{}] add error: {}", FN_NAME, e);
        return Err(ErrResp::ErrDb(Some(e.to_string())));
    }
    Ok(Json(response::PostUnit {
        data: response::PostUnitData {
            unit_id: unit.unit_id,
        },
    }))
}

/// `GET /{base}/api/v1/unit/count`
pub async fn get_unit_count(
    State(state): State<AppState>,
    Extension(token_info): Extension<GetTokenInfoData>,
    Query(query): Query<request::GetUnitCountQuery>,
) -> impl IntoResponse {
    const FN_NAME: &'static str = "get_unit_count";

    let user_id = token_info.user_id.as_str();
    let roles = &token_info.roles;

    let mut owner_cond = None;
    let mut member_cond = None;
    let mut code_contains_cond = None;
    if Role::is_role(roles, Role::ADMIN) || Role::is_role(roles, Role::MANAGER) {
        if let Some(owner) = query.owner.as_ref() {
            if owner.len() > 0 {
                owner_cond = Some(owner.as_str());
            }
        }
        if let Some(member) = query.member.as_ref() {
            if member.len() > 0 {
                member_cond = Some(member.as_str());
            }
        }
    } else {
        member_cond = Some(user_id);
    }
    if let Some(contains) = query.contains.as_ref() {
        if contains.len() > 0 {
            code_contains_cond = Some(contains.as_str());
        }
    }
    let cond = ListQueryCond {
        owner_id: owner_cond,
        member_id: member_cond,
        code_contains: code_contains_cond,
        ..Default::default()
    };
    match state.model.unit().count(&cond).await {
        Err(e) => {
            error!("[{}] count error: {}", FN_NAME, e);
            Err(ErrResp::ErrDb(Some(e.to_string())))
        }
        Ok(count) => Ok(Json(response::GetUnitCount {
            data: response::GetCountData { count },
        })),
    }
}

/// `GET /{base}/api/v1/unit/list`
pub async fn get_unit_list(
    State(state): State<AppState>,
    Extension(token_info): Extension<GetTokenInfoData>,
    Query(query): Query<request::GetUnitListQuery>,
) -> impl IntoResponse {
    const FN_NAME: &'static str = "get_unit_list";

    let user_id = token_info.user_id;
    let roles = token_info.roles;

    let mut owner_cond = None;
    let mut member_cond = None;
    let mut code_contains_cond = None;
    if Role::is_role(&roles, Role::ADMIN) || Role::is_role(&roles, Role::MANAGER) {
        if let Some(owner) = query.owner.as_ref() {
            if owner.len() > 0 {
                owner_cond = Some(owner.as_str());
            }
        }
        if let Some(member) = query.member.as_ref() {
            if member.len() > 0 {
                member_cond = Some(member.as_str());
            }
        }
    } else {
        member_cond = Some(user_id.as_str());
    }
    if let Some(contains) = query.contains.as_ref() {
        if contains.len() > 0 {
            code_contains_cond = Some(contains.as_str());
        }
    }
    let cond = ListQueryCond {
        owner_id: owner_cond,
        member_id: member_cond,
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

    let (list, cursor) = match state.model.unit().list(&opts, None).await {
        Err(e) => {
            error!("[{}] list error: {}", FN_NAME, e);
            return Err(ErrResp::ErrDb(Some(e.to_string())));
        }
        Ok((list, cursor)) => match cursor {
            None => match query.format {
                Some(request::ListFormat::Array) => {
                    return Ok(Json(unit_list_transform(&list)).into_response())
                }
                _ => {
                    return Ok(Json(response::GetUnitList {
                        data: unit_list_transform(&list),
                    })
                    .into_response())
                }
            },
            Some(_) => (list, cursor),
        },
    };

    let body = Body::from_stream(async_stream::stream! {
        let mut owner_cond = None;
        let mut member_cond = Some(user_id.as_str());
        let mut code_contains_cond = None;
        if Role::is_role(&roles, Role::ADMIN) || Role::is_role(&roles, Role::MANAGER) {
            if let Some(owner) = query.owner.as_ref() {
                if owner.len() > 0 {
                    owner_cond = Some(owner.as_str());
                }
            }
            if let Some(member) = query.member.as_ref() {
                if member.len() > 0{
                    member_cond = Some(member.as_str());
                }
            }
        } else {
            member_cond = Some(user_id.as_str());
        }
        if let Some(contains) = query.contains.as_ref() {
            if contains.len() > 0 {
                code_contains_cond = Some(contains.as_str());
            }
        }
        let cond = ListQueryCond {
            owner_id: owner_cond,
            member_id: member_cond,
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
            yield unit_list_transform_bytes(&list, is_first, cursor.is_none(), query.format.as_ref());
            is_first = false;
            if cursor.is_none() {
                break;
            }
            let (_list, _cursor) = match state.model.unit().list(&opts, cursor).await {
                Err(_) => break,
                Ok((list, cursor)) => (list, cursor),
            };
            list = _list;
            cursor = _cursor;
        }
    });
    Ok(([(header::CONTENT_TYPE, ContentType::JSON)], body).into_response())
}

/// `GET /{base}/api/v1/unit/{unitId}`
pub async fn get_unit(
    State(state): State<AppState>,
    Extension(token_info): Extension<GetTokenInfoData>,
    Path(param): Path<request::UnitIdPath>,
) -> impl IntoResponse {
    const FN_NAME: &'static str = "get_unit";

    let user_id = token_info.user_id.as_str();
    let roles = &token_info.roles;
    let unit_id = param.unit_id.as_str();

    match check_unit(FN_NAME, user_id, roles, unit_id, false, &state).await? {
        None => Err(ErrResp::ErrNotFound(None)),
        Some(unit) => Ok(Json(response::GetUnit {
            data: unit_transform(&unit),
        })),
    }
}

/// `PATCH /{base}/api/v1/unit/{unitId}`
pub async fn patch_unit(
    State(state): State<AppState>,
    Extension(token_info): Extension<GetTokenInfoData>,
    Path(param): Path<request::UnitIdPath>,
    Json(mut body): Json<request::PatchUnitBody>,
) -> impl IntoResponse {
    const FN_NAME: &'static str = "patch_unit";

    let user_id = token_info.user_id.as_str();
    let roles = &token_info.roles;
    let unit_id = param.unit_id.as_str();
    let token = &token_info.token;

    // To check if the unit is for the owner.
    let target_unit = match check_unit(FN_NAME, user_id, roles, unit_id, true, &state).await? {
        None => return Err(ErrResp::ErrNotFound(None)),
        Some(unit) => unit,
    };

    let updates = get_updates(FN_NAME, token, &state, &mut body.data, roles, &target_unit).await?;
    let cond = UpdateQueryCond { unit_id };
    if let Err(e) = state.model.unit().update(&cond, &updates).await {
        error!("[{}] update error: {}", FN_NAME, e);
        return Err(ErrResp::ErrDb(Some(e.to_string())));
    }
    Ok(StatusCode::NO_CONTENT)
}

/// `DELETE /{base}/api/v1/unit/{unitId}`
pub async fn delete_unit(
    State(state): State<AppState>,
    Extension(token_info): Extension<GetTokenInfoData>,
    Path(param): Path<request::UnitIdPath>,
) -> impl IntoResponse {
    const FN_NAME: &'static str = "delete_unit";

    let user_id = token_info.user_id.as_str();
    let roles = &token_info.roles;
    let unit_id = param.unit_id.as_str();

    // To check if the unit is for the owner.
    let unit = match check_unit(FN_NAME, user_id, roles, unit_id, true, &state).await {
        Err(e) => return Err(e), // XXX: not use "?" to solve E0282 error.
        Ok(unit) => match unit {
            None => return Ok(StatusCode::NO_CONTENT),
            Some(unit) => unit,
        },
    };

    del_unit_rsc(FN_NAME, unit_id, unit.code.as_str(), &state).await?;
    send_del_ctrl_message(FN_NAME, unit, &state).await?;

    Ok(StatusCode::NO_CONTENT)
}

/// `DELETE /{base}/api/v1/unit/user/{userId}`
pub async fn delete_unit_user(
    State(state): State<AppState>,
    Path(param): Path<request::UserIdPath>,
) -> impl IntoResponse {
    const FN_NAME: &'static str = "delete_unit_user";

    let cond = ListQueryCond {
        owner_id: Some(param.user_id.as_str()),
        ..Default::default()
    };
    let opts = ListOptions {
        cond: &cond,
        offset: None,
        limit: None,
        sort: None,
        cursor_max: Some(LIST_CURSOR_MAX),
    };
    let mut cursor: Option<Box<dyn Cursor>> = None;
    let mut rm_units = vec![];
    loop {
        match state.model.unit().list(&opts, cursor).await {
            Err(e) => {
                error!("[{}] list error: {}", FN_NAME, e);
                return Err(ErrResp::ErrDb(Some(e.to_string())));
            }
            Ok((mut list, _cursor)) => {
                rm_units.append(&mut list);
                if _cursor.is_none() {
                    break;
                }
                cursor = _cursor;
            }
        }
    }
    for unit in rm_units {
        del_unit_rsc(FN_NAME, unit.unit_id.as_str(), unit.code.as_str(), &state).await?;
        send_del_ctrl_message(FN_NAME, unit, &state).await?;
    }

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

async fn get_updates<'a>(
    fn_name: &str,
    token: &str,
    state: &AppState,
    body: &'a mut request::PatchUnitData,
    roles: &HashMap<String, bool>,
    target_unit: &Unit,
) -> Result<Updates<'a>, ErrResp> {
    let mut updates = Updates {
        ..Default::default()
    };
    let mut count = 0;
    if Role::is_role(roles, Role::ADMIN) || Role::is_role(roles, Role::MANAGER) {
        let mut target_owner_id = target_unit.owner_id.as_str();
        if let Some(owner_id) = body.owner_id.as_ref() {
            if owner_id.len() == 0 {
                return Err(ErrResp::ErrParam(Some(
                    "`ownerId` must with at least one character".to_string(),
                )));
            } else if !check_user(fn_name, token, owner_id.as_str(), state).await? {
                return Err(ErrResp::Custom(
                    ErrReq::OWNER_NOT_EXIST.0,
                    ErrReq::OWNER_NOT_EXIST.1,
                    None,
                ));
            }
            target_owner_id = owner_id.as_str();
            updates.owner_id = Some(owner_id.as_str());
            count += 1;

            if body.member_ids.is_none() && !target_unit.member_ids.contains(owner_id) {
                body.member_ids = Some(target_unit.member_ids.clone());
            }
        }
        if let Some(member_ids) = body.member_ids.as_mut() {
            member_ids.sort();
            member_ids.dedup();
            let mut found_owner = false;
            for v in member_ids.iter() {
                if v.len() == 0 {
                    return Err(ErrResp::ErrParam(Some(
                        "`memberIds` item must with at least one character".to_string(),
                    )));
                } else if !check_user(fn_name, token, v.as_str(), state).await? {
                    return Err(ErrResp::Custom(
                        ErrReq::MEMBER_NOT_EXIST.0,
                        ErrReq::MEMBER_NOT_EXIST.1,
                        None,
                    ));
                }
                if v.as_str().cmp(target_owner_id) == Ordering::Equal {
                    found_owner = true;
                }
            }
            if !found_owner {
                member_ids.push(target_owner_id.to_string());
            }
            updates.member_ids = Some(member_ids);
            count += 1;
        }
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

/// Use the Bearer token to check if the user ID is exist.
///
/// # Errors
///
/// Returns OK if status code is 200/404. Otherwise errors will be returned.
async fn check_user(
    fn_name: &str,
    token: &str,
    owner_id: &str,
    state: &AppState,
) -> Result<bool, ErrResp> {
    let uri = format!("{}/api/v1/user/{}", state.auth_base.as_str(), owner_id);
    let req = match state
        .client
        .request(reqwest::Method::GET, uri)
        .header(reqwest::header::AUTHORIZATION, format!("Bearer {}", token))
        .build()
    {
        Err(e) => {
            error!("[{}] create request error: {}", fn_name, e);
            return Err(ErrResp::ErrRsc(Some(format!(
                "create request error: {}",
                e
            ))));
        }
        Ok(req) => req,
    };
    match state.client.execute(req).await {
        Err(e) => {
            error!("[{}] request owner check error: {}", fn_name, e);
            return Err(ErrResp::ErrIntMsg(Some(format!(
                "check owner error: {}",
                e
            ))));
        }
        Ok(resp) => match resp.status() {
            reqwest::StatusCode::UNAUTHORIZED => {
                return Err(ErrResp::ErrAuth(None));
            }
            reqwest::StatusCode::OK => return Ok(true),
            reqwest::StatusCode::NOT_FOUND => return Ok(false),
            _ => {
                error!(
                    "[{}] check owner with status code: {}",
                    fn_name,
                    resp.status()
                );
                return Err(ErrResp::ErrIntMsg(Some(format!(
                    "check owner with status code: {}",
                    resp.status()
                ))));
            }
        },
    }
}

fn unit_list_transform(list: &Vec<Unit>) -> Vec<response::GetUnitData> {
    let mut ret = vec![];
    for unit in list.iter() {
        ret.push(unit_transform(&unit));
    }
    ret
}

fn unit_list_transform_bytes(
    list: &Vec<Unit>,
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
        let json_str = match serde_json::to_string(&unit_transform(item)) {
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

fn unit_transform(unit: &Unit) -> response::GetUnitData {
    response::GetUnitData {
        unit_id: unit.unit_id.clone(),
        code: unit.code.clone(),
        created_at: time_str(&unit.created_at),
        modified_at: time_str(&unit.modified_at),
        owner_id: unit.owner_id.clone(),
        member_ids: unit.member_ids.clone(),
        name: unit.name.clone(),
        info: unit.info.clone(),
    }
}

async fn del_unit_rsc(
    fn_name: &str,
    unit_id: &str,
    unit_code: &str,
    state: &AppState,
) -> Result<(), ErrResp> {
    let cond = ApplicationCond {
        unit_id: Some(unit_id),
        ..Default::default()
    };
    let opts = ApplicationListOpts {
        cond: &cond,
        offset: None,
        limit: None,
        sort: None,
        cursor_max: Some(LIST_CURSOR_MAX),
    };
    let mut cursor: Option<Box<dyn ApplicationCursor>> = None;
    let mut rm_mgrs = vec![];
    loop {
        match state.model.application().list(&opts, cursor).await {
            Err(e) => {
                error!("[{}] list application error: {}", fn_name, e);
                return Err(ErrResp::ErrDb(Some(e.to_string())));
            }
            Ok((list, _cursor)) => {
                for app in list {
                    let key = gen_mgr_key(unit_code, app.code.as_str());
                    match { state.application_mgrs.lock().unwrap().remove(&key) } {
                        None => error!("[{}] get no application manager {}", fn_name, key),
                        Some(old_mgr) => {
                            rm_mgrs.push(old_mgr);
                        }
                    }
                }
                if _cursor.is_none() {
                    break;
                }
                cursor = _cursor;
            }
        }
    }
    for mgr in rm_mgrs {
        if let Err(e) = mgr.close().await {
            error!(
                "[{}] close old application manager {} error: {}",
                fn_name,
                mgr.name(),
                e
            );
        }
    }

    let cond = NetworkCond {
        unit_id: Some(Some(unit_id)),
        ..Default::default()
    };
    let opts = NetworkListOpts {
        cond: &cond,
        offset: None,
        limit: None,
        sort: None,
        cursor_max: Some(LIST_CURSOR_MAX),
    };
    let mut cursor: Option<Box<dyn NetworkCursor>> = None;
    let mut rm_mgrs = vec![];
    loop {
        match state.model.network().list(&opts, cursor).await {
            Err(e) => {
                error!("[{}] list network error: {}", fn_name, e);
                return Err(ErrResp::ErrDb(Some(e.to_string())));
            }
            Ok((list, _cursor)) => {
                for app in list {
                    let key = gen_mgr_key(unit_code, app.code.as_str());
                    match { state.network_mgrs.lock().unwrap().remove(&key) } {
                        None => error!("[{}] get no network manager {}", fn_name, key),
                        Some(old_mgr) => {
                            rm_mgrs.push(old_mgr);
                        }
                    }
                }
                if _cursor.is_none() {
                    break;
                }
                cursor = _cursor;
            }
        }
    }
    for mgr in rm_mgrs {
        if let Err(e) = mgr.close().await {
            error!(
                "[{}] close old network manager {} error: {}",
                fn_name,
                mgr.name(),
                e
            );
        }
    }

    let cond = network_route::QueryCond {
        unit_id: Some(unit_id),
        ..Default::default()
    };
    if let Err(e) = state.model.network_route().del(&cond).await {
        error!("[{}] del network_route error: {}", fn_name, e);
        return Err(ErrResp::ErrDb(Some(e.to_string())));
    }

    let cond = device_route::QueryCond {
        unit_id: Some(unit_id),
        ..Default::default()
    };
    if let Err(e) = state.model.device_route().del(&cond).await {
        error!("[{}] del device_route error: {}", fn_name, e);
        return Err(ErrResp::ErrDb(Some(e.to_string())));
    }

    let cond = dldata_buffer::QueryCond {
        unit_id: Some(unit_id),
        ..Default::default()
    };
    if let Err(e) = state.model.dldata_buffer().del(&cond).await {
        error!("[{}] del dldata_buffer error: {}", fn_name, e);
        return Err(ErrResp::ErrDb(Some(e.to_string())));
    }

    let cond = device::QueryCond {
        unit_id: Some(unit_id),
        ..Default::default()
    };
    if let Err(e) = state.model.device().del(&cond).await {
        error!("[{}] del device error: {}", fn_name, e);
        return Err(ErrResp::ErrDb(Some(e.to_string())));
    }

    let cond = network::QueryCond {
        unit_id: Some(Some(unit_id)),
        ..Default::default()
    };
    if let Err(e) = state.model.network().del(&cond).await {
        error!("[{}] del network error: {}", fn_name, e);
        return Err(ErrResp::ErrDb(Some(e.to_string())));
    }

    let cond = application::QueryCond {
        unit_id: Some(unit_id),
        ..Default::default()
    };
    if let Err(e) = state.model.application().del(&cond).await {
        error!("[{}] del application error: {}", fn_name, e);
        return Err(ErrResp::ErrDb(Some(e.to_string())));
    }

    let cond = QueryCond {
        unit_id: Some(unit_id),
        ..Default::default()
    };
    if let Err(e) = state.model.unit().del(&cond).await {
        error!("[{}] del unit error: {}", fn_name, e);
        return Err(ErrResp::ErrDb(Some(e.to_string())));
    }

    Ok(())
}

/// Send delete control message.
async fn send_del_ctrl_message(fn_name: &str, unit: Unit, state: &AppState) -> Result<(), ErrResp> {
    if state.cache.is_some() {
        let msg = SendCtrlMsg::DelUnit {
            operation: CtrlMsgOp::DEL_UNIT.to_string(),
            new: CtrlDelUnit {
                unit_id: unit.unit_id,
                unit_code: unit.code,
            },
        };
        let payload = match serde_json::to_vec(&msg) {
            Err(e) => {
                error!(
                    "[{}] marshal JSON for {} error: {}",
                    fn_name,
                    CtrlMsgOp::DEL_UNIT,
                    e
                );
                return Err(ErrResp::ErrRsc(Some(format!(
                    "marshal control message error: {}",
                    e
                ))));
            }
            Ok(payload) => payload,
        };
        let ctrl_sender = { state.ctrl_senders.unit.lock().unwrap().clone() };
        if let Err(e) = ctrl_sender.send_msg(payload).await {
            error!(
                "[{}] send control message for {} error: {}",
                fn_name,
                CtrlMsgOp::DEL_UNIT,
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
            RecvCtrlMsg::DelUnit { new } => {
                if let Some(cache) = self.cache.as_ref() {
                    let cond = device_route::DelCacheQueryCond {
                        unit_code: new.unit_code.as_str(),
                        network_code: None,
                        network_addr: None,
                    };
                    if let Err(e) = cache.device_route().del_dldata(&cond).await {
                        error!(
                            "[{}] {} delete device route cache error: {}",
                            FN_NAME, queue_name, e
                        );
                    } else {
                        debug!("[{}] {} delete device route cache", FN_NAME, queue_name);
                    }
                    let cond = device_route::DelCachePubQueryCond {
                        unit_id: new.unit_id.as_str(),
                        device_id: None,
                    };
                    if let Err(e) = cache.device_route().del_dldata_pub(&cond).await {
                        error!(
                            "[{}] {} delete device route cache error: {}",
                            FN_NAME, queue_name, e
                        );
                    } else {
                        debug!("[{}] {} delete device route cache", FN_NAME, queue_name);
                    }
                }
                if let Err(e) = msg.ack().await {
                    error!("[{}] {} ACK error: {}", FN_NAME, queue_name, e);
                }
            }
        }
    }
}
