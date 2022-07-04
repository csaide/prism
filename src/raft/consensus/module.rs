// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

use std::{collections::HashMap, sync::Arc};

use tokio::sync::{mpsc, watch};

use super::{
    Candidate, Client, Cluster, Commiter, ElectionResult, Entry, Flusher, Follower, Frontend,
    Leader, Log, Peer, Raft, Repo, Result, State, StateMachine, Watcher,
};

#[derive(Debug)]
pub struct Module<P, S> {
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

    commit_rx: watch::Receiver<()>,
}

impl<P, S> Module<P, S>
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
    ) -> Result<Module<P, S>> {
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
        Ok(Module {
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
                // If we are in test mode lets break out to simplify testing.
                if cfg!(test) {
                    return;
                }
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
            // If we are in test mode lets break out to simplify testing.
            if cfg!(test) {
                return;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use mockall::Sequence;
    use rstest::rstest;
    use tokio_test::block_on as wait;

    use crate::{
        hash::HashState,
        log,
        raft::{AppendEntriesResponse, MockClient, Mode, RequestVoteResponse},
    };

    use super::*;

    fn happy_path_cli_1() -> MockClient {
        let mut cli = MockClient::default();
        cli.expect_append().never();
        cli.expect_vote().never();
        cli.expect_clone().returning(|| happy_path_cli_1());
        cli
    }

    fn happy_path_cli_2_3() -> MockClient {
        let mut cli = MockClient::default();
        let mut seq = Sequence::default();

        cli.expect_clone()
            .once()
            .returning(|| -> MockClient {
                let mut cli = MockClient::default();
                cli.expect_vote().once().returning(|req| {
                    Ok(RequestVoteResponse {
                        term: req.term,
                        vote_granted: true,
                    })
                });
                cli
            })
            .in_sequence(&mut seq);
        cli.expect_clone()
            .once()
            .returning(|| -> MockClient {
                let mut cli = MockClient::default();
                cli.expect_append().once().returning(|req| {
                    Ok(AppendEntriesResponse {
                        success: true,
                        term: req.term,
                    })
                });
                cli
            })
            .in_sequence(&mut seq);

        cli
    }

    fn happy_path_clients() -> Vec<(String, Peer<MockClient>)> {
        let id_1 = String::from("first");
        let cli_1 = happy_path_cli_1();
        let peer_1 = Peer::with_client(id_1.clone(), cli_1);

        let id_2 = String::from("second");
        let cli_2 = happy_path_cli_2_3();
        let peer_2 = Peer::with_client(id_2.clone(), cli_2);

        let id_3 = String::from("third");
        let cli_3 = happy_path_cli_2_3();
        let peer_3 = Peer::with_client(id_3.clone(), cli_3);

        vec![(id_1, peer_1), (id_2, peer_2), (id_3, peer_3)]
    }

    #[rstest]
    #[case::happy_path(String::from("first"), happy_path_clients)]
    fn test_module(
        #[case] id: String,
        #[case] mut clients: impl FnMut() -> Vec<(String, Peer<MockClient>)>,
    ) {
        let logger = log::noop();
        let db = sled::Config::new()
            .temporary(true)
            .open()
            .expect("Failed to open temp database.");
        let state_machine = HashState::new(&logger);

        let mut module = Module::<MockClient, HashState>::new(
            id,
            HashMap::default(),
            &logger,
            &db,
            state_machine,
        )
        .expect("Failed to generate new module.");

        for (id, peer) in (clients)() {
            module.append_peer(id, peer);
        }

        let dump = module.dump().expect("Failed to dump empty log.");
        assert_eq!(dump.len(), 0);

        assert!(!module.is_leader());

        let _repo = module.get_repo().expect("Should have had a repo to take.");
        assert!(matches!(module.repo, None));

        wait(module.start());
        module.state.set_mode(Mode::Dead);
    }
}
