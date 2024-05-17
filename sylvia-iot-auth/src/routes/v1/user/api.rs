use std::{collections::HashMap, error::Error as StdError, sync::Arc};

use axum::{
    body::{Body, Bytes},
    extract::State,
    http::{header, StatusCode},
    response::IntoResponse,
    Extension,
};
use chrono::{DateTime, Utc};
use log::{error, warn};
use serde_json::{self, Map, Value};

use sylvia_iot_corelib::{
    constants::ContentType,
    err::ErrResp,
    http::{Json, Path, Query},
    role::Role,
    strings::{self, time_str},
};

use super::{
    super::super::{ErrReq, State as AppState},
    request, response,
};
use crate::models::{
    access_token, authorization_code, refresh_token,
    user::{ListOptions, ListQueryCond, QueryCond, SortCond, SortKey, Updates, User},
    Model,
};

#[derive(Default)]
struct GetAdminFields {
    expired: bool,
    disabled: bool,
}

const LIST_LIMIT_DEFAULT: u64 = 100;
const LIST_CURSOR_MAX: u64 = 100;
const ID_RAND_LEN: usize = 8;
const SALT_LEN: usize = 8;

/// `GET /{base}/api/v1/user`
pub async fn get_user(Extension(user): Extension<User>) -> impl IntoResponse {
    Json(response::GetUser {
        data: response::GetUserData {
            account: user.account.clone(),
            created_at: time_str(&user.created_at),
            modified_at: time_str(&user.modified_at),
            verified_at: match user.verified_at {
                None => None,
                Some(time) => Some(time_str(&time)),
            },
            roles: user.roles.clone(),
            name: user.name.clone(),
            info: user.info.clone(),
        },
    })
}

/// `PATCH /{base}/api/v1/user`
pub async fn patch_user(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Json(body): Json<request::PatchUserBody>,
) -> impl IntoResponse {
    const FN_NAME: &'static str = "patch_user";

    let user_id = user.user_id.as_str();
    let updates = get_updates(&body.data)?;
    if let Err(e) = state.model.user().update(user_id, &updates).await {
        error!("[{}] {}", FN_NAME, e);
        return Err(ErrResp::ErrDb(Some(e.to_string())));
    }
    if updates.password.is_some() {
        remove_tokens(&FN_NAME, &state.model, user_id).await;
    }
    Ok(StatusCode::NO_CONTENT)
}

/// `POST /{base}/api/v1/user`
pub async fn post_admin_user(
    State(state): State<AppState>,
    Json(body): Json<request::PostAdminUserBody>,
) -> impl IntoResponse {
    const FN_NAME: &'static str = "post_admin_user";

    let account = body.data.account.to_lowercase();
    if !strings::is_account(account.as_str()) {
        return Err(ErrResp::ErrParam(Some(
            "`account` must be email or [A-Za-z0-9]{1}[A-Za-z0-9-_]*".to_string(),
        )));
    } else if body.data.password.len() == 0 {
        return Err(ErrResp::ErrParam(Some(
            "`password` must at least one character".to_string(),
        )));
    }
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
        account: Some(account.as_str()),
        ..Default::default()
    };
    match state.model.user().get(&cond).await {
        Err(e) => {
            error!("[{}] get error: {}", FN_NAME, e);
            return Err(ErrResp::ErrDb(Some(e.to_string())));
        }
        Ok(user) => match user {
            None => (),
            Some(_) => {
                return Err(ErrResp::Custom(
                    ErrReq::USER_EXIST.0,
                    ErrReq::USER_EXIST.1,
                    None,
                ))
            }
        },
    }

    let now = Utc::now();
    let user_id = strings::random_id(&now, ID_RAND_LEN);
    let salt = strings::randomstring(SALT_LEN);
    let user = User {
        user_id: user_id.clone(),
        account,
        created_at: now,
        modified_at: now,
        verified_at: match body.expired_at {
            None => Some(now),
            Some(_) => None,
        },
        expired_at: body.expired_at,
        disabled_at: None,
        roles: HashMap::new(),
        password: strings::password_hash(body.data.password.as_str(), salt.as_str()),
        salt,
        name: match body.data.name.as_ref() {
            None => "".to_string(),
            Some(name) => name.clone(),
        },
        info: match body.data.info.as_ref() {
            None => Map::new(),
            Some(info) => info.clone(),
        },
    };
    if let Err(e) = state.model.user().add(&user).await {
        error!("[{}] add error: {}", FN_NAME, e);
        return Err(ErrResp::ErrDb(Some(e.to_string())));
    }
    Ok(Json(response::PostAdminUser {
        data: response::PostAdminUserData { user_id },
    }))
}

