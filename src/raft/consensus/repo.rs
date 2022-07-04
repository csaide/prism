// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

use super::{Cluster, Frontend, Raft};

#[derive(Debug)]
pub struct Repo<P> {
    raft: Raft<P>,
    frontend: Frontend<P>,
    cluster: Cluster<P>,
}

impl<P> Repo<P>
where
    P: Clone,
{
    pub fn new(raft: Raft<P>, frontend: Frontend<P>, cluster: Cluster<P>) -> Repo<P> {
        Repo {
            raft,
            frontend,
            cluster,
        }
    }

    pub fn get_raft(&self) -> Raft<P> {
        self.raft.clone()
    }
    pub fn get_frontend(&self) -> Frontend<P> {
        self.frontend.clone()
    }
    pub fn get_cluster(&self) -> Cluster<P> {
        self.cluster.clone()
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, sync::Arc};

    use tokio::sync::{mpsc, watch};

    use crate::{
        log,
        raft::{Log, MockClient, State, Watcher},
    };

    use super::*;

    #[test]
    fn test_repo() {
        let (submit_tx, _) = mpsc::channel(2);
        let (heartbeat_tx, _) = watch::channel(());
        let heartbeat_tx = Arc::new(heartbeat_tx);
        let (commit_tx, _) = watch::channel(());
        let commit_tx = Arc::new(commit_tx);

        let logger = log::noop();
        let peers = HashMap::default();
        let id = String::from("leader");
        let db = sled::Config::new()
            .temporary(true)
            .open()
            .expect("Failed to open temp database.");
        let log = Arc::new(Log::new(&db).expect("Failed to create new Log object."));
        let state = Arc::new(
            State::<MockClient>::new(id, peers, &db).expect("Failed to create new State object."),
        );
        let watcher = Arc::new(Watcher::default());

        let frontend = Frontend::new(
            state.clone(),
            log.clone(),
            watcher.clone(),
            submit_tx.clone(),
        );
        let cluster = Cluster::new(
            &logger,
            state.clone(),
            log.clone(),
            watcher.clone(),
            submit_tx,
        );
        let raft = Raft::new(
            &logger,
            state.clone(),
            log.clone(),
            heartbeat_tx,
            commit_tx.clone(),
        );
        let repo = Repo::new(raft, frontend, cluster);
        let _ = repo.get_cluster();
        let _ = repo.get_frontend();
        let _ = repo.get_raft();
    }
}
