use std::error::Error as StdError;

use clap::{Arg, ArgMatches, Command};
use reqwest::{header, redirect::Policy, ClientBuilder, Method, Response, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json;
use serde_urlencoded;
use url::Url;

use sylvia_iot_corelib::err::ErrResp;

use super::{config, AccessToken, Config, Storage};

#[derive(Deserialize)]
struct GetAuthResponse {
    state: String,
}

#[derive(Serialize)]
struct PostLoginRequest<'a> {
    account: &'a str,
    password: &'a str,
    state: &'a str,
}

#[derive(Deserialize)]
struct PostLoginResponse {
    session_id: String,
}

#[derive(Serialize)]
struct PostAuthorizeRequest<'a> {
    response_type: &'a str,
    client_id: &'a str,
    redirect_uri: &'a str,
    session_id: &'a str,
    allow: &'a str,
}

#[derive(Deserialize)]
struct PostAuthorizeResponse {
    code: String,
}

#[derive(Serialize)]
struct PostTokenRequest<'a> {
    grant_type: &'a str,
    code: &'a str,
    redirect_uri: &'a str,
    client_id: &'a str,
}

pub fn reg_args(cmd: Command) -> Command {
    cmd.about("Log-in with account/password")
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
}

pub async fn run(conf: &Config, args: &ArgMatches) -> Result<Option<()>, Box<dyn StdError>> {
    let token = login(
        args.get_one::<String>("account").unwrap().as_str(),
        args.get_one::<String>("password").unwrap().as_str(),
        conf,
    )
    .await?;
    println!("{}", serde_json::to_string_pretty(&token)?);
    Ok(Some(()))
}

