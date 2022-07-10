// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

use std::sync::Arc;

use tokio::sync::watch::Sender;

use super::{
    AppendEntriesRequest, AppendEntriesResponse, Client, Error, Log, RequestVoteRequest,
    RequestVoteResponse, Result, State,
};

#[tonic::async_trait]
pub trait RaftHandler: Send + Sync + 'static {
    async fn append_entries(
        &self,
        append_request: AppendEntriesRequest,
    ) -> Result<AppendEntriesResponse>;
    async fn vote_request(&self, vote_request: RequestVoteRequest) -> Result<RequestVoteResponse>;
}

#[derive(Debug, Clone)]
pub struct Raft<P> {
    logger: slog::Logger,
    state: Arc<State<P>>,
    log: Arc<Log>,
    heartbeat_tx: Arc<Sender<()>>,
    commit_tx: Arc<Sender<()>>,
}

impl<P> Raft<P>
where
    P: Client + Send + Clone + 'static,
{
    pub fn new(
        logger: &slog::Logger,
        state: Arc<State<P>>,
        log: Arc<Log>,
        heartbeat_tx: Arc<Sender<()>>,
        commit_tx: Arc<Sender<()>>,
    ) -> Raft<P> {
        Raft {
            logger: logger.new(o!("module" => "consensus-raft")),
            state,
            log,
            heartbeat_tx,
            commit_tx,
        }
    }

    pub async fn append(
        &self,
        append_request: AppendEntriesRequest,
    ) -> Result<AppendEntriesResponse> {
        if self.state.is_dead() {
            return Err(Error::Dead);
        }

        let log_ok = |append_request: &AppendEntriesRequest| -> Result<bool> {
            Ok(append_request.prev_log_idx == 0
                || (append_request.prev_log_idx <= self.log.len() as u64
                    && self.log.idx_and_term_match(
                        append_request.prev_log_idx,
                        append_request.prev_log_term,
                    )?))
        };

        let term = self.state.get_current_term();
        if append_request.term > term {
            debug!(self.logger, "Internal term is out of date with leader term.";
                "rpc" => "append",
                "got" => append_request.term,
                "have" => term,
            );
            self.state.transition_follower(Some(append_request.term));
        } else if append_request.term < term {
            debug!(self.logger, "Internal term is greater than request term... ignoring.";
                "rpc" => "append",
                "got" => append_request.term,
                "have" => term,
            );
        } else if append_request.term == term && self.state.is_candidate() {
            debug!(self.logger, "Received heartbeat from server in current term while candidate, step down to follower";
                "rpc" => "append",
                "got" => append_request.term,
                "have" => term,
            );
            self.state.transition_follower(None);
        }

        let term = self.state.get_current_term();
        let success = false;
        let mut response = AppendEntriesResponse { success, term };

        if append_request.term == term && self.state.is_follower() && (log_ok)(&append_request)? {
            debug!(self.logger, "Accepting append entries request.");
            // Accept the request.
            self.heartbeat_tx.send(())?;
            self.state.saw_leader(append_request.leader_id);

            response.success = true;

            if let Some(cfg) = self
                .log
                .append_entries(append_request.prev_log_idx, append_request.entries)?
            {
                info!(self.logger, "Cluster configuration updated!"; "voters" => format!("{:?}", &cfg.voters));
                if !self.state.peers.lock().update(cfg)? {
                    self.state.transition_dead();
                }
            }
            if append_request.leader_commit_idx > self.state.get_commit_idx() {
                self.state
                    .set_commit_idx(append_request.leader_commit_idx.min(self.log.len() as u64));
                self.commit_tx.send(())?;
            }
        }
        Ok(response)
    }

    pub async fn vote(&self, vote_request: RequestVoteRequest) -> Result<RequestVoteResponse> {
        if self.state.is_dead() {
            return Err(Error::Dead);
        }

        debug!(self.logger, "Executing vote RPC."; "term" => vote_request.term, "candidate" => &vote_request.candidate_id);

        let locked_peers = self.state.peers.lock();
        let log_ok = |vote_request: &RequestVoteRequest| -> Result<bool> {
            let (last_log_idx, last_log_term) = self.log.last_log_idx_and_term()?;

            let result = vote_request.last_log_term > last_log_term
                || (vote_request.last_log_term == last_log_term
                    && vote_request.last_log_idx >= last_log_idx);
            Ok(result)
        };

        let grant = |vote_request: &RequestVoteRequest| -> Result<bool> {
            let voted_for = self.state.get_voted_for();
            Ok(vote_request.term == self.state.get_current_term()
                && !self.state.have_leader()
                && locked_peers.contains(&vote_request.candidate_id)
                && (log_ok)(vote_request)?
                && (voted_for.is_none()
                    || voted_for.as_ref().unwrap() == &vote_request.candidate_id))
        };

        let term = self.state.get_current_term();
        if vote_request.term > term && locked_peers.contains(&vote_request.candidate_id) {
            debug!(self.logger, "Internal term is out of date with leader term.";
                "rpc" => "append",
                "got" => vote_request.term,
                "have" => term,
            );
            self.state.transition_follower(Some(vote_request.term));
        }

        let term = self.state.get_current_term();
        let vote_granted = false;
        let mut response = RequestVoteResponse { vote_granted, term };

        if vote_request.term <= term && (grant)(&vote_request)? {
            response.vote_granted = true;
            self.state.set_voted_for(Some(vote_request.candidate_id));
            self.heartbeat_tx.send(())?;
        }
        Ok(response)
    }
}

