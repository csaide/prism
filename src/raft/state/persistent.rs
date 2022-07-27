// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

use super::{Error, Result};

const TREE: &str = "config";
const VOTED_FOR: &str = "voted_for";
const CURRENT_TERM: &str = "current_term";

#[derive(Debug, Clone)]
pub struct PersistentState {
    tree: sled::Tree,
}

impl PersistentState {
    pub fn new(db: &sled::Db) -> Result<PersistentState> {
        db.open_tree(TREE)
            .map(PersistentState::from)
            .map_err(Error::from)
    }

    pub fn get_voted_for(&self) -> Result<Option<String>> {
        self.tree
            .get(VOTED_FOR)?
            .map_or(Ok(None), |ivec| {
                bincode::deserialize::<Option<String>>(&ivec)
            })
            .map_err(Error::from)
    }

    pub fn set_voted_for(&self, voted_for: Option<String>) -> Result<()> {
        let voted_for = bincode::serialize(&voted_for)?;
        self.tree.insert(VOTED_FOR, voted_for)?;
        self.tree.flush().map(|_| ()).map_err(Error::from)
    }

    pub fn matches_term(&self, term: u64) -> Result<bool> {
        self.get_current_term().map(|current| current == term)
    }

    pub fn get_current_term(&self) -> Result<u64> {
        self.tree.get(CURRENT_TERM)?.map_or(Ok(1), |ivec| {
            ivec.as_ref()
                .try_into()
                .map(u64::from_be_bytes)
                .map_err(Error::from)
        })
    }

    pub fn set_current_term(&self, term: u64) -> Result<()> {
        self.tree.insert(CURRENT_TERM, &term.to_be_bytes())?;
        self.tree.flush().map(|_| ()).map_err(Error::from)
    }

    pub fn incr_current_term(&self) -> Result<u64> {
        self.tree
            .update_and_fetch(CURRENT_TERM, |old: Option<&[u8]>| -> Option<Vec<u8>> {
                let new = old
                    .map_or(1, |ivec| {
                        ivec.as_ref().try_into().map(u64::from_be_bytes).unwrap() + 1
                    })
                    .to_be_bytes()
                    .to_vec();
                Some(new)
            })?
            .map(|ivec| {
                ivec.as_ref()
                    .try_into()
                    .map(u64::from_be_bytes)
                    .map_err(Error::from)
            })
            .unwrap_or(Ok(1))
    }
}

impl From<sled::Tree> for PersistentState {
    fn from(tree: sled::Tree) -> PersistentState {
        PersistentState { tree }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_voted_for() {
        let db = sled::Config::new()
            .temporary(true)
            .open()
            .expect("Failed to open temp database.");
        let state = PersistentState::new(&db).expect("Failed to open persistent state.");

        let voted_for = state.get_voted_for().expect("Failed to get voted_for.");
        assert!(voted_for.is_none());

        let name = String::from("hello");
        state
            .set_voted_for(Some(name.clone()))
            .expect("Failed to set voted_for.");

        let voted_for = state.get_voted_for().expect("Failed to get voted_for.");
        assert!(voted_for.is_some());
        let voted_for = voted_for.unwrap();
        assert_eq!(voted_for, name);
    }

    #[test]
    fn test_current_term() {
        let db = sled::Config::new()
            .temporary(true)
            .open()
            .expect("Failed to open temp database.");
        let state = PersistentState::new(&db).expect("Failed to open persistent state.");

        let current_term = state
            .get_current_term()
            .expect("Failed to retrieve current term.");
        assert_eq!(current_term, 1);

        state
            .set_current_term(2)
            .expect("Failed to set current term.");

        let current_term = state
            .get_current_term()
            .expect("Failed to retrieve current term.");
        assert_eq!(current_term, 2);

        let new_term = state
            .incr_current_term()
            .expect("Failed to increment term.");
        assert_eq!(new_term, 3);
        assert!(state.matches_term(3).expect("Failed to match term."));
    }
}
