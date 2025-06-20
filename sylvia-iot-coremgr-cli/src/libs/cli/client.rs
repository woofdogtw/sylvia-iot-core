use std::{error::Error as StdError, fs};

use clap::{
    Arg, ArgMatches, Command,
    builder::{BoolValueParser, RangedU64ValueParser},
};
use reqwest::{Client, Method, StatusCode, header};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use serde_urlencoded;

use sylvia_iot_corelib::err::ErrResp;

use super::{API_RETRY, Config, config, get_csv_filename, refresh};

#[derive(Serialize)]
struct PostReq<'a> {
    data: PostReqData<'a>,
    #[serde(skip_serializing_if = "Option::is_none")]
    credentials: Option<bool>,
}

#[derive(Serialize)]
struct PostReqData<'a> {
    #[serde(rename = "redirectUris")]
    redirect_uris: Vec<&'a str>,
    scopes: Vec<&'a str>,
    #[serde(rename = "userId", skip_serializing_if = "Option::is_none")]
    user_id: Option<&'a str>,
    name: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    image: Option<&'a str>,
}

#[derive(Deserialize)]
struct PostRes {
    data: PostResData,
}

#[derive(Deserialize, Serialize)]
struct PostResData {
    #[serde(rename = "clientId")]
    client_id: String,
}

#[derive(Serialize)]
struct GetCountReq<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    user: Option<&'a str>,
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
    user: Option<&'a str>,
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
    #[serde(rename = "regenSecret", skip_serializing_if = "Option::is_none")]
    regen_secret: Option<bool>,
}

#[derive(Serialize)]
struct PatchReqData<'a> {
    #[serde(rename = "redirectUris", skip_serializing_if = "Option::is_none")]
    redirect_uris: Option<Vec<&'a str>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    scopes: Option<Vec<&'a str>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    image: Option<Value>,
}

#[derive(Deserialize, Serialize)]
struct GetResData {
    #[serde(rename = "clientId")]
    client_id: String,
    #[serde(rename = "createdAt")]
    created_at: String,
    #[serde(rename = "modifiedAt")]
    modified_at: String,
    #[serde(rename = "clientSecret")]
    client_secret: Option<String>,
    #[serde(rename = "redirectUris")]
    redirect_uris: Vec<String>,
    scopes: Vec<String>,
    #[serde(rename = "userId", skip_serializing_if = "Option::is_none")]
    user_id: Option<String>,
    name: String,
    image: Option<String>,
}

