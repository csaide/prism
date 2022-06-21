// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

use std::collections::HashMap;

use super::{Client, Mode, Peer, Peers, PersistentState, Result, VolatileState};

#[derive(Debug)]
pub struct State<P> {
    pub id: String,

    // Volatile state.
    pub peers: Peers<P>,
    volatile: VolatileState,

    // Persistent state.
    persistent: PersistentState,
}

/*
   Mode handling
*/
impl<P> State<P> {
    #[inline]
    pub fn is_leader(&self) -> bool {
        self.volatile.mode.is_leader()
    }

    #[inline]
    pub fn is_candidate(&self) -> bool {
        self.volatile.mode.is_candidate()
    }

    #[inline]
    pub fn is_follower(&self) -> bool {
        self.volatile.mode.is_follower()
    }

    #[inline]
    pub fn is_dead(&self) -> bool {
        self.volatile.mode.is_dead()
    }

    #[inline]
    pub fn set_mode(&self, mode: Mode) {
        self.volatile.mode.set(mode)
    }
}

/*
   Leader tracking
*/
impl<P> State<P> {
    pub fn saw_leader(&self, leader: String) {
        self.volatile.leader.saw_leader(leader)
    }

    pub fn lost_leader(&self) {
        self.volatile.leader.lost_leader()
    }

    pub fn have_leader(&self) -> bool {
        self.volatile.leader.have_leader()
    }
}

/*
    Commit, last cluster cfg index, and last applied index tracking.
*/
impl<P> State<P> {
    pub fn set_commit_idx(&self, commit_idx: u128) {
        self.volatile.commit_idx.set(commit_idx)
    }

    pub fn get_commit_idx(&self) -> u128 {
        self.volatile.commit_idx.get()
    }

    pub fn set_last_applied_idx(&self, last_applied_idx: u128) {
        self.volatile.last_applied_idx.set(last_applied_idx)
    }

    pub fn get_last_applied_idx(&self) -> u128 {
        self.volatile.last_applied_idx.get()
    }

    pub fn set_last_cluster_config_idx(&self, idx: u128) {
        self.volatile.last_cluster_config_idx.set(idx)
    }

    pub fn matches_last_cluster_config_idx(&self, idx: u128) -> bool {
        idx >= self.volatile.last_cluster_config_idx.get()
    }
}

/*
    Persistent state handling.
*/
impl<P> State<P> {
    /*
        Current term handling.
    */
    pub fn matches_term(&self, term: u128) -> bool {
        match self.persistent.matches_term(term) {
            Ok(matches) => matches,
            Err(_) => unreachable!(),
        }
    }

    pub fn set_current_term(&self, term: u128) {
        self.persistent.set_current_term(term).unwrap();
    }

    pub fn get_current_term(&self) -> u128 {
        self.persistent.get_current_term().unwrap()
    }

    pub fn incr_current_term(&self) -> u128 {
        self.persistent.incr_current_term().unwrap_or(0)
    }

    /*
        Voted for handling.
    */
    pub fn set_voted_for(&self, voted_for: Option<String>) {
        self.persistent.set_voted_for(voted_for).unwrap();
    }

    pub fn get_voted_for(&self) -> Option<String> {
        self.persistent.get_voted_for().unwrap()
    }
}

impl<P> State<P>
where
    P: Client + Send + Clone + 'static,
{
    pub fn new(id: String, peers: HashMap<String, Peer<P>>, db: &sled::Db) -> Result<State<P>> {
        let peers = Peers::bootstrap(id.clone(), peers, None);
        let persistent = PersistentState::new(db)?;
        let volatile = VolatileState::default();
        Ok(State {
            id,
            volatile,
            persistent,
            peers,
        })
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
