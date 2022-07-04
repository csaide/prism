// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

use std::sync::Arc;

use tokio::sync::mpsc::Sender;

use super::{
    Command, Entry, Error, Log, MutateStateRequest, MutateStateResponse, ReadStateRequest,
    ReadStateResponse, RegisterClientRequest, RegisterClientResponse, Registration, Result, State,
    Watcher, NOT_LEADER, OK,
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
pub struct Frontend<P> {
    state: Arc<State<P>>,
    log: Arc<Log>,
    watcher: Arc<Watcher>,
    submit_tx: Sender<()>,
}

impl<P> Frontend<P> {
    pub fn new(
        state: Arc<State<P>>,
        log: Arc<Log>,
        watcher: Arc<Watcher>,
        submit_tx: Sender<()>,
    ) -> Frontend<P> {
        Frontend {
            state,
            log,
            watcher,
            submit_tx,
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

    async fn submit_registration(&self, reg: Registration) -> Result<u128> {
        let idx = self.log.append(Entry::Registration(reg))?;

        let rx = self.watcher.register_registration_watch(idx);
        self.submit_tx.send(()).await?;
        rx.await?;

        Ok(idx)
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
        let reg = Registration { term };

        let idx = self.submit_registration(reg).await?;

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

    pub async fn read(&self, _read_request: ReadStateRequest) -> Result<ReadStateResponse> {
        unimplemented!()
    }
}

#[tonic::async_trait]
impl<P: Send + Sync + Clone + 'static> FrontendHandler for Frontend<P> {
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
        hash::{Command, HashState},
        log,
        raft::{cluster::MockClient, Commiter, Mode},
    };

    use super::*;

    fn test_setup(mode: Mode) -> (Frontend<MockClient>, Arc<State<MockClient>>) {
        let logger = log::noop();
        let id = String::from("leader");
        let db = sled::Config::new()
            .temporary(true)
            .open()
            .expect("Failed to open temp database.");
        let state = Arc::new(
            State::<MockClient>::new(id.clone(), HashMap::default(), &db)
                .expect("Failed to create new State object."),
        );
        state.set_mode(mode);

        let log = Arc::new(Log::new(&db).expect("Failed to create new persistent Log."));
        let watcher = Arc::new(Watcher::default());
        let state_machine = Arc::new(HashState::new(&logger));
        let (commit_tx, commit_rx) = watch::channel(());
        let (submit_tx, mut submit_rx) = mpsc::channel(10);

        let mut commiter = Commiter::new(
            &logger,
            state.clone(),
            log.clone(),
            state_machine.clone(),
            commit_rx,
            watcher.clone(),
        );
        let frontend = Frontend::new(state.clone(), log.clone(), watcher.clone(), submit_tx);

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
            client_id: 1u128.to_be_bytes().to_vec(),
            leader_hint: String::from("leader"),
            status: OK.to_string(),
        })
    )]
    #[tokio::test]
    async fn test_registration(
        #[case] mode: Mode,
        #[case] expected: Result<RegisterClientResponse>,
    ) {
        let (frontend, state) = test_setup(mode);
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
                .map(u128::from_be_bytes)
                .expect("Failed to cast client id to u128");
            let expected_id = expected
                .client_id
                .try_into()
                .map(u128::from_be_bytes)
                .expect("Failed to cast client id to u128");
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
        let (frontend, state) = test_setup(mode);
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
}