/// Log-in and get access token from account and password.
async fn login(account: &str, password: &str, config: &Config) -> Result<AccessToken, ErrResp> {
    let client = match ClientBuilder::new().redirect(Policy::none()).build() {
        Err(e) => {
            let msg = format!("[auth] create client error: {}", e);
            return Err(ErrResp::ErrRsc(Some(msg)));
        }
        Ok(client) => client,
    };
    let uri = format!(
        "{}/oauth2/auth?response_type=code&redirect_uri={}&client_id={}",
        config.auth, config.redirect_uri, config.client_id
    );
    let req = match client.request(Method::GET, uri.as_str()).build() {
        Err(e) => {
            let msg = format!("[auth] create request error: {}", e);
            return Err(ErrResp::ErrRsc(Some(msg)));
        }
        Ok(req) => req,
    };
    let resp = match client.execute(req).await {
        Err(e) => {
            let msg = format!("[auth] execute request error: {}", e);
            return Err(ErrResp::ErrRsc(Some(msg)));
        }
        Ok(resp) => resp,
    };
    let status_code = resp.status();
    if status_code != StatusCode::FOUND {
        let body = match resp.bytes().await {
            Err(e) => format!("(wrong body: {})", e),
            Ok(bytes) => match String::from_utf8(bytes.to_vec()) {
                Err(e) => format!("(body not UTF-8: {})", e),
                Ok(body) => body,
            },
        };
        let msg = format!("[auth] unexpected status: {}, body: {}", status_code, body);
        return Err(ErrResp::ErrIntMsg(Some(msg)));
    }
    let state = match read_location(&resp) {
        Err(e) => {
            let msg = format!("[auth] response location header error: {}", e);
            return Err(ErrResp::ErrIntMsg(Some(msg)));
        }
        Ok(url) => match url.query() {
            None => {
                let msg = format!("[auth] response location empty");
                return Err(ErrResp::ErrIntMsg(Some(msg)));
            }
            Some(query) => match serde_urlencoded::from_str::<GetAuthResponse>(query) {
                Err(e) => {
                    let msg = format!("[auth] response location format error: {}", e);
                    return Err(ErrResp::ErrIntMsg(Some(msg)));
                }
                Ok(req) => req.state,
            },
        },
    };

    let uri = format!("{}/oauth2/login", config.auth);
    let body = PostLoginRequest {
        account,
        password,
        state: state.as_str(),
    };
    let req = match client
        .request(Method::POST, uri.as_str())
        .form(&body)
        .build()
    {
        Err(e) => {
            let msg = format!("[login] create request error: {}", e);
            return Err(ErrResp::ErrRsc(Some(msg)));
        }
        Ok(req) => req,
    };
    let resp = match client.execute(req).await {
        Err(e) => {
            let msg = format!("[login] execute request error: {}", e);
            return Err(ErrResp::ErrRsc(Some(msg)));
        }
        Ok(resp) => resp,
    };
    let status_code = resp.status();
    if status_code != StatusCode::FOUND {
        let body = match resp.bytes().await {
            Err(e) => format!("(wrong body: {})", e),
            Ok(bytes) => match String::from_utf8(bytes.to_vec()) {
                Err(e) => format!("(body not UTF-8: {})", e),
                Ok(body) => body,
            },
        };
        let msg = format!("[login] unexpected status: {}, body: {}", status_code, body);
        return Err(ErrResp::ErrIntMsg(Some(msg)));
    }
    let session_id = match read_location(&resp) {
        Err(e) => {
            let msg = format!("[login] response location header error: {}", e);
            return Err(ErrResp::ErrIntMsg(Some(msg)));
        }
        Ok(url) => match url.query() {
            None => {
                let msg = format!("[login] response location empty");
                return Err(ErrResp::ErrIntMsg(Some(msg)));
            }
            Some(query) => match serde_urlencoded::from_str::<PostLoginResponse>(query) {
                Err(e) => {
                    let msg = format!("[login] response location format error: {}", e);
                    return Err(ErrResp::ErrIntMsg(Some(msg)));
                }
                Ok(req) => req.session_id,
            },
        },
    };

    let uri = format!("{}/oauth2/authorize", config.auth);
    let body = PostAuthorizeRequest {
        response_type: "code",
        client_id: config.client_id.as_str(),
        redirect_uri: config.redirect_uri.as_str(),
        session_id: session_id.as_str(),
        allow: "yes",
    };
    let req = match client
        .request(Method::POST, uri.as_str())
        .form(&body)
        .build()
    {
        Err(e) => {
            let msg = format!("[authorize] create request error: {}", e);
            return Err(ErrResp::ErrRsc(Some(msg)));
        }
        Ok(req) => req,
    };
    let resp = match client.execute(req).await {
        Err(e) => {
            let msg = format!("[authorize] execute request error: {}", e);
            return Err(ErrResp::ErrRsc(Some(msg)));
        }
        Ok(resp) => resp,
    };
    let status_code = resp.status();
    if status_code != StatusCode::FOUND {
        let body = match resp.bytes().await {
            Err(e) => format!("(wrong body: {})", e),
            Ok(bytes) => match String::from_utf8(bytes.to_vec()) {
                Err(e) => format!("(body not UTF-8: {})", e),
                Ok(body) => body,
            },
        };
        let msg = format!(
            "[authorize] unexpected status: {}, body: {}",
            status_code, body
        );
        return Err(ErrResp::ErrIntMsg(Some(msg)));
    }
    let code = match read_location(&resp) {
        Err(e) => {
            let msg = format!("[authorize] response location header error: {}", e);
            return Err(ErrResp::ErrIntMsg(Some(msg)));
        }
        Ok(url) => match url.query() {
            None => {
                let msg = format!("[authorize] response location empty");
                return Err(ErrResp::ErrIntMsg(Some(msg)));
            }
            Some(query) => match serde_urlencoded::from_str::<PostAuthorizeResponse>(query) {
                Err(e) => {
                    let msg = format!("[authorize] response location format error: {}", e);
                    return Err(ErrResp::ErrIntMsg(Some(msg)));
                }
                Ok(req) => req.code,
            },
        },
    };

    let uri = format!("{}/oauth2/token", config.auth);
    let body = PostTokenRequest {
        grant_type: "authorization_code",
        code: code.as_str(),
        redirect_uri: config.redirect_uri.as_str(),
        client_id: config.client_id.as_str(),
    };
    let req = match client
        .request(Method::POST, uri.as_str())
        .form(&body)
        .build()
    {
        Err(e) => {
            let msg = format!("[token] create request error: {}", e);
            return Err(ErrResp::ErrRsc(Some(msg)));
        }
        Ok(req) => req,
    };
    let resp = match client.execute(req).await {
        Err(e) => {
            let msg = format!("[token] execute request error: {}", e);
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
        let msg = format!("[token] unexpected status: {}, body: {}", status_code, body);
        return Err(ErrResp::ErrIntMsg(Some(msg)));
    }
    match serde_json::from_str::<AccessToken>(body.as_str()) {
        Err(e) => {
            let msg = format!("[token] unexpected token error: {}, body: {}", e, body);
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

fn read_location(resp: &Response) -> Result<Url, String> {
    let location = match resp.headers().get(header::LOCATION) {
        None => return Err("no location header".to_string()),
        Some(location) => match location.to_str() {
            Err(e) => return Err(format!("location to_str() error: {}", e)),
            Ok(location) => location,
        },
    };
    match Url::parse(location) {
        Err(e) => match e {
            url::ParseError::RelativeUrlWithoutBase => {
                let url_with_base = format!("http://localhost{}", location);
                match Url::parse(url_with_base.as_str()) {
                    Err(e) => return Err(format!("parse url with base error: {}", e)),
                    Ok(url) => return Ok(url),
                }
            }
            _ => return Err(format!("parse url error: {}", e)),
        },
        Ok(url) => return Ok(url),
    }
}
