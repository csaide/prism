// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

use std::sync::Arc;

use tokio::sync::{mpsc, watch};
use tokio::time::{timeout, Duration};

use super::{AppendEntriesRequest, AppendEntriesResponse, Client, Log, Metadata, Result};

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
                error!(self.logger, "Failed to retrieve last log u128."; "error" => e.to_string());
                return;
            }
        };
        self.metadata.transition_leader(last_log_idx);

        let duration = Duration::from_millis(50);
        while self.metadata.is_leader() {
            let saved_term = self.metadata.get_current_term();

            let (tx, rx) = mpsc::channel::<(usize, String, Result<AppendEntriesResponse>)>(
                self.metadata.peers.len(),
            );
            self.send_requests(saved_term, &tx);
            drop(tx);

            self.handle_responses(saved_term, rx).await;
            match timeout(duration, self.submit_rx.changed()).await {
                Ok(resp) if resp.is_err() => return, // Channel was closed, this is an unexpected situation, abort.
                _ => {}                              // Happy path we got a submit, fire append.
            }
        }
    }

    pub fn send_requests(
        &self,
        saved_term: u128,
        tx: &mpsc::Sender<(usize, String, Result<AppendEntriesResponse>)>,
    ) {
        for (peer_id, peer) in self.metadata.peers.lock().iter() {
            let prev_log_idx = peer.next_idx - 1;
            let mut prev_log_term = 0;
            if prev_log_idx > 0 {
                prev_log_term = self
                    .log
                    .get(prev_log_idx)
                    .map(|entry| entry.term())
                    .unwrap_or(0);
            }

            let entries = self.log.range(peer.next_idx, u128::MAX).unwrap_or_default();

            let req = AppendEntriesRequest {
                leader_commit_idx: self.metadata.get_commit_idx(),
                leader_id: self.metadata.id.clone(),
                prev_log_idx,
                prev_log_term,
                term: saved_term,
                entries,
            };

            let mut cli = peer.clone();
            let tx = tx.clone();
            let id = peer_id.clone();
            tokio::task::spawn(async move {
                let entries = req.entries.len();
                let resp = cli.append(req).await;
                let _ = tx.send((entries, id, resp)).await;
            });
        }
    }

    pub async fn handle_responses(
        &self,
        saved_term: u128,
        mut rx: mpsc::Receiver<(usize, String, Result<AppendEntriesResponse>)>,
    ) {
        while let Some(evt) = rx.recv().await {
            let (entries, peer_id, resp) = evt;
            let resp = match resp {
                Ok(resp) => resp,
                Err(e) => {
                    error!(self.logger, "Failed to execute AppendEntries rpc."; "error" => e.to_string());
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

            let mut locked_peers = self.metadata.peers.lock();
            let mut peer = match locked_peers.get_mut(&peer_id) {
                Some(peer) => peer,
                None => continue,
            };
            if !resp.success {
                // info!(self.logger, "failed to successfully append.");
                peer.next_idx -= 1;
                continue;
            }

            peer.next_idx += entries as u128;
            peer.match_idx = peer.next_idx - 1;

            let saved_commit = self.metadata.get_commit_idx();
            let start = saved_commit + 1;
            for idx in start..self.log.len() as u128 {
                match self.log.get(idx) {
                    Ok(entry) if entry.term() != saved_term => continue,
                    Err(e) => {
                        error!(self.logger, "Failed to pull entry."; "error" => e.to_string());
                        continue;
                    }
                    _ => {}
                };
                if locked_peers.idx_matches(idx) {
                    self.metadata.set_commit_idx(idx);
                }
            }
            if saved_commit != self.metadata.get_commit_idx() {
                if let Err(e) = self.commit_tx.send(()) {
                    error!(self.logger, "Failed to send commit notification."; "error" => e.to_string());
                }
            }
        }
    }
}
