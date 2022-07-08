// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

use super::{Error, Result};

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
            Some(ivec) => bincode::deserialize(&ivec).map_err(|e| Error::Serialize(e.to_string())),
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

    pub fn matches_term(&self, term: u128) -> Result<bool> {
        self.get_current_term().map(|current| current == term)
    }

    pub fn get_current_term(&self) -> Result<u128> {
        match self.tree.get("current_term")? {
            Some(ivec) => ivec
                .as_ref()
                .try_into()
                .map(u128::from_be_bytes)
                .map_err(Error::from),
            None => Ok(1),
        }
    }

    pub fn set_current_term(&self, term: u128) -> Result<()> {
        self.tree
            .insert("current_term", &term.to_be_bytes())
            .map(|_| ())
            .map_err(Error::from)
    }

    pub fn incr_current_term(&self) -> Result<u128> {
        self.tree
            .update_and_fetch("current_term", |old: Option<&[u8]>| -> Option<Vec<u8>> {
                let number = match old {
                    Some(bytes) => {
                        let array: [u8; 16] = bytes.try_into().unwrap();
                        let number = u128::from_be_bytes(array);
                        number + 1
                    }
                    None => 1,
                };

                Some(number.to_be_bytes().to_vec())
            })?
            .map(|ivec| {
                ivec.as_ref()
                    .try_into()
                    .map(u128::from_be_bytes)
                    .map_err(Error::from)
            })
            .unwrap_or(Ok(1))
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
