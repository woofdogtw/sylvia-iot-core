use std::{
    error::Error as StdError,
    fs,
    io::{Error as IoError, ErrorKind},
};

use chrono::Utc;
use clap::{Arg, ArgMatches, Command};
use serde::Deserialize;
use tokio;

use sylvia_iot_coremgr_cli::libs::cli as coremgr_cli_libs;
use sylvia_router::libs::{cli, config::Config};

#[derive(Deserialize)]
struct AppConfig {
    #[serde(rename = "coremgrCli")]
    coremgr_cli: coremgr_cli_libs::Config,
    router: Config,
}

const PROJ_NAME: &'static str = env!("CARGO_BIN_NAME");
const PROJ_VER: &'static str = env!("CARGO_PKG_VERSION");

#[tokio::main]
async fn main() -> Result<(), Box<dyn StdError>> {
    let (conf, args) = match init_config() {
        Err(e) => return Err(e),
        Ok((conf, args)) => (conf, args),
    };
    let start = Utc::now().timestamp_millis();
    match coremgr_cli_libs::run(&conf.coremgr_cli, &args).await {
        Err(e) => {
            let diff = Utc::now().timestamp_millis() - start;
            println!("Error ({} ms): {}", diff, e);
            Err(e)
        }
        Ok(None) => match cli::run(&conf.router, &conf.coremgr_cli, &args).await {
            Err(e) => {
                let diff = Utc::now().timestamp_millis() - start;
                println!("Error ({} ms): {}", diff, e);
                Err(e)
            }
            Ok(None) => {
                println!("Sub-command not support");
                Err(Box::new(IoError::new(
                    ErrorKind::InvalidInput,
                    "Sub-command not support",
                )))
            }
            Ok(Some(_)) => {
                println!("OK ({} ms)", Utc::now().timestamp_millis() - start);
                Ok(())
            }
        },
        Ok(Some(_)) => {
            println!("OK ({} ms)", Utc::now().timestamp_millis() - start);
            Ok(())
        }
    }
}

fn init_config() -> Result<(AppConfig, ArgMatches), Box<dyn StdError>> {
    let mut args = Command::new(PROJ_NAME).version(PROJ_VER).arg(
        Arg::new("file")
            .short('f')
            .long("file")
            .help("config file")
            .num_args(1),
    );
    args = coremgr_cli_libs::reg_args(args);
    args = cli::reg_args(args);
    let args = args.get_matches();

    if let Some(value) = args.get_one::<String>("file") {
        let conf_str = fs::read_to_string(value)?;
        return Ok((json5::from_str(conf_str.as_str())?, args));
    }

    // Never run here.
    Err(Box::new(IoError::new(ErrorKind::InvalidInput, "use -h")))
}
