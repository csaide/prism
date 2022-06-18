// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

use std::sync::Arc;

use tokio::sync::watch;

use crate::rpc::raft::Payload;

use super::{Client, Log, Metadata, StateMachine};

pub struct Commiter<P, S> {
    logger: slog::Logger,
    metadata: Arc<Metadata<P>>,
    log: Arc<Log>,
    state_machine: Arc<S>,
    commit_rx: watch::Receiver<()>,
}

impl<P, S> Commiter<P, S>
where
    P: Client + Clone,
    S: StateMachine,
{
    pub fn new(
        logger: &slog::Logger,
        metadata: Arc<Metadata<P>>,
        log: Arc<Log>,
        state_machine: Arc<S>,
        commit_rx: watch::Receiver<()>,
    ) -> Commiter<P, S> {
        Commiter {
            logger: logger.new(o!("module" => "commiter")),
            metadata,
            log,
            state_machine,
            commit_rx,
        }
    }

    pub async fn exec(&mut self) {
        while let Ok(_) = self.commit_rx.changed().await {
            let last_applied_idx = self.metadata.get_last_applied_idx();
            let commit_idx = self.metadata.get_commit_idx();

            let mut entries = if commit_idx > last_applied_idx {
                match self.log.range(last_applied_idx + 1, commit_idx + 1) {
                    Ok(entries) => entries,
                    Err(e) => {
                        error!(self.logger, "Failed to pull new log entries to apply."; "error" => e.to_string());
                        continue;
                    }
                }
            } else {
                continue;
            };

            for entry in entries.drain(..) {
                use Payload::*;
                match entry {
                    Command(cmd) => self.state_machine.apply(cmd),
                    _ => continue,
                }
            }

            self.metadata.set_commit_idx(commit_idx);
        }
    }
}
