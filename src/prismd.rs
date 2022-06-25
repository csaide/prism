// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

use std::collections::HashMap;
use std::time::Duration;

use crate::raft::{ConsensusMod, Peer};
use crate::rpc::cluster::{AddRequest, ClusterClient};
use crate::rpc::server::serve;
use crate::{hash, log};

use exitcode::ExitCode;
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
}

pub async fn run() -> ExitCode {
    let cfg = match crate::base_config::<PrismdConfig>(PRISMD) {
        Ok(cfg) => cfg,
        Err(code) => return code,
    };

    let root_logger = log::new(&cfg.log_config, PRISMD, crate_version!());

    let db = match sled::Config::new()
        .path(&cfg.db_path)
        .temporary(true)
        .open()
    {
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
            let peer = Peer::new(peer_addr.clone());
            peers.insert(peer_addr, peer);
        }
    }

    if let Some(join) = cfg.join {
        let join_logger = root_logger.clone();
        let port = cfg.rpc_port;
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
                    member: format!("grpc://127.0.0.1:{}", port),
                    replica: false,
                };
                match client.add(req).await {
                    Err(e) => {
                        error!(join_logger, "Failed to add self to cluster... retrying."; "error" => e.to_string());

                        continue;
                    }
                    _ => break,
                };
            }
            info!(join_logger, "Added self to cluster.")
        });
    }

    let mut cm = match ConsensusMod::new(
        format!("grpc://127.0.0.1:{}", cfg.rpc_port),
        peers,
        &root_logger,
        &db,
        sm,
    ) {
        Ok(cm) => cm,
        Err(e) => {
            error!(root_logger, "Failed to create internal consensus module."; "error" => e.to_string());
            return exitcode::IOERR;
        }
    };

    let addr = format!("0.0.0.0:{}", cfg.rpc_port).parse().unwrap();
    let repo = cm.get_repo().unwrap();
    let srv = serve(addr, repo, root_logger.clone());

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
