use std::error::Error as StdError;

use axum::{
    Extension,
    body::{Body, Bytes},
    extract::State,
    http::header,
    response::IntoResponse,
};
use chrono::{TimeZone, Utc};
use csv::WriterBuilder;
use log::error;
use serde_json;

use sylvia_iot_corelib::{
    constants::ContentType,
    err::ErrResp,
    http::{Json, Query},
    strings,
};

use super::{
    super::{
        super::{State as AppState, middleware::GetTokenInfoData},
        get_unit_cond,
    },
    request, response,
};
use crate::models::application_dldata::{
    ApplicationDlData, ListOptions, ListQueryCond, SortCond, SortKey,
};

const LIST_LIMIT_DEFAULT: u64 = 100;
const LIST_CURSOR_MAX: u64 = 100;
const CSV_FIELDS: &'static str =
    "dataId,proc,resp,status,unitId,deviceId,networkCode,networkAddr,profile,data,extension\n";

/// `GET /{base}/api/v1/application-dldata/count`
pub async fn get_count(
    State(state): State<AppState>,
    Extension(token_info): Extension<GetTokenInfoData>,
    Query(mut query): Query<request::GetCountQuery>,
) -> impl IntoResponse {
    const FN_NAME: &'static str = "get_count";

    if let Some(network) = query.network {
        query.network = Some(network.to_lowercase());
    }
    if let Some(addr) = query.addr {
        query.addr = Some(addr.to_lowercase());
    }

    let unit_cond = get_unit_cond(FN_NAME, &token_info, query.unit.as_ref(), &state).await?;
    let cond = match get_list_cond(&query, &unit_cond).await {
        Err(e) => return Err(e.into_response()),
        Ok(cond) => cond,
    };
    match state.model.application_dldata().count(&cond).await {
        Err(e) => {
            error!("[{}] count error: {}", FN_NAME, e);
            Err(ErrResp::ErrDb(Some(e.to_string())).into_response())
        }
        Ok(count) => Ok(Json(response::GetCount {
            data: response::GetCountData { count },
        })),
    }
}

