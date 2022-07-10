// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

use std::sync::Arc;

use tokio::sync::{mpsc::Sender, watch};

use crate::raft::{Client, StateMachine};

use super::{
    Command, Entry, Error, Log, MutateStateRequest, MutateStateResponse, Quorum, ReadStateRequest,
    ReadStateResponse, RegisterClientRequest, RegisterClientResponse, Result, State, Watcher,
    NOT_LEADER, OK,
};

#[tonic::async_trait]
pub trait FrontendHandler: Send + Sync + Clone + 'static {
    async fn mutate_request(
        &self,
        mutate_request: MutateStateRequest,
    ) -> Result<MutateStateResponse>;
    async fn read_request(&self, read_request: ReadStateRequest) -> Result<ReadStateResponse>;
    async fn register_client(
        &self,
        register_request: RegisterClientRequest,
    ) -> Result<RegisterClientResponse>;
}

#[derive(Debug, Clone)]
pub struct Frontend<P, S> {
    state: Arc<State<P>>,
    log: Arc<Log>,
    watcher: Arc<Watcher>,
    submit_tx: Sender<()>,
    applied_rx: watch::Receiver<()>,
    quorum: Arc<Quorum<P>>,
    state_machine: Arc<S>,
}

impl<P, S> Frontend<P, S>
where
    P: Client + Clone,
    S: StateMachine + 'static,
{
    pub fn new(
        logger: &slog::Logger,
        state: Arc<State<P>>,
        log: Arc<Log>,
        watcher: Arc<Watcher>,
        commit_tx: Arc<watch::Sender<()>>,
        applied_rx: watch::Receiver<()>,
        submit_tx: Sender<()>,
        state_machine: Arc<S>,
    ) -> Frontend<P, S> {
        let quorum = Arc::new(Quorum::new(logger, state.clone(), log.clone(), commit_tx));
        Frontend {
            state,
            log,
            watcher,
            submit_tx,
            applied_rx,
            quorum,
            state_machine,
        }
    }

    async fn submit_command(&self, command: Vec<u8>) -> Result<Vec<u8>> {
        let current_term = self.state.get_current_term();
        let idx = self.log.append(Entry::Command(Command {
            term: current_term,
            data: command,
        }))?;

        let rx = self.watcher.register_command_watch(idx);
        self.submit_tx.send(()).await?;

        let res = rx.await.map_err(Error::from)?;
        res
    }

    async fn submit_registration(&self, term: u64) -> Result<u64> {
        let idx = self.log.append(Entry::Registration(term))?;

        let rx = self.watcher.register_registration_watch(idx);
        self.submit_tx.send(()).await?;
        rx.await?;

        Ok(idx)
    }

    async fn submit_query(&self, query: Vec<u8>) -> Result<Vec<u8>> {
        let read_idx = self.state.get_commit_idx();

        if self.quorum.exec().await {
            while self.state.get_last_applied_idx() < read_idx {
                let mut applied_rx = self.applied_rx.clone();
                let _ = applied_rx.changed().await;
            }
            return self.state_machine.read(query);
        }
        Err(Error::NoQuorum)
    }

    pub async fn register(&self, _: RegisterClientRequest) -> Result<RegisterClientResponse> {
        if self.state.is_dead() {
            return Err(Error::Dead);
        }
        if !self.state.is_leader() {
            return Ok(RegisterClientResponse {
                client_id: Vec::default(),
                leader_hint: self.state.current_leader().unwrap_or_default(),
                status: NOT_LEADER.to_string(),
            });
        }

        let term = self.state.get_current_term();
        let idx = self.submit_registration(term).await?;

        Ok(RegisterClientResponse {
            client_id: idx.to_be_bytes().to_vec(),
            leader_hint: self.state.id.clone(),
            status: OK.to_string(),
        })
    }

    pub async fn mutate(&self, mutate_request: MutateStateRequest) -> Result<MutateStateResponse> {
        if self.state.is_dead() {
            return Err(Error::Dead);
        }
        if !self.state.is_leader() {
            return Ok(MutateStateResponse {
                response: Vec::default(),
                leader_hint: self.state.current_leader().unwrap_or_default(),
                status: NOT_LEADER.to_string(),
            });
        }

        let res = self.submit_command(mutate_request.command).await?;
        Ok(MutateStateResponse {
            leader_hint: self.state.id.clone(),
            response: res,
            status: OK.to_string(),
        })
    }

    pub async fn read(&self, read_request: ReadStateRequest) -> Result<ReadStateResponse> {
        if self.state.is_dead() {
            return Err(Error::Dead);
        }
        if !self.state.is_leader() {
            return Ok(ReadStateResponse {
                leader_hint: self.state.current_leader().unwrap_or_default(),
                response: Vec::default(),
                status: NOT_LEADER.to_string(),
            });
        }
        let resp = self.submit_query(read_request.query).await?;
        Ok(ReadStateResponse {
            leader_hint: self.state.id.clone(),
            response: resp,
            status: OK.to_string(),
        })
    }
}

