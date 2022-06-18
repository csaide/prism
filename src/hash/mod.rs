// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use serde_derive::{Deserialize, Serialize};

use crate::raft::StateMachine;

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
    state: Arc<Mutex<HashMap<String, u128>>>,
}

impl HashState {
    pub fn new() -> HashState {
        let state = Arc::new(Mutex::new(HashMap::default()));
        HashState { state }
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

impl Default for HashState {
    fn default() -> Self {
        Self::new()
    }
}

impl StateMachine for HashState {
    fn apply(&self, command: crate::raft::Command) {
        let cmd = match bincode::deserialize(command.data.as_ref()) {
            Ok(cmd) => cmd,
            Err(_) => return,
        };

        let mut state = self.state.lock().unwrap();
        use Command::*;
        match cmd {
            Insert(key, value) => state.insert(key, value),
            Remove(key) => state.remove(&key),
        };
    }
}
