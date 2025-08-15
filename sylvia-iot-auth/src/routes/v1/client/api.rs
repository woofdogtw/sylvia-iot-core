use std::{error::Error as StdError, sync::Arc};

use axum::{
    Extension,
    body::{Body, Bytes},
    extract::State,
    http::{StatusCode, header},
    response::IntoResponse,
};
use chrono::Utc;
use log::{error, warn};

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
    Model, access_token, authorization_code,
    client::{
        Client, ListOptions, ListQueryCond, QueryCond, SortCond, SortKey, UpdateQueryCond, Updates,
    },
    refresh_token,
    user::{QueryCond as UserQueryCond, User},
};

const LIST_LIMIT_DEFAULT: u64 = 100;
const LIST_CURSOR_MAX: u64 = 100;
const ID_RAND_LEN: usize = 8;
const SECRET_LEN: usize = 16;

/// `POST /{base}/api/v1/client`
pub async fn post_client(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Json(mut body): Json<request::PostClientBody>,
) -> impl IntoResponse {
    const FN_NAME: &'static str = "post_client";

    body.data.redirect_uris.sort();
    body.data.redirect_uris.dedup();
    for v in body.data.redirect_uris.iter() {
        if !strings::is_uri(v.as_str()) {
            return Err(ErrResp::ErrParam(Some(
                "`redirectUris` must with invalid item(s)".to_string(),
            )));
        }
    }
    body.data.scopes.sort();
    body.data.scopes.dedup();
    for v in body.data.scopes.iter() {
        if !strings::is_scope(v.as_str()) {
            return Err(ErrResp::ErrParam(Some(
                "`scopes` with invalid item(s)".to_string(),
            )));
        }
    }
    let user_id = match Role::is_role(&user.roles, Role::ADMIN) {
        false => user.user_id,
        true => match body.data.user_id {
            None => user.user_id,
            Some(user_id) => {
                if user_id.len() == 0 {
                    return Err(ErrResp::ErrParam(Some(
                        "`userId` must not be empty".to_string(),
                    )));
                }
                let cond = UserQueryCond {
                    user_id: Some(user_id.as_str()),
                    ..Default::default()
                };
                match state.model.user().get(&cond).await {
                    Err(e) => {
                        error!("[{}] get error: {}", FN_NAME, e);
                        return Err(ErrResp::ErrDb(Some(e.to_string())));
                    }
                    Ok(None) => {
                        return Err(ErrResp::Custom(
                            ErrReq::USER_NOT_EXIST.0,
                            ErrReq::USER_NOT_EXIST.1,
                            None,
                        ));
                    }
                    Ok(_) => user_id,
                }
            }
        },
    };

    let now = Utc::now();
    let client_id = strings::random_id(&now, ID_RAND_LEN);
    let mut client = Client {
        client_id: client_id.clone(),
        created_at: now,
        modified_at: now,
        client_secret: None,
        redirect_uris: body.data.redirect_uris.clone(),
        scopes: body.data.scopes.clone(),
        user_id,
        name: body.data.name.clone(),
        image_url: match body.data.image.as_ref() {
            None => None,
            Some(url) => Some(url.clone()),
        },
    };
    if let Some(credentials) = body.credentials {
        if credentials {
            client.client_secret = Some(strings::randomstring(SECRET_LEN));
        }
    }
    if let Err(e) = state.model.client().add(&client).await {
        error!("[{}] add error: {}", FN_NAME, e);
        return Err(ErrResp::ErrDb(Some(e.to_string())));
    }
    Ok(Json(response::PostClient {
        data: response::PostClientData { client_id },
    }))
}

