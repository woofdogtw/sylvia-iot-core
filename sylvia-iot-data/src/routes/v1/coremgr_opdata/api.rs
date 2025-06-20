use std::error::Error as StdError;

use axum::{
    Extension,
    body::{Body, Bytes},
    extract::State,
    http::{HeaderValue, header},
    response::{IntoResponse, Response},
};
use chrono::{TimeZone, Utc};
use csv::WriterBuilder;
use log::error;
use serde_json;

use sylvia_iot_corelib::{
    constants::ContentType,
    err::ErrResp,
    http::{Json, Query},
    role::Role,
    strings,
};

use super::{
    super::{
        super::{ErrReq, State as AppState, middleware::GetTokenInfoData},
        get_user_inner,
    },
    request, response,
};
use crate::models::coremgr_opdata::{CoremgrOpData, ListOptions, ListQueryCond, SortCond, SortKey};

const LIST_LIMIT_DEFAULT: u64 = 100;
const LIST_CURSOR_MAX: u64 = 100;
const CSV_FIELDS: &'static str =
    "dataId,reqTime,resTime,latencyMs,status,method,path,body,userId,clientId,errCode,errMessage\n";

/// `GET /{base}/api/v1/coremgr-opdata/count`
pub async fn get_count(
    State(state): State<AppState>,
    Extension(token_info): Extension<GetTokenInfoData>,
    Query(query): Query<request::GetCountQuery>,
) -> impl IntoResponse {
    const FN_NAME: &'static str = "get_count";

    let user_cond = get_user_cond(FN_NAME, &token_info, query.user.as_ref(), &state).await?;
    let cond = match get_list_cond(&query, &user_cond).await {
        Err(e) => return Err(e.into_response()),
        Ok(cond) => cond,
    };
    match state.model.coremgr_opdata().count(&cond).await {
        Err(e) => {
            error!("[{}] count error: {}", FN_NAME, e);
            Err(ErrResp::ErrDb(Some(e.to_string())).into_response())
        }
        Ok(count) => Ok(Json(response::GetCount {
            data: response::GetCountData { count },
        })),
    }
}

/// `GET /{base}/api/v1/coremgr-opdata/list`
pub async fn get_list(
    State(state): State<AppState>,
    Extension(token_info): Extension<GetTokenInfoData>,
    Query(query): Query<request::GetListQuery>,
) -> impl IntoResponse {
    const FN_NAME: &'static str = "get_list";

    let cond_query = request::GetCountQuery {
        user: query.user.clone(),
        tfield: query.tfield.clone(),
        tstart: query.tstart,
        tend: query.tend,
    };
    let user_cond = match get_user_cond(FN_NAME, &token_info, query.user.as_ref(), &state).await {
        Err(e) => return Ok(e),
        Ok(cond) => cond,
    };
    let cond = match get_list_cond(&cond_query, &user_cond).await {
        Err(e) => return Err(e),
        Ok(cond) => cond,
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
            Some(limit) => Some(limit),
        },
        sort: Some(sort_cond.as_slice()),
        cursor_max: Some(LIST_CURSOR_MAX),
    };

    let (list, cursor) = match state.model.coremgr_opdata().list(&opts, None).await {
        Err(e) => {
            error!("[{}] list error: {}", FN_NAME, e);
            return Err(ErrResp::ErrDb(Some(e.to_string())));
        }
        Ok((list, cursor)) => match cursor {
            None => match query.format.as_ref() {
                Some(request::ListFormat::Array) => {
                    return Ok(Json(list_transform(&list)).into_response());
                }
                Some(request::ListFormat::Csv) => {
                    let bytes = match list_transform_bytes(&list, true, true, query.format.as_ref())
                    {
                        Err(e) => {
                            let e = format!("transform CSV error: {}", e);
                            return Err(ErrResp::ErrUnknown(Some(e)));
                        }
                        Ok(bytes) => bytes,
                    };
                    return Ok((
                        [
                            (header::CONTENT_TYPE, ContentType::CSV),
                            (
                                header::CONTENT_DISPOSITION,
                                "attachment;filename=coremgr-opdata.csv",
                            ),
                        ],
                        bytes,
                    )
                        .into_response());
                }
                _ => {
                    return Ok(Json(response::GetList {
                        data: list_transform(&list),
                    })
                    .into_response());
                }
            },
            Some(_) => (list, cursor),
        },
    };

    let query_format = query.format.clone();
    let body = Body::from_stream(async_stream::stream! {
        let cond_query = request::GetCountQuery {
            user: query.user.clone(),
            tfield: query.tfield.clone(),
            tstart: query.tstart,
            tend: query.tend,
        };
        let cond = match get_list_cond(&cond_query, &user_cond).await {
            Err(_) => return,
            Ok(cond) => cond,
        };
        let opts = ListOptions {
            cond: &cond,
            offset: query.offset,
            limit: match query.limit {
                None => Some(LIST_LIMIT_DEFAULT),
                Some(limit) => Some(limit),
            },
            sort: Some(sort_cond.as_slice()),
            cursor_max: Some(LIST_CURSOR_MAX),
        };

        let mut list = list;
        let mut cursor = cursor;
        let mut is_first = true;
        loop {
            yield list_transform_bytes(&list, is_first, cursor.is_none(), query.format.as_ref());
            is_first = false;
            if cursor.is_none() {
                break;
            }
            let (_list, _cursor) = match state.model.coremgr_opdata().list(&opts, cursor).await {
                Err(_) => break,
                Ok((list, cursor)) => (list, cursor),
            };
            list = _list;
            cursor = _cursor;
        }
    });
    match query_format {
        Some(request::ListFormat::Csv) => Ok((
            [
                (header::CONTENT_TYPE, ContentType::CSV),
                (
                    header::CONTENT_DISPOSITION,
                    "attachment;filename=coremgr-opdata.csv",
                ),
            ],
            body,
        )
            .into_response()),
        _ => Ok(([(header::CONTENT_TYPE, ContentType::JSON)], body).into_response()),
    }
}

