use std::error::Error as StdError;

use clap::{ArgMatches, Command};

use sylvia_iot_coremgr_cli::libs::cli::Config as CoremgrCliConfig;

use super::config::Config;

pub mod net;
pub mod sys;

const API_RETRY: usize = 3;

/// To register Clap arguments.
pub fn reg_args(cmd: Command) -> Command {
    cmd.subcommand(sys::reg_args(Command::new("router.sys")))
        .subcommand(net::reg_args(Command::new("router.net")))
}

pub async fn run(
    conf: &Config,
    cm_conf: &CoremgrCliConfig,
    args: &ArgMatches,
) -> Result<Option<()>, Box<dyn StdError>> {
    match args.subcommand() {
        Some(("router.sys", args)) => sys::run(conf, cm_conf, args).await,
        Some(("router.net", args)) => net::run(conf, cm_conf, args).await,
        _ => Ok(None),
    }
}