/// `GET /{base}/api/v1/user/count`
pub async fn get_admin_user_count(
    State(state): State<AppState>,
    Query(query): Query<request::GetAdminUserCountQuery>,
) -> impl IntoResponse {
    const FN_NAME: &'static str = "get_admin_user_count";

    let mut account_cond = None;
    let mut account_contains_cond = None;
    if let Some(account) = query.account.as_ref() {
        if account.len() > 0 {
            account_cond = Some(account.as_str());
        }
    }
    if account_cond.is_none() {
        if let Some(contains) = query.contains.as_ref() {
            if contains.len() > 0 {
                account_contains_cond = Some(contains.as_str());
            }
        }
    }
    let cond = ListQueryCond {
        account: account_cond,
        account_contains: account_contains_cond,
        ..Default::default()
    };
    match state.model.user().count(&cond).await {
        Err(e) => {
            error!("[{}] count error: {}", FN_NAME, e);
            Err(ErrResp::ErrDb(Some(e.to_string())))
        }
        Ok(count) => Ok(Json(response::GetAdminUserCount {
            data: response::GetCountData { count },
        })),
    }
}

/// `GET /{base}/api/v1/user/list`
pub async fn get_admin_user_list(
    State(state): State<AppState>,
    Query(query): Query<request::GetAdminUserListQuery>,
) -> impl IntoResponse {
    const FN_NAME: &'static str = "get_admin_user_list";

    let mut account_cond = None;
    let mut account_contains_cond = None;
    if let Some(account) = query.account.as_ref() {
        if account.len() > 0 {
            account_cond = Some(account.as_str());
        }
    }
    if account_cond.is_none() {
        if let Some(contains) = query.contains.as_ref() {
            if contains.len() > 0 {
                account_contains_cond = Some(contains.as_str());
            }
        }
    }
    let cond = ListQueryCond {
        account: account_cond,
        account_contains: account_contains_cond,
        ..Default::default()
    };
    let fields_cond = get_list_fields(&query.fields);
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

    let (list, cursor) = match state.model.user().list(&opts, None).await {
        Err(e) => {
            error!("[{}] list error: {}", FN_NAME, e);
            return Err(ErrResp::ErrDb(Some(e.to_string())));
        }
        Ok((list, cursor)) => match cursor {
            None => match query.format {
                Some(request::ListFormat::Array) => {
                    return Ok(Json(user_list_transform(&list, &fields_cond)).into_response())
                }
                _ => {
                    return Ok(Json(response::GetAdminUserList {
                        data: user_list_transform(&list, &fields_cond),
                    })
                    .into_response())
                }
            },
            Some(_) => (list, cursor),
        },
    };

    let body = Body::from_stream(async_stream::stream! {
        let mut account_cond = None;
        let mut account_contains_cond = None;
        if let Some(account) = query.account.as_ref() {
            if account.len() > 0 {
                account_cond = Some(account.as_str());
            }
        }
        if account_cond.is_none() {
            if let Some(contains) = query.contains.as_ref() {
                if contains.len() > 0 {
                    account_contains_cond = Some(contains.as_str());
                }
            }
        }
        let cond = ListQueryCond {
            account: account_cond,
            account_contains: account_contains_cond,
            ..Default::default()
        };
        let fields_cond = get_list_fields(&query.fields);
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
            yield user_list_transform_bytes(&list, &fields_cond, is_first, cursor.is_none(), query.format.as_ref());
            is_first = false;
            if cursor.is_none() {
                break;
            }
            let (_list, _cursor) = match state.model.user().list(&opts, cursor).await {
                Err(_) => break,
                Ok((list, cursor)) => (list, cursor),
            };
            list = _list;
            cursor = _cursor;
        }
    });
    Ok(([(header::CONTENT_TYPE, ContentType::JSON)], body).into_response())
}

