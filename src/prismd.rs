// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::log;

use exitcode::ExitCode;
use structopt::clap::{crate_version, AppSettings};
use structopt::StructOpt;

const PRISMD: &str = "prismd";

/// Overall primsd binary configuration.
#[derive(Debug, Clone, StructOpt)]
#[structopt(
    global_settings = &[AppSettings::DeriveDisplayOrder],
    author = "Christian Saide <me@csaide.dev>",
    about = "Run an instance of primsd.",
    version = crate_version!()
)]
struct PrismdConfig {
    #[structopt(flatten)]
    log_config: log::Config,
    #[structopt(
        long = "rpc-port",
        short = "p",
        env = "PRISM_RPC_PORT",
        help = "The port to listen on for server to server communication.",
        long_help = "Sets the port to use for server to server RPC's including heartbeat, replication, and leader elections.",
        default_value = "8080",
        takes_value = true
    )]
    rpc_port: u16,
    #[structopt(
        long = "db-path",
        short = "d",
        env = "PRISM_DB_PATH",
        help = "The top level directory path to store application data.",
        long_help = "Sets the port to use for server to server RPC's including heartbeat, replication, and leader elections.",
        default_value = "8080",
        takes_value = true
    )]
    db_path: String,
}

pub async fn run() -> ExitCode {
    let cfg = match crate::base_config::<PrismdConfig>(PRISMD) {
        Ok(cfg) => cfg,
        Err(code) => return code,
    };

    let root_logger = log::new(&cfg.log_config, PRISMD, crate_version!());
    info!(root_logger, "Hello world!");

    exitcode::OK
}
