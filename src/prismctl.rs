// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

use std::collections::HashMap;

use exitcode::ExitCode;
use structopt::clap::{crate_version, AppSettings};
use structopt::StructOpt;
use tokio::select;
use tokio::time::{interval, Duration};

use crate::hash::Command;
use crate::raft::{ConsensusMod, Peer};
use crate::rpc::raft::RaftServiceClient;
use crate::rpc::server::serve;
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
}

pub async fn run() -> ExitCode {
    let cfg = match crate::base_config::<PrismctlConfig>(PRISMCTL) {
        Ok(cfg) => cfg,
        Err(code) => return code,
    };

    let root_logger = log::new(&cfg.log_config, PRISMCTL, crate_version!());
    info!(root_logger, "Hello world!");

    let addr1 = "grpc://127.0.0.1:8081";
    let addr2 = "grpc://127.0.0.1:8082";
    let addr3 = "grpc://127.0.0.1:8083";

    let db1 = sled::Config::default().temporary(true).open().unwrap();
    let sm1 = hash::HashState::new();
    let mut cm1 = ConsensusMod::new(
        addr1.to_string(),
        HashMap::default(),
        &root_logger,
        &db1,
        sm1.clone(),
    )
    .unwrap();
    let sock_addr1 = addr1.replace("grpc://", "").parse().unwrap();
    let srv1 = serve(sock_addr1, cm1.clone(), root_logger.clone());

    let db2 = sled::Config::default().temporary(true).open().unwrap();
    let sm2 = hash::HashState::new();
    let mut cm2 = ConsensusMod::new(
        addr2.to_string(),
        HashMap::default(),
        &root_logger,
        &db2,
        sm2.clone(),
    )
    .unwrap();
    let sock_addr2 = addr2.replace("grpc://", "").parse().unwrap();
    let srv2 = serve(sock_addr2, cm2.clone(), root_logger.clone());

    let db3 = sled::Config::default().temporary(true).open().unwrap();
    let sm3 = hash::HashState::new();
    let mut cm3 = ConsensusMod::new(
        addr3.to_string(),
        HashMap::default(),
        &root_logger,
        &db3,
        sm3.clone(),
    )
    .unwrap();
    let sock_addr3 = addr3.replace("grpc://", "").parse().unwrap();
    let srv3 = serve(sock_addr3, cm3.clone(), root_logger.clone());

    let cm_submit1 = cm1.clone();
    let cm_submit2 = cm2.clone();
    let cm_submit3 = cm3.clone();

    let submit_logger = root_logger.clone();
    let submit = async move {
        let mut interval = interval(Duration::from_secs(1));
        let mut idx = 0;
        loop {
            let payload = Command::Insert(String::from("hello"), idx);
            let payload = payload.to_bytes();

            interval.tick().await;

            if cm_submit1.is_leader() {
                cm_submit1.submit_command(payload.clone()).await.unwrap();
            } else if cm_submit2.is_leader() {
                cm_submit2.submit_command(payload.clone()).await.unwrap();
            } else if cm_submit3.is_leader() {
                cm_submit3.submit_command(payload.clone()).await.unwrap();
            }

            interval.tick().await;

            for entry in sm1.dump() {
                info!(submit_logger, "SM1 Entry: {:?}", entry);
            }
            for entry in sm2.dump() {
                info!(submit_logger, "SM2 Entry: {:?}", entry);
            }
            for entry in sm3.dump() {
                info!(submit_logger, "SM3 Entry: {:?}", entry);
            }

            for entry in cm_submit1.dump().unwrap() {
                info!(submit_logger, "CM1 Entry: {:?}", entry);
            }
            for entry in cm_submit2.dump().unwrap() {
                info!(submit_logger, "CM2 Entry: {:?}", entry);
            }
            for entry in cm_submit3.dump().unwrap() {
                info!(submit_logger, "CM3 Entry: {:?}", entry);
            }

            idx += 1;
        }
    };

    let server_logger = root_logger.clone();
    tokio::task::spawn(async move {
        select! {
            res = srv1 => {
                if let Err(e) = res {
                    error!(server_logger, "srv1 exited main loop...."; "error" => e.to_string());
                }
            }
            res = srv2 => {
                if let Err(e) = res {
                    error!(server_logger, "srv2 exited main loop...."; "error" => e.to_string());
                }
            }
            res = srv3 => {
                if let Err(e) = res {
                    error!(server_logger, "srv3 exited main loop...."; "error" => e.to_string());
                }
            }
        }
    });
    tokio::time::sleep(Duration::from_secs(1)).await;

    let peer1 = Peer::with_client(RaftServiceClient::connect(addr1).await.unwrap());
    let peer2 = Peer::with_client(RaftServiceClient::connect(addr2).await.unwrap());
    let peer3 = Peer::with_client(RaftServiceClient::connect(addr3).await.unwrap());

    cm1.append_peer(addr2.to_string(), peer2.clone());
    cm1.append_peer(addr3.to_string(), peer3.clone());

    cm2.append_peer(addr1.to_string(), peer1.clone());
    cm2.append_peer(addr3.to_string(), peer3.clone());

    cm3.append_peer(addr1.to_string(), peer1.clone());
    cm3.append_peer(addr2.to_string(), peer2.clone());

    select! {
        _ = cm1.start() => {
            error!(root_logger, "cm1 exited main loop....");
        }
        _ = cm2.start() => {
            error!(root_logger, "cm2 exited main loop....");
        }
        _ = cm3.start() => {
            error!(root_logger, "cm3 exited main loop....");
        }
        _ = submit => {
            error!(root_logger, "submit routine exited....")
        }
    };

    exitcode::OK
}
