// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

use std::sync::Arc;

use super::{AppendEntriesRequest, AppendEntriesResponse, Client, Log, Peer, Result, State};

pub struct Syncer<P> {
    logger: slog::Logger,
    state: Arc<State<P>>,
    log: Log,
    peer: Peer<P>,
}

impl<P> Syncer<P>
where
    P: Client + Send + Clone + 'static,
{
    pub fn new(logger: &slog::Logger, state: Arc<State<P>>, log: Log, peer: Peer<P>) -> Syncer<P> {
        Syncer {
            logger: logger.clone(),
            state,
            log,
            peer,
        }
    }

    pub async fn exec(&mut self) {
        while !self.state.is_dead() {
            let term = self.state.get_current_term();
            let (entries, resp) = self.send_request(term).await;
            let resp = match resp {
                Ok(resp) => resp,
                Err(e) => {
                    error!(self.logger, "Failed to execute append entries request."; "error" => e.to_string());
                    continue;
                }
            };
            if resp.success {
                self.peer.next_idx += entries as u64;
                self.peer.match_idx = self.peer.next_idx - 1;
                // We are being "dumb" and just sending the entire log all at once
                // this is really not an effective solution, so come back and fix this.
                return;
            } else {
                self.peer.next_idx -= 1;
                continue;
            }
        }
    }

    pub async fn send_request(&mut self, term: u64) -> (usize, Result<AppendEntriesResponse>) {
        let prev_log_idx = self.peer.next_idx - 1;
        let mut prev_log_term = 0;
        if prev_log_idx > 0 {
            prev_log_term = self
                .log
                .get(prev_log_idx)
                .unwrap_or(None)
                .map(|entry| entry.term())
                .unwrap_or(0);
        }

        let entries = self
            .log
            .range(self.peer.next_idx..)
            .entries()
            .filter_map(|entry| entry.ok())
            .collect();
        let req = AppendEntriesRequest {
            leader_commit_idx: self.state.get_commit_idx(),
            leader_id: self.state.id.clone(),
            prev_log_idx,
            prev_log_term,
            term,
            entries,
        };
        (req.entries.len(), self.peer.append(req).await)
    }
}
