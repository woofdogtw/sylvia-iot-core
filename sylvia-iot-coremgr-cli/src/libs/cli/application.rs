use std::{error::Error as StdError, fs};

use clap::{Arg, ArgMatches, Command, builder::RangedU64ValueParser};
use hex;
use reqwest::{Client, Method, StatusCode, header};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use serde_urlencoded;

use sylvia_iot_corelib::err::ErrResp;

use super::{API_RETRY, Config, config, get_csv_filename, refresh, validate_code, validate_json};

#[derive(Serialize)]
struct PostReq<'a> {
    data: PostReqData<'a>,
}

#[derive(Serialize)]
struct PostReqData<'a> {
    code: &'a str,
    #[serde(rename = "unitId")]
    unit_id: &'a str,
    #[serde(rename = "hostUri")]
    host_uri: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    info: Option<Map<String, Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    ttl: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    length: Option<usize>,
}

#[derive(Deserialize)]
struct PostRes {
    data: PostResData,
}

#[derive(Deserialize, Serialize)]
struct PostResData {
    #[serde(rename = "applicationId")]
    application_id: String,
    password: String,
}

#[derive(Serialize)]
struct GetCountReq<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    unit: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    code: Option<&'a str>,
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
    code: Option<&'a str>,
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
    #[serde(rename = "hostUri", skip_serializing_if = "Option::is_none")]
    host_uri: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    info: Option<Map<String, Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    ttl: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    length: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    password: Option<&'a str>,
}

#[derive(Deserialize, Serialize)]
struct GetResData {
    #[serde(rename = "applicationId")]
    application_id: String,
    code: String,
    #[serde(rename = "unitId")]
    unit_id: String,
    #[serde(rename = "unitCode")]
    unit_code: String,
    #[serde(rename = "createdAt")]
    created_at: String,
    #[serde(rename = "modifiedAt")]
    modified_at: String,
    #[serde(rename = "hostUri")]
    host_uri: String,
    name: String,
    info: Map<String, Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    ttl: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    length: Option<usize>,
}

#[derive(Deserialize, Serialize)]
struct GetStatsRes {
    data: GetStatsResData,
}

#[derive(Deserialize, Serialize)]
struct GetStatsResData {
    uldata: Stats,
    #[serde(rename = "dldataResp")]
    dldata_resp: Stats,
    #[serde(rename = "dldataResult")]
    dldata_result: Stats,
}

#[derive(Deserialize, Serialize)]
struct Stats {
    consumers: usize,
    messages: usize,
    #[serde(rename = "publishRate")]
    pub publish_rate: f64,
    #[serde(rename = "deliverRate")]
    pub deliver_rate: f64,
}

#[derive(Serialize)]
struct PostDlDataReq<'a> {
    data: PostDlDataReqData<'a>,
}

#[derive(Serialize)]
struct PostDlDataReqData<'a> {
    #[serde(rename = "deviceId")]
    device_id: &'a str,
    payload: &'a str,
}