#[tonic::async_trait]
impl<P> RaftHandler for Raft<P>
where
    P: Client + Send + Clone + 'static,
{
    async fn append_entries(
        &self,
        append_request: AppendEntriesRequest,
    ) -> Result<AppendEntriesResponse> {
        self.append(append_request).await
    }
    async fn vote_request(&self, vote_request: RequestVoteRequest) -> Result<RequestVoteResponse> {
        self.vote(vote_request).await
    }
}

#[cfg(test)]
pub mod mock {
    use mockall::mock;

    mock! {
        #[derive(Debug)]
        pub RaftHandler<P> {}

        #[tonic::async_trait]
        impl<P: Send + Sync + Clone + 'static> super::RaftHandler for RaftHandler<P> {
            async fn append_entries(
                &self,
                append_request: super::AppendEntriesRequest,
            ) -> super::Result<super::AppendEntriesResponse>;
            async fn vote_request(&self, vote_request: super::RequestVoteRequest) -> super::Result<super::RequestVoteResponse>;
        }

        impl<P: Send + Sync + Clone + 'static> Clone for RaftHandler<P> {
            fn clone(&self) -> Self;
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, sync::Arc};

    use rstest::rstest;
    use tokio::sync::watch;

    use crate::{
        logging,
        raft::{ClusterConfig, Command, Entry, MockClient, Mode, Peer},
    };

    use super::*;

    fn test_setup(mode: Mode) -> (Raft<MockClient>, Arc<State<MockClient>>) {
        let logger = logging::noop();
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
        state.set_current_term(1);
        let leader_id = String::from("leader");
        let leader = Peer::new(leader_id.clone());
        state.peers.lock().append(leader_id, leader);

        let log = Arc::new(Log::new(&db).expect("Failed to create new persistent Log."));
        let (commit_tx, mut commit_rx) = watch::channel(());
        let commit_tx = Arc::new(commit_tx);
        let commit_state = state.clone();
        let commit_logger = logger.clone();
        tokio::task::spawn(async move {
            while !commit_state.is_dead() && commit_rx.changed().await.is_ok() {
                debug!(commit_logger, "Got commit");
            }
        });

        let (heartbeat_tx, mut heartbeat_rx) = watch::channel(());
        let heartbeat_tx = Arc::new(heartbeat_tx);
        let heartbeat_state = state.clone();
        let heartbeat_logger = logger.clone();
        tokio::task::spawn(async move {
            while !heartbeat_state.is_dead() && heartbeat_rx.changed().await.is_ok() {
                debug!(heartbeat_logger, "Got commit");
            }
        });

        (
            Raft::new(&logger, state.clone(), log.clone(), heartbeat_tx, commit_tx),
            state,
        )
    }

    #[rstest]
    #[case::dead(
        Mode::Dead,
        vec![
            (AppendEntriesRequest {
                entries: Vec::default(),
                leader_commit_idx:1,
                leader_id:            String::from("leader"),
                prev_log_idx:1,
                prev_log_term:1,
                term: 1,
            },
            Err(Error::Dead))
        ]
    )]
    #[case::smaller_term(
        Mode::Follower,
        vec![
            (AppendEntriesRequest{
                entries: Vec::default(),
                leader_commit_idx: 0,
                leader_id:  String::from("leader"),
                prev_log_idx: 0,
                prev_log_term: 0,
                term: 0
            },
            Ok(AppendEntriesResponse{
                success: false,
                term: 1
            }))
        ]
    )]
    #[case::greater_term(
        Mode::Follower,
        vec![
            (AppendEntriesRequest{
                entries: Vec::default(),
                leader_commit_idx: 1,
                leader_id:  String::from("leader"),
                prev_log_idx: 0,
                prev_log_term: 0,
                term: 2,
            },
            Ok(AppendEntriesResponse{
                success: true,
                term: 2
            }))
        ]
    )]
    #[case::candidate_same_term(
        Mode::Candidate,
        vec![
            (AppendEntriesRequest{
                entries: Vec::default(),
                leader_commit_idx: 1,
                leader_id:  String::from("leader"),
                prev_log_idx: 0,
                prev_log_term: 0,
                term: 1,
            },
            Ok(AppendEntriesResponse{
                success: true,
                term: 1
            }))
        ]
    )]
    #[case::same_term_cluster_cfg(
        Mode::Follower,
        vec![
            (AppendEntriesRequest{
                entries: vec![Entry::ClusterConfig(ClusterConfig{
                    replicas: Vec::default(),
                    term: 1,
                    voters: Vec::default(),
                })],
                leader_commit_idx: 1,
                leader_id:  String::from("leader"),
                prev_log_idx: 0,
                prev_log_term: 0,
                term: 1,
            },
            Ok(AppendEntriesResponse{
                success: true,
                term: 1
            }))
        ]
    )]
    #[case::happy_path(
        Mode::Follower,
        vec![
            (AppendEntriesRequest{
                entries: vec![Entry::Command(Command{
                    term: 1,
                    data: vec![0x00,0x01,0x02],
                }),Entry::Command(Command{
                    term: 1,
                    data: vec![0x00,0x01,0x02],
                })],
                leader_commit_idx: 2,
                leader_id:  String::from("leader"),
                prev_log_idx: 0,
                prev_log_term: 0,
                term: 1,
            },
            Ok(AppendEntriesResponse{
                success: true,
                term: 1,
            })),
            (AppendEntriesRequest{
                entries: vec![Entry::Command(Command{
                    term: 1,
                    data: vec![0x00,0x01,0x02],
                }),Entry::Command(Command{
                    term: 1,
                    data: vec![0x00,0x01,0x02],
                })],
                leader_commit_idx: 4,
                leader_id:  String::from("leader"),
                prev_log_idx: 2,
                prev_log_term: 1,
                term: 2,
            },
            Ok(AppendEntriesResponse{
                success: true,
                term: 2,
            }))
        ]
    )]
    #[tokio::test]
    async fn test_append_entries(
        #[case] mode: Mode,
        #[case] req_resp_pairs: Vec<(AppendEntriesRequest, Result<AppendEntriesResponse>)>,
    ) {
        let (raft, state) = test_setup(mode);
        for (request, expected) in req_resp_pairs {
            let resp = raft.append_entries(request).await;
            if expected.is_ok() {
                assert!(resp.is_ok());
                let resp = resp.unwrap();
                let expected = expected.unwrap();

                assert_eq!(resp.success, expected.success);
                assert_eq!(resp.term, expected.term);
            } else {
                assert!(resp.is_err());
            }
        }
        state.transition_dead();
    }

    #[rstest]
    #[case::dead(
        Mode::Dead,
        vec![
            (RequestVoteRequest {
                candidate_id: String::from("leader"),
                last_log_idx: 0,
                last_log_term: 0,
                term: 1,
            },
            Err(Error::Dead))
        ]
    )]
    #[case::greater_term(
        Mode::Follower,
        vec![
            (RequestVoteRequest {
                candidate_id: String::from("leader"),
                last_log_idx: 0,
                last_log_term: 0,
                term: 2,
            },
            Ok(RequestVoteResponse{
                vote_granted: true,
                term: 2,
            }))
        ]
    )]
    #[tokio::test]
    async fn test_vote_request(
        #[case] mode: Mode,
        #[case] req_resp_pairs: Vec<(RequestVoteRequest, Result<RequestVoteResponse>)>,
    ) {
        let (raft, state) = test_setup(mode);
        for (request, expected) in req_resp_pairs {
            let resp = raft.vote_request(request).await;
            if expected.is_ok() {
                assert!(resp.is_ok());
                let resp = resp.unwrap();
                let expected = expected.unwrap();

                assert_eq!(resp.vote_granted, expected.vote_granted);
                assert_eq!(resp.term, expected.term);
            } else {
                assert!(resp.is_err());
            }
        }
        state.transition_dead();
    }
}
