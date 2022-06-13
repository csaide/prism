// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

use exitcode::ExitCode;
use structopt::clap::{crate_version, AppSettings};
use structopt::StructOpt;
use tokio::select;
use tokio::time::{interval, Duration};

use crate::log;
use crate::raft::ConsensusMod;
use crate::rpc::raft::RaftServiceClient;
use crate::rpc::server::serve;

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
    let mut cm1 = ConsensusMod::new(addr1.to_string(), Vec::default(), &root_logger, &db1).unwrap();
    let sock_addr1 = addr1.replace("grpc://", "").parse().unwrap();
    let srv1 = serve(sock_addr1, cm1.clone(), root_logger.clone());

    let db2 = sled::Config::default().temporary(true).open().unwrap();
    let mut cm2 = ConsensusMod::new(addr2.to_string(), Vec::default(), &root_logger, &db2).unwrap();
    let sock_addr2 = addr2.replace("grpc://", "").parse().unwrap();
    let srv2 = serve(sock_addr2, cm2.clone(), root_logger.clone());

    let db3 = sled::Config::default().temporary(true).open().unwrap();
    let mut cm3 = ConsensusMod::new(addr3.to_string(), Vec::default(), &root_logger, &db3).unwrap();
    let sock_addr3 = addr3.replace("grpc://", "").parse().unwrap();
    let srv3 = serve(sock_addr3, cm3.clone(), root_logger.clone());

    let cm_submit1 = cm1.clone();
    let cm_submit2 = cm2.clone();
    let cm_submit3 = cm3.clone();

    let submit_logger = root_logger.clone();
    let submit = async move {
        let payload = vec![0x01, 0x02, 0x03];

        let mut interval = interval(Duration::from_secs(1));
        loop {
            interval.tick().await;
            if cm_submit1.is_leader() {
                cm_submit1.submit(payload.clone()).await.unwrap();
            } else if cm_submit2.is_leader() {
                cm_submit2.submit(payload.clone()).await.unwrap();
            } else if cm_submit3.is_leader() {
                cm_submit3.submit(payload.clone()).await.unwrap();
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

    let peer1 = RaftServiceClient::connect(addr1).await.unwrap();
    let peer2 = RaftServiceClient::connect(addr2).await.unwrap();
    let peer3 = RaftServiceClient::connect(addr3).await.unwrap();

    cm1.append_peer(peer2.clone());
    cm1.append_peer(peer3.clone());

    cm2.append_peer(peer1.clone());
    cm2.append_peer(peer3.clone());

    cm3.append_peer(peer1.clone());
    cm3.append_peer(peer2.clone());

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