/// `GET /{base}/api/v1/client/count`
pub async fn get_client_count(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Query(query): Query<request::GetClientCountQuery>,
) -> impl IntoResponse {
    const FN_NAME: &'static str = "get_client_count";

    let user_id = match Role::is_role(&user.roles, Role::ADMIN) {
        false => Some(user.user_id),
        true => query.user,
    };
    let cond = ListQueryCond {
        user_id: match user_id.as_ref() {
            None => None,
            Some(user_id) => Some(user_id.as_str()),
        },
        ..Default::default()
    };
    match state.model.client().count(&cond).await {
        Err(e) => {
            error!("[{}] count error: {}", FN_NAME, e);
            Err(ErrResp::ErrDb(Some(e.to_string())))
        }
        Ok(count) => Ok(Json(response::GetClientCount {
            data: response::GetCountData { count },
        })),
    }
}

/// `GET /{base}/api/v1/client/list`
pub async fn get_client_list(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Query(query): Query<request::GetClientListQuery>,
) -> impl IntoResponse {
    const FN_NAME: &'static str = "get_client_list";

    let mut is_admin = false;
    let user_id = match Role::is_role(&user.roles, Role::ADMIN) {
        false => Some(user.user_id),
        true => {
            is_admin = true;
            query.user.clone()
        }
    };
    let cond = ListQueryCond {
        user_id: match user_id.as_ref() {
            None => None,
            Some(user_id) => Some(user_id.as_str()),
        },
        ..Default::default()
    };
    let sort_cond = match get_sort_cond(&query.sort) {
        Err(e) => return Err(e),
        Ok(cond) => cond,
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

    let (list, cursor) = match state.model.client().list(&opts, None).await {
        Err(e) => {
            error!("[{}] list error: {}", FN_NAME, e);
            return Err(ErrResp::ErrDb(Some(e.to_string())));
        }
        Ok((list, cursor)) => match cursor {
            None => match query.format {
                Some(request::ListFormat::Array) => {
                    return Ok(Json(client_list_transform(&list, is_admin)).into_response());
                }
                _ => {
                    return Ok(Json(response::GetClientList {
                        data: client_list_transform(&list, is_admin),
                    })
                    .into_response());
                }
            },
            Some(_) => (list, cursor),
        },
    };

    let body = Body::from_stream(async_stream::stream! {
        let user_id = user_id;
        let cond = ListQueryCond {
            user_id: match user_id.as_ref() {
                None => None,
                Some(user_id) => Some(user_id.as_str()),
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
            yield client_list_transform_bytes(&list, is_admin, is_first, cursor.is_none(), query.format.as_ref());
            is_first = false;
            if cursor.is_none() {
                break;
            }
            let (_list, _cursor) = match state.model.client().list(&opts, cursor).await {
                Err(_) => break,
                Ok((list, cursor)) => (list, cursor),
            };
            list = _list;
            cursor = _cursor;
        }
    });
    Ok(([(header::CONTENT_TYPE, ContentType::JSON)], body).into_response())
}

/// `GET /{base}/api/v1/client/{clientId}`
pub async fn get_client(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Path(param): Path<request::ClientIdPath>,
) -> impl IntoResponse {
    const FN_NAME: &'static str = "get_client";

    let mut is_admin = false;
    let user_id = match Role::is_role(&user.roles, Role::ADMIN) {
        false => Some(user.user_id),
        true => {
            is_admin = true;
            None
        }
    };
    let cond = QueryCond {
        user_id: match user_id.as_ref() {
            None => None,
            Some(user_id) => Some(user_id.as_str()),
        },
        client_id: Some(param.client_id.as_str()),
    };
    match state.model.client().get(&cond).await {
        Err(e) => {
            error!("[{}] get error: {}", FN_NAME, e);
            return Err(ErrResp::ErrDb(Some(e.to_string())));
        }
        Ok(client) => match client {
            None => return Err(ErrResp::ErrNotFound(None)),
            Some(client) => Ok(Json(response::GetClient {
                data: client_transform(&client, is_admin),
            })),
        },
    }
}

/// `PATCH /{base}/api/v1/client/{clientId}`
pub async fn patch_client(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Path(param): Path<request::ClientIdPath>,
    Json(mut body): Json<request::PatchClientBody>,
) -> impl IntoResponse {
    const FN_NAME: &'static str = "patch_client";

    if let Some(data) = body.data.as_mut() {
        if let Some(redirect_uris) = data.redirect_uris.as_mut() {
            redirect_uris.sort();
            redirect_uris.dedup();
            for v in redirect_uris {
                if !strings::is_uri(v.as_str()) {
                    return Err(ErrResp::ErrParam(Some(
                        "`redirectUris` must with invalid item(s)".to_string(),
                    )));
                }
            }
        }
        if let Some(scopes) = data.scopes.as_mut() {
            scopes.sort();
            scopes.dedup();
            for v in scopes {
                if !strings::is_scope(v.as_str()) {
                    return Err(ErrResp::ErrParam(Some(
                        "`scopes` with invalid item(s)".to_string(),
                    )));
                }
            }
        }
    }

    let mut is_admin = false;
    if Role::is_role(&user.roles, Role::ADMIN) {
        is_admin = true;
    }
    let user_id = user.user_id;

    let cond = QueryCond {
        client_id: Some(param.client_id.as_str()),
        ..Default::default()
    };
    let client = match state.model.client().get(&cond).await {
        Err(e) => {
            error!("[{}] get error: {}", FN_NAME, e);
            return Err(ErrResp::ErrDb(Some(e.to_string())));
        }
        Ok(client) => match client {
            None => return Err(ErrResp::ErrNotFound(None)),
            Some(client) => {
                if !is_admin && client.user_id != user_id {
                    warn!(
                        "[{}] {} try to patch other client",
                        FN_NAME,
                        user_id.as_str()
                    );
                    return Err(ErrResp::ErrNotFound(None));
                }
                client
            }
        },
    };

    let cond = UpdateQueryCond {
        user_id: client.user_id.as_str(),
        client_id: param.client_id.as_str(),
    };
    let updates = get_updates(&body, client.client_secret.is_some())?;
    if let Err(e) = state.model.client().update(&cond, &updates).await {
        error!("[{}] update error: {}", FN_NAME, e);
        return Err(ErrResp::ErrDb(Some(e.to_string())));
    }
    if updates.client_secret.is_some() {
        remove_tokens(&FN_NAME, &state.model, cond.client_id).await;
    }
    Ok(StatusCode::NO_CONTENT)
}

/// `DELETE /{base}/api/v1/client/{clientId}`
pub async fn delete_client(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Extension(client): Extension<Client>,
    Path(param): Path<request::ClientIdPath>,
) -> impl IntoResponse {
    const FN_NAME: &'static str = "delete_client";

    let user_id = match Role::is_role(&user.roles, Role::ADMIN) {
        false => Some(user.user_id),
        true => None,
    };
    if client.client_id.as_str().eq(param.client_id.as_str()) {
        return Err(ErrResp::ErrPerm(Some(
            "cannot delete the client itself".to_string(),
        )));
    }

    let cond = QueryCond {
        user_id: match user_id.as_ref() {
            None => None,
            Some(user_id) => Some(user_id.as_str()),
        },
        client_id: Some(param.client_id.as_str()),
    };
    match state.model.client().del(&cond).await {
        Err(e) => {
            error!("[{}] del error: {}", FN_NAME, e);
            return Err(ErrResp::ErrDb(Some(e.to_string())));
        }
        Ok(_) => Ok(StatusCode::NO_CONTENT),
    }
}

/// `DELETE /{base}/api/v1/client/user/{userId}`
pub async fn delete_client_user(
    State(state): State<AppState>,
    Path(param): Path<request::UserIdPath>,
) -> impl IntoResponse {
    const FN_NAME: &'static str = "delete_client_user";

    let cond = QueryCond {
        user_id: Some(param.user_id.as_str()),
        ..Default::default()
    };
    match state.model.client().del(&cond).await {
        Err(e) => {
            error!("[{}] del error: {}", FN_NAME, e);
            return Err(ErrResp::ErrDb(Some(e.to_string())));
        }
        Ok(_) => Ok(StatusCode::NO_CONTENT),
    }
}

fn get_sort_cond(sort_args: &Option<String>) -> Result<Vec<SortCond>, ErrResp> {
    match sort_args.as_ref() {
        None => Ok(vec![SortCond {
            key: SortKey::Name,
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
                        "created" => SortKey::CreatedAt,
                        "modified" => SortKey::ModifiedAt,
                        "name" => SortKey::Name,
                        _ => {
                            return Err(ErrResp::ErrParam(Some(format!(
                                "invalid sort key {}",
                                field
                            ))));
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
                            ))));
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

fn get_updates(
    body: &'_ request::PatchClientBody,
    has_secret: bool,
) -> Result<Updates<'_>, ErrResp> {
    let mut updates = Updates {
        ..Default::default()
    };
    let mut count = 0;
    if let Some(body) = body.data.as_ref() {
        if let Some(redirect_uris) = body.redirect_uris.as_ref() {
            updates.redirect_uris = Some(redirect_uris);
            count += 1;
        }
        if let Some(scopes) = body.scopes.as_ref() {
            updates.scopes = Some(scopes);
            count += 1;
        }
        if let Some(name) = body.name.as_ref() {
            updates.name = Some(name.as_str());
            count += 1;
        }
        if let Some(image) = body.image.as_ref() {
            updates.image_url = match image.as_ref() {
                None => Some(None),
                Some(image) => Some(Some(image.as_str())),
            };
            count += 1;
        }
    }
    if let Some(regen_secret) = body.regen_secret {
        if regen_secret {
            if !has_secret {
                return Err(ErrResp::ErrParam(Some(
                    "cannot re-generate secret for public client".to_string(),
                )));
            }
            updates.client_secret = Some(Some(strings::randomstring(SECRET_LEN)));
            count += 1;
        }
    }
    if count == 0 {
        return Err(ErrResp::ErrParam(Some(
            "at least one parameter".to_string(),
        )));
    }
    updates.modified_at = Some(Utc::now());
    Ok(updates)
}

fn client_list_transform(list: &Vec<Client>, is_admin: bool) -> Vec<response::GetClientData> {
    let mut ret = vec![];
    for client in list.iter() {
        ret.push(client_transform(&client, is_admin));
    }
    ret
}

fn client_list_transform_bytes(
    list: &Vec<Client>,
    is_admin: bool,
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
        let json_str = match serde_json::to_string(&client_transform(item, is_admin)) {
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

fn client_transform(client: &Client, is_admin: bool) -> response::GetClientData {
    response::GetClientData {
        client_id: client.client_id.clone(),
        created_at: time_str(&client.created_at),
        modified_at: time_str(&client.modified_at),
        client_secret: match client.client_secret.as_ref() {
            None => None,
            Some(secret) => Some(secret.clone()),
        },
        redirect_uris: client.redirect_uris.clone(),
        scopes: client.scopes.clone(),
        user_id: match is_admin {
            false => None,
            true => Some(client.user_id.clone()),
        },
        name: client.name.clone(),
        image: match client.image_url.as_ref() {
            None => None,
            Some(image) => Some(image.clone()),
        },
    }
}

async fn remove_tokens(fn_name: &str, model: &Arc<dyn Model>, client_id: &str) {
    let cond = authorization_code::QueryCond {
        client_id: Some(client_id),
        ..Default::default()
    };
    if let Err(e) = model.authorization_code().del(&cond).await {
        error!("[{}] delete access token error: {}", fn_name, e);
    }
    let cond = access_token::QueryCond {
        client_id: Some(client_id),
        ..Default::default()
    };
    if let Err(e) = model.access_token().del(&cond).await {
        error!("[{}] delete access token error: {}", fn_name, e);
    }
    let cond = refresh_token::QueryCond {
        client_id: Some(client_id),
        ..Default::default()
    };
    if let Err(e) = model.refresh_token().del(&cond).await {
        error!("[{}] delete refresh token error: {}", fn_name, e);
    }
}
