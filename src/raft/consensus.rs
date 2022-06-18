// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::watch::{self, Receiver, Sender};

use crate::raft::{Commiter, Leader};
use crate::rpc::raft::{
    AppendRequest, AppendResponse, Command, Payload, VoteRequest, VoteResponse,
};

use super::{
    Candidate, Client, ElectionResult, Error, Follower, Log, Metadata, Peer, Result, StateMachine,
};

#[derive(Debug, Clone)]
pub struct ConsensusMod<P, S> {
    logger: slog::Logger,

    // Persistent state.
    metadata: Arc<Metadata<P>>,
    log: Arc<Log>,
    state_machine: Arc<S>,

    heartbeat_tx: Arc<Sender<()>>,
    heartbeat_rx: Receiver<()>,
    submit_tx: Arc<Sender<()>>,
    submit_rx: Receiver<()>,
    commit_tx: Arc<Sender<()>>,
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
        let (heartbeat_tx, heartbeat_rx) = watch::channel(());
        let (submit_tx, submit_rx) = watch::channel(());
        let (commit_tx, commit_rx) = watch::channel(());
        let logger = logger.new(o!("id" => id.clone()));
        let log = Log::new(db)?;
        Ok(ConsensusMod {
            logger,
            metadata: Arc::new(Metadata::new(id, peers, db)?),
            log: Arc::new(log),
            state_machine: Arc::new(state_machine),
            heartbeat_tx: Arc::new(heartbeat_tx),
            heartbeat_rx,
            submit_tx: Arc::new(submit_tx),
            submit_rx,
            commit_tx: Arc::new(commit_tx),
            commit_rx,
        })
    }

    pub fn append_peer(&self, id: String, peer: Peer<P>) {
        self.metadata.peers.append(id, peer);
    }

    pub fn is_leader(&self) -> bool {
        self.metadata.is_leader()
    }

    pub fn dump(&self) -> Result<Vec<Payload>> {
        self.log.dump()
    }

    async fn follower_loop(&mut self) {
        debug!(self.logger, "Started follower loop!");
        Follower::new(&self.logger, self.heartbeat_rx.clone())
            .exec()
            .await
    }

    async fn candidate_loop(&self, saved_term: i64) -> ElectionResult {
        Candidate::new(
            &self.logger,
            self.metadata.clone(),
            self.log.clone(),
            saved_term,
        )
        .exec()
        .await
    }

    async fn leader_loop(&self) {
        debug!(self.logger, "Started leader loop!");
        Leader::new(
            &self.logger,
            self.metadata.clone(),
            self.log.clone(),
            self.commit_tx.clone(),
            self.submit_rx.clone(),
        )
        .exec()
        .await
    }

    fn commit_loop(&self) {
        debug!(self.logger, "Started commit loop!");
        let mut commiter = Commiter::new(
            &self.logger,
            self.metadata.clone(),
            self.log.clone(),
            self.state_machine.clone(),
            self.commit_rx.clone(),
        );
        tokio::task::spawn(async move { commiter.exec().await });
    }

    pub async fn start(&mut self) {
        self.commit_loop();
        loop {
            info!(self.logger, "Starting worker!");
            self.follower_loop().await;

            let saved_term = self.metadata.transition_candidate();

            match self.candidate_loop(saved_term).await {
                ElectionResult::Failed => continue,
                ElectionResult::Success => {
                    info!(self.logger, "Won the election!!!"; "id" => &self.metadata.id)
                }
            };

            self.leader_loop().await;
        }
    }

    pub async fn submit(&self, command: Vec<u8>) -> Result<()> {
        if !self.metadata.is_leader() {
            return Err(Error::InvalidState);
        }

        let current_term = self.metadata.get_current_term();
        self.log.append(Payload::Command(Command {
            term: current_term,
            data: command,
        }))?;

        self.submit_tx.send(()).map_err(Error::from)
    }

    pub async fn append(&self, append_request: AppendRequest) -> Result<AppendResponse> {
        debug!(self.logger, "Executing append RPC.");

        let term = self.metadata.get_current_term();
        let success = false;
        let mut response = AppendResponse { success, term };

        if append_request.term > term {
            debug!(self.logger, "Internal term is out of date with leader term."; "rpc" => "append", "got" => append_request.term, "have" => term);
            self.metadata.transition_follower(Some(append_request.term));
            return Ok(response);
        }

        if append_request.term < term {
            debug!(self.logger, "Internal term is newer than peer."; "rpc" => "append", "got" => append_request.term, "have" => term);
            return Ok(response);
        }

        if !self.metadata.is_follower() {
            debug!(self.logger, "Internal state is out of date with leader term."; "rpc" => "append", "got" => append_request.term, "have" => term);
            self.metadata.transition_follower(None);
            return Ok(response);
        }

        self.heartbeat_tx.send(())?;
        if append_request.prev_log_idx == -1
            || (append_request.prev_log_idx < self.log.len() as i64
                && self.log.idx_and_term_match(
                    append_request.prev_log_idx,
                    append_request.prev_log_term,
                )?)
        {
            response.success = true;

            self.log
                .append_entries(append_request.prev_log_idx, append_request.entries)?;
            if append_request.leader_commit_idx > self.metadata.get_commit_idx() {
                self.metadata
                    .set_commit_idx(append_request.leader_commit_idx.min(self.log.len() as i64));
                self.commit_tx.send(())?;
            }
        }

        Ok(response)
    }

    pub async fn vote(&self, vote_request: VoteRequest) -> Result<VoteResponse> {
        debug!(self.logger, "Executing vote RPC.");

        let term = self.metadata.get_current_term();
        let vote_granted = false;
        let mut response = VoteResponse { vote_granted, term };

        let (last_log_idx, last_log_term) = self.log.last_log_idx_and_term()?;

        if vote_request.term > term {
            debug!(self.logger, "Internal term is out of date with leader term."; "rpc" => "vote", "got" => vote_request.term, "have" => term);
            self.metadata.transition_follower(Some(vote_request.term));
            return Ok(response);
        }

        let voted_for = self.metadata.get_voted_for();
        if self.metadata.matches_term(vote_request.term)
            && (voted_for.is_none() || voted_for.as_ref().unwrap() == &vote_request.candidate_id)
            && (vote_request.last_log_term > last_log_term
                || (vote_request.last_log_term == last_log_term
                    && vote_request.last_log_idx >= last_log_idx))
        {
            response.vote_granted = true;
            self.metadata.set_voted_for(Some(vote_request.candidate_id))
        }

        Ok(response)
    }
}

#[tonic::async_trait]
pub trait ConcensusRepo: Send + Sync + 'static {
    async fn append_entries(&self, mut append_request: AppendRequest) -> Result<AppendResponse>;
    async fn vote_request(&self, vote_request: VoteRequest) -> Result<VoteResponse>;
}

#[tonic::async_trait]
impl<P, S> ConcensusRepo for ConsensusMod<P, S>
where
    P: Client + Send + Clone + 'static,
    S: StateMachine + Send + Sync + 'static,
{
    async fn append_entries(&self, append_request: AppendRequest) -> Result<AppendResponse> {
        self.append(append_request).await
    }
    async fn vote_request(&self, vote_request: VoteRequest) -> Result<VoteResponse> {
        self.vote(vote_request).await
    }
}
