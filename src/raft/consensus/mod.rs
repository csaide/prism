// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::{
    mpsc,
    watch::{self, Receiver},
};

use super::*;

const OK: &str = "OK";
const NOT_LEADER: &str = "NOT_LEADER";

mod cluster;
mod frontend;
mod raft;
mod repo;

pub use cluster::{Cluster, ClusterHandler};
pub use frontend::{Frontend, FrontendHandler};
pub use raft::{Raft, RaftHandler};
pub use repo::{Repo, Repository};

#[derive(Debug)]
pub struct ConsensusMod<P, S> {
    logger: slog::Logger,

    state: Arc<State<P>>,
    log: Arc<Log>,
    state_machine: Arc<S>,
    watcher: Arc<Watcher>,

    // RPC Handlers
    repo: Option<Repo<P>>,

    // Mode modules.
    leader: Leader<P>,
    follower: Follower<P>,
    candidate: Candidate<P>,

    commit_rx: Receiver<()>,
}

impl<P, S> ConsensusMod<P, S>
where
    P: Client + Send + Clone + 'static,
    S: StateMachine + Send + 'static,
{
    pub fn new(
        id: String,
        peers: HashMap<String, Peer<P>>,
        logger: &slog::Logger,
        db: &sled::Db,
        state_machine: S,
    ) -> Result<ConsensusMod<P, S>> {
        // Note fix this later and persist it.
        let (submit_tx, submit_rx) = mpsc::channel(2);
        let (heartbeat_tx, heartbeat_rx) = watch::channel(());
        let heartbeat_tx = Arc::new(heartbeat_tx);
        let (commit_tx, commit_rx) = watch::channel(());
        let commit_tx = Arc::new(commit_tx);

        let logger = logger.new(o!("id" => id.clone()));
        let log = Arc::new(Log::new(db)?);
        let state = Arc::new(State::new(id, peers, db)?);
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

        let leader = Leader::new(&logger, state.clone(), log.clone(), commit_tx, submit_rx);
        let follower = Follower::new(&logger, state.clone(), heartbeat_rx);
        let candidate = Candidate::new(&logger, state.clone(), log.clone());
        Ok(ConsensusMod {
            logger,
            state,
            log,
            watcher,
            repo: Some(repo),
            leader,
            follower,
            candidate,
            state_machine: Arc::new(state_machine),
            commit_rx,
        })
    }

    pub fn append_peer(&self, id: String, peer: Peer<P>) {
        self.state.peers.lock().append(id, peer);
    }

    pub fn is_leader(&self) -> bool {
        self.state.is_leader()
    }

    pub fn dump(&self) -> Result<Vec<Entry>> {
        self.log.dump()
    }

    pub fn get_repo(&mut self) -> Option<Repo<P>> {
        self.repo.take()
    }

    async fn follower_loop(&mut self) {
        debug!(self.logger, "Started follower loop!");
        self.follower.exec().await
    }

    async fn candidate_loop(&self, saved_term: u128) -> ElectionResult {
        debug!(self.logger, "Started candidate loop!");
        self.candidate.exec(saved_term).await
    }

    async fn leader_loop(&mut self) {
        debug!(self.logger, "Started leader loop!");
        self.leader.exec().await
    }

    fn commit_loop(&self) {
        debug!(self.logger, "Started commit loop!");
        let mut commiter = Commiter::new(
            &self.logger,
            self.state.clone(),
            self.log.clone(),
            self.state_machine.clone(),
            self.commit_rx.clone(),
            self.watcher.clone(),
        );
        tokio::task::spawn(async move { commiter.exec().await });
    }

    fn flush_loop(&self) {
        debug!(self.logger, "Started flush loop!");
        let flusher = Flusher::new(&self.logger, self.log.clone(), self.state.clone());
        tokio::task::spawn(async move { flusher.exec().await });
    }

    pub async fn start(&mut self) {
        self.commit_loop();
        self.flush_loop();
        while !self.state.is_dead() {
            info!(self.logger, "Starting worker!");
            self.follower_loop().await;

            if self.state.peers.lock().len() < 2 {
                continue;
            }

            let saved_term = self.state.transition_candidate();

            match self.candidate_loop(saved_term).await {
                ElectionResult::Failed => continue,
                ElectionResult::Success => {
                    info!(self.logger, "Won the election!!!")
                }
            };

            self.leader_loop().await;
        }
    }
}
