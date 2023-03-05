use std::error::Error as StdError;

use actix_web::{
    web::{self, Bytes},
    HttpMessage, HttpRequest, HttpResponse, Responder,
};
use chrono::Utc;
use log::{error, warn};

use sylvia_iot_corelib::{
    err::ErrResp,
    role::Role,
    strings::{self, time_str},
};

use super::{
    super::super::{ErrReq, State},
    request, response,
};
use crate::models::{
    client::{
        Client, ListOptions, ListQueryCond, QueryCond, SortCond, SortKey, UpdateQueryCond, Updates,
    },
    user::{QueryCond as UserQueryCond, User},
};

const LIST_LIMIT_DEFAULT: u64 = 100;
const LIST_CURSOR_MAX: u64 = 100;
const ID_RAND_LEN: usize = 8;
const SECRET_LEN: usize = 16;

/// `POST /{base}/api/v1/client`
pub async fn post_client(
    req: HttpRequest,
    mut body: web::Json<request::PostClientBody>,
    state: web::Data<State>,
) -> impl Responder {
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
    let user_id = match req.extensions_mut().get::<User>() {
        None => {
            error!("[{}] user not found", FN_NAME);
            return Err(ErrResp::ErrUnknown(Some("user not found".to_string())));
        }
        Some(user) => {
            if !Role::is_role(&user.roles, Role::ADMIN) {
                user.user_id.clone()
            } else {
                match body.data.user_id.as_ref() {
                    None => user.user_id.clone(),
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
                                ))
                            }
                            Ok(_) => user_id.clone(),
                        }
                    }
                }
            }
        }
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
    Ok(HttpResponse::Ok().json(response::PostClient {
        data: response::PostClientData { client_id },
    }))
}

/// `GET /{base}/api/v1/client/count`
pub async fn get_client_count(
    req: HttpRequest,
    query: web::Query<request::GetClientCountQuery>,
    state: web::Data<State>,
) -> impl Responder {
    const FN_NAME: &'static str = "get_client_count";

    let user_id = match req.extensions_mut().get::<User>() {
        None => {
            error!("[{}] user not found", FN_NAME);
            return Err(ErrResp::ErrUnknown(Some("user not found".to_string())));
        }
        Some(user) => {
            if Role::is_role(&user.roles, Role::ADMIN) {
                match query.user.as_ref() {
                    None => None,
                    Some(user_id) => Some(user_id.clone()),
                }
            } else {
                Some(user.user_id.clone())
            }
        }
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
        Ok(count) => Ok(HttpResponse::Ok().json(response::GetClientCount {
            data: response::GetCountData { count },
        })),
    }
}

/// `GET /{base}/api/v1/client/list`
pub async fn get_client_list(
    req: HttpRequest,
    query: web::Query<request::GetClientListQuery>,
    state: web::Data<State>,
) -> impl Responder {
    const FN_NAME: &'static str = "get_client_list";

    let mut is_admin = false;
    let user_id = match req.extensions_mut().get::<User>() {
        None => {
            error!("[{}] user not found", FN_NAME);
            return Err(ErrResp::ErrUnknown(Some("user not found".to_string())));
        }
        Some(user) => {
            if Role::is_role(&user.roles, Role::ADMIN) {
                is_admin = true;
                match query.user.as_ref() {
                    None => None,
                    Some(user_id) => Some(user_id.clone()),
                }
            } else {
                Some(user.user_id.clone())
            }
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
                    return Ok(HttpResponse::Ok().json(client_list_transform(&list, is_admin)))
                }
                _ => {
                    return Ok(HttpResponse::Ok().json(response::GetClientList {
                        data: client_list_transform(&list, is_admin),
                    }))
                }
            },
            Some(_) => (list, cursor),
        },
    };

    // TODO: detect client disconnect
    let stream = async_stream::stream! {
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
    };
    Ok(HttpResponse::Ok().streaming(stream))
}

/// `GET /{base}/api/v1/client/{clientId}`
pub async fn get_client(
    req: HttpRequest,
    param: web::Path<request::ClientIdPath>,
    state: web::Data<State>,
) -> impl Responder {
    const FN_NAME: &'static str = "get_client";

    let mut is_admin = false;
    let user_id = match req.extensions_mut().get::<User>() {
        None => {
            error!("[{}] user not found", FN_NAME);
            return Err(ErrResp::ErrUnknown(Some("user not found".to_string())));
        }
        Some(user) => {
            if Role::is_role(&user.roles, Role::ADMIN) {
                is_admin = true;
                None
            } else {
                Some(user.user_id.clone())
            }
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
            Some(client) => Ok(HttpResponse::Ok().json(response::GetClient {
                data: client_transform(&client, is_admin),
            })),
        },
    }
}

/// `PATCH /{base}/api/v1/client/{clientId}`
pub async fn patch_client(
    req: HttpRequest,
    param: web::Path<request::ClientIdPath>,
    mut body: web::Json<request::PatchClientBody>,
    state: web::Data<State>,
) -> impl Responder {
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
    let user_id = match req.extensions_mut().get::<User>() {
        None => {
            error!("[{}] user not found", FN_NAME);
            return Err(ErrResp::ErrUnknown(Some("user not found".to_string())));
        }
        Some(user) => {
            if Role::is_role(&user.roles, Role::ADMIN) {
                is_admin = true;
            }
            user.user_id.clone()
        }
    };

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
    Ok(HttpResponse::NoContent().finish())
}

/// `DELETE /{base}/api/v1/client/{clientId}`
pub async fn delete_client(
    req: HttpRequest,
    param: web::Path<request::ClientIdPath>,
    state: web::Data<State>,
) -> impl Responder {
    const FN_NAME: &'static str = "delete_client";

    let user_id = match req.extensions_mut().get::<User>() {
        None => {
            error!("[{}] user not found", FN_NAME);
            return Err(ErrResp::ErrUnknown(Some("user not found".to_string())));
        }
        Some(user) => {
            if Role::is_role(&user.roles, Role::ADMIN) {
                None
            } else {
                Some(user.user_id.clone())
            }
        }
    };
    match req.extensions_mut().get::<Client>() {
        None => {
            error!("[{}] client not found", FN_NAME);
            return Err(ErrResp::ErrUnknown(Some("client not found".to_string())));
        }
        Some(client) => {
            if client.client_id.as_str().eq(param.client_id.as_str()) {
                return Err(ErrResp::ErrPerm(Some(
                    "cannot delete the client itself".to_string(),
                )));
            }
        }
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
        Ok(_) => Ok(HttpResponse::NoContent().finish()),
    }
}

/// `DELETE /{base}/api/v1/client/user/{userId}`
pub async fn delete_client_user(
    param: web::Path<request::UserIdPath>,
    state: web::Data<State>,
) -> impl Responder {
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
        Ok(_) => Ok(HttpResponse::NoContent().finish()),
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

fn get_updates(body: &request::PatchClientBody, has_secret: bool) -> Result<Updates, ErrResp> {
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
