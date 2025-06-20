use std::error::Error as StdError;

use clap::{ArgMatches, Command};
use reqwest::{Client, Method, StatusCode, header};
use serde::{Deserialize, Serialize};

use sylvia_iot_coremgr_cli::libs::cli::{Config as CoremgrCliConfig, auth, config};
use sylvia_iot_sdk::util::err::ErrResp;

use super::{super::config::Config, API_RETRY};

#[derive(Deserialize)]
struct GetUsageRes {
    data: GetUsageResData,
}

#[derive(Deserialize, Serialize)]
struct GetUsageResData {
    cpu: Vec<usize>,
    mem: UsageData,
    disk: UsageData,
}

#[derive(Deserialize)]
struct GetTimeRes {
    data: GetTimeResData,
}

#[derive(Deserialize, Serialize)]
struct GetTimeResData {
    time: String,
}

#[derive(Deserialize, Serialize)]
struct UsageData {
    total: u64,
    used: u64,
}

pub fn reg_args(cmd: Command) -> Command {
    cmd.about("System management")
        .subcommand(Command::new("usage").about("Get system resource usage"))
        .subcommand(Command::new("time").about("Get system time"))
}

pub async fn run(
    conf: &Config,
    cm_conf: &CoremgrCliConfig,
    args: &ArgMatches,
) -> Result<Option<()>, Box<dyn StdError>> {
    match args.subcommand() {
        Some(("usage", _args)) => {
            let data = usage_get(conf, cm_conf).await?;
            println!("{}", serde_json::to_string_pretty(&data)?);
            Ok(Some(()))
        }
        Some(("time", _args)) => {
            let data = time_get(conf, cm_conf).await?;
            println!("{}", serde_json::to_string_pretty(&data)?);
            Ok(Some(()))
        }
        _ => Ok(None),
    }
}

async fn usage_get(
    config: &Config,
    cm_conf: &CoremgrCliConfig,
) -> Result<GetUsageResData, ErrResp> {
    let storage = match config::read_storage() {
        Err(e) => {
            let msg = format!("[storage] read storage error: {}", e);
            return Err(ErrResp::ErrRsc(Some(msg)));
        }
        Ok(storage) => storage,
    };

    let client = Client::new();
    let uri = format!("{}/api/v1/sys/usage", config.router);
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
                token = auth::refresh(cm_conf).await?.access_token;
                continue;
            }
            let msg = format!("[API] unexpected status: {}, body: {}", status_code, body);
            return Err(ErrResp::ErrIntMsg(Some(msg)));
        }
        match serde_json::from_str::<GetUsageRes>(body.as_str()) {
            Err(e) => {
                let msg = format!("[API] unexpected token error: {}, body: {}", e, body);
                return Err(ErrResp::ErrIntMsg(Some(msg)));
            }
            Ok(res) => return Ok(res.data),
        }
    }
}

async fn time_get(config: &Config, cm_conf: &CoremgrCliConfig) -> Result<GetTimeResData, ErrResp> {
    let storage = match config::read_storage() {
        Err(e) => {
            let msg = format!("[storage] read storage error: {}", e);
            return Err(ErrResp::ErrRsc(Some(msg)));
        }
        Ok(storage) => storage,
    };

    let client = Client::new();
    let uri = format!("{}/api/v1/sys/time", config.router);
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
                token = auth::refresh(cm_conf).await?.access_token;
                continue;
            }
            let msg = format!("[API] unexpected status: {}, body: {}", status_code, body);
            return Err(ErrResp::ErrIntMsg(Some(msg)));
        }
        match serde_json::from_str::<GetTimeRes>(body.as_str()) {
            Err(e) => {
                let msg = format!("[API] unexpected token error: {}, body: {}", e, body);
                return Err(ErrResp::ErrIntMsg(Some(msg)));
            }
            Ok(res) => return Ok(res.data),
        }
    }
}
