use std::{collections::HashMap, error::Error as StdError, fs};

use clap::{
    builder::{BoolValueParser, RangedU64ValueParser},
    Arg, ArgMatches, Command,
};
use reqwest::{header, Client, Method, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use serde_urlencoded;

use sylvia_iot_corelib::err::ErrResp;

use super::{
    config, get_csv_filename, refresh, validate_json, validate_timestr, Config, API_RETRY,
};

#[derive(Serialize)]
struct PostReq<'a> {
    data: PostReqData<'a>,
    #[serde(rename = "expiredAt", skip_serializing_if = "Option::is_none")]
    expired_at: Option<&'a str>,
}

#[derive(Serialize)]
struct PostReqData<'a> {
    account: &'a str,
    password: &'a str,
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
    #[serde(rename = "userId")]
    user_id: String,
}

#[derive(Serialize)]
struct GetCountReq<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    account: Option<&'a str>,
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
    fields: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    sort: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    format: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    account: Option<&'a str>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<PatchReqData<'a>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    disable: Option<bool>,
}

#[derive(Serialize)]
struct PatchReqData<'a> {
    #[serde(rename = "verifiedAt", skip_serializing_if = "Option::is_none")]
    verified_at: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    roles: Option<HashMap<String, bool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    password: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    info: Option<Map<String, Value>>,
}

#[derive(Deserialize, Serialize)]
struct GetResData {
    #[serde(rename = "userId", skip_serializing_if = "Option::is_none")]
    user_id: Option<String>,
    account: String,
    #[serde(rename = "createdAt")]
    created_at: String,
    #[serde(rename = "modifiedAt")]
    modified_at: String,
    #[serde(rename = "verifiedAt")]
    verified_at: Option<String>,
    #[serde(rename = "expiredAt", skip_serializing_if = "Option::is_none")]
    expired_at: Option<Value>,
    #[serde(rename = "disabledAt", skip_serializing_if = "Option::is_none")]
    disabled_at: Option<Value>,
    roles: Map<String, Value>,
    name: String,
    info: Map<String, Value>,
}