#[tonic::async_trait]
impl<P, S> FrontendHandler for Frontend<P, S>
where
    P: Client + Send + Sync + Clone + 'static,
    S: StateMachine + Clone + Send + Sync + 'static,
{
    async fn mutate_request(
        &self,
        mutate_request: MutateStateRequest,
    ) -> Result<MutateStateResponse> {
        self.mutate(mutate_request).await
    }
    async fn read_request(&self, read_request: ReadStateRequest) -> Result<ReadStateResponse> {
        self.read(read_request).await
    }
    async fn register_client(
        &self,
        register_request: RegisterClientRequest,
    ) -> Result<RegisterClientResponse> {
        self.register(register_request).await
    }
}

#[cfg(test)]
pub mod mock {
    use mockall::mock;

    mock! {
        #[derive(Debug)]
        pub FrontendHandler<P> {}

        #[tonic::async_trait]
        impl<P: Send + Sync + Clone + 'static> super::FrontendHandler for FrontendHandler<P> {
            async fn mutate_request(
                &self,
                mutate_request: super::MutateStateRequest,
            ) -> super::Result<super::MutateStateResponse>;
            async fn read_request(&self, read_request: super::ReadStateRequest) -> super::Result<super::ReadStateResponse>;
            async fn register_client(
                &self,
                register_request: super::RegisterClientRequest,
            ) -> super::Result<super::RegisterClientResponse>;
        }

