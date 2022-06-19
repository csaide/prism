// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

use std::sync::Arc;

use rand::Rng;
use tokio::sync::watch;
use tokio::time::{timeout, Duration};

use super::{Client, Metadata};

pub struct Follower<P> {
    logger: slog::Logger,
    metadata: Arc<Metadata<P>>,
    heartbeat_rx: watch::Receiver<()>,
    min_timeout_ms: u64,
    max_timeout_ms: u64,
}

impl<P> Follower<P>
where
    P: Client + Clone,
{
    pub fn new(
        logger: &slog::Logger,
        metadata: Arc<Metadata<P>>,
        heartbeat_rx: watch::Receiver<()>,
    ) -> Follower<P> {
        Follower {
            metadata,
            logger: logger.new(o!("module" => "follower")),
            heartbeat_rx,
            min_timeout_ms: 150,
            max_timeout_ms: 300,
        }
    }
    pub async fn exec(&mut self) {
        let dur = rand::thread_rng().gen_range(self.min_timeout_ms..self.max_timeout_ms + 1);
        let dur = Duration::from_millis(dur);

        while !self.metadata.is_dead() {
            let timed_out = timeout(dur, self.heartbeat_rx.changed()).await.is_err();

            if timed_out {
                debug!(
                    self.logger,
                    "Timed out waiting for heartbeat starting election."
                );
                self.metadata.lost_leader();
                return;
            }
            self.metadata.saw_leader();
            debug!(self.logger, "Got heartbeat re-setting heartbeat.");
        }
    }
}

#[cfg(test)]
#[cfg(not(tarpaulin_include))]
mod tests {
    use std::collections::HashMap;

    use crate::log;
    use crate::raft::test_harness::MockPeer;

    use super::*;

    #[test]
    fn test_follower() {
        let logger = log::noop();

        let (heartbeat_tx, heartbeat_rx) = watch::channel(());
        let db = sled::Config::new()
            .temporary(true)
            .open()
            .expect("Failed to open database.");
        let metadata = Arc::new(
            Metadata::<MockPeer>::new(String::from("id"), HashMap::default(), &db)
                .expect("Failed to create shared metadata."),
        );
        let mut follower = Follower::new(&logger, metadata, heartbeat_rx);
        follower.max_timeout_ms = 2;
        follower.min_timeout_ms = 1;
        heartbeat_tx.send(()).expect("Failed to send wake up.");

        tokio_test::block_on(follower.exec());

        // The general point of the Follower state representation is to loop waiting for one of:
        // - A heartbeat to be received.
        // - A randomly chosen timeout value, after which an election is triggered by returning from the Future.
        assert!(true);
    }
}
