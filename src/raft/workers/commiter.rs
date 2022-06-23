// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

use std::sync::Arc;

use tokio::sync::watch;

use super::{Client, Entry, Log, State, StateMachine, Watcher};

pub struct Commiter<P, S> {
    logger: slog::Logger,
    state: Arc<State<P>>,
    log: Arc<Log>,
    state_machine: Arc<S>,
    commit_rx: watch::Receiver<()>,
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
        log: Arc<Log>,
        state_machine: Arc<S>,
        commit_rx: watch::Receiver<()>,
        watcher: Arc<Watcher>,
    ) -> Commiter<P, S> {
        Commiter {
            logger: logger.new(o!("module" => "commiter")),
            state,
            log,
            state_machine,
            commit_rx,
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

            let mut entries = match self.log.range(last_applied_idx + 1, commit_idx + 1) {
                Ok(entries) => entries,
                Err(e) => {
                    error!(self.logger, "Failed to pull new log entries to apply."; "error" => e.to_string());
                    continue;
                }
            };

            for (idx, entry) in entries.drain(..) {
                use Entry::*;
                match entry {
                    // Put response on oneshot tx here....
                    Command(cmd) => {
                        info!(self.logger, "Executing command."; "idx" => idx);
                        let result = self.state_machine.apply(cmd);
                        self.watcher.command_applied(idx, result)
                    }
                    _ => continue,
                }
            }

            self.state.set_last_applied_idx(commit_idx);
        }
    }
}