async fn get_list_cond<'a>(
    query: &'a request::GetCountQuery,
    user_id: &'a Option<String>,
) -> Result<ListQueryCond<'a>, ErrResp> {
    let mut cond = ListQueryCond {
        user_id: match user_id.as_ref() {
            None => None,
            Some(user_id) => Some(user_id.as_str()),
        },
        ..Default::default()
    };
    if let Some(start) = query.tstart.as_ref() {
        match query.tfield.as_ref() {
            None => return Err(ErrResp::ErrParam(Some("missing `tfield`".to_string()))),
            Some(tfield) => match tfield.as_str() {
                "req" => cond.req_gte = Some(Utc.timestamp_nanos(*start * 1000000)),
                "res" => cond.res_gte = Some(Utc.timestamp_nanos(*start * 1000000)),
                _ => return Err(ErrResp::ErrParam(Some("invalid `tfield`".to_string()))),
            },
        }
    }
    if let Some(end) = query.tend.as_ref() {
        match query.tfield.as_ref() {
            None => return Err(ErrResp::ErrParam(Some("missing `tfield`".to_string()))),
            Some(tfield) => match tfield.as_str() {
                "req" => cond.req_lte = Some(Utc.timestamp_nanos(*end * 1000000)),
                "res" => cond.res_lte = Some(Utc.timestamp_nanos(*end * 1000000)),
                _ => return Err(ErrResp::ErrParam(Some("invalid `tfield`".to_string()))),
            },
        }
    }

    Ok(cond)
}

async fn get_user_cond(
    fn_name: &str,
    token_info: &GetTokenInfoData,
    query_user: Option<&String>,
    state: &AppState,
) -> Result<Option<String>, Response> {
    if !Role::is_role(&token_info.roles, Role::ADMIN)
        && !Role::is_role(&token_info.roles, Role::MANAGER)
    {
        return Ok(Some(token_info.user_id.clone()));
    }
    let auth_base = state.auth_base.as_str();
    let client = state.client.clone();

    match query_user {
        None => Ok(None),
        Some(user_id) => match user_id.len() {
            0 => Ok(None),
            _ => {
                let token =
                    match HeaderValue::from_str(format!("Bearer {}", token_info.token).as_str()) {
                        Err(e) => {
                            error!("[{}] get token error: {}", fn_name, e);
                            return Err(ErrResp::ErrRsc(Some(format!("get token error: {}", e)))
                                .into_response());
                        }
                        Ok(value) => value,
                    };
                match get_user_inner(fn_name, &client, auth_base, user_id, &token).await {
                    Err(e) => {
                        error!("[{}] get unit error", fn_name);
                        return Err(e);
                    }
                    Ok(unit) => match unit {
                        None => {
                            return Err(ErrResp::Custom(
                                ErrReq::USER_NOT_EXIST.0,
                                ErrReq::USER_NOT_EXIST.1,
                                None,
                            )
                            .into_response());
                        }
                        Some(_) => Ok(Some(user_id.clone())),
                    },
                }
            }
        },
    }
}

