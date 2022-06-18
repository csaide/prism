// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

use std::sync::RwLock;

use super::{Client, Error, Peer, Peers, Result};

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum State {
    Leader,
    Follower,
    Candidate,
    Dead,
}

#[derive(Debug)]
pub struct PersistentState {
    tree: sled::Tree,
}

impl PersistentState {
    pub fn new(db: &sled::Db) -> Result<PersistentState> {
        let tree = db.open_tree("config")?;
        Ok(PersistentState { tree })
    }

    pub fn get_voted_for(&self) -> Result<Option<String>> {
        match self.tree.get("voted_for")? {
            Some(ivec) => {
                Ok(bincode::deserialize(&ivec).map_err(|e| Error::Serialize(e.to_string()))?)
            }
            None => Ok(None),
        }
    }

    pub fn set_voted_for(&self, voted_for: Option<String>) -> Result<()> {
        self.tree.insert(
            "voted_for",
            bincode::serialize(&voted_for).map_err(|e| Error::Serialize(e.to_string()))?,
        )?;
        self.tree.flush()?;
        Ok(())
    }

    pub fn get_current_term(&self) -> Result<i64> {
        match self.tree.get("current_term")? {
            Some(ivec) => ivec
                .as_ref()
                .try_into()
                .map(i64::from_be_bytes)
                .map_err(Error::from),
            None => Ok(-1),
        }
    }

    pub fn set_current_term(&self, term: i64) -> Result<()> {
        self.tree.insert("current_term", &term.to_be_bytes())?;
        Ok(())
    }

    pub fn incr_current_term(&self) -> Result<i64> {
        match self.tree.fetch_and_update(
            "current_term",
            |old: Option<&[u8]>| -> Option<Vec<u8>> {
                let number = match old {
                    Some(bytes) => {
                        let array: [u8; 8] = bytes.try_into().unwrap();
                        let number = u64::from_be_bytes(array);
                        number + 1
                    }
                    None => 0,
                };

                Some(number.to_be_bytes().to_vec())
            },
        )? {
            Some(ivec) => ivec
                .as_ref()
                .try_into()
                .map(i64::from_be_bytes)
                .map_err(Error::from),
            None => Ok(-1),
        }
    }
}

#[derive(Debug)]
pub struct Metadata<P> {
    pub id: String,

    // Volatile state.
    pub state: RwLock<State>,
    pub last_applied_idx: RwLock<i64>,
    pub commit_idx: RwLock<i64>,
    pub peers: Peers<P>,

    // Persistent state.
    persistent: PersistentState,
}

impl<P> Metadata<P>
where
    P: Client + Send + Clone + 'static,
{
    pub fn new(id: String, peers: Vec<Peer<P>>, db: &sled::Db) -> Result<Metadata<P>> {
        let peers = Peers::bootstrap(peers);
        let persistent = PersistentState::new(db)?;
        Ok(Metadata {
            id,
            state: RwLock::new(State::Follower),
            commit_idx: RwLock::new(-1),
            last_applied_idx: RwLock::new(-1),
            persistent,
            peers,
        })
    }

    pub fn is_leader(&self) -> bool {
        *self.state.read().unwrap() == State::Leader
    }

    pub fn is_candidate(&self) -> bool {
        *self.state.read().unwrap() == State::Candidate
    }

    pub fn is_follower(&self) -> bool {
        *self.state.read().unwrap() == State::Follower
    }

    pub fn matches_term(&self, term: i64) -> bool {
        match self.persistent.get_current_term() {
            Ok(current) => current == term,
            Err(_) => unreachable!(),
        }
    }

    pub fn set_state(&self, state: State) {
        let mut val = self.state.write().unwrap();
        *val = state
    }

    pub fn set_current_term(&self, term: i64) {
        if let Err(_) = self.persistent.set_current_term(term) {
            unreachable!()
        }
    }
    pub fn get_current_term(&self) -> i64 {
        match self.persistent.get_current_term() {
            Ok(term) => term,
            Err(_) => unreachable!(),
        }
    }
    pub fn set_voted_for(&self, voted_for: Option<String>) {
        if let Err(_) = self.persistent.set_voted_for(voted_for) {
            unreachable!()
        }
    }

    pub fn get_voted_for(&self) -> Option<String> {
        match self.persistent.get_voted_for() {
            Ok(voted_for) => voted_for,
            Err(_) => unreachable!(),
        }
    }

    pub fn set_commit_idx(&self, commit_idx: i64) {
        let mut val = self.commit_idx.write().unwrap();
        *val = commit_idx
    }

    pub fn get_commit_idx(&self) -> i64 {
        *self.commit_idx.read().unwrap()
    }

    pub fn set_last_applied_idx(&self, last_applied_idx: i64) {
        let mut val = self.last_applied_idx.write().unwrap();
        *val = last_applied_idx
    }

    pub fn get_last_applied_idx(&self) -> i64 {
        *self.last_applied_idx.read().unwrap()
    }

    pub fn incr_current_term(&self) -> i64 {
        match self.persistent.incr_current_term() {
            Ok(term) => term,
            Err(_) => 0,
        }
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

    pub fn transition_leader(&self, last_log_idx: i64) {
        self.set_state(State::Leader);
        self.peers.reset(last_log_idx);
    }
}
