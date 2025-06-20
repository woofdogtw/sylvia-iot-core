use std::{error::Error as StdError, fs};

use clap::{Arg, ArgMatches, Command, builder::RangedU64ValueParser};
use reqwest::{Client, Method, StatusCode, header};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use serde_urlencoded;

use sylvia_iot_corelib::err::ErrResp;

use super::{API_RETRY, Config, config, get_csv_filename, refresh, validate_json};

#[derive(Serialize)]
struct PostReq<'a> {
    data: PostReqData<'a>,
}

#[derive(Serialize)]
struct PostReqData<'a> {
    #[serde(rename = "unitId")]
    unit_id: &'a str,
    #[serde(rename = "networkId")]
    network_id: &'a str,
    #[serde(rename = "networkAddr")]
    network_addr: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    profile: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    info: Option<Map<String, Value>>,
}

#[derive(Deserialize)]
struct PostRes {
    data: PostResData,
}

#[derive(Deserialize, Serialize)]
struct PostResData {
    #[serde(rename = "deviceId")]
    device_id: String,
}

#[derive(Serialize)]
struct PostBulkReq<'a> {
    data: PostBulkReqData<'a>,
}

#[derive(Serialize)]
struct PostBulkReqData<'a> {
    #[serde(rename = "unitId")]
    unit_id: &'a str,
    #[serde(rename = "networkId")]
    network_id: &'a str,
    #[serde(rename = "networkAddrs")]
    network_addrs: Vec<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    profile: Option<&'a str>,
}

#[derive(Serialize)]
struct PostRangeReq<'a> {
    data: PostRangeReqData<'a>,
}

#[derive(Serialize)]
struct PostRangeReqData<'a> {
    #[serde(rename = "unitId")]
    unit_id: &'a str,
    #[serde(rename = "networkId")]
    network_id: &'a str,
    #[serde(rename = "startAddr")]
    start_addr: &'a str,
    #[serde(rename = "endAddr")]
    end_addr: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    profile: Option<&'a str>,
}

#[derive(Serialize)]
struct GetCountReq<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    unit: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    network: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    addr: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    profile: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    contains: Option<&'a str>,
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
    network: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    addr: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    profile: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    contains: Option<&'a str>,
}

#[derive(Deserialize)]
struct GetListRes {
    data: Vec<GetResData>,
}

#[derive(Deserialize)]
struct GetRes {
    data: GetResData,
}

#[derive(Serialize)]
struct PatchReq<'a> {
    data: PatchReqData<'a>,
}

#[derive(Serialize)]
struct PatchReqData<'a> {
    #[serde(rename = "networkId", skip_serializing_if = "Option::is_none")]
    network_id: Option<&'a str>,
    #[serde(rename = "networkAddr", skip_serializing_if = "Option::is_none")]
    network_addr: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    profile: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    info: Option<Map<String, Value>>,
}

#[derive(Deserialize, Serialize)]
struct GetResData {
    #[serde(rename = "deviceId")]
    device_id: String,
    #[serde(rename = "unitId")]
    unit_id: String,
    #[serde(rename = "unitCode")]
    unit_code: Option<String>,
    #[serde(rename = "networkId")]
    network_id: String,
    #[serde(rename = "networkCode")]
    network_code: String,
    #[serde(rename = "networkAddr")]
    network_addr: String,
    #[serde(rename = "createdAt")]
    created_at: String,
    #[serde(rename = "modifiedAt")]
    modified_at: String,
    profile: String,
    name: String,
    info: Map<String, Value>,
}

