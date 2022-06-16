// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

use std::sync::Arc;

use tokio::sync::{mpsc, watch};
use tokio::time::{timeout, Duration};

use crate::rpc::raft::{AppendRequest, AppendResponse, Entry};

use super::{Client, Log, Metadata, Result};

pub struct Leader<P> {
    logger: slog::Logger,
    metadata: Arc<Metadata<P>>,
    log: Arc<Log>,
    commit_tx: Arc<watch::Sender<()>>,
    submit_rx: watch::Receiver<()>,
}

impl<P> Leader<P>
where
    P: Client + Send + Clone + 'static,
{
    pub fn new(
        logger: &slog::Logger,
        metadata: Arc<Metadata<P>>,
        log: Arc<Log>,
        commit_tx: Arc<watch::Sender<()>>,
        submit_rx: watch::Receiver<()>,
    ) -> Leader<P> {
        Leader {
            logger: logger.clone(),
            metadata,
            log,
            commit_tx,
            submit_rx,
        }
    }
    pub async fn exec(&mut self) {
        let (last_log_idx, _) = match self.log.last_log_idx_and_term() {
            Ok(tuple) => tuple,
            Err(e) => {
                error!(self.logger, "Failed to retrieve last log i64."; "error" => e.to_string());
                return;
            }
        };
        if let Err(e) = self.metadata.transition_leader(last_log_idx) {
            error!(self.logger, "Failed to transition self to leader."; "error" => e.to_string());
            self.metadata.transition_follower(None);
            return;
        }

        let duration = Duration::from_millis(50);
        while self.metadata.is_leader() {
            let saved_term = *self.metadata.current_term.read().unwrap();

            let (tx, rx) = mpsc::channel::<(usize, usize, Result<AppendResponse>)>(
                self.metadata.peers.lock().unwrap().len(),
            );
            self.send_requests(saved_term, &tx);
            drop(tx);

            self.handle_responses(saved_term, rx).await;
            match timeout(duration, self.submit_rx.changed()).await {
                Err(_) => {}                         // Timed out waiting for a submit, fire heartbeat.
                Ok(resp) if resp.is_err() => return, // Channel was closed, this is an unexpected situation, abort.
                _ => {}                              // Happy path we got a submit, fire append.
            }
        }
    }

    pub fn send_requests(
        &self,
        saved_term: i64,
        tx: &mpsc::Sender<(usize, usize, Result<AppendResponse>)>,
    ) {
        let peers = self.metadata.peers.lock().unwrap();
        for (peer_idx, peer) in peers.iter().enumerate() {
            let prev_log_idx = peer.next_idx - 1;
            let mut prev_log_term = -1;
            if prev_log_idx >= 0 {
                prev_log_term = match self.log.get(prev_log_idx) {
                    Ok(entry) => entry.term(),
                    Err(e) => {
                        error!(self.logger, "Failed to pull previous log entry."; "error" => e.to_string());
                        -1
                    }
                };
            }

            let entries = match self.log.range(peer.next_idx, i64::MAX) {
                Ok(mut entries) => entries
                    .drain(..)
                    .map(|payload| Entry {
                        payload: Some(payload),
                    })
                    .collect(),
                Err(e) => {
                    error!(self.logger, "Failed to return range of entries."; "error" => e.to_string());
                    Vec::default()
                }
            };

            let req = AppendRequest {
                leader_commit_idx: *self.metadata.commit_idx.read().unwrap(),
                leader_id: self.metadata.id.clone(),
                prev_log_idx,
                prev_log_term,
                term: saved_term,
                entries,
            };

            let mut cli = peer.clone();
            let tx = tx.clone();
            tokio::task::spawn(async move {
                let entries = req.entries.len();
                let resp = cli.append(req).await;
                let _ = tx.send((entries, peer_idx, resp)).await;
            });
        }
    }

    pub async fn handle_responses(
        &self,
        saved_term: i64,
        mut rx: mpsc::Receiver<(usize, usize, Result<AppendResponse>)>,
    ) {
        while let Some(resp) = rx.recv().await {
            let (entries, peer_idx, resp) = resp;
            let resp = match resp {
                Ok(resp) => resp,
                Err(e) => {
                    error!(self.logger, "Failed to execute VoteRequest rpc."; "error" => e.to_string());
                    continue;
                }
            };

            if !self.metadata.is_leader() {
                return;
            }
            if resp.term > saved_term {
                self.metadata.transition_follower(Some(resp.term));
                return;
            }
            if resp.term < saved_term {
                continue;
            }

            let mut commit = false;
            let mut peers = self.metadata.peers.lock().unwrap();
            let mut peer = &mut peers[peer_idx];
            if resp.success {
                peer.next_idx += entries as i64;
                peer.match_idx = peer.next_idx - 1;

                let saved_commit = *self.metadata.commit_idx.read().unwrap();
                let start = saved_commit + 1;
                for idx in start..self.log.len() as i64 {
                    let entry = match self.log.get(idx) {
                        Ok(entry) => entry,
                        Err(e) => {
                            error!(self.logger, "Failed to pull entry."; "error" => e.to_string());
                            continue;
                        }
                    };
                    if entry.term() != saved_term {
                        continue;
                    }
                    let mut matches = 1;
                    for peer in peers.iter() {
                        if peer.match_idx >= idx {
                            matches += 1;
                        }
                    }
                    if matches * 2 > peers.len() {
                        let mut commit_idx = self.metadata.commit_idx.write().unwrap();
                        *commit_idx = idx;
                    }
                }
                if saved_commit != *self.metadata.commit_idx.read().unwrap() {
                    commit = true;
                }
            } else {
                peer.next_idx -= 1;
            }
            if commit {
                if let Err(e) = self.commit_tx.send(()) {
                    error!(self.logger, "Failed to send commit notification."; "error" => e.to_string());
                }
            }
        }
    }
}
