use std::{collections::HashMap, error::Error as StdError};

use clap::{ArgMatches, Command};
use reqwest::{Client, Method, StatusCode, header};
use serde::{Deserialize, Serialize};
use serde_json;

use sylvia_iot_corelib::err::ErrResp;

use super::{API_RETRY, AccessToken, Config, Storage, config};

#[derive(Deserialize, Serialize)]
struct GetTokenInfo {
    #[serde(rename = "userId")]
    user_id: String,
    account: String,
    roles: HashMap<String, bool>,
    name: String,
    #[serde(rename = "clientId")]
    client_id: String,
    scopes: Vec<String>,
}

#[derive(Serialize)]
struct PostRefreshRequest<'a> {
    grant_type: &'a str,
    refresh_token: &'a str,
    client_id: &'a str,
}

#[derive(Deserialize, Serialize)]
struct GetTokenInfoRes {
    data: GetTokenInfo,
}

pub fn reg_args(cmd: Command) -> Command {
    cmd.about("Authentication operations")
        .subcommand(Command::new("tokeninfo").about("Get current token information"))
        .subcommand(Command::new("logout").about("Log-out the user"))
        .subcommand(Command::new("refresh").about("Refresh access token"))
}

pub async fn run(conf: &Config, args: &ArgMatches) -> Result<Option<()>, Box<dyn StdError>> {
    match args.subcommand() {
        Some(("refresh", _)) => {
            let token = refresh(conf).await?;
            println!("{}", serde_json::to_string_pretty(&token)?);
            Ok(Some(()))
        }
        Some(("tokeninfo", _)) => {
            let tokeninfo = tokeninfo(conf).await?;
            println!("{}", serde_json::to_string_pretty(&tokeninfo)?);
            Ok(Some(()))
        }
        Some(("logout", _)) => {
            logout(conf).await?;
            Ok(Some(()))
        }
        _ => Ok(None),
    }
}

/// Refresh token. Please use login when receiving error.
pub async fn refresh(config: &Config) -> Result<AccessToken, ErrResp> {
    let storage = match config::read_storage() {
        Err(e) => {
            let msg = format!("[storage] read storage error: {}", e);
            return Err(ErrResp::ErrRsc(Some(msg)));
        }
        Ok(storage) => storage,
    };

    let client = Client::new();
    let uri = format!("{}/oauth2/refresh", config.auth);
    let body = PostRefreshRequest {
        grant_type: "refresh_token",
        refresh_token: storage.refresh_token.as_str(),
        client_id: config.client_id.as_str(),
    };
    let req = match client
        .request(Method::POST, uri.as_str())
        .form(&body)
        .build()
    {
        Err(e) => {
            let msg = format!("[refresh] create request error: {}", e);
            return Err(ErrResp::ErrRsc(Some(msg)));
        }
        Ok(req) => req,
    };
    let resp = match client.execute(req).await {
        Err(e) => {
            let msg = format!("[refresh] execute request error: {}", e);
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
        let msg = format!(
            "[refresh] unexpected status: {}, body: {}",
            status_code, body
        );
        return Err(ErrResp::ErrIntMsg(Some(msg)));
    }
    match serde_json::from_str::<AccessToken>(body.as_str()) {
        Err(e) => {
            let msg = format!("[refresh] unexpected token error: {}, body: {}", e, body);
            Err(ErrResp::ErrIntMsg(Some(msg)))
        }
        Ok(token) => {
            if let Err(e) = config::write_storage(&Storage {
                access_token: token.access_token.clone(),
                refresh_token: token.refresh_token.clone(),
            }) {
                let msg = format!("[storage] write storage error: {}", e);
                return Err(ErrResp::ErrRsc(Some(msg)));
            }
            Ok(token)
        }
    }
}

async fn tokeninfo(config: &Config) -> Result<GetTokenInfo, ErrResp> {
    let storage = match config::read_storage() {
        Err(e) => {
            let msg = format!("[storage] read storage error: {}", e);
            return Err(ErrResp::ErrRsc(Some(msg)));
        }
        Ok(storage) => storage,
    };

    let client = Client::new();
    let uri = format!("{}/api/v1/auth/tokeninfo", config.coremgr);
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
        match serde_json::from_str::<GetTokenInfoRes>(body.as_str()) {
            Err(e) => {
                let msg = format!("[API] unexpected token error: {}, body: {}", e, body);
                return Err(ErrResp::ErrIntMsg(Some(msg)));
            }
            Ok(res) => return Ok(res.data),
        }
    }
}

async fn logout(config: &Config) -> Result<(), ErrResp> {
    let storage = match config::read_storage() {
        Err(e) => {
            let msg = format!("[storage] read storage error: {}", e);
            return Err(ErrResp::ErrRsc(Some(msg)));
        }
        Ok(storage) => storage,
    };

    let client = Client::new();
    let uri = format!("{}/api/v1/auth/logout", config.coremgr);
    let token = storage.access_token;

    let req = match client
        .request(Method::POST, uri.as_str())
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
    if status_code == StatusCode::NO_CONTENT || status_code == StatusCode::UNAUTHORIZED {
        return Ok(());
    }
    let msg = format!("[API] unexpected status: {}, body: {}", status_code, body);
    Err(ErrResp::ErrIntMsg(Some(msg)))
}