pub fn reg_args(cmd: Command) -> Command {
    cmd.about("Device management")
        .subcommand(
            Command::new("add")
                .about("Add device")
                .arg(
                    Arg::new("unitid")
                        .short('u')
                        .long("unitid")
                        .help("Unit ID")
                        .num_args(1)
                        .required(true),
                )
                .arg(
                    Arg::new("netid")
                        .long("netid")
                        .help("Network ID")
                        .num_args(1)
                        .required(true),
                )
                .arg(
                    Arg::new("address")
                        .short('a')
                        .long("address")
                        .help("Network address")
                        .num_args(1)
                        .required(true),
                )
                .arg(
                    Arg::new("profile")
                        .short('p')
                        .long("profile")
                        .help("Device profile")
                        .num_args(1),
                )
                .arg(
                    Arg::new("name")
                        .short('n')
                        .long("name")
                        .help("Display name")
                        .num_args(1),
                )
                .arg(
                    Arg::new("info")
                        .short('i')
                        .long("info")
                        .help("Information in JSON object format")
                        .num_args(1)
                        .value_parser(validate_json),
                ),
        )
        .subcommand(
            Command::new("add-bulk")
                .about("Add device in bulk")
                .arg(
                    Arg::new("unitid")
                        .short('u')
                        .long("unitid")
                        .help("Unit ID")
                        .num_args(1)
                        .required(true),
                )
                .arg(
                    Arg::new("netid")
                        .long("netid")
                        .help("Network ID")
                        .num_args(1)
                        .required(true),
                )
                .arg(
                    Arg::new("addresses")
                        .short('a')
                        .long("addresses")
                        .help("Network addresses")
                        .num_args(1..=1024)
                        .required(true),
                )
                .arg(
                    Arg::new("profile")
                        .short('p')
                        .long("profile")
                        .help("Device profile")
                        .num_args(1),
                ),
        )
        .subcommand(
            Command::new("add-range")
                .about("Add device in range")
                .arg(
                    Arg::new("unitid")
                        .short('u')
                        .long("unitid")
                        .help("Unit ID")
                        .num_args(1)
                        .required(true),
                )
                .arg(
                    Arg::new("netid")
                        .long("netid")
                        .help("Network ID")
                        .num_args(1)
                        .required(true),
                )
                .arg(
                    Arg::new("start")
                        .short('s')
                        .long("start")
                        .help("Start network address")
                        .required(true),
                )
                .arg(
                    Arg::new("end")
                        .short('e')
                        .long("end")
                        .help("End network address")
                        .required(true),
                )
                .arg(
                    Arg::new("profile")
                        .short('p')
                        .long("profile")
                        .help("Device profile")
                        .num_args(1),
                ),
        )
        .subcommand(
            Command::new("count")
                .about("Get device count")
                .arg(
                    Arg::new("unitid")
                        .long("unitid")
                        .help("The specified unit ID")
                        .num_args(1),
                )
                .arg(
                    Arg::new("netid")
                        .long("netid")
                        .help("The specified network ID")
                        .num_args(1),
                )
                .arg(
                    Arg::new("address")
                        .long("address")
                        .help("The specified network address")
                        .num_args(1),
                )
                .arg(
                    Arg::new("profile")
                        .short('p')
                        .long("profile")
                        .help("Device profile")
                        .num_args(1),
                )
                .arg(
                    Arg::new("contains")
                        .long("contains")
                        .help("The partial word of device name (case insensitive)")
                        .num_args(1),
                ),
        )
        .subcommand(
            Command::new("list")
                .about("Get device list")
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
                            "network:asc",
                            "network:desc",
                            "addr:asc",
                            "addr:desc",
                            "created:asc",
                            "created:desc",
                            "modified:asc",
                            "modified:desc",
                            "name:asc",
                            "name:desc",
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
                    Arg::new("unitid")
                        .long("unitid")
                        .help("The specified unit ID")
                        .num_args(1),
                )
                .arg(
                    Arg::new("netid")
                        .long("netid")
                        .help("The specified network ID")
                        .num_args(1),
                )
                .arg(
                    Arg::new("address")
                        .long("address")
                        .help("The specified network address")
                        .num_args(1),
                )
                .arg(
                    Arg::new("profile")
                        .short('p')
                        .long("profile")
                        .help("Device profile")
                        .num_args(1),
                )
                .arg(
                    Arg::new("contains")
                        .long("contains")
                        .help("The partial word of device name (case insensitive)")
                        .num_args(1),
                ),
        )
        .subcommand(
            Command::new("get")
                .about("Get device detail information")
                .arg(
                    Arg::new("devid")
                        .short('i')
                        .long("devid")
                        .help("Device ID")
                        .num_args(1)
                        .required(true),
                ),
        )
        .subcommand(
            Command::new("update")
                .about("Update device information")
                .arg(
                    Arg::new("devid")
                        .short('i')
                        .long("devid")
                        .help("Device ID")
                        .num_args(1)
                        .required(true),
                )
                .arg(
                    Arg::new("netid")
                        .long("netid")
                        .help("Network ID")
                        .num_args(1),
                )
                .arg(
                    Arg::new("address")
                        .short('a')
                        .long("address")
                        .help("Network address")
                        .num_args(1),
                )
                .arg(
                    Arg::new("profile")
                        .short('p')
                        .long("profile")
                        .help("Device profile")
                        .num_args(1),
                )
                .arg(
                    Arg::new("name")
                        .short('n')
                        .long("name")
                        .help("Display name")
                        .num_args(1),
                )
                .arg(
                    Arg::new("info")
                        .long("info")
                        .help("Information in JSON object format")
                        .num_args(1)
                        .value_parser(validate_json),
                ),
        )
        .subcommand(
            Command::new("delete").about("Delete a device").arg(
                Arg::new("devid")
                    .short('i')
                    .long("devid")
                    .help("Device ID")
                    .num_args(1)
                    .required(true),
            ),
        )
        .subcommand(
            Command::new("delete-bulk")
                .about("Delete device in bulk")
                .arg(
                    Arg::new("unitid")
                        .short('u')
                        .long("unitid")
                        .help("Unit ID")
                        .num_args(1)
                        .required(true),
                )
                .arg(
                    Arg::new("netid")
                        .long("netid")
                        .help("Network ID")
                        .num_args(1)
                        .required(true),
                )
                .arg(
                    Arg::new("addresses")
                        .short('a')
                        .long("addresses")
                        .help("Network addresses")
                        .num_args(1..=1024)
                        .required(true),
                ),
        )
        .subcommand(
            Command::new("delete-range")
                .about("Delete device in range")
                .arg(
                    Arg::new("unitid")
                        .short('u')
                        .long("unitid")
                        .help("Unit ID")
                        .num_args(1)
                        .required(true),
                )
                .arg(
                    Arg::new("netid")
                        .long("netid")
                        .help("Network ID")
                        .num_args(1)
                        .required(true),
                )
                .arg(
                    Arg::new("start")
                        .short('s')
                        .long("start")
                        .help("Start network address")
                        .required(true),
                )
                .arg(
                    Arg::new("end")
                        .short('e')
                        .long("end")
                        .help("End network address")
                        .required(true),
                ),
        )
}