        impl<P: Send + Sync + Clone + 'static> Clone for FrontendHandler<P> {
            fn clone(&self) -> Self;
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, sync::Arc, time::Duration};

    use rstest::rstest;
    use tokio::sync::{mpsc, watch};

    use crate::{
        hash::{Command, HashState, Query},
        log,
        raft::{cluster::MockClient, AppendEntriesResponse, Commiter, Mode, Peer},
    };

    use super::*;

    fn test_setup(
        mode: Mode,
        peers: HashMap<String, Peer<MockClient>>,
    ) -> (Frontend<MockClient, HashState>, Arc<State<MockClient>>) {
        let logger = log::noop();
        let id = String::from("leader");
        let db = sled::Config::new()
            .temporary(true)
            .open()
            .expect("Failed to open temp database.");
        let state = Arc::new(
            State::<MockClient>::new(id.clone(), peers, &db)
                .expect("Failed to create new State object."),
        );
        state.set_mode(mode);

        let log = Arc::new(Log::new(&db).expect("Failed to create new persistent Log."));
        let watcher = Arc::new(Watcher::default());
        let state_machine = Arc::new(HashState::new(&logger));
        let (applied_tx, applied_rx) = watch::channel(());
        let applied_tx = Arc::new(applied_tx);
        let (commit_tx, commit_rx) = watch::channel(());
        let commit_tx = Arc::new(commit_tx);
        let (submit_tx, mut submit_rx) = mpsc::channel(10);

        let mut commiter = Commiter::new(
            &logger,
            state.clone(),
            log.clone(),
            state_machine.clone(),
            commit_rx,
            applied_tx,
            watcher.clone(),
        );
        let frontend = Frontend::new(
            &logger,
            state.clone(),
            log.clone(),
            watcher.clone(),
            commit_tx.clone(),
            applied_rx,
            submit_tx,
            state_machine,
        );

        tokio::task::spawn(async move { commiter.exec().await });
        let commit_state = state.clone();
        let commit_log = log.clone();
        tokio::task::spawn(async move {
            while !commit_state.is_dead() {
                tokio::time::sleep(Duration::from_millis(10)).await;
                let (last_log_idx, _) = commit_log
                    .last_log_idx_and_term()
                    .expect("Failed to get last log idx.");
                commit_state.set_commit_idx(last_log_idx);
                let _ = commit_tx.send(());
            }
        });
        let submit_state = state.clone();
        tokio::task::spawn(async move {
            while !submit_state.is_dead() && submit_rx.recv().await.is_some() {
                debug!(logger, "Got submit");
            }
        });
        (frontend, state)
    }

    #[rstest]
    #[case::dead(Mode::Dead, Err(Error::Dead))]
    #[case::not_leader(
        Mode::Follower,
        Ok(RegisterClientResponse {
            client_id:Vec::default(),
            leader_hint: String::default(),
            status: NOT_LEADER.to_string(),
        })
    )]
    #[case::leader(
        Mode::Leader,
        Ok(RegisterClientResponse {
            client_id: 1u64.to_be_bytes().to_vec(),
            leader_hint: String::from("leader"),
            status: OK.to_string(),
        })
    )]
    #[tokio::test]
    async fn test_registration(
        #[case] mode: Mode,
        #[case] expected: Result<RegisterClientResponse>,
    ) {
        let (frontend, state) = test_setup(mode, HashMap::default());
        let resp = frontend.register_client(RegisterClientRequest {}).await;
        if expected.is_ok() {
            assert!(resp.is_ok());
            let resp = resp.unwrap();
            let expected = expected.unwrap();
            assert_eq!(resp.status, expected.status);
            assert_eq!(resp.leader_hint, expected.leader_hint);
            assert_eq!(resp.client_id.len(), expected.client_id.len());
            if expected.client_id.len() == 0 {
                return;
            }
            let resp_id = resp
                .client_id
                .try_into()
                .map(u64::from_be_bytes)
                .expect("Failed to cast client id to u64");
            let expected_id = expected
                .client_id
                .try_into()
                .map(u64::from_be_bytes)
                .expect("Failed to cast client id to u64");
            assert_eq!(resp_id, expected_id);
        } else {
            assert!(resp.is_err());
        }
        state.transition_dead();
    }

    #[rstest]
    #[case::dead(
        Mode::Dead,
        Command::Insert(String::from("hello"), 123),
        Err(Error::Dead)
    )]
    #[case::not_leader(
        Mode::Follower,
        Command::Insert(String::from("hello"), 123),
        Ok(MutateStateResponse {
            response: Vec::default(),
            leader_hint: String::default(),
            status: NOT_LEADER.to_string(),
        })
    )]
    #[case::leader(
        Mode::Leader,
        Command::Insert(String::from("hello"), 123),
        Ok(MutateStateResponse {
            response: Vec::default(),
            leader_hint: String::from("leader"),
            status: OK.to_string(),
        })
    )]
    #[tokio::test]
    async fn test_mutation(
        #[case] mode: Mode,
        #[case] cmd: Command,
        #[case] expected: Result<MutateStateResponse>,
    ) {
        let (frontend, state) = test_setup(mode, HashMap::default());
        let resp = frontend
            .mutate_request(MutateStateRequest {
                client_id: Vec::default(),
                sequence_num: 0,
                command: cmd.to_bytes(),
            })
            .await;
        if expected.is_ok() {
            assert!(resp.is_ok());
            let resp = resp.unwrap();
            let expected = expected.unwrap();
            assert_eq!(resp.status, expected.status);
            assert_eq!(resp.leader_hint, expected.leader_hint);
            assert_eq!(resp.response, expected.response);
        } else {
            assert!(resp.is_err());
        }
        state.transition_dead();
    }

    fn mock_client_success() -> MockClient {
        let mut mock = MockClient::default();
        mock.expect_clone().once().returning(|| -> MockClient {
            let mut mock = MockClient::default();
            mock.expect_append()
                .once()
                .returning(|req| -> Result<AppendEntriesResponse> {
                    Ok(AppendEntriesResponse {
                        success: true,
                        term: req.term,
                    })
                });
            mock
        });
        mock
    }

    fn happy_path() -> HashMap<String, Peer<MockClient>> {
        let mut peers = HashMap::default();

        let id1 = String::from("peer1");
        let peer1 = mock_client_success();
        peers.insert(id1.clone(), Peer::with_client(id1, peer1));

        let id2 = String::from("peer2");
        let peer2 = mock_client_success();
        peers.insert(id2.clone(), Peer::with_client(id2, peer2));

        let id3 = String::from("leader");
        let peer3 = MockClient::default();
        peers.insert(id3.clone(), Peer::with_client(id3, peer3));
        peers
    }

    #[rstest]
    #[case::dead(
        Mode::Dead,
        Query {key: String::from("read")},
        Err(Error::Dead),
        HashMap::default(),
    )]
    #[case::not_leader(
        Mode::Follower,
        Query {key: String::from("read")},
        Ok(ReadStateResponse {
            response: Vec::default(),
            leader_hint: String::default(),
            status: NOT_LEADER.to_string(),
        }),
        HashMap::default(),
    )]
    #[case::no_quorum(
        Mode::Leader,
        Query {key: String::from("read")},
        Err(Error::NoQuorum),
        HashMap::default(),
    )]
    #[case::happy_path(
        Mode::Leader,
        Query {key: String::from("read")},
        Ok(ReadStateResponse {
            status: String::from("OK"),
            response: bincode::serialize::<Option<u64>>(&None).expect("Failed to serialize"),
            leader_hint: String::from("leader"),
        }),
        happy_path(),
    )]
    #[tokio::test]
    async fn test_read(
        #[case] mode: Mode,
        #[case] query: Query,
        #[case] expected: Result<ReadStateResponse>,
        #[case] peers: HashMap<String, Peer<MockClient>>,
    ) {
        let (frontend, state) = test_setup(mode, peers);
        let query = query.to_bytes();
        let resp = frontend.read_request(ReadStateRequest { query }).await;
        if expected.is_ok() {
            assert!(resp.is_ok());
            let resp = resp.unwrap();
            let expected = expected.unwrap();
            assert_eq!(resp, expected);
        } else {
            assert!(resp.is_err());
        }
        state.transition_dead();
    }
}
