use std::{collections::HashMap, error::Error as StdError};

use axum::{
    body::{Body, Bytes},
    extract::State,
    http::{header, StatusCode},
    response::IntoResponse,
    Extension,
};
use log::error;
use serde_json;

use sylvia_iot_corelib::{
    constants::ContentType,
    err::ErrResp,
    http::{Json, Path, Query},
    role::Role,
    strings::time_str,
};

use super::{
    super::{
        super::{middleware::GetTokenInfoData, ErrReq, State as AppState},
        lib::check_unit,
    },
    request, response,
};
use crate::models::dldata_buffer::{
    DlDataBuffer, ListOptions, ListQueryCond, QueryCond, SortCond, SortKey,
};

const LIST_LIMIT_DEFAULT: u64 = 100;
const LIST_CURSOR_MAX: u64 = 100;

/// `GET /{base}/api/v1/dldata-buffer/count`
pub async fn get_dldata_buffer_count(
    State(state): State<AppState>,
    Extension(token_info): Extension<GetTokenInfoData>,
    Query(query): Query<request::GetDlDataBufferCountQuery>,
) -> impl IntoResponse {
    const FN_NAME: &'static str = "get_dldata_buffer_count";

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
    match state.model.dldata_buffer().count(&cond).await {
        Err(e) => {
            error!("[{}] count error: {}", FN_NAME, e);
            Err(ErrResp::ErrDb(Some(e.to_string())))
        }
        Ok(count) => Ok(Json(response::GetDlDataBufferCount {
            data: response::GetCountData { count },
        })),
    }
}

/// `GET /{base}/api/v1/dldata-buffer/list`
pub async fn get_dldata_buffer_list(
    State(state): State<AppState>,
    Extension(token_info): Extension<GetTokenInfoData>,
    Query(query): Query<request::GetDlDataBufferListQuery>,
) -> impl IntoResponse {
    const FN_NAME: &'static str = "get_dldata_buffer_list";

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

    let (list, cursor) = match state.model.dldata_buffer().list(&opts, None).await {
        Err(e) => {
            error!("[{}] list error: {}", FN_NAME, e);
            return Err(ErrResp::ErrDb(Some(e.to_string())));
        }
        Ok((list, cursor)) => match cursor {
            None => match query.format {
                Some(request::ListFormat::Array) => {
                    return Ok(Json(data_list_transform(&list)).into_response())
                }
                _ => {
                    return Ok(Json(response::GetDlDataBufferList {
                        data: data_list_transform(&list),
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
            yield data_list_transform_bytes(&list, is_first, cursor.is_none(), query.format.as_ref());
            is_first = false;
            if cursor.is_none() {
                break;
            }
            let (_list, _cursor) = match state.model.dldata_buffer().list(&opts, cursor).await {
                Err(_) => break,
                Ok((list, cursor)) => (list, cursor),
            };
            list = _list;
            cursor = _cursor;
        }
    });
    Ok(([(header::CONTENT_TYPE, ContentType::JSON)], body).into_response())
}

/// `DELETE /{base}/api/v1/dldata-buffer/{dataId}`
pub async fn delete_dldata_buffer(
    State(state): State<AppState>,
    Extension(token_info): Extension<GetTokenInfoData>,
    Path(param): Path<request::DataIdPath>,
) -> impl IntoResponse {
    const FN_NAME: &'static str = "delete_dldata_buffer";

    let user_id = token_info.user_id.as_str();
    let roles = &token_info.roles;
    let data_id = param.data_id.as_str();

    // To check if the dldata buffer is for the user.
    match check_data(FN_NAME, data_id, user_id, true, roles, &state).await? {
        None => return Ok(StatusCode::NO_CONTENT),
        Some(_) => (),
    }

    let cond = QueryCond {
        data_id: Some(data_id),
        ..Default::default()
    };
    if let Err(e) = state.model.dldata_buffer().del(&cond).await {
        error!("[{}] del error: {}", FN_NAME, e);
        return Err(ErrResp::ErrDb(Some(e.to_string())));
    }

    Ok(StatusCode::NO_CONTENT)
}

fn get_sort_cond(sort_args: &Option<String>) -> Result<Vec<SortCond>, ErrResp> {
    match sort_args.as_ref() {
        None => Ok(vec![
            SortCond {
                key: SortKey::ApplicationCode,
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
                        "created" => SortKey::CreatedAt,
                        "expired" => SortKey::ExpiredAt,
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

/// To check if the user ID can access the dldata buffer. Choose `only_owner` to check if the user
/// is the unit owner or one of unit members.
///
/// # Errors
///
/// Returns OK if the device is found or not. Otherwise errors will be returned.
async fn check_data(
    fn_name: &str,
    data_id: &str,
    user_id: &str,
    only_owner: bool, // to check if this `user_id` is the owner.
    roles: &HashMap<String, bool>,
    state: &AppState,
) -> Result<Option<DlDataBuffer>, ErrResp> {
    let data = match state.model.dldata_buffer().get(data_id).await {
        Err(e) => {
            error!("[{}] get error: {}", fn_name, e);
            return Err(ErrResp::ErrDb(Some(e.to_string())));
        }
        Ok(data) => match data {
            None => return Ok(None),
            Some(data) => data,
        },
    };
    let unit_id = data.unit_id.as_str();
    match check_unit(fn_name, user_id, roles, unit_id, only_owner, state).await? {
        None => Ok(None),
        Some(_) => Ok(Some(data)),
    }
}

fn data_list_transform(list: &Vec<DlDataBuffer>) -> Vec<response::GetDlDataBufferData> {
    let mut ret = vec![];
    for data in list.iter() {
        ret.push(data_transform(&data));
    }
    ret
}

fn data_list_transform_bytes(
    list: &Vec<DlDataBuffer>,
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
        let json_str = match serde_json::to_string(&data_transform(item)) {
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

fn data_transform(data: &DlDataBuffer) -> response::GetDlDataBufferData {
    response::GetDlDataBufferData {
        data_id: data.data_id.clone(),
        unit_id: data.unit_id.clone(),
        application_id: data.application_id.clone(),
        application_code: data.application_code.clone(),
        device_id: data.device_id.clone(),
        network_id: data.network_id.clone(),
        network_addr: data.network_addr.clone(),
        created_at: time_str(&data.created_at),
        expired_at: time_str(&data.expired_at),
    }
}