pub async fn run(conf: &Config, args: &ArgMatches) -> Result<Option<()>, Box<dyn StdError>> {
    match args.subcommand() {
        Some(("add", args)) => {
            let data = add(conf, args).await?;
            println!("{}", serde_json::to_string_pretty(&data)?);
            Ok(Some(()))
        }
        Some(("add-bulk", args)) => {
            add_bulk(conf, args).await?;
            Ok(Some(()))
        }
        Some(("add-range", args)) => {
            add_range(conf, args).await?;
            Ok(Some(()))
        }
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
        Some(("get", args)) => {
            let data = get(conf, args).await?;
            println!("{}", serde_json::to_string_pretty(&data)?);
            Ok(Some(()))
        }
        Some(("update", args)) => {
            update(conf, args).await?;
            Ok(Some(()))
        }
        Some(("delete", args)) => {
            delete(conf, args).await?;
            Ok(Some(()))
        }
        Some(("delete-bulk", args)) => {
            delete_bulk(conf, args).await?;
            Ok(Some(()))
        }
        Some(("delete-range", args)) => {
            delete_range(conf, args).await?;
            Ok(Some(()))
        }
        _ => Ok(None),
    }
}

async fn add(config: &Config, args: &ArgMatches) -> Result<PostResData, ErrResp> {
    let storage = match config::read_storage() {
        Err(e) => {
            let msg = format!("[storage] read storage error: {}", e);
            return Err(ErrResp::ErrRsc(Some(msg)));
        }
        Ok(storage) => storage,
    };

    let body = PostReq {
        data: PostReqData {
            unit_id: args.get_one::<String>("unitid").unwrap().as_str(),
            network_id: args.get_one::<String>("netid").unwrap().as_str(),
            network_addr: args.get_one::<String>("address").unwrap().as_str(),
            profile: match args.get_one::<String>("profile") {
                None => None,
                Some(v) => Some(v.as_str()),
            },
            name: match args.get_one::<String>("name") {
                None => None,
                Some(v) => Some(v.as_str()),
            },
            info: match args.get_one::<String>("info") {
                None => None,
                Some(v) => Some(serde_json::from_str::<Map<String, Value>>(v.as_str()).unwrap()),
            },
        },
    };
    let client = Client::new();
    let uri = format!("{}/api/v1/device", config.coremgr);
    let mut token = storage.access_token;

    let mut retry = 0;
    loop {
        let req = match client
            .request(Method::POST, uri.as_str())
            .header(header::AUTHORIZATION, format!("Bearer {}", token))
            .json(&body)
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
        match serde_json::from_str::<PostRes>(body.as_str()) {
            Err(e) => {
                let msg = format!("[API] unexpected token error: {}, body: {}", e, body);
                return Err(ErrResp::ErrIntMsg(Some(msg)));
            }
            Ok(res) => return Ok(res.data),
        }
    }
}

