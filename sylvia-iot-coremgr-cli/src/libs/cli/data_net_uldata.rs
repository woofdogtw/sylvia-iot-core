use std::{error::Error as StdError, fs};

use chrono::DateTime;
use clap::{Arg, ArgMatches, Command, builder::RangedU64ValueParser};
use reqwest::{Client, Method, StatusCode, header};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use serde_urlencoded;

use sylvia_iot_corelib::err::ErrResp;

use super::{API_RETRY, Config, config, get_csv_filename, refresh, validate_timestr};

#[derive(Serialize)]
struct GetCountReq<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    unit: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    device: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    network: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    addr: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tfield: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tstart: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tend: Option<i64>,
}

#[derive(Deserialize)]
struct GetCountRes {
    data: GetCountResData,
}

#[derive(Deserialize, Serialize)]
struct GetCountResData {
    count: usize,
}

#[derive(Serialize)]
struct GetListReq<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    offset: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    limit: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    sort: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    format: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    unit: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    device: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    network: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    addr: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tfield: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tstart: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tend: Option<i64>,
}

#[derive(Deserialize)]
struct GetListRes {
    data: Vec<GetResData>,
}

#[derive(Deserialize, Serialize)]
struct GetResData {
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
    data: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    extension: Option<Map<String, Value>>,
}

pub fn reg_args(cmd: Command) -> Command {
    cmd.about("Network uplink data")
        .subcommand(
            Command::new("count")
                .about("Get data count")
                .arg(
                    Arg::new("unit")
                        .long("unit")
                        .help("The specified unit ID")
                        .num_args(1),
                )
                .arg(
                    Arg::new("device")
                        .long("device")
                        .help("The specified device ID")
                        .num_args(1),
                )
                .arg(
                    Arg::new("network")
                        .long("network")
                        .help("The specified network code")
                        .num_args(1),
                )
                .arg(
                    Arg::new("addr")
                        .long("addr")
                        .help("The specified network code")
                        .num_args(1),
                )
                .arg(
                    Arg::new("tfield")
                        .long("tfield")
                        .help("Time field to filter data")
                        .num_args(1..)
                        .value_parser(["proc", "time"]),
                )
                .arg(
                    Arg::new("tstart")
                        .long("tstart")
                        .help("Start time (RFC3339)")
                        .num_args(1)
                        .value_parser(validate_timestr),
                )
                .arg(
                    Arg::new("tend")
                        .long("tend")
                        .help("End time (RFC3339)")
                        .num_args(1)
                        .value_parser(validate_timestr),
                ),
        )
        .subcommand(
            Command::new("list")
                .about("Get data list")
                .arg(
                    Arg::new("offset")
                        .short('o')
                        .long("offset")
                        .help("Item offset")
                        .num_args(1)
                        .value_parser(RangedU64ValueParser::<u64>::new().range(0..)),
                )
                .arg(
                    Arg::new("limit")
                        .short('l')
                        .long("limit")
                        .help("Items of a page")
                        .num_args(1)
                        .value_parser(RangedU64ValueParser::<u64>::new().range(0..)),
                )
                .arg(
                    Arg::new("sort")
                        .short('s')
                        .long("sort")
                        .help("Sort conditions")
                        .num_args(1..)
                        .value_parser([
                            "proc:asc",
                            "proc:desc",
                            "time:asc",
                            "time:desc",
                            "network:asc",
                            "network:desc",
                            "addr:asc",
                            "addr:desc",
                        ]),
                )
                .arg(
                    Arg::new("format")
                        .long("format")
                        .help("List format (default is array in `data` field)")
                        .num_args(1)
                        .value_parser(["array", "csv"]),
                )
                .arg(
                    Arg::new("unit")
                        .long("unit")
                        .help("The specified unit ID")
                        .num_args(1),
                )
                .arg(
                    Arg::new("device")
                        .long("device")
                        .help("The specified device ID")
                        .num_args(1),
                )
                .arg(
                    Arg::new("network")
                        .long("network")
                        .help("The specified network code")
                        .num_args(1),
                )
                .arg(
                    Arg::new("addr")
                        .long("addr")
                        .help("The specified network code")
                        .num_args(1),
                )
                .arg(
                    Arg::new("tfield")
                        .long("tfield")
                        .help("Time field to filter data")
                        .num_args(1..)
                        .value_parser(["proc", "time"]),
                )
                .arg(
                    Arg::new("tstart")
                        .long("tstart")
                        .help("Start time (RFC3339)")
                        .num_args(1)
                        .value_parser(validate_timestr),
                )
                .arg(
                    Arg::new("tend")
                        .long("tend")
                        .help("End time (RFC3339)")
                        .num_args(1)
                        .value_parser(validate_timestr),
                ),
        )
}

