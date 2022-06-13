// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

use std::sync::{Mutex, RwLock};

use super::Result;

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum State {
    Leader,
    Follower,
    Candidate,
    Dead,
}

#[derive(Debug)]
pub struct Metadata<P> {
    pub id: String,
    pub state: RwLock<State>,
    pub commit_idx: RwLock<i64>,
    pub peers: Mutex<Vec<P>>,
    pub next_idx: Mutex<Vec<i64>>,
    pub match_idx: Mutex<Vec<i64>>,

    pub voted_for: RwLock<Option<String>>,
    pub current_term: RwLock<i64>,
}

impl<P> Metadata<P> {
    pub fn new(id: String, peers: Vec<P>) -> Metadata<P> {
        let next_idx = Mutex::new(Vec::default());
        let match_idx = Mutex::new(Vec::default());
        Metadata {
            id,
            state: RwLock::new(State::Follower),
            current_term: RwLock::new(-1),
            commit_idx: RwLock::new(-1),
            voted_for: RwLock::new(None),
            peers: Mutex::new(peers),
            next_idx,
            match_idx,
        }
    }

    pub fn is_leader(&self) -> bool {
        *self.state.read().unwrap() == State::Leader
    }
    pub fn is_candidate(&self) -> bool {
        *self.state.read().unwrap() == State::Candidate
    }

    pub fn matches_term(&self, term: i64) -> bool {
        *self.current_term.read().unwrap() == term
    }

    pub fn set_state(&self, state: State) {
        let mut val = self.state.write().unwrap();
        *val = state
    }

    pub fn set_current_term(&self, term: i64) {
        let mut val = self.current_term.write().unwrap();
        *val = term
    }

    pub fn set_voted_for(&self, voted_for: Option<String>) {
        let mut val = self.voted_for.write().unwrap();
        *val = voted_for
    }

    pub fn incr_current_term(&self) -> i64 {
        let mut val = self.current_term.write().unwrap();
        *val += 1;
        *val
    }

    pub fn transition_follower(&self, term: Option<i64>) {
        self.set_state(State::Follower);
        if let Some(term) = term {
            self.set_current_term(term);
        }
        self.set_voted_for(None);
    }

    pub fn transition_candidate(&self) -> i64 {
        self.set_state(State::Candidate);
        self.set_voted_for(Some(self.id.clone()));
        self.incr_current_term()
    }

    pub fn transition_leader(&self, last_log_idx: i64) -> Result<()> {
        self.set_state(State::Leader);
        let peers = self.peers.lock().unwrap().len();
        let mut next_idx = self.next_idx.lock().unwrap();
        let mut match_idx = self.match_idx.lock().unwrap();

        next_idx.clear();
        match_idx.clear();

        for _ in 0..peers {
            next_idx.push(last_log_idx);
            match_idx.push(0);
        }
        Ok(())
    }
}
