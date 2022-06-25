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
                || (append_request.prev_log_idx <= self.log.len() as u128
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
                    .set_commit_idx(append_request.leader_commit_idx.min(self.log.len() as u128));
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