pub async fn run(conf: &Config, args: &ArgMatches) -> Result<Option<()>, Box<dyn StdError>> {
    match args.subcommand() {
        Some(("count", args)) => {
            let data = count(conf, args).await?;
            println!("{}", serde_json::to_string_pretty(&data)?);
            Ok(Some(()))
        }
        Some(("list", args)) => {
            let data = list(conf, args).await?;
            println!("{}", serde_json::to_string_pretty(&data)?);
            Ok(Some(()))
        }
        _ => Ok(None),
    }
}

async fn count(config: &Config, args: &ArgMatches) -> Result<GetCountResData, ErrResp> {
    let storage = match config::read_storage() {
        Err(e) => {
            let msg = format!("[storage] read storage error: {}", e);
            return Err(ErrResp::ErrRsc(Some(msg)));
        }
        Ok(storage) => storage,
    };

    let query = GetCountReq {
        unit: match args.get_one::<String>("unit") {
            None => None,
            Some(v) => Some(v.as_str()),
        },
        device: match args.get_one::<String>("device") {
            None => None,
            Some(v) => Some(v.as_str()),
        },
        network: match args.get_one::<String>("network") {
            None => None,
            Some(v) => Some(v.as_str()),
        },
        addr: match args.get_one::<String>("addr") {
            None => None,
            Some(v) => Some(v.as_str()),
        },
        tfield: match args.get_one::<String>("tfield") {
            None => None,
            Some(v) => Some(v.as_str()),
        },
        tstart: match args.get_one::<String>("tstart") {
            None => None,
            Some(v) => match DateTime::parse_from_rfc3339(v.as_str()) {
                Err(_) => None,
                Ok(v) => Some(v.timestamp_millis()),
            },
        },
        tend: match args.get_one::<String>("tend") {
            None => None,
            Some(v) => match DateTime::parse_from_rfc3339(v.as_str()) {
                Err(_) => None,
                Ok(v) => Some(v.timestamp_millis()),
            },
        },
    };
    let client = Client::new();
    let uri = match serde_urlencoded::to_string(&query) {
        Err(e) => {
            let msg = format!("[query] cannot encode query string: {}", e);
            return Err(ErrResp::ErrRsc(Some(msg)));
        }
        Ok(query) => format!("{}/api/v1/network-uldata/count?{}", config.data, query),
    };
    let mut token = storage.access_token;

    let mut retry = 0;
    loop {
        let req = match client
            .request(Method::GET, uri.as_str())
            .header(header::AUTHORIZATION, format!("Bearer {}", token))
            .build()
        {
            Err(e) => {
                let msg = format!("[API] create request error: {}", e);
                return Err(ErrResp::ErrRsc(Some(msg)));
            }
            Ok(req) => req,
        };
        let resp = match client.execute(req).await {
            Err(e) => {
                let msg = format!("[API] execute request error: {}", e);
                return Err(ErrResp::ErrRsc(Some(msg)));
            }
            Ok(resp) => resp,
        };
        let status_code = resp.status();
        let body = match resp.bytes().await {
            Err(e) => format!("(wrong body: {})", e),
            Ok(bytes) => match String::from_utf8(bytes.to_vec()) {
                Err(e) => format!("(body not UTF-8: {})", e),
                Ok(body) => body,
            },
        };
        if status_code != StatusCode::OK {
            retry += 1;
            if retry <= API_RETRY && status_code == StatusCode::UNAUTHORIZED {
                token = refresh(config).await?.access_token;
                continue;
            }
            let msg = format!("[API] unexpected status: {}, body: {}", status_code, body);
            return Err(ErrResp::ErrIntMsg(Some(msg)));
        }
        match serde_json::from_str::<GetCountRes>(body.as_str()) {
            Err(e) => {
                let msg = format!("[API] unexpected token error: {}, body: {}", e, body);
                return Err(ErrResp::ErrIntMsg(Some(msg)));
            }
            Ok(res) => return Ok(res.data),
        }
    }
}

