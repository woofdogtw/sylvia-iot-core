use std::error::Error as StdError;

use actix_web::{
    http::header::{self, HeaderValue},
    web::{self, Bytes},
    HttpMessage, HttpRequest, HttpResponse, Responder, ResponseError,
};
use chrono::{TimeZone, Utc};
use csv::WriterBuilder;
use log::error;
use serde_json;

use sylvia_iot_corelib::{err::ErrResp, role::Role, strings};

use super::{
    super::{
        super::{middleware::FullTokenInfo, ErrReq, State},
        get_unit_inner,
    },
    request, response,
};
use crate::models::network_dldata::{ListOptions, ListQueryCond, NetworkDlData, SortCond, SortKey};

const LIST_LIMIT_DEFAULT: u64 = 100;
const LIST_CURSOR_MAX: u64 = 100;
const CSV_FIELDS: &'static str =
    "dataId,proc,pub,resp,status,unitId,deviceId,networkCode,networkAddr,data,extension\n";

/// `GET /{base}/api/v1/network-dldata/count`
pub async fn get_count(
    req: HttpRequest,
    query: web::Query<request::GetCountQuery>,
    state: web::Data<State>,
) -> impl Responder {
    const FN_NAME: &'static str = "get_count";

    let mut query: request::GetCountQuery = (*query).clone();
    if let Some(network) = query.network {
        query.network = Some(network.to_lowercase());
    }
    if let Some(addr) = query.addr {
        query.addr = Some(addr.to_lowercase());
    }

    let unit_cond = match get_unit_cond(FN_NAME, &req, query.unit.as_ref(), &state).await {
        Err(e) => return e,
        Ok(cond) => cond,
    };
    let cond = match get_list_cond(&query, &unit_cond).await {
        Err(e) => return e.error_response(),
        Ok(cond) => cond,
    };
    match state.model.network_dldata().count(&cond).await {
        Err(e) => {
            error!("[{}] count error: {}", FN_NAME, e);
            ErrResp::ErrDb(Some(e.to_string())).error_response()
        }
        Ok(count) => HttpResponse::Ok().json(response::GetCount {
            data: response::GetCountData { count },
        }),
    }
}

