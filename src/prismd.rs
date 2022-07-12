// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

use std::collections::HashMap;
use std::ffi::OsString;
use std::time::Duration;

use crate::raft::{Module, Peer};
use crate::rpc::cluster::{AddRequest, ClusterClient};
use crate::rpc::server::serve;
use crate::{hash, logging};

use exitcode::ExitCode;
use futures::stream::StreamExt;
use signal_hook::consts::signal::*;
use signal_hook_tokio::Signals;
use structopt::clap::{crate_version, AppSettings};
use structopt::StructOpt;
use tokio::select;
use tokio::time::interval;

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
    log_config: logging::Config,
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
        default_value = "db",
        takes_value = true
    )]
    db_path: String,
    #[structopt(
        long = "bootstrap",
        short = "b",
        env = "PRISM_BOOTSTRAP_PEERS",
        help = "The initial set of peers to cluster with.",
        long_help = "The initial set of peers to bootstrap a new cluster with. Note that this is ignored when operating within an existing cluster.",
        default_value = "",
        takes_value = true
    )]
    bootstrap: Option<Vec<String>>,
    #[structopt(
        long = "join",
        short = "J",
        env = "PRISM_JOIN_PEER",
        help = "The initial set of peers to cluster with.",
        long_help = "The initial set of peers to bootstrap a new cluster with. Note that this is ignored when operating within an existing cluster.",
        takes_value = true
    )]
    join: Option<String>,
    #[structopt(
        long = "local-host",
        short = "a",
        env = "PRISM_LOCAL_HOST",
        help = "The hostname of this peer.",
        takes_value = true
    )]
    local_host: Option<String>,
    #[structopt(
        long = "replica",
        short = "r",
        env = "PRISM_REPLICA",
        help = "Run in replica mode.",
        takes_value = false
    )]
    replica: bool,
}

pub async fn run(args: Vec<OsString>) -> ExitCode {
    let cfg = match crate::base::config::<PrismdConfig>(args, PRISMD) {
        Ok(cfg) => cfg,
        Err((code, _)) if code == exitcode::CONFIG => return code,
        Err((code, msg)) => {
            println!("{}", msg);
            return code;
        }
    };

    let root_logger = logging::new(&cfg.log_config, PRISMD, crate_version!());

    let db = match sled::Config::new().path(&cfg.db_path).open() {
        Ok(db) => db,
        Err(e) => {
            error!(root_logger, "Failed to open internal database."; "error" => e.to_string(), "path" => &cfg.db_path);
            return exitcode::IOERR;
        }
    };
    let sm = hash::HashState::new(&root_logger);

    let mut peers = HashMap::default();
    if let Some(mut bootstrap) = cfg.bootstrap {
        for peer_addr in bootstrap.drain(..) {
            let peer = Peer::voter(peer_addr.clone());
            peers.insert(peer_addr, peer);
        }
    }

    let id = format!(
        "grpc://{}:{}",
        cfg.local_host.unwrap_or_else(|| String::from("127.0.0.1")),
        cfg.rpc_port
    );

    if let Some(mut join) = cfg.join {
        let join_logger = root_logger.clone();
        let member = id.clone();
        tokio::task::spawn(async move {
            let mut ticker = interval(Duration::from_millis(100));
            loop {
                ticker.tick().await;
                let mut client = match ClusterClient::connect(join.clone()).await {
                    Ok(client) => client,
                    Err(e) => {
                        error!(join_logger, "Failed to connect to leader to join cluster."; "error" => e.to_string());
                        return;
                    }
                };
                let req = AddRequest {
                    member: member.clone(),
                    replica: cfg.replica,
                };
                match client.add(req).await {
                    Err(e) => {
                        error!(join_logger, "Failed to add self to cluster... retrying."; "error" => e.to_string());
                        continue;
                    }
                    Ok(res) => {
                        let res = res.into_inner();
                        match res.status.as_str() {
                            "OK" => {
                                info!(join_logger, "Added self to cluster.");
                                break;
                            }
                            _ => {
                                warn!(join_logger, "Failed to add self to cluster... retrying."; "error" => "peer not leader", "new_leader" => &res.leader_hint);
                                join = res.leader_hint;
                                continue;
                            }
                        }
                    }
                };
            }
        });
    }

    let mut cm = match Module::new(id, peers, &root_logger, &db, sm, cfg.replica) {
        Ok(cm) => cm,
        Err(e) => {
            error!(root_logger, "Failed to create internal consensus module."; "error" => e.to_string());
            return exitcode::IOERR;
        }
    };

    let addr = format!("0.0.0.0:{}", cfg.rpc_port).parse().unwrap();
    let repo = cm.get_repo().unwrap();

    let mut signals = match Signals::new(&[SIGHUP, SIGTERM, SIGINT, SIGQUIT]) {
        Ok(signals) => signals,
        Err(e) => {
            error!(root_logger, "Failed to register signal handler."; "error" => e.to_string());
            return exitcode::IOERR;
        }
    };
    let fut = async move {
        signals.next().await;
    };

    let srv = serve(addr, repo, root_logger.clone(), fut);

    select! {
        _ = cm.start() => {
            error!(root_logger, "Consensus module exited.....");
        }
        _ = srv => {
            error!(root_logger, "RPC server exited......")
        }
    }
    exitcode::IOERR
}
