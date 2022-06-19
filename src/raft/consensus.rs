// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::watch::{self, Receiver, Sender};

use crate::raft::{Commiter, Leader};

use super::{
    AddServerRequest, AddServerResponse, AppendEntriesRequest, AppendEntriesResponse, Candidate,
    Client, ClusterConfig, Command, ElectionResult, Entry, Error, Follower, Log, Metadata, Peer,
    RemoveServerRequest, RemoveServerResponse, RequestVoteRequest, RequestVoteResponse, Result,
    StateMachine, Syncer,
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

    pub fn dump(&self) -> Result<Vec<Entry>> {
        self.log.dump()
    }

    async fn follower_loop(&mut self) {
        debug!(self.logger, "Started follower loop!");
        Follower::new(
            &self.logger,
            self.metadata.clone(),
            self.heartbeat_rx.clone(),
        )
        .exec()
        .await
    }

    async fn candidate_loop(&self, saved_term: u128) -> ElectionResult {
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
        while !self.metadata.is_dead() {
            info!(self.logger, "Starting worker!");
            self.follower_loop().await;

            if self.metadata.peers.len() < 2 {
                continue;
            }

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

    pub async fn submit_command(&self, command: Vec<u8>) -> Result<()> {
        if self.metadata.is_dead() {
            return Err(Error::Dead);
        }
        if !self.metadata.is_leader() {
            return Err(Error::InvalidState);
        }

        let current_term = self.metadata.get_current_term();
        self.log.append(Entry::Command(Command {
            term: current_term,
            data: command,
        }))?;

        self.submit_tx.send(()).map_err(Error::from)
    }

    async fn submit_cluster_config(&self, cfg: ClusterConfig) -> Result<()> {
        if self.metadata.is_dead() {
            return Err(Error::Dead);
        }
        if !self.metadata.is_leader() {
            return Err(Error::InvalidState);
        }
        info!(self.logger, "Appending cluster config entry.");
        let last_cluster_config_idx = self.log.append(Entry::ClusterConfig(cfg))?;

        info!(self.logger, "Setting last cluster config index.");
        self.metadata
            .set_last_cluster_config_idx(last_cluster_config_idx);

        info!(self.logger, "Waking leader loop for replication.");
        self.submit_tx.send(()).map_err(Error::from)
    }

    pub async fn append(
        &self,
        append_request: AppendEntriesRequest,
    ) -> Result<AppendEntriesResponse> {
        if self.metadata.is_dead() {
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

        let term = self.metadata.get_current_term();
        if append_request.term > term {
            debug!(self.logger, "Internal term is out of date with leader term.";
                "rpc" => "append",
                "got" => append_request.term,
                "have" => term,
            );
            self.metadata.transition_follower(Some(append_request.term));
        } else if append_request.term < term {
            debug!(self.logger, "Internal term is greater than request term... ignoring.";
                "rpc" => "append",
                "got" => append_request.term,
                "have" => term,
            );
        } else if append_request.term == term && self.metadata.is_candidate() {
            debug!(self.logger, "Received heartbeat from server in current term while candidate, step down to follower";
                "rpc" => "append",
                "got" => append_request.term,
                "have" => term,
            );
            self.metadata.transition_follower(None);
        }

        let term = self.metadata.get_current_term();
        let success = false;
        let mut response = AppendEntriesResponse { success, term };

        if append_request.term == term && self.metadata.is_follower() && (log_ok)(&append_request)?
        {
            debug!(self.logger, "Accepting append entries request.");
            // Accept the request.
            self.heartbeat_tx.send(())?;
            response.success = true;

            if let Some(cfg) = self
                .log
                .append_entries(append_request.prev_log_idx, append_request.entries)?
            {
                info!(self.logger, "Cluster configuration updated!"; "voters" => format!("{:?}", &cfg.voters));
                if !self.metadata.peers.update(cfg).await? {
                    self.metadata.transition_dead();
                }
            }
            if append_request.leader_commit_idx > self.metadata.get_commit_idx() {
                self.metadata
                    .set_commit_idx(append_request.leader_commit_idx.min(self.log.len() as u128));
                self.commit_tx.send(())?;
            }
        }
        Ok(response)
    }

    pub async fn vote(&self, vote_request: RequestVoteRequest) -> Result<RequestVoteResponse> {
        if self.metadata.is_dead() {
            return Err(Error::Dead);
        }

        debug!(self.logger, "Executing vote RPC."; "term" => vote_request.term, "candidate" => &vote_request.candidate_id);

        let log_ok = |vote_request: &RequestVoteRequest| -> Result<bool> {
            let (last_log_idx, last_log_term) = self.log.last_log_idx_and_term()?;

            let result = vote_request.last_log_term > last_log_term
                || (vote_request.last_log_term == last_log_term
                    && vote_request.last_log_idx >= last_log_idx);
            Ok(result)
        };

        let grant = |vote_request: &RequestVoteRequest| -> Result<bool> {
            let voted_for = self.metadata.get_voted_for();
            Ok(vote_request.term == self.metadata.get_current_term()
                && !self.metadata.have_leader()
                && self.metadata.peers.contains(&vote_request.candidate_id)
                && (log_ok)(vote_request)?
                && (voted_for.is_none()
                    || voted_for.as_ref().unwrap() == &vote_request.candidate_id))
        };

        let term = self.metadata.get_current_term();
        if vote_request.term > term && self.metadata.peers.contains(&vote_request.candidate_id) {
            debug!(self.logger, "Internal term is out of date with leader term.";
                "rpc" => "append",
                "got" => vote_request.term,
                "have" => term,
            );
            self.metadata.transition_follower(Some(vote_request.term));
        }

        let term = self.metadata.get_current_term();
        let vote_granted = false;
        let mut response = RequestVoteResponse { vote_granted, term };

        if vote_request.term <= term && (grant)(&vote_request)? {
            response.vote_granted = true;
            self.metadata.set_voted_for(Some(vote_request.candidate_id));
            self.heartbeat_tx.send(())?;
        }
        Ok(response)
    }

    pub async fn add_member(
        &self,
        server_request: AddServerRequest<P>,
    ) -> Result<AddServerResponse> {
        if self.metadata.is_dead() {
            return Err(Error::Dead);
        }
        if !self.metadata.is_leader() {
            return Err(Error::InvalidState);
        }

        Syncer::new(
            &self.logger,
            self.metadata.clone(),
            self.log.clone(),
            server_request.peer.clone(),
        )
        .exec()
        .await;

        self.metadata
            .peers
            .append(server_request.id, server_request.peer);

        let mut cfg = self.metadata.peers.to_cluster_config();
        cfg.term = self.metadata.get_current_term();
        self.submit_cluster_config(cfg).await?;

        Ok(AddServerResponse {
            leader_hint: self.metadata.id.clone(),
            status: "OK".to_string(),
        })
    }

    pub async fn remove_member(
        &self,
        server_request: RemoveServerRequest,
    ) -> Result<RemoveServerResponse> {
        if self.metadata.is_dead() {
            return Err(Error::Dead);
        }
        if !self.metadata.is_leader() {
            return Err(Error::InvalidState);
        }
        self.metadata.peers.remove(&server_request.id);

        let mut cfg = self.metadata.peers.to_cluster_config();
        cfg.term = self.metadata.get_current_term();
        self.submit_cluster_config(cfg).await?;

        Ok(RemoveServerResponse {
            leader_hint: self.metadata.id.clone(),
            status: "OK".to_string(),
        })
    }
}

#[tonic::async_trait]
pub trait ConcensusRepo<P>: Send + Sync + 'static {
    async fn append_entries(
        &self,
        mut append_request: AppendEntriesRequest,
    ) -> Result<AppendEntriesResponse>;
    async fn vote_request(&self, vote_request: RequestVoteRequest) -> Result<RequestVoteResponse>;
    async fn add_server(&self, add_request: AddServerRequest<P>) -> Result<AddServerResponse>;
    async fn remove_server(
        &self,
        remove_request: RemoveServerRequest,
    ) -> Result<RemoveServerResponse>;
}

#[tonic::async_trait]
impl<P, S> ConcensusRepo<P> for ConsensusMod<P, S>
where
    P: Client + Send + Clone + 'static,
    S: StateMachine + Send + Sync + 'static,
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
    async fn add_server(&self, add_request: AddServerRequest<P>) -> Result<AddServerResponse> {
        self.add_member(add_request).await
    }
    async fn remove_server(
        &self,
        remove_request: RemoveServerRequest,
    ) -> Result<RemoveServerResponse> {
        self.remove_member(remove_request).await
    }
}