async fn list(config: &Config, args: &ArgMatches) -> Result<Vec<GetResData>, ErrResp> {
    let storage = match config::read_storage() {
        Err(e) => {
            let msg = format!("[storage] read storage error: {}", e);
            return Err(ErrResp::ErrRsc(Some(msg)));
        }
        Ok(storage) => storage,
    };

    let query = GetListReq {
        offset: match args.get_one::<u64>("offset") {
            None => None,
            Some(v) => Some(*v),
        },
        limit: match args.get_one::<u64>("limit") {
            None => None,
            Some(v) => Some(*v),
        },
        sort: match args.get_many::<String>("sort") {
            None => None,
            Some(v) => {
                let values: Vec<String> = v.map(|x| x.clone()).collect();
                Some(values.join(","))
            }
        },
        format: match args.get_one::<String>("format") {
            None => None,
            Some(v) => Some(v.as_str()),
        },
        unit: match args.get_one::<String>("unit") {
            None => None,
            Some(v) => Some(v.as_str()),
        },
        device: match args.get_one::<String>("device") {
            None => None,
            Some(v) => Some(v.as_str()),
        },
        network: match args.get_one::<String>("network") {
            None => None,
            Some(v) => Some(v.as_str()),
        },
        addr: match args.get_one::<String>("addr") {
            None => None,
            Some(v) => Some(v.as_str()),
        },
        tfield: match args.get_one::<String>("tfield") {
            None => None,
            Some(v) => Some(v.as_str()),
        },
        tstart: match args.get_one::<String>("tstart") {
            None => None,
            Some(v) => match DateTime::parse_from_rfc3339(v.as_str()) {
                Err(_) => None,
                Ok(v) => Some(v.timestamp_millis()),
            },
        },
        tend: match args.get_one::<String>("tend") {
            None => None,
            Some(v) => match DateTime::parse_from_rfc3339(v.as_str()) {
                Err(_) => None,
                Ok(v) => Some(v.timestamp_millis()),
            },
        },
    };
    let client = Client::new();
    let uri = match serde_urlencoded::to_string(&query) {
        Err(e) => {
            let msg = format!("[query] cannot encode query string: {}", e);
            return Err(ErrResp::ErrRsc(Some(msg)));
        }
        Ok(query) => format!("{}/api/v1/network-uldata/list?{}", config.data, query),
    };
    let mut token = storage.access_token;

    let mut retry = 0;
    loop {
        let req = match client
            .request(Method::GET, uri.as_str())
            .header(header::AUTHORIZATION, format!("Bearer {}", token))
            .build()
        {
            Err(e) => {
                let msg = format!("[API] create request error: {}", e);
                return Err(ErrResp::ErrRsc(Some(msg)));
            }
            Ok(req) => req,
        };
        let resp = match client.execute(req).await {
            Err(e) => {
                let msg = format!("[API] execute request error: {}", e);
                return Err(ErrResp::ErrRsc(Some(msg)));
            }
            Ok(resp) => resp,
        };
        let status_code = resp.status();
        let csv_filename = get_csv_filename(&resp);
        let body = match resp.bytes().await {
            Err(e) => format!("(wrong body: {})", e),
            Ok(bytes) => match String::from_utf8(bytes.to_vec()) {
                Err(e) => format!("(body not UTF-8: {})", e),
                Ok(body) => body,
            },
        };
        if status_code != StatusCode::OK {
            retry += 1;
            if retry <= API_RETRY && status_code == StatusCode::UNAUTHORIZED {
                token = refresh(config).await?.access_token;
                continue;
            }
            let msg = format!("[API] unexpected status: {}, body: {}", status_code, body);
            return Err(ErrResp::ErrIntMsg(Some(msg)));
        }
        match query.format {
            Some("array") => match serde_json::from_str::<Vec<GetResData>>(body.as_str()) {
                Err(e) => {
                    let msg = format!("[API] unexpected token error: {}, body: {}", e, body);
                    return Err(ErrResp::ErrIntMsg(Some(msg)));
                }
                Ok(data) => return Ok(data),
            },
            Some("csv") => {
                if csv_filename.len() > 0 {
                    match fs::write(csv_filename.as_str(), body) {
                        Err(e) => {
                            let msg = format!("[fs] write CSV file error: {}", e);
                            return Err(ErrResp::ErrRsc(Some(msg)));
                        }
                        Ok(()) => return Ok(vec![]),
                    }
                }
                return Ok(vec![]);
            }
            _ => match serde_json::from_str::<GetListRes>(body.as_str()) {
                Err(e) => {
                    let msg = format!("[API] unexpected token error: {}, body: {}", e, body);
                    return Err(ErrResp::ErrIntMsg(Some(msg)));
                }
                Ok(res) => return Ok(res.data),
            },
        }
    }
}