/// `GET /{base}/api/v1/application-dldata/list`
pub async fn get_list(
    State(state): State<AppState>,
    Extension(token_info): Extension<GetTokenInfoData>,
    Query(query): Query<request::GetListQuery>,
) -> impl IntoResponse {
    const FN_NAME: &'static str = "get_list";

    let cond_query = request::GetCountQuery {
        unit: query.unit.clone(),
        device: query.device.clone(),
        network: match query.network.as_ref() {
            None => None,
            Some(network) => Some(network.to_lowercase()),
        },
        addr: match query.addr.as_ref() {
            None => None,
            Some(addr) => Some(addr.to_lowercase()),
        },
        profile: query.profile.clone(),
        tfield: query.tfield.clone(),
        tstart: query.tstart,
        tend: query.tend,
    };
    let unit_cond = match get_unit_cond(FN_NAME, &token_info, query.unit.as_ref(), &state).await {
        Err(e) => return Ok(e),
        Ok(cond) => cond,
    };
    let cond = match get_list_cond(&cond_query, &unit_cond).await {
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

    let (list, cursor) = match state.model.application_dldata().list(&opts, None).await {
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
                                "attachment;filename=application-dldata.csv",
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
            unit: query.unit.clone(),
            device: query.device.clone(),
            network: query.network.clone(),
            addr: query.addr.clone(),
            profile: query.profile.clone(),
            tfield: query.tfield.clone(),
            tstart: query.tstart,
            tend: query.tend,
        };
        let cond = match get_list_cond(&cond_query, &unit_cond).await {
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
            let (_list, _cursor) = match state.model.application_dldata().list(&opts, cursor).await {
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
                    "attachment;filename=application-dldata.csv",
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
    unit_cond: &'a Option<String>,
) -> Result<ListQueryCond<'a>, ErrResp> {
    let mut cond = ListQueryCond {
        unit_id: match unit_cond.as_ref() {
            None => None,
            Some(unit_id) => Some(unit_id.as_str()),
        },
        ..Default::default()
    };
    if let Some(device_id) = query.device.as_ref() {
        if device_id.len() > 0 {
            cond.device_id = Some(device_id.as_str());
        }
    }
    if let Some(network_code) = query.network.as_ref() {
        if network_code.len() > 0 {
            cond.network_code = Some(network_code.as_str());
        }
    }
    if let Some(network_addr) = query.addr.as_ref() {
        if network_addr.len() > 0 {
            cond.network_addr = Some(network_addr.as_str());
        }
    }
    if let Some(profile) = query.profile.as_ref() {
        if profile.len() > 0 {
            cond.profile = Some(profile.as_str());
        }
    }
    if let Some(start) = query.tstart.as_ref() {
        match query.tfield.as_ref() {
            None => return Err(ErrResp::ErrParam(Some("missing `tfield`".to_string()))),
            Some(tfield) => match tfield.as_str() {
                "proc" => cond.proc_gte = Some(Utc.timestamp_nanos(*start * 1000000)),
                "resp" => cond.resp_gte = Some(Utc.timestamp_nanos(*start * 1000000)),
                _ => return Err(ErrResp::ErrParam(Some("invalid `tfield`".to_string()))),
            },
        }
    }
    if let Some(end) = query.tend.as_ref() {
        match query.tfield.as_ref() {
            None => return Err(ErrResp::ErrParam(Some("missing `tfield`".to_string()))),
            Some(tfield) => match tfield.as_str() {
                "proc" => cond.proc_lte = Some(Utc.timestamp_nanos(*end * 1000000)),
                "resp" => cond.resp_lte = Some(Utc.timestamp_nanos(*end * 1000000)),
                _ => return Err(ErrResp::ErrParam(Some("invalid `tfield`".to_string()))),
            },
        }
    }

    Ok(cond)
}

fn get_sort_cond(sort_args: &Option<String>) -> Result<Vec<SortCond>, ErrResp> {
    match sort_args.as_ref() {
        None => Ok(vec![SortCond {
            key: SortKey::Proc,
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
                        "proc" => SortKey::Proc,
                        "resp" => SortKey::Resp,
                        "network" => SortKey::NetworkCode,
                        "addr" => SortKey::NetworkAddr,
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

fn list_transform(list: &Vec<ApplicationDlData>) -> Vec<response::GetListData> {
    let mut ret = vec![];
    for item in list.iter() {
        ret.push(data_transform(&item));
    }
    ret
}

fn list_transform_bytes(
    list: &Vec<ApplicationDlData>,
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

fn data_transform(data: &ApplicationDlData) -> response::GetListData {
    response::GetListData {
        data_id: data.data_id.clone(),
        proc: strings::time_str(&data.proc),
        resp: match data.resp {
            None => None,
            Some(resp) => Some(strings::time_str(&resp)),
        },
        status: data.status,
        unit_id: data.unit_id.clone(),
        device_id: data.device_id.clone(),
        network_code: data.network_code.clone(),
        network_addr: data.network_addr.clone(),
        profile: data.profile.clone(),
        data: data.data.clone(),
        extension: data.extension.clone(),
    }
}

fn data_transform_csv(data: &ApplicationDlData) -> response::GetListCsvData {
    response::GetListCsvData {
        data_id: data.data_id.clone(),
        proc: strings::time_str(&data.proc),
        resp: match data.resp {
            None => "".to_string(),
            Some(resp) => strings::time_str(&resp),
        },
        status: data.status,
        unit_id: data.unit_id.clone(),
        device_id: match data.device_id.as_ref() {
            None => "".to_string(),
            Some(device_id) => device_id.clone(),
        },
        network_code: match data.network_code.as_ref() {
            None => "".to_string(),
            Some(network_code) => network_code.clone(),
        },
        network_addr: match data.network_addr.as_ref() {
            None => "".to_string(),
            Some(network_addr) => network_addr.clone(),
        },
        profile: data.profile.clone(),
        data: data.data.clone(),
        extension: match data.extension.as_ref() {
            None => "".to_string(),
            Some(extension) => match serde_json::to_string(extension) {
                Err(_) => "".to_string(),
                Ok(extension) => extension,
            },
        },
    }
}