pub fn reg_args(cmd: Command) -> Command {
    cmd.about("Application management")
        .subcommand(
            Command::new("add")
                .about("Add application")
                .arg(
                    Arg::new("code")
                        .short('c')
                        .long("code")
                        .help("Application code. Format: [A-Za-z0-9]{1}[A-Za-z0-9-_]*")
                        .required(true)
                        .num_args(1)
                        .value_parser(validate_code),
                )
                .arg(
                    Arg::new("unitid")
                        .short('u')
                        .long("unitid")
                        .help("Unit ID")
                        .required(true)
                        .num_args(1),
                )
                .arg(
                    Arg::new("host")
                        .long("host")
                        .help("Host URI [scheme://host]")
                        .required(true)
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
                )
                .arg(
                    Arg::new("ttl")
                        .long("ttl")
                        .help("Message TTL in milliseconds (0 is unlimited)")
                        .num_args(1)
                        .value_parser(
                            RangedU64ValueParser::<u64>::new().range(0..=(usize::MAX as u64)),
                        ),
                )
                .arg(
                    Arg::new("length")
                        .long("length")
                        .help("Queue length (0 is unlimited)")
                        .num_args(1)
                        .value_parser(
                            RangedU64ValueParser::<u64>::new().range(0..=(usize::MAX as u64)),
                        ),
                ),
        )
        .subcommand(
            Command::new("count")
                .about("Get application count")
                .arg(
                    Arg::new("unitid")
                        .long("unitid")
                        .help("The specified unit ID")
                        .num_args(1),
                )
                .arg(
                    Arg::new("code")
                        .long("code")
                        .help("The specified application code")
                        .num_args(1),
                )
                .arg(
                    Arg::new("contains")
                        .long("contains")
                        .help("The partial word of application code (case insensitive)")
                        .num_args(1),
                ),
        )
        .subcommand(
            Command::new("list")
                .about("Get application list")
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
                            "code:asc",
                            "code:desc",
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
                    Arg::new("code")
                        .long("code")
                        .help("The specified application code")
                        .num_args(1),
                )
                .arg(
                    Arg::new("contains")
                        .long("contains")
                        .help("The partial word of application code (case insensitive)")
                        .num_args(1),
                ),
        )
        .subcommand(
            Command::new("get")
                .about("Get application detail information")
                .arg(
                    Arg::new("appid")
                        .short('i')
                        .long("appid")
                        .help("Application ID")
                        .required(true)
                        .num_args(1),
                ),
        )
        .subcommand(
            Command::new("update")
                .about("Update application information")
                .arg(
                    Arg::new("appid")
                        .short('i')
                        .long("appid")
                        .help("Application ID")
                        .required(true)
                        .num_args(1),
                )
                .arg(
                    Arg::new("host")
                        .long("host")
                        .help("Host URI (scheme://host)")
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
                )
                .arg(
                    Arg::new("ttl")
                        .long("ttl")
                        .help("Message TTL in milliseconds (0 is unlimited)")
                        .num_args(1)
                        .value_parser(
                            RangedU64ValueParser::<u64>::new().range(0..=(usize::MAX as u64)),
                        ),
                )
                .arg(
                    Arg::new("length")
                        .long("length")
                        .help("Queue length (0 is unlimited)")
                        .num_args(1)
                        .value_parser(
                            RangedU64ValueParser::<u64>::new().range(0..=(usize::MAX as u64)),
                        ),
                )
                .arg(
                    Arg::new("password")
                        .short('p')
                        .long("password")
                        .help("Password to connect to the broker (required when changing `hostUri`")
                        .num_args(1),
                ),
        )
        .subcommand(
            Command::new("delete").about("Delete an application").arg(
                Arg::new("appid")
                    .short('i')
                    .long("appid")
                    .help("Application ID")
                    .required(true)
                    .num_args(1),
            ),
        )
        .subcommand(
            Command::new("stats")
                .about("Get application queue statistics")
                .arg(
                    Arg::new("appid")
                        .short('i')
                        .long("appid")
                        .help("Application ID")
                        .required(true)
                        .num_args(1),
                ),
        )
        .subcommand(
            Command::new("dldata")
                .about("Send downlink data to a device")
                .arg(
                    Arg::new("appid")
                        .short('i')
                        .long("appid")
                        .help("Application ID")
                        .required(true)
                        .num_args(1),
                )
                .arg(
                    Arg::new("devid")
                        .short('d')
                        .long("devid")
                        .help("The target device ID")
                        .required(true)
                        .num_args(1),
                )
                .arg(
                    Arg::new("payload")
                        .short('p')
                        .long("payload")
                        .help("Payload")
                        .required(true)
                        .num_args(1),
                )
                .arg(
                    Arg::new("type")
                        .short('t')
                        .long("type")
                        .help("Payload type")
                        .default_value("string")
                        .num_args(1)
                        .value_parser(["hex", "string"]),
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
        Some(("stats", args)) => {
            let data = stats(conf, args).await?;
            println!("{}", serde_json::to_string_pretty(&data)?);
            Ok(Some(()))
        }
        Some(("dldata", args)) => {
            dldata(conf, args).await?;
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
            code: args.get_one::<String>("code").unwrap().as_str(),
            unit_id: args.get_one::<String>("unitid").unwrap().as_str(),
            host_uri: args.get_one::<String>("host").unwrap().as_str(),
            name: match args.get_one::<String>("name") {
                None => None,
                Some(v) => Some(v.as_str()),
            },
            info: match args.get_one::<String>("info") {
                None => None,
                Some(v) => Some(serde_json::from_str::<Map<String, Value>>(v.as_str()).unwrap()),
            },
            ttl: match args.get_one::<u64>("ttl") {
                None => None,
                Some(v) => Some(*v as usize),
            },
            length: match args.get_one::<u64>("length") {
                None => None,
                Some(v) => Some(*v as usize),
            },
        },
    };
    let client = Client::new();
    let uri = format!("{}/api/v1/application", config.coremgr);
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
        code: match args.get_one::<String>("code") {
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
        Ok(query) => format!("{}/api/v1/application/count?{}", config.coremgr, query),
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
        code: match args.get_one::<String>("code") {
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
        Ok(query) => format!("{}/api/v1/application/list?{}", config.coremgr, query),
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
        "{}/api/v1/application/{}",
        config.coremgr,
        args.get_one::<String>("appid").unwrap()
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
            host_uri: match args.get_one::<String>("host") {
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
            ttl: match args.get_one::<u64>("ttl") {
                None => None,
                Some(v) => Some(*v as usize),
            },
            length: match args.get_one::<u64>("length") {
                None => None,
                Some(v) => Some(*v as usize),
            },
            password: match args.get_one::<String>("password") {
                None => None,
                Some(v) => Some(v.as_str()),
            },
        },
    };
    let client = Client::new();
    let uri = format!(
        "{}/api/v1/application/{}",
        config.coremgr,
        args.get_one::<String>("appid").unwrap()
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
        "{}/api/v1/application/{}",
        config.coremgr,
        args.get_one::<String>("appid").unwrap()
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

async fn stats(config: &Config, args: &ArgMatches) -> Result<GetStatsResData, ErrResp> {
    let storage = match config::read_storage() {
        Err(e) => {
            let msg = format!("[storage] read storage error: {}", e);
            return Err(ErrResp::ErrRsc(Some(msg)));
        }
        Ok(storage) => storage,
    };

    let client = Client::new();
    let uri = format!(
        "{}/api/v1/application/{}/stats",
        config.coremgr,
        args.get_one::<String>("appid").unwrap()
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
        match serde_json::from_str::<GetStatsRes>(body.as_str()) {
            Err(e) => {
                let msg = format!("[API] unexpected token error: {}, body: {}", e, body);
                return Err(ErrResp::ErrIntMsg(Some(msg)));
            }
            Ok(res) => return Ok(res.data),
        }
    }
}

async fn dldata(config: &Config, args: &ArgMatches) -> Result<(), ErrResp> {
    let storage = match config::read_storage() {
        Err(e) => {
            let msg = format!("[storage] read storage error: {}", e);
            return Err(ErrResp::ErrRsc(Some(msg)));
        }
        Ok(storage) => storage,
    };

    let payload = match args.get_one::<String>("type").unwrap().as_str() {
        "hex" => args.get_one::<String>("payload").unwrap().clone(),
        _ => hex::encode(args.get_one::<String>("payload").unwrap().as_bytes()),
    };
    let body = PostDlDataReq {
        data: PostDlDataReqData {
            device_id: args.get_one::<String>("devid").unwrap().as_str(),
            payload: payload.as_str(),
        },
    };
    let client = Client::new();
    let uri = format!(
        "{}/api/v1/application/{}/dldata",
        config.coremgr,
        args.get_one::<String>("appid").unwrap()
    );
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
