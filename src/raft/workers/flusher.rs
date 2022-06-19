// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

use std::{sync::Arc, time::Duration};

use tokio::time::interval;

use crate::raft::{Client, Log, State};

pub struct Flusher<P> {
    logger: slog::Logger,
    log: Arc<Log>,
    state: Arc<State<P>>,
}

impl<P> Flusher<P>
where
    P: Client + Clone,
{
    pub fn new(logger: &slog::Logger, log: Arc<Log>, state: Arc<State<P>>) -> Flusher<P> {
        Flusher {
            logger: logger.new(o!("module" => "flusher")),
            log,
            state,
        }
    }

    pub async fn exec(&self) {
        let mut ticker = interval(Duration::from_millis(1000));
        loop {
            ticker.tick().await;

            if let Err(e) = self.log.flush().await {
                error!(self.logger, "Failed to flush log to disk."; "error" => e.to_string());
                self.state.transition_dead();
                return;
            }
        }
    }
}
