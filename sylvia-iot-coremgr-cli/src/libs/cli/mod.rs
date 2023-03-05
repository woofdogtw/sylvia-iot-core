use std::error::Error as StdError;

use chrono::DateTime;
use clap::{ArgMatches, Command};
use reqwest::{header, Response};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use sylvia_iot_corelib::strings;

mod application;
pub mod auth;
mod client;
pub mod config;
mod data_app_dldata;
mod data_app_uldata;
mod data_coremgr_opdata;
mod data_net_dldata;
mod data_net_uldata;
mod device;
mod device_route;
mod dldata_buffer;
mod login;
mod network;
mod network_route;
mod unit;
mod user;

use auth::refresh;

/// Application configurations.
#[derive(Deserialize)]
pub struct Config {
    /// **sylvia-auth** API base path with host. For example: `http://localhost:1080/auth`.
    auth: String,
    /// **sylvia-coremgr** API base path with host. For example: `http://localhost:3080/coremgr`.
    coremgr: String,
    /// **sylvia-data** API base path with host. For example: `http://localhost:4080/data`.
    data: String,
    /// Client ID of **coremgr-cli**.
    #[serde(rename = "clientId")]
    client_id: String,
    /// Redirect URI of **coremgr-cli**.
    #[serde(rename = "redirectUri")]
    redirect_uri: String,
}

#[derive(Deserialize, Serialize)]
pub struct Storage {
    #[serde(rename = "accessToken")]
    pub access_token: String,
    #[serde(rename = "refreshToken")]
    pub refresh_token: String,
}

#[derive(Deserialize, Serialize)]
pub struct AccessToken {
    pub access_token: String,
    pub refresh_token: String,
    #[serde(rename = "token_type")]
    _token_type: String,
    #[serde(rename = "expires_in")]
    _expires_in: u64,
}

const API_RETRY: usize = 3;

/// To register Clap arguments.
pub fn reg_args(cmd: Command) -> Command {
    cmd.subcommand(login::reg_args(Command::new("login")))
        .subcommand(auth::reg_args(Command::new("auth")))
        .subcommand(user::reg_args(Command::new("user")))
        .subcommand(client::reg_args(Command::new("client")))
        .subcommand(unit::reg_args(Command::new("unit")))
        .subcommand(application::reg_args(Command::new("application")))
        .subcommand(network::reg_args(Command::new("network")))
        .subcommand(device::reg_args(Command::new("device")))
        .subcommand(device_route::reg_args(Command::new("device-route")))
        .subcommand(network_route::reg_args(Command::new("network-route")))
        .subcommand(dldata_buffer::reg_args(Command::new("dldata-buffer")))
        .subcommand(data_app_uldata::reg_args(Command::new("data.app-uldata")))
        .subcommand(data_app_dldata::reg_args(Command::new("data.app-dldata")))
        .subcommand(data_net_uldata::reg_args(Command::new("data.net-uldata")))
        .subcommand(data_net_dldata::reg_args(Command::new("data.net-dldata")))
        .subcommand(data_coremgr_opdata::reg_args(Command::new(
            "data.coremgr-opdata",
        )))
}

pub async fn run(conf: &Config, args: &ArgMatches) -> Result<Option<()>, Box<dyn StdError>> {
    match args.subcommand() {
        Some(("login", args)) => login::run(conf, args).await,
        Some(("auth", args)) => auth::run(conf, args).await,
        Some(("user", args)) => user::run(conf, args).await,
        Some(("client", args)) => client::run(conf, args).await,
        Some(("unit", args)) => unit::run(conf, args).await,
        Some(("application", args)) => application::run(conf, args).await,
        Some(("network", args)) => network::run(conf, args).await,
        Some(("device", args)) => device::run(conf, args).await,
        Some(("device-route", args)) => device_route::run(conf, args).await,
        Some(("network-route", args)) => network_route::run(conf, args).await,
        Some(("dldata-buffer", args)) => dldata_buffer::run(conf, args).await,
        Some(("data.app-uldata", args)) => data_app_uldata::run(conf, args).await,
        Some(("data.app-dldata", args)) => data_app_dldata::run(conf, args).await,
        Some(("data.net-uldata", args)) => data_net_uldata::run(conf, args).await,
        Some(("data.net-dldata", args)) => data_net_dldata::run(conf, args).await,
        Some(("data.coremgr-opdata", args)) => data_coremgr_opdata::run(conf, args).await,
        _ => Ok(None),
    }
}

fn validate_code(code_str: &str) -> Result<String, String> {
    match strings::is_code(code_str) {
        false => Err("should be ^[a-z0-9]{1}[a-z0-9_-]*$".to_string()),
        true => Ok(code_str.to_string()),
    }
}

fn validate_json(json_str: &str) -> Result<Map<String, Value>, String> {
    match serde_json::from_str::<Map<String, Value>>(json_str) {
        Err(e) => Err(e.to_string()),
        Ok(v) => Ok(v),
    }
}

fn validate_timestr(time_str: &str) -> Result<String, String> {
    match DateTime::parse_from_rfc3339(time_str) {
        Err(e) => Err(e.to_string()),
        Ok(_) => Ok(time_str.to_string()),
    }
}

fn get_csv_filename(resp: &Response) -> String {
    if let Some(v) = resp.headers().get(header::CONTENT_DISPOSITION) {
        if let Ok(v) = String::from_utf8(v.as_bytes().to_vec()) {
            let splits = v.split("filename=");
            for s in splits.into_iter() {
                if s.contains(".csv") {
                    return s.to_string();
                }
            }
        }
    }
    "".to_string()
}
