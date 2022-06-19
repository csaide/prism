// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

use std::collections::HashMap;
use std::sync::RwLock;

use super::{Client, Mode, Peer, Peers, PersistentState, Result};

#[derive(Debug)]
pub struct State<P> {
    pub id: String,

    // Volatile state.
    pub mode: RwLock<Mode>,
    pub have_leader: RwLock<bool>,
    pub last_cluster_config_idx: RwLock<u128>,
    pub last_applied_idx: RwLock<u128>,
    pub commit_idx: RwLock<u128>,
    pub peers: Peers<P>,

    // Persistent state.
    persistent: PersistentState,
}

impl<P> State<P>
where
    P: Client + Send + Clone + 'static,
{
    pub fn new(id: String, peers: HashMap<String, Peer<P>>, db: &sled::Db) -> Result<State<P>> {
        let peers = Peers::bootstrap(id.clone(), peers, None);
        let persistent = PersistentState::new(db)?;
        Ok(State {
            id,
            have_leader: RwLock::new(false),
            mode: RwLock::new(Mode::Follower),
            last_cluster_config_idx: RwLock::new(0),
            commit_idx: RwLock::new(0),
            last_applied_idx: RwLock::new(0),
            persistent,
            peers,
        })
    }

    pub fn is_leader(&self) -> bool {
        *self.mode.read().unwrap() == Mode::Leader
    }

    pub fn is_candidate(&self) -> bool {
        *self.mode.read().unwrap() == Mode::Candidate
    }

    pub fn is_follower(&self) -> bool {
        *self.mode.read().unwrap() == Mode::Follower
    }

    pub fn is_dead(&self) -> bool {
        *self.mode.read().unwrap() == Mode::Dead
    }

    pub fn saw_leader(&self) {
        *self.have_leader.write().unwrap() = true;
    }

    pub fn lost_leader(&self) {
        *self.have_leader.write().unwrap() = false;
    }

    pub fn have_leader(&self) -> bool {
        *self.have_leader.read().unwrap()
    }

    pub fn matches_term(&self, term: u128) -> bool {
        match self.persistent.get_current_term() {
            Ok(current) => current == term,
            Err(_) => unreachable!(),
        }
    }

    pub fn set_mode(&self, mode: Mode) {
        let mut val = self.mode.write().unwrap();
        *val = mode
    }

    pub fn set_current_term(&self, term: u128) {
        self.persistent.set_current_term(term).unwrap();
    }
    pub fn get_current_term(&self) -> u128 {
        self.persistent.get_current_term().unwrap()
    }
    pub fn set_voted_for(&self, voted_for: Option<String>) {
        self.persistent.set_voted_for(voted_for).unwrap();
    }

    pub fn get_voted_for(&self) -> Option<String> {
        self.persistent.get_voted_for().unwrap()
    }

    pub fn set_commit_idx(&self, commit_idx: u128) {
        let mut val = self.commit_idx.write().unwrap();
        *val = commit_idx
    }

    pub fn get_commit_idx(&self) -> u128 {
        *self.commit_idx.read().unwrap()
    }

    pub fn set_last_applied_idx(&self, last_applied_idx: u128) {
        let mut val = self.last_applied_idx.write().unwrap();
        *val = last_applied_idx
    }

    pub fn get_last_applied_idx(&self) -> u128 {
        *self.last_applied_idx.read().unwrap()
    }

    pub fn incr_current_term(&self) -> u128 {
        self.persistent.incr_current_term().unwrap_or(0)
    }

    pub fn set_last_cluster_config_idx(&self, idx: u128) {
        let mut val = self.last_cluster_config_idx.write().unwrap();
        *val = idx
    }

    pub fn matches_last_cluster_config_idx(&self, idx: u128) -> bool {
        idx >= *self.last_cluster_config_idx.read().unwrap()
    }

    pub fn transition_follower(&self, term: Option<u128>) {
        self.set_mode(Mode::Follower);
        if let Some(term) = term {
            self.set_current_term(term);
        }
        self.set_voted_for(None);
    }

    pub fn transition_candidate(&self) -> u128 {
        self.set_mode(Mode::Candidate);
        self.set_voted_for(Some(self.id.clone()));
        self.incr_current_term()
    }

    pub fn transition_leader(&self, last_log_idx: u128) {
        self.set_mode(Mode::Leader);
        self.peers.lock().reset(last_log_idx);
    }

    pub fn transition_dead(&self) {
        self.set_mode(Mode::Dead);
    }
}