async fn add_bulk(config: &Config, args: &ArgMatches) -> Result<(), ErrResp> {
    let storage = match config::read_storage() {
        Err(e) => {
            let msg = format!("[storage] read storage error: {}", e);
            return Err(ErrResp::ErrRsc(Some(msg)));
        }
        Ok(storage) => storage,
    };

    let body = PostBulkReq {
        data: PostBulkReqData {
            unit_id: args.get_one::<String>("unitid").unwrap().as_str(),
            network_id: args.get_one::<String>("netid").unwrap().as_str(),
            network_addrs: args
                .get_many::<String>("addresses")
                .unwrap()
                .map(|x| x.as_str())
                .collect(),
            profile: match args.get_one::<String>("profile") {
                None => None,
                Some(v) => Some(v.as_str()),
            },
        },
    };
    let client = Client::new();
    let uri = format!("{}/api/v1/device/bulk", config.coremgr);
    let mut token = storage.access_token;

    let mut retry = 0;
    loop {
        let req = match client
            .request(Method::POST, uri.as_str())
            .header(header::AUTHORIZATION, format!("Bearer {}", token))
            .json(&body)
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
        if status_code != StatusCode::NO_CONTENT {
            retry += 1;
            if retry <= API_RETRY && status_code == StatusCode::UNAUTHORIZED {
                token = refresh(config).await?.access_token;
                continue;
            }
            let body = match resp.bytes().await {
                Err(e) => format!("(wrong body: {})", e),
                Ok(bytes) => match String::from_utf8(bytes.to_vec()) {
                    Err(e) => format!("(body not UTF-8: {})", e),
                    Ok(body) => body,
                },
            };
            let msg = format!("[API] unexpected status: {}, body: {}", status_code, body);
            return Err(ErrResp::ErrIntMsg(Some(msg)));
        }
        return Ok(());
    }
}

