// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

use std::sync::Arc;

use tokio::sync::{mpsc, watch};
use tokio::time::{timeout, Duration};

use crate::raft::Quorum;

use super::{Client, Log, State};

#[derive(Debug)]
pub struct Leader<P> {
    logger: slog::Logger,
    state: Arc<State<P>>,
    log: Log,
    submit_rx: mpsc::Receiver<()>,
    quorum: Quorum<P>,
}

impl<P> Leader<P>
where
    P: Client + Send + Clone + 'static,
{
    pub fn new(
        logger: &slog::Logger,
        state: Arc<State<P>>,
        log: Log,
        commit_tx: Arc<watch::Sender<()>>,
        submit_rx: mpsc::Receiver<()>,
    ) -> Leader<P> {
        let quorum = Quorum::new(logger, state.clone(), log.clone(), commit_tx);
        Leader {
            logger: logger.clone(),
            state,
            log,
            submit_rx,
            quorum,
        }
    }

    pub async fn exec(&mut self) {
        let (last_log_idx, _) = match self.log.last_log_idx_and_term() {
            Ok(tuple) => tuple,
            Err(e) => {
                error!(self.logger, "Failed to retrieve last log u64."; "error" => e.to_string());
                return;
            }
        };
        self.state.transition_leader(last_log_idx);

        let duration = Duration::from_millis(50);
        while self.state.is_leader() {
            if !self.quorum.exec().await {
                if cfg!(test) {
                    return;
                }
                continue;
            }

            match timeout(duration, self.submit_rx.recv()).await {
                Ok(resp) if resp.is_none() => return, // Channel was closed, this is an unexpected situation, abort.
                _ => {}                               // Happy path we got a submit, fire append.
            }

            // If we are in test mode lets break out to simplify testing.
            if cfg!(test) {
                return;
            }
        }
    }
}
