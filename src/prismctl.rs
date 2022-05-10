// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::log;

use exitcode::ExitCode;
use structopt::clap::{crate_version, AppSettings};
use structopt::StructOpt;

const PRISMCTL: &str = "prismctl";

/// Overall primsd binary configuration.
#[derive(Debug, Clone, StructOpt)]
#[structopt(
    global_settings = &[AppSettings::DeriveDisplayOrder],
    author = "Christian Saide <me@csaide.dev>",
    about = "Manage a prismd instance or cluster.",
    version = crate_version!()
)]
struct PrismctlConfig {
    #[structopt(flatten)]
    log_config: log::Config,
}

pub async fn run() -> ExitCode {
    let cfg = match crate::base_config::<PrismctlConfig>(PRISMCTL) {
        Ok(cfg) => cfg,
        Err(code) => return code,
    };

    let root_logger = log::new(&cfg.log_config, PRISMCTL, crate_version!());
    info!(root_logger, "Hello world!");
    exitcode::OK
}