pub fn reg_args(cmd: Command) -> Command {
    cmd.about("User management")
        .subcommand(
            Command::new("add")
                .about("Add user")
                .arg(
                    Arg::new("account")
                        .short('a')
                        .long("account")
                        .help("Account name")
                        .num_args(1)
                        .required(true),
                )
                .arg(
                    Arg::new("password")
                        .short('p')
                        .long("password")
                        .help("Password")
                        .num_args(1)
                        .required(true),
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
                    Arg::new("expired")
                        .short('x')
                        .long("expired")
                        .help("Expired time (RFC3339/ISO8601)")
                        .num_args(1)
                        .value_parser(validate_timestr),
                ),
        )
        .subcommand(
            Command::new("count")
                .about("Get user count")
                .arg(
                    Arg::new("account")
                        .long("account")
                        .help("The specified account name (case insensitive)")
                        .num_args(1),
                )
                .arg(
                    Arg::new("contains")
                        .long("contains")
                        .help("The partial word of account name (case insensitive)")
                        .num_args(1),
                ),
        )
        .subcommand(
            Command::new("list")
                .about("Get user list")
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
                            "account:asc",
                            "account:desc",
                            "created:asc",
                            "created:desc",
                            "modified:asc",
                            "modified:desc",
                            "verified:asc",
                            "verified:desc",
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
                    Arg::new("account")
                        .long("account")
                        .help("The specified account name (case insensitive)")
                        .num_args(1),
                )
                .arg(
                    Arg::new("contains")
                        .long("contains")
                        .help("The partial word of account name (case insensitive)")
                        .num_args(1),
                )
                .arg(
                    Arg::new("fields")
                        .long("fields")
                        .help("Specify more fields to display")
                        .num_args(1..)
                        .value_parser(["expired", "disabled"]),
                ),
        )
        .subcommand(
            Command::new("get")
                .about("Get user detail information")
                .arg(
                    Arg::new("userid")
                        .short('i')
                        .long("userid")
                        .help("User ID (only useful for administrators and managers)")
                        .num_args(1),
                ),
        )
        .subcommand(
            Command::new("update")
                .about("Update user information")
                .arg(
                    Arg::new("userid")
                        .short('i')
                        .long("userid")
                        .help("User ID (only useful for administrators and managers)")
                        .num_args(1),
                )
                .arg(
                    Arg::new("verified")
                        .short('v')
                        .long("verified")
                        .help("Verified time (only useful for administrators and managers if `userid` is provided)")
                        .num_args(1)
                        .value_parser(validate_timestr),
                )
                .arg(
                    Arg::new("roles")
                        .short('r')
                        .long("roles")
                        .help("Set enabled roles")
                        .num_args(0..),
                )
                .arg(
                    Arg::new("password")
                        .short('p')
                        .long("password")
                        .help("Modify password (only available for administrators if `userid` is provided)")
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
                    Arg::new("disable")
                        .long("disable")
                        .help("Disable the user (only available for administrators if `userid` is provided)")
                        .num_args(1)
                        .value_parser(BoolValueParser::new()),
                ),
        )
        .subcommand(
            Command::new("delete")
                .about("Delete a user (only useful for administrators)")
                .arg(
                    Arg::new("userid")
                        .short('i')
                        .long("userid")
                        .help("User ID")
                        .num_args(1)
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
            account: args.get_one::<String>("account").unwrap().as_str(),
            password: args.get_one::<String>("password").unwrap().as_str(),
            name: match args.get_one::<String>("name") {
                None => None,
                Some(v) => Some(v.as_str()),
            },
            info: match args.get_one::<String>("info") {
                None => None,
                Some(v) => Some(serde_json::from_str::<Map<String, Value>>(v.as_str()).unwrap()),
            },
        },
        expired_at: match args.get_one::<String>("expired") {
            None => None,
            Some(v) => Some(v.as_str()),
        },
    };
    let client = Client::new();
    let uri = format!("{}/api/v1/user", config.coremgr);
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
        account: match args.get_one::<String>("account") {
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
        Ok(query) => format!("{}/api/v1/user/count?{}", config.coremgr, query),
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
        fields: match args.get_many::<String>("fields") {
            None => None,
            Some(v) => {
                let values: Vec<String> = v.map(|x| x.clone()).collect();
                Some(values.join(","))
            }
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
        account: match args.get_one::<String>("account") {
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
        Ok(query) => format!("{}/api/v1/user/list?{}", config.coremgr, query),
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
    let uri = match args.get_one::<String>("userid") {
        None => format!("{}/api/v1/user", config.coremgr),
        Some(id) => format!("{}/api/v1/user/{}", config.coremgr, id),
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

    let mut body = PatchReq {
        data: None,
        disable: None,
    };
    if args.get_one::<String>("verified").is_some()
        || args.get_many::<String>("roles").is_some()
        || args.get_one::<String>("password").is_some()
        || args.get_one::<String>("name").is_some()
        || args.get_one::<String>("info").is_some()
    {
        body.data = Some(PatchReqData {
            verified_at: match args.get_one::<String>("verified") {
                None => None,
                Some(v) => Some(v.as_str()),
            },
            roles: match args.get_many::<String>("roles") {
                None => None,
                Some(v) => Some(v.map(|x| (x.to_string(), true)).collect()),
            },
            password: match args.get_one::<String>("password") {
                None => None,
                Some(v) => Some(v.as_str()),
            },
            name: match args.get_one::<String>("name") {
                None => None,
                Some(v) => Some(v.as_str()),
            },
            info: match args.get_one::<String>("info") {
                None => None,
                Some(v) => Some(serde_json::from_str::<Map<String, Value>>(v).unwrap()),
            },
        });
    }
    if let Some(v) = args.get_one::<bool>("disable") {
        body.disable = Some(*v);
    }
    let client = Client::new();
    let uri = match args.get_one::<String>("userid") {
        None => format!("{}/api/v1/user", config.coremgr),
        Some(id) => format!("{}/api/v1/user/{}", config.coremgr, id),
    };
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
        "{}/api/v1/user/{}",
        config.coremgr,
        args.get_one::<String>("userid").unwrap()
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
