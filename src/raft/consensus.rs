// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

use std::sync::{Arc, Mutex};

use rand::Rng;
use tokio::sync::mpsc;
use tokio::sync::watch::{self, Receiver, Sender};
use tokio::time::{interval, timeout, Duration};

use crate::rpc::raft::{AppendRequest, AppendResponse, VoteRequest, VoteResponse};

use super::{ElectionResult, Metadata, Peer, Result, State};

#[derive(Debug, Clone)]
pub struct ConsensusMod<P> {
    logger: slog::Logger,

    id: String,
    peers: Arc<Mutex<Vec<P>>>,

    // Persistent state.
    metadata: Arc<Metadata>,

    // Volatile general data.
    heartbeat_tx: Arc<Sender<()>>,
    heartbeat_rx: Receiver<()>,
}

impl<P> ConsensusMod<P>
where
    P: Peer + Send + Clone + 'static,
{
    pub fn new(id: String, peers: Vec<P>, logger: &slog::Logger) -> ConsensusMod<P> {
        // Note fix this later and persist it.
        let (heartbeat_tx, heartbeat_rx) = watch::channel(());
        ConsensusMod {
            logger: logger.new(o!("id" => id.clone())),
            id,
            peers: Arc::new(Mutex::new(peers)),
            metadata: Arc::new(Metadata::new()),
            heartbeat_tx: Arc::new(heartbeat_tx),
            heartbeat_rx,
        }
    }

    pub fn append_peer(&self, peer: P) {
        self.peers.lock().unwrap().push(peer);
    }

    fn transition_follower(&self, new_term: Option<u64>) {
        self.metadata.set_state(State::Follower);
        if let Some(new_term) = new_term {
            self.metadata.set_current_term(new_term);
        }
        self.metadata.set_voted_for(None);
    }

    fn transition_candidate(&self) -> u64 {
        self.metadata.set_state(State::Candidate);
        self.metadata.set_voted_for(Some(self.id.clone()));
        self.metadata.incr_current_term()
    }

    async fn follower_loop(&mut self) {
        let dur = rand::thread_rng().gen_range(150..301);
        let dur = Duration::from_millis(dur);

        loop {
            let timed_out = timeout(dur, self.heartbeat_rx.changed()).await.is_err();

            // Finally if we timedout waiting for a heartbeat kickoff an election.
            if timed_out {
                return;
            }
        }
    }

    async fn candidate_loop(&self, saved_term: u64) -> ElectionResult {
        let request = VoteRequest {
            candidate_id: self.id.clone(),
            last_log_idx: 0,
            last_log_term: 0,
            term: *self.metadata.current_term.read().unwrap(),
        };

        let peers = self.peers.lock().unwrap();
        let (tx, mut rx) = mpsc::channel::<Result<VoteResponse>>(peers.len());
        for peer in &*peers {
            let mut cli = peer.clone();
            let req = request.clone();
            let tx = tx.clone();
            tokio::task::spawn(async move {
                let resp = cli.vote(req).await;
                let _ = tx.send(resp).await;
            });
        }
        drop(tx);

        let mut votes: usize = 1;
        for result in rx.recv().await {
            let resp = match result {
                Ok(resp) => resp,
                Err(e) => {
                    error!(self.logger, "Failed to execute VoteRequest rpc."; "error" => e.to_string());
                    continue;
                }
            };

            if !self.metadata.is_candidate() {
                return ElectionResult::Failed;
            }

            if resp.term > saved_term {
                debug!(
                    self.logger,
                    "Encountered response with newer term, bailing out."
                );
                self.transition_follower(Some(resp.term));
                return ElectionResult::Failed;
            } else if resp.term < saved_term {
                debug!(
                    self.logger,
                    "Encountered response with older term, ignoring."
                );
                continue;
            } else if resp.vote_granted {
                votes += 1;
                info!(self.logger, "Vote granted!"; "peers" => peers.len(), "votes" => votes);
                if votes * 2 > peers.len() {
                    info!(self.logger, "returning success");
                    return ElectionResult::Success;
                }
            }
        }
        self.transition_follower(None);
        ElectionResult::Failed
    }

    async fn leader_loop(&self) {
        debug!(self.logger, "Started leader loop!");
        self.metadata.set_state(State::Leader);
        let mut interval = interval(Duration::from_millis(50));

        while self.metadata.is_leader() {
            let request = AppendRequest {
                leader_commit_idx: 0,
                leader_id: self.id.clone(),
                prev_log_idx: 0,
                prev_log_term: 0,
                term: *self.metadata.current_term.read().unwrap(),
                entries: Vec::default(),
            };

            let peers = self.peers.lock().unwrap();
            let (tx, mut rx) = mpsc::channel::<Result<AppendResponse>>(peers.len());
            for peer in &*peers {
                let mut cli = peer.clone();
                let req = request.clone();
                let tx = tx.clone();
                tokio::task::spawn(async move {
                    let resp = cli.append(req).await;
                    let _ = tx.send(resp).await;
                });
            }
            drop(tx);

            let saved_term = *self.metadata.current_term.read().unwrap();
            for resp in rx.recv().await {
                let resp = match resp {
                    Ok(resp) => resp,
                    Err(e) => {
                        error!(self.logger, "Failed to execute VoteRequest rpc."; "error" => e.to_string());
                        continue;
                    }
                };
                if !self.metadata.is_leader() {
                    return;
                }
                if resp.term > saved_term {
                    self.transition_follower(Some(resp.term));
                    return;
                }
            }
            interval.tick().await;
        }
    }

    pub async fn start(&mut self) {
        loop {
            info!(self.logger, "Starting worker!");
            self.follower_loop().await;

            let saved_term = self.transition_candidate();

            match self.candidate_loop(saved_term).await {
                ElectionResult::Failed => continue,
                ElectionResult::Success => {
                    info!(self.logger, "Won the election!!!"; "id" => &self.id)
                }
            };

            self.leader_loop().await;
        }
    }

    pub async fn append(&self, append_request: AppendRequest) -> Result<AppendResponse> {
        info!(self.logger, "Executing append RPC.");

        let mut success = false;

        if append_request.term > *self.metadata.current_term.read().unwrap() {
            debug!(self.logger, "Internal term is out of date with leader term."; "rpc" => "append", "got" => append_request.term, "have" => *self.metadata.current_term.read().unwrap());
            self.transition_follower(Some(append_request.term));
        }

        if append_request.term == *self.metadata.current_term.read().unwrap() {
            if *self.metadata.state.read().unwrap() != State::Follower {
                debug!(self.logger, "Internal state is out of date with leader term."; "rpc" => "append", "got" => append_request.term, "have" => *self.metadata.current_term.read().unwrap());
                self.transition_follower(None);
            }

            self.heartbeat_tx.send(())?;
            success = true;
            // TODO(csaide): Add log management.
        }

        Ok(AppendResponse {
            success,
            term: *self.metadata.current_term.read().unwrap(),
        })
    }

    pub async fn vote(&self, vote_request: VoteRequest) -> Result<VoteResponse> {
        info!(self.logger, "Executing vote RPC.");

        let mut vote_granted = false;

        if vote_request.term > *self.metadata.current_term.read().unwrap() {
            debug!(self.logger, "Internal term is out of date with leader term."; "rpc" => "vote", "got" => vote_request.term, "have" => *self.metadata.current_term.read().unwrap());
            self.transition_follower(Some(vote_request.term));
        }

        if self.metadata.matches_term(vote_request.term) {
            debug!(self.logger, "Matched term");
            let mut voted_for = self.metadata.voted_for.write().unwrap();
            vote_granted = match &*voted_for {
                None => {
                    *voted_for = Some(vote_request.candidate_id);
                    true
                }
                Some(voted_for) if voted_for == &vote_request.candidate_id => true,
                _ => false,
            };
        }

        Ok(VoteResponse {
            vote_granted,
            term: *self.metadata.current_term.read().unwrap(),
        })
    }
}
