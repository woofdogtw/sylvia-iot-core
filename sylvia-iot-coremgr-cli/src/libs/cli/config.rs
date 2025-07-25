//! Program configurations.

use std::{env, error::Error as StdError, fs};

use clap::ArgMatches;
use dirs;
use serde_json;

use sylvia_iot_corelib::strings;

use super::{Config, Storage};

const DEF_AUTH: &'static str = "http://localhost:1080/auth";
const DEF_COREMGR: &'static str = "http://localhost:3080/coremgr";
const DEF_DATA: &'static str = "http://localhost:4080/data";
const STORAGE_FILE: &'static str = ".sylvia-iot-coremgr-cli.json";

pub fn read_storage() -> Result<Storage, Box<dyn StdError>> {
    let conf_str = fs::read_to_string(storage_path())?;
    Ok(serde_json::from_str(conf_str.as_str())?)
}

pub fn write_storage(storage: &Storage) -> Result<(), Box<dyn StdError>> {
    let json_str = serde_json::to_string_pretty(storage)?;
    Ok(fs::write(storage_path(), json_str)?)
}

/// To read input arguments from command-line arguments and environment variables.
pub fn read_args(args: &ArgMatches) -> Config {
    Config {
        auth: match args.get_one::<String>("coremgr-cli.auth") {
            None => match env::var("COREMGRCLI_AUTH") {
                Err(_) => DEF_AUTH.to_string(),
                Ok(v) => match strings::is_uri(v.as_str()) {
                    false => panic!("invalid `coremgr-cli.auth"),
                    true => v,
                },
            },
            Some(v) => match strings::is_uri(v) {
                false => panic!("invalid `coremgr-cli.auth"),
                true => v.clone(),
            },
        },
        coremgr: match args.get_one::<String>("coremgr-cli.coremgr") {
            None => match env::var("COREMGRCLI_COREMGR") {
                Err(_) => DEF_COREMGR.to_string(),
                Ok(v) => match strings::is_uri(v.as_str()) {
                    false => panic!("invalid `coremgr-cli.coremgr"),
                    true => v,
                },
            },
            Some(v) => match strings::is_uri(v) {
                false => panic!("invalid `coremgr-cli.coremgr"),
                true => v.clone(),
            },
        },
        data: match args.get_one::<String>("coremgr-cli.data") {
            None => match env::var("COREMGRCLI_DATA") {
                Err(_) => DEF_DATA.to_string(),
                Ok(v) => match strings::is_uri(v.as_str()) {
                    false => panic!("invalid `coremgr-cli.data"),
                    true => v,
                },
            },
            Some(v) => match strings::is_uri(v) {
                false => panic!("invalid `coremgr-cli.data"),
                true => v.clone(),
            },
        },
        client_id: match args.get_one::<String>("coremgr-cli.client-id") {
            None => match env::var("COREMGRCLI_CLIENT_ID") {
                Err(_) => panic!("missing `coremgr-cli.client-id` or `COREMGRCLI_CLIENT_ID`"),
                Ok(v) => match v.len() {
                    0 => panic!("invalid `coremgr-cli.client-id"),
                    _ => v,
                },
            },
            Some(v) => v.to_string(),
        },
        redirect_uri: match args.get_one::<String>("coremgr-cli.redirect-uri") {
            None => match env::var("COREMGRCLI_REDIRECT_URI") {
                Err(_) => panic!("missing `coremgr-cli.redirect-uri` or `COREMGRCLI_REDIRECT_URI`"),
                Ok(v) => match strings::is_uri(v.as_str()) {
                    false => panic!("invalid `coremgr-cli.redirect-uri"),
                    true => v,
                },
            },
            Some(v) => match strings::is_uri(v) {
                false => panic!("invalid `coremgr-cli.redirect-uri"),
                true => v.clone(),
            },
        },
    }
}

fn storage_path() -> String {
    match dirs::home_dir() {
        None => match env::current_dir() {
            Err(_) => STORAGE_FILE.to_string(),
            Ok(dir) => match dir.to_str() {
                None => STORAGE_FILE.to_string(),
                Some(dir) => format!("{}/{}", dir, STORAGE_FILE),
            },
        },
        Some(dir) => match dir.to_str() {
            None => match env::current_dir() {
                Err(_) => STORAGE_FILE.to_string(),
                Ok(dir) => match dir.to_str() {
                    None => STORAGE_FILE.to_string(),
                    Some(dir) => format!("{}/{}", dir, STORAGE_FILE),
                },
            },
            Some(dir) => format!("{}/{}", dir, STORAGE_FILE),
        },
    }
}