async fn add_range(config: &Config, args: &ArgMatches) -> Result<(), ErrResp> {
    let storage = match config::read_storage() {
        Err(e) => {
            let msg = format!("[storage] read storage error: {}", e);
            return Err(ErrResp::ErrRsc(Some(msg)));
        }
        Ok(storage) => storage,
    };

    let body = PostRangeReq {
        data: PostRangeReqData {
            unit_id: args.get_one::<String>("unitid").unwrap().as_str(),
            network_id: args.get_one::<String>("netid").unwrap().as_str(),
            start_addr: args.get_one::<String>("start").unwrap().as_str(),
            end_addr: args.get_one::<String>("end").unwrap().as_str(),
            profile: match args.get_one::<String>("profile") {
                None => None,
                Some(v) => Some(v.as_str()),
            },
        },
    };
    let client = Client::new();
    let uri = format!("{}/api/v1/device/range", config.coremgr);
    let mut token = storage.access_token;

    let mut retry = 0;
    loop {
        let req = match client
            .request(Method::POST, uri.as_str())
            .header(header::AUTHORIZATION, format!("Bearer {}", token))
            .json(&body)
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
        if status_code != StatusCode::NO_CONTENT {
            retry += 1;
            if retry <= API_RETRY && status_code == StatusCode::UNAUTHORIZED {
                token = refresh(config).await?.access_token;
                continue;
            }
            let body = match resp.bytes().await {
                Err(e) => format!("(wrong body: {})", e),
                Ok(bytes) => match String::from_utf8(bytes.to_vec()) {
                    Err(e) => format!("(body not UTF-8: {})", e),
                    Ok(body) => body,
                },
            };
            let msg = format!("[API] unexpected status: {}, body: {}", status_code, body);
            return Err(ErrResp::ErrIntMsg(Some(msg)));
        }
        return Ok(());
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
        unit: match args.get_one::<String>("unitid") {
            None => None,
            Some(v) => Some(v.as_str()),
        },
        network: match args.get_one::<String>("netid") {
            None => None,
            Some(v) => Some(v.as_str()),
        },
        addr: match args.get_one::<String>("address") {
            None => None,
            Some(v) => Some(v.as_str()),
        },
        profile: match args.get_one::<String>("profile") {
            None => None,
            Some(v) => Some(v.as_str()),
        },
        contains: match args.get_one::<String>("contains") {
            None => None,
            Some(v) => Some(v.as_str()),
        },
    };
    let client = Client::new();
    let uri = match serde_urlencoded::to_string(&query) {
        Err(e) => {
            let msg = format!("[query] cannot encode query string: {}", e);
            return Err(ErrResp::ErrRsc(Some(msg)));
        }
        Ok(query) => format!("{}/api/v1/device/count?{}", config.coremgr, query),
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
        unit: match args.get_one::<String>("unitid") {
            None => None,
            Some(v) => Some(v.as_str()),
        },
        network: match args.get_one::<String>("netid") {
            None => None,
            Some(v) => Some(v.as_str()),
        },
        addr: match args.get_one::<String>("address") {
            None => None,
            Some(v) => Some(v.as_str()),
        },
        profile: match args.get_one::<String>("profile") {
            None => None,
            Some(v) => Some(v.as_str()),
        },
        contains: match args.get_one::<String>("contains") {
            None => None,
            Some(v) => Some(v.as_str()),
        },
    };
    let client = Client::new();
    let uri = match serde_urlencoded::to_string(&query) {
        Err(e) => {
            let msg = format!("[query] cannot encode query string: {}", e);
            return Err(ErrResp::ErrRsc(Some(msg)));
        }
        Ok(query) => format!("{}/api/v1/device/list?{}", config.coremgr, query),
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

async fn get(config: &Config, args: &ArgMatches) -> Result<GetResData, ErrResp> {
    let storage = match config::read_storage() {
        Err(e) => {
            let msg = format!("[storage] read storage error: {}", e);
            return Err(ErrResp::ErrRsc(Some(msg)));
        }
        Ok(storage) => storage,
    };

    let client = Client::new();
    let uri = format!(
        "{}/api/v1/device/{}",
        config.coremgr,
        args.get_one::<String>("devid").unwrap()
    );
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
        match serde_json::from_str::<GetRes>(body.as_str()) {
            Err(e) => {
                let msg = format!("[API] unexpected token error: {}, body: {}", e, body);
                return Err(ErrResp::ErrIntMsg(Some(msg)));
            }
            Ok(res) => return Ok(res.data),
        }
    }
}

async fn update(config: &Config, args: &ArgMatches) -> Result<(), ErrResp> {
    let storage = match config::read_storage() {
        Err(e) => {
            let msg = format!("[storage] read storage error: {}", e);
            return Err(ErrResp::ErrRsc(Some(msg)));
        }
        Ok(storage) => storage,
    };

    let body = PatchReq {
        data: PatchReqData {
            network_id: match args.get_one::<String>("netid") {
                None => None,
                Some(v) => Some(v.as_str()),
            },
            network_addr: match args.get_one::<String>("address") {
                None => None,
                Some(v) => Some(v.as_str()),
            },
            profile: match args.get_one::<String>("profile") {
                None => None,
                Some(v) => Some(v.as_str()),
            },
            name: match args.get_one::<String>("name") {
                None => None,
                Some(v) => Some(v.as_str()),
            },
            info: match args.get_one::<String>("info") {
                None => None,
                Some(v) => Some(serde_json::from_str::<Map<String, Value>>(v.as_str()).unwrap()),
            },
        },
    };
    let client = Client::new();
    let uri = format!(
        "{}/api/v1/device/{}",
        config.coremgr,
        args.get_one::<String>("devid").unwrap()
    );
    let mut token = storage.access_token;

    let mut retry = 0;
    loop {
        let req = match client
            .request(Method::PATCH, uri.as_str())
            .header(header::AUTHORIZATION, format!("Bearer {}", token))
            .json(&body)
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
        if status_code != StatusCode::NO_CONTENT {
            retry += 1;
            if retry <= API_RETRY && status_code == StatusCode::UNAUTHORIZED {
                token = refresh(config).await?.access_token;
                continue;
            }
            let msg = format!("[API] unexpected status: {}, body: {}", status_code, body);
            return Err(ErrResp::ErrIntMsg(Some(msg)));
        }
        return Ok(());
    }
}

async fn delete(config: &Config, args: &ArgMatches) -> Result<(), ErrResp> {
    let storage = match config::read_storage() {
        Err(e) => {
            let msg = format!("[storage] read storage error: {}", e);
            return Err(ErrResp::ErrRsc(Some(msg)));
        }
        Ok(storage) => storage,
    };

    let client = Client::new();
    let uri = format!(
        "{}/api/v1/device/{}",
        config.coremgr,
        args.get_one::<String>("devid").unwrap()
    );
    let mut token = storage.access_token;

    let mut retry = 0;
    loop {
        let req = match client
            .request(Method::DELETE, uri.as_str())
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
        if status_code != StatusCode::NO_CONTENT {
            retry += 1;
            if retry <= API_RETRY && status_code == StatusCode::UNAUTHORIZED {
                token = refresh(config).await?.access_token;
                continue;
            }
            let msg = format!("[API] unexpected status: {}, body: {}", status_code, body);
            return Err(ErrResp::ErrIntMsg(Some(msg)));
        }
        return Ok(());
    }
}

async fn delete_bulk(config: &Config, args: &ArgMatches) -> Result<(), ErrResp> {
    let storage = match config::read_storage() {
        Err(e) => {
            let msg = format!("[storage] read storage error: {}", e);
            return Err(ErrResp::ErrRsc(Some(msg)));
        }
        Ok(storage) => storage,
    };

    let body = PostBulkReq {
        data: PostBulkReqData {
            unit_id: args.get_one::<String>("unitid").unwrap().as_str(),
            network_id: args.get_one::<String>("netid").unwrap().as_str(),
            network_addrs: args
                .get_many::<String>("addresses")
                .unwrap()
                .map(|x| x.as_str())
                .collect(),
            profile: None,
        },
    };
    let client = Client::new();
    let uri = format!("{}/api/v1/device/bulk-delete", config.coremgr);
    let mut token = storage.access_token;

    let mut retry = 0;
    loop {
        let req = match client
            .request(Method::POST, uri.as_str())
            .header(header::AUTHORIZATION, format!("Bearer {}", token))
            .json(&body)
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
        if status_code != StatusCode::NO_CONTENT {
            retry += 1;
            if retry <= API_RETRY && status_code == StatusCode::UNAUTHORIZED {
                token = refresh(config).await?.access_token;
                continue;
            }
            let body = match resp.bytes().await {
                Err(e) => format!("(wrong body: {})", e),
                Ok(bytes) => match String::from_utf8(bytes.to_vec()) {
                    Err(e) => format!("(body not UTF-8: {})", e),
                    Ok(body) => body,
                },
            };
            let msg = format!("[API] unexpected status: {}, body: {}", status_code, body);
            return Err(ErrResp::ErrIntMsg(Some(msg)));
        }
        return Ok(());
    }
}

async fn delete_range(config: &Config, args: &ArgMatches) -> Result<(), ErrResp> {
    let storage = match config::read_storage() {
        Err(e) => {
            let msg = format!("[storage] read storage error: {}", e);
            return Err(ErrResp::ErrRsc(Some(msg)));
        }
        Ok(storage) => storage,
    };

    let body = PostRangeReq {
        data: PostRangeReqData {
            unit_id: args.get_one::<String>("unitid").unwrap().as_str(),
            network_id: args.get_one::<String>("netid").unwrap().as_str(),
            start_addr: args.get_one::<String>("start").unwrap().as_str(),
            end_addr: args.get_one::<String>("end").unwrap().as_str(),
            profile: None,
        },
    };
    let client = Client::new();
    let uri = format!("{}/api/v1/device/range-delete", config.coremgr);
    let mut token = storage.access_token;

    let mut retry = 0;
    loop {
        let req = match client
            .request(Method::POST, uri.as_str())
            .header(header::AUTHORIZATION, format!("Bearer {}", token))
            .json(&body)
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
        if status_code != StatusCode::NO_CONTENT {
            retry += 1;
            if retry <= API_RETRY && status_code == StatusCode::UNAUTHORIZED {
                token = refresh(config).await?.access_token;
                continue;
            }
            let body = match resp.bytes().await {
                Err(e) => format!("(wrong body: {})", e),
                Ok(bytes) => match String::from_utf8(bytes.to_vec()) {
                    Err(e) => format!("(body not UTF-8: {})", e),
                    Ok(body) => body,
                },
            };
            let msg = format!("[API] unexpected status: {}, body: {}", status_code, body);
            return Err(ErrResp::ErrIntMsg(Some(msg)));
        }
        return Ok(());
    }
}