/// `GET /{base}/api/v1/network-dldata/list`
pub async fn get_list(
    req: HttpRequest,
    query: web::Query<request::GetListQuery>,
    state: web::Data<State>,
) -> impl Responder {
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
        tfield: query.tfield.clone(),
        tstart: query.tstart,
        tend: query.tend,
    };
    let unit_cond = match get_unit_cond(FN_NAME, &req, query.unit.as_ref(), &state).await {
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

    let (list, cursor) = match state.model.network_dldata().list(&opts, None).await {
        Err(e) => {
            error!("[{}] list error: {}", FN_NAME, e);
            return Err(ErrResp::ErrDb(Some(e.to_string())));
        }
        Ok((list, cursor)) => match cursor {
            None => match query.format.as_ref() {
                Some(request::ListFormat::Array) => {
                    return Ok(HttpResponse::Ok().json(list_transform(&list)))
                }
                Some(request::ListFormat::Csv) => {
                    let bytes = match list_transform_bytes(&list, true, true, query.format.as_ref())
                    {
                        Err(e) => {
                            return Err(ErrResp::ErrUnknown(Some(format!(
                                "transform CSV error: {}",
                                e
                            ))))
                        }
                        Ok(bytes) => bytes,
                    };
                    return Ok(HttpResponse::Ok()
                        .insert_header((header::CONTENT_TYPE, "text/csv"))
                        .insert_header((
                            header::CONTENT_DISPOSITION,
                            "attachment;filename=network-dldata.csv",
                        ))
                        .body(bytes));
                }
                _ => {
                    return Ok(HttpResponse::Ok().json(response::GetList {
                        data: list_transform(&list),
                    }))
                }
            },
            Some(_) => (list, cursor),
        },
    };

    // TODO: detect client disconnect
    let query_format = query.format.clone();
    let stream = async_stream::stream! {
        let query = query.0.clone();
        let cond_query = request::GetCountQuery {
            unit: query.unit.clone(),
            device: query.device.clone(),
            network: query.network.clone(),
            addr: query.addr.clone(),
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
            let (_list, _cursor) = match state.model.network_dldata().list(&opts, cursor).await {
                Err(_) => break,
                Ok((list, cursor)) => (list, cursor),
            };
            list = _list;
            cursor = _cursor;
        }
    };
    match query_format {
        Some(request::ListFormat::Csv) => Ok(HttpResponse::Ok()
            .insert_header((header::CONTENT_TYPE, "text/csv"))
            .insert_header((
                header::CONTENT_DISPOSITION,
                "attachment;filename=network-dldata.csv",
            ))
            .streaming(stream)),
        _ => Ok(HttpResponse::Ok().streaming(stream)),
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
    if let Some(start) = query.tstart.as_ref() {
        match query.tfield.as_ref() {
            None => return Err(ErrResp::ErrParam(Some("missing `tfield`".to_string()))),
            Some(tfield) => match tfield.as_str() {
                "proc" => cond.proc_gte = Some(Utc.timestamp_nanos(*start * 1000000)),
                "pub" => cond.pub_gte = Some(Utc.timestamp_nanos(*start * 1000000)),
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
                "pub" => cond.pub_lte = Some(Utc.timestamp_nanos(*end * 1000000)),
                "resp" => cond.resp_lte = Some(Utc.timestamp_nanos(*end * 1000000)),
                _ => return Err(ErrResp::ErrParam(Some("invalid `tfield`".to_string()))),
            },
        }
    }

    Ok(cond)
}

async fn get_unit_cond(
    fn_name: &str,
    req: &HttpRequest,
    query_unit: Option<&String>,
    state: &web::Data<State>,
) -> Result<Option<String>, HttpResponse> {
    let token_info = match req.extensions_mut().get::<FullTokenInfo>() {
        None => {
            error!("[{}] token not found", fn_name);
            return Err(
                ErrResp::ErrUnknown(Some("token info not found".to_string())).error_response(),
            );
        }
        Some(token_info) => token_info.clone(),
    };
    let broker_base = state.broker_base.as_str();
    let client = state.client.clone();

    match query_unit {
        None => {
            if !Role::is_role(&token_info.info.roles, Role::ADMIN)
                && !Role::is_role(&token_info.info.roles, Role::MANAGER)
            {
                return Err(ErrResp::ErrParam(Some("missing `unit`".to_string())).error_response());
            }
            Ok(None)
        }
        Some(unit_id) => match unit_id.len() {
            0 => Ok(None),
            _ => {
                let token = match HeaderValue::from_str(token_info.token.as_str()) {
                    Err(e) => {
                        error!("[{}] get token error: {}", fn_name, e);
                        return Err(ErrResp::ErrUnknown(Some(format!("get token error: {}", e)))
                            .error_response());
                    }
                    Ok(value) => value,
                };
                match get_unit_inner(fn_name, &client, broker_base, unit_id, &token).await {
                    Err(e) => {
                        error!("[{}] get unit error", fn_name);
                        return Err(e);
                    }
                    Ok(unit) => match unit {
                        None => {
                            return Err(ErrResp::Custom(
                                ErrReq::UNIT_NOT_EXIST.0,
                                ErrReq::UNIT_NOT_EXIST.1,
                                None,
                            )
                            .error_response())
                        }
                        Some(_) => Ok(Some(unit_id.clone())),
                    },
                }
            }
        },
    }
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
                        "pub" => SortKey::Pub,
                        "resp" => SortKey::Resp,
                        "network" => SortKey::NetworkCode,
                        "addr" => SortKey::NetworkAddr,
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

fn list_transform(list: &Vec<NetworkDlData>) -> Vec<response::GetListData> {
    let mut ret = vec![];
    for item in list.iter() {
        ret.push(data_transform(&item));
    }
    ret
}

fn list_transform_bytes(
    list: &Vec<NetworkDlData>,
    with_start: bool,
    with_end: bool,
    format: Option<&request::ListFormat>,
) -> Result<Bytes, Box<dyn StdError>> {
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

fn data_transform(data: &NetworkDlData) -> response::GetListData {
    response::GetListData {
        data_id: data.data_id.clone(),
        proc: strings::time_str(&data.proc),
        publish: strings::time_str(&data.publish),
        resp: match data.resp {
            None => None,
            Some(resp) => Some(strings::time_str(&resp)),
        },
        status: data.status,
        unit_id: data.unit_id.clone(),
        device_id: data.device_id.clone(),
        network_code: data.network_code.clone(),
        network_addr: data.network_addr.clone(),
        data: data.data.clone(),
        extension: data.extension.clone(),
    }
}

fn data_transform_csv(data: &NetworkDlData) -> response::GetListCsvData {
    response::GetListCsvData {
        data_id: data.data_id.clone(),
        proc: strings::time_str(&data.proc),
        publish: strings::time_str(&data.publish),
        resp: match data.resp {
            None => "".to_string(),
            Some(resp) => strings::time_str(&resp),
        },
        status: data.status,
        unit_id: data.unit_id.clone(),
        device_id: data.device_id.clone(),
        network_code: data.network_code.clone(),
        network_addr: data.network_addr.clone(),
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
