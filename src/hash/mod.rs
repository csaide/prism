// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use serde_derive::{Deserialize, Serialize};

use crate::raft::{Error, StateMachine};

#[derive(Debug, Serialize, Deserialize)]
pub enum Command {
    Insert(String, u128),
    Remove(String),
}

impl Command {
    pub fn to_bytes(&self) -> Vec<u8> {
        bincode::serialize(self).unwrap()
    }
}

#[derive(Debug, Clone)]
pub struct HashState {
    logger: slog::Logger,
    state: Arc<Mutex<HashMap<String, u128>>>,
}

impl HashState {
    pub fn new(logger: &slog::Logger) -> HashState {
        let state = Arc::new(Mutex::new(HashMap::default()));
        HashState {
            logger: logger.clone(),
            state,
        }
    }

    pub fn dump(&self) -> Vec<(String, u128)> {
        self.state
            .lock()
            .unwrap()
            .iter()
            .map(|(key, val)| (key.clone(), *val))
            .collect()
    }
}

impl StateMachine for HashState {
    fn apply(&self, command: crate::raft::Command) -> crate::raft::Result<Vec<u8>> {
        let cmd = match bincode::deserialize(command.data.as_ref()) {
            Ok(cmd) => cmd,
            Err(e) => return Err(Error::Serialize(e.to_string())),
        };

        let mut state = self.state.lock().unwrap();
        use Command::*;
        match cmd {
            Insert(key, value) => {
                info!(self.logger, "Got insert command!"; "key" => &key, "value" => value);
                state.insert(key, value)
            }
            Remove(key) => {
                info!(self.logger, "Got remove command!"; "key" => &key);
                state.remove(&key)
            }
        };
        Ok(Vec::default())
    }
}
