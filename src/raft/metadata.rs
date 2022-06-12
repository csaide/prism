// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

use std::sync::RwLock;

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum State {
    Leader,
    Follower,
    Candidate,
    Dead,
}

#[derive(Debug)]
pub struct Metadata {
    pub state: RwLock<State>,
    pub current_term: RwLock<u64>,
    pub voted_for: RwLock<Option<String>>,
}

impl Metadata {
    pub fn new() -> Metadata {
        Metadata {
            state: RwLock::new(State::Follower),
            current_term: RwLock::new(0),
            voted_for: RwLock::new(None),
        }
    }

    pub fn is_leader(&self) -> bool {
        *self.state.read().unwrap() == State::Leader
    }
    pub fn is_candidate(&self) -> bool {
        *self.state.read().unwrap() == State::Candidate
    }

    pub fn matches_term(&self, term: u64) -> bool {
        *self.current_term.read().unwrap() == term
    }

    pub fn set_state(&self, state: State) {
        let mut val = self.state.write().unwrap();
        *val = state
    }

    pub fn set_current_term(&self, term: u64) {
        let mut val = self.current_term.write().unwrap();
        *val = term
    }

    pub fn set_voted_for(&self, voted_for: Option<String>) {
        let mut val = self.voted_for.write().unwrap();
        *val = voted_for
    }

    pub fn incr_current_term(&self) -> u64 {
        let mut val = self.current_term.write().unwrap();
        *val += 1;
        *val
    }
}
