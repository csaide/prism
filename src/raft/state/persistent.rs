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
        self.tree.insert("current_term", &term.to_be_bytes())?;
        Ok(())
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
