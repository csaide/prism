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

    pub fn current_leader(&self) -> Option<String> {
        self.volatile.leader.leader()
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

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use crate::raft::MockClient;

    use super::*;

    fn test_state() -> State<MockClient> {
        let db = sled::Config::new()
            .temporary(true)
            .open()
            .expect("Failed to open temporary database.");
        State::<MockClient>::new(String::from("hello"), HashMap::default(), &db)
            .expect("Failed to create new state object.")
    }

    #[test]
    fn test_transitions() {
        let state = test_state();

        state.transition_dead();
        assert!(state.is_dead());

        state.transition_leader(1);
        assert!(state.is_leader());
        assert!(state
            .peers
            .lock()
            .iter()
            .all(|(_, peer)| peer.match_idx == 0 && peer.next_idx == 2));

        state.set_current_term(1);
        let term = state.transition_candidate();
        assert!(state.is_candidate());
        let voted_for = state
            .get_voted_for()
            .expect("Should have set voted for in transition_candidate");
        assert_eq!(voted_for, "hello");
        assert_eq!(term, 2);
        assert!(state.matches_term(2));

        state.transition_follower(None);
        assert!(state.is_follower());
        assert!(state.matches_term(2));
        assert!(state.get_voted_for().is_none());

        state.transition_follower(Some(3));
        assert!(state.is_follower());
        assert!(state.matches_term(3));
        assert!(state.get_voted_for().is_none());
    }

    #[test]
    fn test_index_handling() {
        let state = test_state();

        assert_eq!(state.get_commit_idx(), 0);
        state.set_commit_idx(1);
        assert_eq!(state.get_commit_idx(), 1);

        assert_eq!(state.get_last_applied_idx(), 0);
        state.set_last_applied_idx(1);
        assert_eq!(state.get_last_applied_idx(), 1);

        assert!(state.matches_last_cluster_config_idx(0));
        state.set_last_cluster_config_idx(1);
        assert!(state.matches_last_cluster_config_idx(1));
    }

    #[test]
    fn test_leader_tracking() {
        let state = test_state();

        assert!(!state.have_leader());
        assert!(matches!(state.current_leader(), None));

        state.saw_leader(String::from("leader"));
        assert!(state.have_leader());
        let leader = state
            .current_leader()
            .expect("Should have been a value got None");
        assert_eq!(leader, "leader");

        state.lost_leader();
        assert!(!state.have_leader());
        assert!(matches!(state.current_leader(), None));
    }

    #[rstest]
    #[case::leader(Mode::Leader)]
    #[case::candidate(Mode::Candidate)]
    #[case::follower(Mode::Follower)]
    #[case::dead(Mode::Dead)]
    fn test_mode_handling(#[case] mode: Mode) {
        let state = test_state();
        state.set_mode(mode.clone());
        match mode {
            Mode::Leader => assert!(state.is_leader()),
            Mode::Candidate => assert!(state.is_candidate()),
            Mode::Follower => assert!(state.is_follower()),
            Mode::Dead => assert!(state.is_dead()),
        }
    }
}
