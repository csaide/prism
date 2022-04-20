// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::log;

use exitcode::ExitCode;
use structopt::clap::{self, crate_version, ErrorKind};
use structopt::StructOpt;

const PRISMD: &str = "prismd";

/// Overall primsd binary configuration.
#[derive(Debug, Clone, StructOpt)]
#[structopt(
    global_settings = &[clap::AppSettings::DeriveDisplayOrder],
    author = "Christian Saide <me@csaide.dev>",
    about = "Run an instance of primsd.",
    version = crate_version!()
)]
struct PrismctlConfig {
    #[structopt(flatten)]
    log_config: log::Config,
}

pub async fn run() -> ExitCode {
    let setup_logger = log::default(PRISMD, crate_version!());
    let cfg = match PrismctlConfig::from_args_safe() {
        Ok(cfg) => cfg,
        Err(err)
            if err.kind == ErrorKind::HelpDisplayed || err.kind == ErrorKind::VersionDisplayed =>
        {
            println!("{}", err.message);
            return exitcode::USAGE;
        }
        Err(err) => {
            crit!(setup_logger, "Failed to parse provided configuration."; "error" => err.to_string());
            return exitcode::CONFIG;
        }
    };

    let root_logger = log::new(&cfg.log_config, PRISMD, crate_version!());
    info!(root_logger, "Hello world!");
    exitcode::OK
}
