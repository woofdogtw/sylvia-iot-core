//! Program configurations.

use std::{env, error::Error as StdError, fs};

use clap::{Arg, ArgMatches, Command};
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
    Ok(write_private_file(
        storage_path().as_str(),
        json_str.as_bytes(),
    )?)
}

/// To write the file that only the owner can read and write because it contains tokens.
#[cfg(unix)]
fn write_private_file(path: &str, content: &[u8]) -> std::io::Result<()> {
    use std::{
        fs::{OpenOptions, Permissions},
        io::Write,
        os::unix::fs::{OpenOptionsExt, PermissionsExt},
    };

    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(path)?;
    // The mode only applies to new files, so also fix files created by previous versions.
    file.set_permissions(Permissions::from_mode(0o600))?;
    file.write_all(content)
}

#[cfg(not(unix))]
fn write_private_file(path: &str, content: &[u8]) -> std::io::Result<()> {
    fs::write(path, content)
}

/// To register Clap arguments.
pub fn reg_args(cmd: Command) -> Command {
    cmd.arg(
        Arg::new("coremgr-cli.auth")
            .long("coremgr-cli.auth")
            .help("sylvia-iot-auth host (ex: http://localhost:1080/auth)")
            .num_args(1),
    )
    .arg(
        Arg::new("coremgr-cli.coremgr")
            .long("coremgr-cli.coremgr")
            .help("sylvia-iot-coremgr host (ex: http://localhost:3080/coremgr)")
            .num_args(1),
    )
    .arg(
        Arg::new("coremgr-cli.data")
            .long("coremgr-cli.data")
            .help("sylvia-iot-data host (ex: http://localhost:4080/data)")
            .num_args(1),
    )
    .arg(
        Arg::new("coremgr-cli.client-id")
            .long("coremgr-cli.client-id")
            .help("CLI client ID")
            .num_args(1),
    )
    .arg(
        Arg::new("coremgr-cli.redirect-uri")
            .long("coremgr-cli.redirect-uri")
            .help("CLI client redirect URI")
            .num_args(1),
    )
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
