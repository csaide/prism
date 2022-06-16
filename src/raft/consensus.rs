// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

use std::sync::Arc;

use tokio::sync::watch::{self, Receiver, Sender};

use crate::raft::{Commiter, Leader};
use crate::rpc::raft::{
    AppendRequest, AppendResponse, Command, Payload, VoteRequest, VoteResponse,
};

use super::{
    Candidate, Client, ElectionResult, Error, Follower, Log, Metadata, Peer, Result, State,
    StateMachine,
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
        peers: Vec<Peer<P>>,
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
            metadata: Arc::new(Metadata::new(id, peers)),
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

    pub fn append_peer(&self, peer: Peer<P>) {
        self.metadata.peers.lock().unwrap().push(peer);
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

        let current_term = self.metadata.current_term.read().unwrap();
        self.log.append(Payload::Command(Command {
            term: *current_term,
            data: command,
        }))?;

        self.submit_tx.send(()).map_err(Error::from)
    }

    pub async fn append(&self, mut append_request: AppendRequest) -> Result<AppendResponse> {
        // info!(self.logger, "Executing append RPC.");

        let mut success = false;

        if append_request.term > *self.metadata.current_term.read().unwrap() {
            debug!(self.logger, "Internal term is out of date with leader term."; "rpc" => "append", "got" => append_request.term, "have" => *self.metadata.current_term.read().unwrap());
            self.metadata.transition_follower(Some(append_request.term));
        }

        if append_request.term == *self.metadata.current_term.read().unwrap() {
            if *self.metadata.state.read().unwrap() != State::Follower {
                debug!(self.logger, "Internal state is out of date with leader term."; "rpc" => "append", "got" => append_request.term, "have" => *self.metadata.current_term.read().unwrap());
                self.metadata.transition_follower(None);
            }

            self.heartbeat_tx.send(())?;
            if append_request.prev_log_idx == -1
                || (append_request.prev_log_idx < self.log.len() as i64
                    && self.log.idx_and_term_match(
                        append_request.prev_log_idx,
                        append_request.prev_log_term,
                    )?)
            {
                success = true;

                let mut log_insert_index = append_request.prev_log_idx + 1;
                let mut entries_insert_index: i64 = 0;
                loop {
                    if log_insert_index >= self.log.len() as i64
                        || entries_insert_index >= append_request.entries.len() as i64
                    {
                        break;
                    }
                    if !self.log.idx_and_term_match(
                        log_insert_index,
                        append_request.entries[entries_insert_index as usize].term(),
                    )? {
                        break;
                    }
                    log_insert_index += 1;
                    entries_insert_index += 1;
                }

                for entry in append_request
                    .entries
                    .drain(entries_insert_index as usize..)
                {
                    if entry.payload.is_none() {
                        continue;
                    }

                    self.log.insert(log_insert_index, entry.payload.unwrap())?;
                    log_insert_index += 1;
                }

                let mut commit = false;
                {
                    let mut commit_idx = self.metadata.commit_idx.write().unwrap();
                    if append_request.leader_commit_idx > *commit_idx {
                        *commit_idx = append_request.leader_commit_idx.min(self.log.len() as i64);
                        commit = true;
                    }
                }
                if commit {
                    self.commit_tx.send(())?;
                }
            }
        }

        Ok(AppendResponse {
            success,
            term: *self.metadata.current_term.read().unwrap(),
        })
    }

    pub async fn vote(&self, vote_request: VoteRequest) -> Result<VoteResponse> {
        // info!(self.logger, "Executing vote RPC.");

        let mut vote_granted = false;

        let (last_log_idx, last_log_term) = self.log.last_log_idx_and_term()?;

        if vote_request.term > *self.metadata.current_term.read().unwrap() {
            debug!(self.logger, "Internal term is out of date with leader term."; "rpc" => "vote", "got" => vote_request.term, "have" => *self.metadata.current_term.read().unwrap());
            self.metadata.transition_follower(Some(vote_request.term));
        }

        let mut voted_for = self.metadata.voted_for.write().unwrap();

        if self.metadata.matches_term(vote_request.term)
            && (voted_for.is_none() || voted_for.as_ref().unwrap() == &vote_request.candidate_id)
            && (vote_request.last_log_term > last_log_term
                || (vote_request.last_log_term == last_log_term
                    && vote_request.last_log_idx >= last_log_idx))
        {
            vote_granted = true;
            *voted_for = Some(vote_request.candidate_id);
        }

        Ok(VoteResponse {
            vote_granted,
            term: *self.metadata.current_term.read().unwrap(),
        })
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
