// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

use exitcode::ExitCode;
use structopt::clap::{crate_version, AppSettings};
use structopt::StructOpt;

use crate::rpc::frontend::{FrontendClient, MutateRequest};
use crate::{hash, log};

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
    #[structopt(
        long = "cluster-leader",
        short = "c",
        env = "PRISM_LEADER_HOST",
        help = "The cluster leader to operate against.",
        long_help = "Sets hostname of the leader of the cluster to operate against.",
        default_value = "grpc://127.0.0.1:8081",
        takes_value = true
    )]
    leader: String,
}

pub async fn run() -> ExitCode {
    let cfg = match crate::base_config::<PrismctlConfig>(PRISMCTL) {
        Ok(cfg) => cfg,
        Err(code) => return code,
    };

    let root_logger = log::new(&cfg.log_config, PRISMCTL, crate_version!());
    info!(root_logger, "Hello world!");

    let mut client = match FrontendClient::connect(cfg.leader).await {
        Ok(client) => client,
        Err(e) => {
            error!(root_logger, "Failed to connect to leader."; "error" => e.to_string());
            return exitcode::IOERR;
        }
    };

    let cmd = hash::Command::Insert("hello".to_string(), 42);
    let req = MutateRequest {
        command: cmd.to_bytes(),
        ..Default::default()
    };

    match client.mutate(req).await {
        Ok(_) => {} // TODO(csaide): Response handling yo.
        Err(e) => {
            error!(root_logger, "Failed to call mutate rpc."; "error" => e.to_string());
        }
    };

    exitcode::OK
}