/// `GET /{base}/api/v1/user/{userId}`
pub async fn get_admin_user(
    State(state): State<AppState>,
    Path(param): Path<request::UserIdPath>,
) -> impl IntoResponse {
    const FN_NAME: &'static str = "get_admin_user";

    let cond = QueryCond {
        user_id: Some(param.user_id.as_str()),
        ..Default::default()
    };
    match state.model.user().get(&cond).await {
        Err(e) => {
            error!("[{}] get error: {}", FN_NAME, e);
            Err(ErrResp::ErrDb(Some(e.to_string())))
        }
        Ok(user) => match user {
            None => Err(ErrResp::ErrNotFound(None)),
            Some(user) => Ok(Json(response::GetAdminUser {
                data: user_transform(
                    &user,
                    &GetAdminFields {
                        ..Default::default()
                    },
                ),
            })),
        },
    }
}

/// `PATCH /{base}/api/v1/user/{userId}`
pub async fn patch_admin_user(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Path(param): Path<request::UserIdPath>,
    Json(body): Json<request::PatchAdminUserBody>,
) -> impl IntoResponse {
    const FN_NAME: &'static str = "patch_admin_user";

    let cond = QueryCond {
        user_id: Some(param.user_id.as_str()),
        ..Default::default()
    };
    let target_user = match state.model.user().get(&cond).await {
        Err(e) => {
            error!("[{}] get error: {}", FN_NAME, e);
            return Err(ErrResp::ErrDb(Some(e.to_string())));
        }
        Ok(user) => match user {
            None => return Err(ErrResp::ErrNotFound(None)),
            Some(user) => user,
        },
    };

    let user_id = param.user_id.as_str();
    let updates = get_admin_updates(
        &body,
        Role::is_role(&user.roles, Role::ADMIN),
        &target_user,
        user.user_id.as_str(),
    )?;
    if let Err(e) = state.model.user().update(user_id, &updates).await {
        error!("[{}] update error: {}", FN_NAME, e);
        return Err(ErrResp::ErrDb(Some(e.to_string())));
    }
    if updates.password.is_some() {
        remove_tokens(&FN_NAME, &state.model, user_id).await;
    }
    Ok(StatusCode::NO_CONTENT)
}

/// `DELETE /{base}/api/v1/user/{userId}`
pub async fn delete_admin_user(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Path(param): Path<request::UserIdPath>,
) -> impl IntoResponse {
    const FN_NAME: &'static str = "delete_admin_user";

    if user.user_id == param.user_id {
        return Err(ErrResp::ErrPerm(Some("cannot delete oneself".to_string())));
    }
    if let Err(e) = state.model.user().del(param.user_id.as_str()).await {
        error!("[{}] del error: {}", FN_NAME, e);
        return Err(ErrResp::ErrDb(Some(e.to_string())));
    }
    Ok(StatusCode::NO_CONTENT)
}