pub fn reg_args(cmd: Command) -> Command {
    cmd.about("Client management")
        .subcommand(
            Command::new("add")
                .about("Add client")
                .arg(
                    Arg::new("redirect")
                        .short('r')
                        .long("redirect")
                        .help("Redirect URIs")
                        .num_args(0..)
                        .required(true),
                )
                .arg(
                    Arg::new("scopes")
                        .short('s')
                        .long("scopes")
                        .help("Scopes")
                        .num_args(0..)
                        .required(true),
                )
                .arg(
                    Arg::new("userid")
                        .short('u')
                        .long("userid")
                        .help("User ID (only avaliable for administrators)")
                        .num_args(1),
                )
                .arg(
                    Arg::new("name")
                        .short('n')
                        .long("name")
                        .help("Display name")
                        .num_args(1)
                        .required(true),
                )
                .arg(
                    Arg::new("image")
                        .short('m')
                        .long("image")
                        .help("Client image URI (no value to clear image)")
                        .num_args(0..=1),
                )
                .arg(
                    Arg::new("credentials")
                        .short('c')
                        .long("credentials")
                        .help("Create the client with secret")
                        .num_args(1)
                        .value_parser(BoolValueParser::new()),
                ),
        )
        .subcommand(
            Command::new("count").about("Get client count").arg(
                Arg::new("userid")
                    .long("userid")
                    .help("The specified user ID (only avaliable for administrators)")
                    .num_args(1),
            ),
        )
        .subcommand(
            Command::new("list")
                .about("Get client list")
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
                    Arg::new("userid")
                        .long("userid")
                        .help("The specified user ID (only avaliable for administrators)")
                        .num_args(1),
                ),
        )
        .subcommand(
            Command::new("get")
                .about("Get client detail information")
                .arg(
                    Arg::new("clientid")
                        .short('i')
                        .long("clientid")
                        .help("Client ID")
                        .num_args(1)
                        .required(true),
                ),
        )
        .subcommand(
            Command::new("update")
                .about("Update client information")
                .arg(
                    Arg::new("clientid")
                        .short('i')
                        .long("clientid")
                        .help("Client ID")
                        .num_args(1)
                        .required(true),
                )
                .arg(
                    Arg::new("redirect")
                        .short('r')
                        .long("redirect")
                        .help("Redirect URIs")
                        .num_args(0..),
                )
                .arg(
                    Arg::new("scopes")
                        .short('s')
                        .long("scopes")
                        .help("Scopes")
                        .num_args(0..),
                )
                .arg(
                    Arg::new("name")
                        .short('n')
                        .long("name")
                        .help("Display name")
                        .num_args(1),
                )
                .arg(
                    Arg::new("image")
                        .short('m')
                        .long("image")
                        .help("Client image URI (no value to clear image)")
                        .num_args(1),
                )
                .arg(
                    Arg::new("regen")
                        .long("regen")
                        .help("Re-generate the client secret")
                        .num_args(1)
                        .value_parser(["true"]),
                ),
        )
        .subcommand(
            Command::new("delete").about("Delete a client").arg(
                Arg::new("clientid")
                    .short('i')
                    .long("clientid")
                    .help("Client ID")
                    .num_args(1)
                    .required(true),
            ),
        )
        .subcommand(
            Command::new("delete-user")
                .about("Delete clients of a user")
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
        Some(("delete-user", args)) => {
            delete_user(conf, args).await?;
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
            redirect_uris: args
                .get_many::<String>("redirect")
                .unwrap()
                .map(|x| x.as_str())
                .collect(),
            scopes: args
                .get_many::<String>("scopes")
                .unwrap()
                .map(|x| x.as_str())
                .collect(),
            user_id: match args.get_one::<String>("userid") {
                None => None,
                Some(v) => Some(v.as_str()),
            },
            name: args.get_one::<String>("name").unwrap(),
            image: match args.get_one::<String>("image") {
                None => None,
                Some(v) => Some(v.as_str()),
            },
        },
        credentials: match args.get_one::<bool>("credentials") {
            None => None,
            Some(v) => Some(*v),
        },
    };
    let client = Client::new();
    let uri = format!("{}/api/v1/client", config.coremgr);
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
        user: match args.get_one::<String>("userid") {
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
        Ok(query) => format!("{}/api/v1/client/count?{}", config.coremgr, query),
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
        user: match args.get_one::<String>("userid") {
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
        Ok(query) => format!("{}/api/v1/client/list?{}", config.coremgr, query),
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
        "{}/api/v1/client/{}",
        config.coremgr,
        args.get_one::<String>("clientid").unwrap()
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

    let mut body = PatchReq {
        data: None,
        regen_secret: None,
    };
    if args.get_many::<String>("redirect").is_some()
        || args.get_many::<String>("scopes").is_some()
        || args.get_one::<String>("name").is_some()
        || args.get_one::<String>("image").is_some()
    {
        body.data = Some(PatchReqData {
            redirect_uris: match args.get_many::<String>("redirect") {
                None => None,
                Some(v) => Some(v.map(|x| x.as_str()).collect()),
            },
            scopes: match args.get_many::<String>("scopes") {
                None => None,
                Some(v) => Some(v.map(|x| x.as_str()).collect()),
            },
            name: match args.get_one::<String>("name") {
                None => None,
                Some(v) => Some(v.as_str()),
            },
            image: match args.get_one::<String>("image") {
                None => None,
                Some(v) => match v.len() {
                    0 => Some(Value::Null),
                    _ => Some(Value::String(v.clone())),
                },
            },
        });
    }
    if args.get_one::<String>("regen").is_some() {
        body.regen_secret = Some(true);
    }
    let client = Client::new();
    let uri = format!(
        "{}/api/v1/client/{}",
        config.coremgr,
        args.get_one::<String>("clientid").unwrap()
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
        "{}/api/v1/client/{}",
        config.coremgr,
        args.get_one::<String>("clientid").unwrap()
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

async fn delete_user(config: &Config, args: &ArgMatches) -> Result<(), ErrResp> {
    let storage = match config::read_storage() {
        Err(e) => {
            let msg = format!("[storage] read storage error: {}", e);
            return Err(ErrResp::ErrRsc(Some(msg)));
        }
        Ok(storage) => storage,
    };

    let client = Client::new();
    let uri = format!(
        "{}/api/v1/client/user/{}",
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