fn get_sort_cond(sort_args: &Option<String>) -> Result<Vec<SortCond>, ErrResp> {
    match sort_args.as_ref() {
        None => Ok(vec![SortCond {
            key: SortKey::ReqTime,
            asc: false,
        }]),
        Some(args) => {
            let mut args = args.split(",");
            let mut sort_cond = vec![];
            while let Some(arg) = args.next() {
                let mut cond = arg.split(":");
                let key = match cond.next() {
                    None => return Err(ErrResp::ErrParam(Some("wrong sort argument".to_string()))),
                    Some(field) => match field {
                        "req" => SortKey::ReqTime,
                        "res" => SortKey::ResTime,
                        "latency" => SortKey::Latency,
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

fn list_transform(list: &Vec<CoremgrOpData>) -> Vec<response::GetListData> {
    let mut ret = vec![];
    for item in list.iter() {
        ret.push(data_transform(&item));
    }
    ret
}

fn list_transform_bytes(
    list: &Vec<CoremgrOpData>,
    with_start: bool,
    with_end: bool,
    format: Option<&request::ListFormat>,
) -> Result<Bytes, Box<dyn StdError + Send + Sync>> {
    let mut build_str = match with_start {
        false => "".to_string(),
        true => match format {
            Some(request::ListFormat::Array) => "[".to_string(),
            Some(request::ListFormat::Csv) => {
                let bom = String::from_utf8(vec![0xEF, 0xBB, 0xBF])?;
                format!("{}{}", bom, CSV_FIELDS)
            }
            _ => "{\"data\":[".to_string(),
        },
    };
    let mut is_first = with_start;

    for item in list {
        match format {
            Some(request::ListFormat::Csv) => {
                let mut writer = WriterBuilder::new().has_headers(false).from_writer(vec![]);
                writer.serialize(data_transform_csv(item))?;
                build_str += String::from_utf8(writer.into_inner()?)?.as_str();
            }
            _ => {
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
        }
    }

    if with_end {
        build_str += match format {
            Some(request::ListFormat::Array) => "]",
            Some(request::ListFormat::Csv) => "",
            _ => "]}",
        }
    }
    Ok(Bytes::copy_from_slice(build_str.as_str().as_bytes()))
}

fn data_transform(data: &CoremgrOpData) -> response::GetListData {
    response::GetListData {
        data_id: data.data_id.clone(),
        req_time: strings::time_str(&data.req_time),
        res_time: strings::time_str(&data.res_time),
        latency_ms: data.latency_ms,
        status: data.status,
        source_ip: data.source_ip.clone(),
        method: data.method.clone(),
        path: data.path.clone(),
        body: data.body.clone(),
        user_id: data.user_id.clone(),
        client_id: data.client_id.clone(),
        err_code: data.err_code.clone(),
        err_message: data.err_message.clone(),
    }
}

fn data_transform_csv(data: &CoremgrOpData) -> response::GetListCsvData {
    response::GetListCsvData {
        data_id: data.data_id.clone(),
        req_time: strings::time_str(&data.req_time),
        res_time: strings::time_str(&data.res_time),
        latency_ms: data.latency_ms,
        status: data.status,
        source_ip: data.source_ip.clone(),
        method: data.method.clone(),
        path: data.path.clone(),
        body: match data.body.as_ref() {
            None => "".to_string(),
            Some(body) => match serde_json::to_string(body) {
                Err(_) => "".to_string(),
                Ok(body) => body,
            },
        },
        user_id: data.user_id.clone(),
        client_id: data.client_id.clone(),
        err_code: match data.err_code.as_ref() {
            None => "".to_string(),
            Some(err_code) => err_code.clone(),
        },
        err_message: match data.err_message.as_ref() {
            None => "".to_string(),
            Some(err_message) => err_message.clone(),
        },
    }
}