fn get_updates(body: &request::PatchUserData) -> Result<Updates, ErrResp> {
    let mut updates = Updates {
        ..Default::default()
    };
    let mut count = 0;
    if let Some(password) = body.password.as_ref() {
        if password.len() == 0 {
            return Err(ErrResp::ErrParam(Some(
                "`password` must at least one character".to_string(),
            )));
        }
        let salt = strings::randomstring(SALT_LEN);
        updates.password = Some(strings::password_hash(password, salt.as_str()));
        updates.salt = Some(salt);
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

fn get_sort_cond(sort_args: &Option<String>) -> Result<Vec<SortCond>, ErrResp> {
    match sort_args.as_ref() {
        None => Ok(vec![SortCond {
            key: SortKey::Account,
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
                        "account" => SortKey::Account,
                        "created" => SortKey::CreatedAt,
                        "modified" => SortKey::ModifiedAt,
                        "verified" => SortKey::VerifiedAt,
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

fn get_list_fields(fields_cond: &Option<String>) -> GetAdminFields {
    let mut ret_fields = GetAdminFields {
        expired: false,
        disabled: false,
    };
    if let Some(fields_args) = fields_cond {
        let mut fields = fields_args.split(",");
        while let Some(field) = fields.next() {
            match field {
                "expired" => ret_fields.expired = true,
                "disabled" => ret_fields.disabled = true,
                _ => (),
            }
        }
    }
    ret_fields
}

fn get_admin_updates<'a>(
    body: &'a request::PatchAdminUserBody,
    is_admin: bool,
    target_user: &User,
    op_user_id: &str,
) -> Result<Updates<'a>, ErrResp> {
    if !is_admin && Role::is_role(&target_user.roles, Role::ADMIN) {
        warn!("{} try to patch admin", op_user_id);
        return Err(ErrResp::ErrPerm(None));
    }

    let mut updates = Updates {
        ..Default::default()
    };
    let mut count = 0;
    if let Some(data) = body.data.as_ref() {
        if let Some(verified_at) = data.verified_at.as_ref() {
            if is_admin {
                updates.verified_at = match DateTime::parse_from_rfc3339(verified_at.as_str()) {
                    Err(e) => {
                        return Err(ErrResp::ErrParam(Some(format!(
                            "wrong `verified_at`: {}",
                            e
                        ))))
                    }
                    Ok(time) => Some(time.into()),
                };
                updates.expired_at = Some(None);
                count += 1;
            }
        }
        if let Some(roles) = data.roles.as_ref() {
            if !is_admin {
                if Role::is_role(roles, Role::ADMIN) || Role::is_role(roles, Role::SERVICE) {
                    warn!("{} try to patch user to role admin/service", op_user_id);
                    return Err(ErrResp::ErrPerm(None));
                }
            }
            updates.roles = Some(roles);
            count += 1;
        }
        if let Some(password) = data.password.as_ref() {
            if is_admin {
                if password.len() == 0 {
                    return Err(ErrResp::ErrParam(Some(
                        "`password` must at least one character".to_string(),
                    )));
                }
                let salt = strings::randomstring(SALT_LEN);
                updates.password = Some(strings::password_hash(password, salt.as_str()));
                updates.salt = Some(salt);
                count += 1;
            }
        }
        if let Some(name) = data.name.as_ref() {
            if is_admin {
                updates.name = Some(name.as_str());
                count += 1;
            }
        }
        if let Some(info) = data.info.as_ref() {
            if is_admin {
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
        }
    }
    if let Some(disable) = body.disable.as_ref() {
        if !is_admin
            && (Role::is_role(&target_user.roles, Role::ADMIN)
                || Role::is_role(&target_user.roles, Role::MANAGER))
        {
            warn!("{} try to disable admin/manager", op_user_id);
            return Err(ErrResp::ErrPerm(None));
        }
        updates.disabled_at = match disable {
            false => Some(None),
            true => Some(Some(Utc::now())),
        };
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

fn user_list_transform(
    list: &Vec<User>,
    fields_cond: &GetAdminFields,
) -> Vec<response::GetAdminUserData> {
    let mut ret = vec![];
    for user in list.iter() {
        ret.push(user_transform(&user, fields_cond));
    }
    ret
}

fn user_list_transform_bytes(
    list: &Vec<User>,
    fields_cond: &GetAdminFields,
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
        let json_str = match serde_json::to_string(&user_transform(item, fields_cond)) {
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

fn user_transform(user: &User, fields_cond: &GetAdminFields) -> response::GetAdminUserData {
    response::GetAdminUserData {
        user_id: user.user_id.clone(),
        account: user.account.clone(),
        created_at: time_str(&user.created_at),
        modified_at: time_str(&user.modified_at),
        verified_at: match user.verified_at.as_ref() {
            None => None,
            Some(value) => Some(time_str(value)),
        },
        expired_at: match fields_cond.expired {
            false => None,
            true => match user.expired_at.as_ref() {
                None => Some(Value::Null),
                Some(value) => Some(Value::String(time_str(value))),
            },
        },
        disabled_at: match fields_cond.disabled {
            false => None,
            true => match user.disabled_at.as_ref() {
                None => Some(Value::Null),
                Some(value) => Some(Value::String(time_str(value))),
            },
        },
        roles: user.roles.clone(),
        name: user.name.clone(),
        info: user.info.clone(),
    }
}

async fn remove_tokens(fn_name: &str, model: &Arc<dyn Model>, user_id: &str) {
    let cond = authorization_code::QueryCond {
        user_id: Some(user_id),
        ..Default::default()
    };
    if let Err(e) = model.authorization_code().del(&cond).await {
        error!("[{}] delete access token error: {}", fn_name, e);
    }
    let cond = access_token::QueryCond {
        user_id: Some(user_id),
        ..Default::default()
    };
    if let Err(e) = model.access_token().del(&cond).await {
        error!("[{}] delete access token error: {}", fn_name, e);
    }
    let cond = refresh_token::QueryCond {
        user_id: Some(user_id),
        ..Default::default()
    };
    if let Err(e) = model.refresh_token().del(&cond).await {
        error!("[{}] delete refresh token error: {}", fn_name, e);
    }
}
