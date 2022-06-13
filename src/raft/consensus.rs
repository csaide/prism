// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

use std::sync::{Arc, Mutex};

use rand::Rng;
use tokio::sync::mpsc;
use tokio::sync::watch::{self, Receiver, Sender};
use tokio::time::{interval, timeout, Duration};

use crate::rpc::raft::{AppendRequest, AppendResponse, Entry, VoteRequest, VoteResponse};

use super::{ElectionResult, Error, Log, Metadata, Peer, Result, State};

#[derive(Debug, Clone)]
pub struct ConsensusMod<P> {
    logger: slog::Logger,

    id: String,
    peers: Arc<Mutex<Vec<P>>>,

    // Persistent state.
    metadata: Arc<Metadata>,
    log: Arc<Log>,

    // Volatile general data.
    heartbeat_tx: Arc<Sender<()>>,
    heartbeat_rx: Receiver<()>,

    next_idx: Arc<Mutex<Vec<i64>>>,
    match_idx: Arc<Mutex<Vec<i64>>>,
}

impl<P> ConsensusMod<P>
where
    P: Peer + Send + Clone + 'static,
{
    pub fn new(
        id: String,
        peers: Vec<P>,
        logger: &slog::Logger,
        db: &sled::Db,
    ) -> Result<ConsensusMod<P>> {
        // Note fix this later and persist it.
        let (heartbeat_tx, heartbeat_rx) = watch::channel(());
        let log = Log::new(db)?;
        let next_idx = Arc::new(Mutex::new(Vec::default()));
        let match_idx = Arc::new(Mutex::new(Vec::default()));
        Ok(ConsensusMod {
            logger: logger.new(o!("id" => id.clone())),
            id,
            peers: Arc::new(Mutex::new(peers)),
            metadata: Arc::new(Metadata::new()),
            log: Arc::new(log),
            heartbeat_tx: Arc::new(heartbeat_tx),
            heartbeat_rx,
            next_idx,
            match_idx,
        })
    }

    pub fn append_peer(&self, peer: P) {
        self.peers.lock().unwrap().push(peer);
    }

    pub fn is_leader(&self) -> bool {
        self.metadata.is_leader()
    }
    pub fn dump(&self) -> Result<Vec<Entry>> {
        self.log.dump()
    }

    fn transition_follower(&self, new_term: Option<i64>) {
        self.metadata.set_state(State::Follower);
        if let Some(new_term) = new_term {
            self.metadata.set_current_term(new_term);
        }
        self.metadata.set_voted_for(None);
    }

    fn transition_candidate(&self) -> i64 {
        self.metadata.set_state(State::Candidate);
        self.metadata.set_voted_for(Some(self.id.clone()));
        self.metadata.incr_current_term()
    }

    fn transition_leader(&self) -> Result<()> {
        self.metadata.set_state(State::Leader);
        let (last_log_idx, _) = self.log.last_log_idx_and_term()?;

        let peers = self.peers.lock().unwrap().len();

        let mut next_idx = self.next_idx.lock().unwrap();
        next_idx.clear();

        let mut match_idx = self.match_idx.lock().unwrap();
        match_idx.clear();

        for _ in 0..peers {
            next_idx.push(last_log_idx);
            match_idx.push(0);
        }
        Ok(())
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

    async fn candidate_loop(&self, saved_term: i64) -> ElectionResult {
        let (last_log_idx, last_log_term) = match self.log.last_log_idx_and_term() {
            Ok(tuple) => tuple,
            Err(e) => {
                error!(self.logger, "Failed to pull last log index and term."; "error" => e.to_string());
                return ElectionResult::Failed;
            }
        };

        let request = VoteRequest {
            candidate_id: self.id.clone(),
            last_log_idx,
            last_log_term,
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
        while let Some(resp) = rx.recv().await {
            let resp = match resp {
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
        if let Err(e) = self.transition_leader() {
            error!(self.logger, "Failed to transition self to leader."; "error" => e.to_string());
            self.transition_follower(None);
            return;
        }

        let mut interval = interval(Duration::from_millis(50));

        while self.metadata.is_leader() {
            let saved_term = *self.metadata.current_term.read().unwrap();

            let peers = self.peers.lock().unwrap();
            let mut next_indexes = self.next_idx.lock().unwrap();
            let mut match_indexes = self.match_idx.lock().unwrap();

            let (tx, mut rx) =
                mpsc::channel::<(usize, usize, i64, Result<AppendResponse>)>(peers.len());
            for (peer_idx, peer) in peers.iter().enumerate() {
                let next_idx = next_indexes[peer_idx];

                let prev_log_idx = next_idx - 1;
                let mut prev_log_term = -1;
                if prev_log_idx >= 0 {
                    prev_log_term = match self.log.get(prev_log_idx) {
                        Ok(entry) => entry.term,
                        Err(e) => {
                            error!(self.logger, "Failed to pull previous log entry."; "error" => e.to_string());
                            -1
                        }
                    };
                }

                let entries = match self.log.range(next_idx, 100) {
                    Ok(entries) => entries,
                    Err(e) => {
                        error!(self.logger, "Failed to return range of entries."; "error" => e.to_string());
                        Vec::default()
                    }
                };

                let req = AppendRequest {
                    leader_commit_idx: *self.metadata.commit_idx.read().unwrap(),
                    leader_id: self.id.clone(),
                    prev_log_idx,
                    prev_log_term,
                    term: saved_term,
                    entries,
                };

                let mut cli = peer.clone();
                let tx = tx.clone();
                tokio::task::spawn(async move {
                    let entries = req.entries.len();
                    let resp = cli.append(req).await;
                    let _ = tx.send((entries, peer_idx, next_idx, resp)).await;
                });
            }
            drop(tx);

            while let Some(resp) = rx.recv().await {
                let (entries, peer_idx, next_idx, resp) = resp;
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
                if resp.term < saved_term {
                    continue;
                }

                if resp.success {
                    next_indexes[peer_idx] = next_idx + entries as i64;
                    match_indexes[peer_idx] = next_indexes[peer_idx] - 1;

                    let saved_commit = *self.metadata.commit_idx.read().unwrap();
                    let start = saved_commit + 1;
                    for idx in start..self.log.len() as i64 {
                        let entry = match self.log.get(idx) {
                            Ok(entry) => entry,
                            Err(e) => {
                                error!(self.logger, "Failed to pull entry."; "error" => e.to_string());
                                continue;
                            }
                        };
                        if entry.term != saved_term {
                            continue;
                        }
                        let mut matches = 1;
                        for match_idx in match_indexes.iter() {
                            if match_idx >= &idx {
                                matches += 1;
                            }
                        }
                        if matches * 2 > peers.len() {
                            let mut commit_idx = self.metadata.commit_idx.write().unwrap();
                            *commit_idx = idx;
                        }
                    }
                    if saved_commit != *self.metadata.commit_idx.read().unwrap() {
                        info!(self.logger, "commited index!");
                    }
                } else {
                    next_indexes[peer_idx] = next_idx - 1;
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

    pub async fn submit(&self, command: Vec<u8>) -> Result<()> {
        if !self.metadata.is_leader() {
            return Err(Error::InvalidState);
        }
        let current_term = self.metadata.current_term.read().unwrap();
        self.log.append(*current_term, command)
    }

    pub async fn append(&self, mut append_request: AppendRequest) -> Result<AppendResponse> {
        // info!(self.logger, "Executing append RPC.");

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
                        append_request.entries[entries_insert_index as usize].term,
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
                    self.log.insert(log_insert_index, entry)?;
                    log_insert_index += 1;
                }

                let mut commit_idx = self.metadata.commit_idx.write().unwrap();
                if append_request.leader_commit_idx > *commit_idx {
                    *commit_idx = append_request.leader_commit_idx.min(self.log.len() as i64);
                    info!(self.logger, "Set new commit!");
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
            self.transition_follower(Some(vote_request.term));
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
