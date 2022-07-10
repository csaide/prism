// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

use std::ffi::OsString;

use exitcode::ExitCode;
use structopt::clap::{crate_version, AppSettings};
use structopt::StructOpt;

use crate::hash::Query;
use crate::rpc::frontend::{FrontendClient, MutateRequest, ReadRequest};
use crate::{hash, logging};

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
    log_config: logging::Config,
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
    #[structopt(subcommand)]
    command: Command,
}

#[derive(Debug, Clone, StructOpt)]
enum Command {
    Insert {
        #[structopt(
            long = "key",
            short = "k",
            env = "PRISM_KEY",
            help = "The key to operate on.",
            long_help = "The named value to interact with.",
            required = true,
            takes_value = true
        )]
        key: String,
        #[structopt(
            long = "value",
            short = "v",
            env = "PRISM_VALUE",
            help = "The value to insert at the specified key.",
            long_help = "The value to insert at the named key.",
            required = true,
            takes_value = true
        )]
        value: u128,
    },
    Remove {
        #[structopt(
            long = "key",
            short = "k",
            env = "PRISM_KEY",
            help = "The key to operate on.",
            long_help = "The named value to interact with.",
            required = true,
            takes_value = true
        )]
        key: String,
    },
    Query {
        #[structopt(
            long = "key",
            short = "k",
            env = "PRISM_KEY",
            help = "The key to operate on.",
            long_help = "The named value to interact with.",
            required = true,
            takes_value = true
        )]
        key: String,
    },
}

pub async fn run(args: Vec<OsString>) -> ExitCode {
    let cfg = match crate::base::config::<PrismctlConfig>(args, PRISMCTL) {
        Ok(cfg) => cfg,
        Err((code, _)) if code == exitcode::CONFIG => return code,
        Err((code, msg)) => {
            println!("{}", msg);
            return code;
        }
    };

    let root_logger = logging::new(&cfg.log_config, PRISMCTL, crate_version!());
    info!(root_logger, "Hello world!");

    let mut client = match FrontendClient::connect(cfg.leader).await {
        Ok(client) => client,
        Err(e) => {
            error!(root_logger, "Failed to connect to leader."; "error" => e.to_string());
            return exitcode::IOERR;
        }
    };

    match cfg.command {
        Command::Insert { key, value } => {
            let cmd = hash::Command::Insert(key, value);
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
        }
        Command::Remove { key } => {
            let cmd = hash::Command::Remove(key);
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
        }
        Command::Query { key } => {
            let query = Query { key: key.clone() };
            let query = query.to_bytes();
            let req = ReadRequest { query };

            match client.read(req).await {
                Ok(resp) => {
                    let resp = resp.into_inner();
                    let val: Option<u128> = bincode::deserialize(resp.response.as_slice())
                        .expect("Failed to deserialize value");
                    info!(root_logger, "Got response!"; "status" => resp.status, "leader" => resp.leader_hint, "key" => key, "value" => val);
                }
                Err(e) => {
                    error!(root_logger, "Failed to call read rpc."; "error" => e.to_string());
                }
            }
        }
    }

    exitcode::OK
}
