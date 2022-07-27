// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

use std::sync::Arc;

use tokio::sync::watch;

use super::{Client, Entry, Log, State, StateMachine, Watcher};

pub struct Commiter<P, S> {
    logger: slog::Logger,
    state: Arc<State<P>>,
    log: Log,
    state_machine: Arc<S>,
    commit_rx: watch::Receiver<()>,
    applied_tx: Arc<watch::Sender<()>>,
    watcher: Arc<Watcher>,
}

impl<P, S> Commiter<P, S>
where
    P: Client + Clone,
    S: StateMachine,
{
    pub fn new(
        logger: &slog::Logger,
        state: Arc<State<P>>,
        log: Log,
        state_machine: Arc<S>,
        commit_rx: watch::Receiver<()>,
        applied_tx: Arc<watch::Sender<()>>,
        watcher: Arc<Watcher>,
    ) -> Commiter<P, S> {
        Commiter {
            logger: logger.new(o!("module" => "commiter")),
            state,
            log,
            state_machine,
            commit_rx,
            applied_tx,
            watcher,
        }
    }

    pub async fn exec(&mut self) {
        while !self.state.is_dead() && self.commit_rx.changed().await.is_ok() {
            let last_applied_idx = self.state.get_last_applied_idx();
            let commit_idx = self.state.get_commit_idx();

            if commit_idx <= last_applied_idx {
                continue;
            };

            let entries = self
                .log
                .range(last_applied_idx + 1..=commit_idx + 1)
                .filter_map(|entry| entry.ok());

            for (idx, entry) in entries {
                use Entry::*;
                match entry {
                    // Put response on oneshot tx here....
                    Command(cmd) => {
                        info!(self.logger, "Executing command."; "idx" => idx);
                        let result = self.state_machine.apply(cmd);
                        self.watcher.command_applied(idx, result)
                    }
                    ClusterConfig(_) => {
                        info!(self.logger, "Commited cluster config."; "idx" => idx);
                        self.watcher.cluster_config_applied(idx);
                    }
                    Registration(_) => {
                        info!(self.logger, "Commited client registration."; "idx" => idx);
                        self.watcher.registration_applied(idx);
                    }
                    Noop(_) => {
                        info!(self.logger, "Commited noop leader append."; "idx" => idx);
                        // TODO(csaide): Add watcher integration.
                    }
                    None => unreachable!(),
                }
            }

            self.state.set_last_applied_idx(commit_idx);
            if let Err(e) = self.applied_tx.send(()) {
                error!(self.logger, "Failed to send applied broadcast."; "error" => e.to_string());
                return;
            }
        }
    }
}
