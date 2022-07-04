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

#[derive(Debug, Serialize, Deserialize)]
pub struct Query {
    pub key: String,
}

impl Query {
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

    fn read(&self, query: Vec<u8>) -> crate::raft::Result<Vec<u8>> {
        let query: Query = match bincode::deserialize(query.as_slice()) {
            Ok(query) => query,
            Err(e) => return Err(Error::Serialize(e.to_string())),
        };

        let state = self.state.lock().unwrap();
        bincode::serialize(&state.get(&query.key)).map_err(|e| Error::Serialize(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use crate::log;

    use super::*;

    #[rstest]
    #[case::apply(vec![Command::Insert(String::from("hello"), 0)], vec![(String::from("hello"), 0)])]
    #[case::apply_remove(vec![Command::Insert(String::from("hello"), 0), Command::Remove(String::from("hello"))], vec![])]
    fn test_apply(#[case] cmds: Vec<Command>, #[case] expected_entries: Vec<(String, u128)>) {
        let hs = HashState::new(&log::noop());
        for cmd in cmds {
            let serialized = cmd.to_bytes();
            hs.apply(crate::raft::Command {
                term: 1,
                data: serialized,
            })
            .expect("Failed to call apply");
        }

        let dump = hs.dump();
        assert_eq!(dump.len(), expected_entries.len());
        for (idx, expected) in expected_entries.iter().enumerate() {
            assert_eq!(expected.0, dump[idx].0);
            assert_eq!(expected.1, dump[idx].1);
        }
    }

    #[test]
    fn test_apply_deserialize_fail() {
        let hs = HashState::new(&log::noop());
        let res = hs.apply(crate::raft::Command {
            term: 1,
            data: Vec::default(),
        });
        assert!(res.is_err());
        let res = res.unwrap_err();
        assert!(matches!(res, Error::Serialize(..)));
    }

    #[rstest]
    #[case::happy_path(vec![Command::Insert(String::from("hello"), 0)], Query{key: String::from("hello")}, Some(0))]
    #[case::empty(vec![], Query{key: String::from("hello")}, None)]
    fn test_read(#[case] cmds: Vec<Command>, #[case] query: Query, #[case] expected: Option<u128>) {
        let hs = HashState::new(&log::noop());
        for cmd in cmds {
            let serialized = cmd.to_bytes();
            hs.apply(crate::raft::Command {
                term: 1,
                data: serialized,
            })
            .expect("Failed to call apply");
        }

        let query = query.to_bytes();
        let read = hs.read(query).expect("Failed to call read.");
        let read: Option<u128> =
            bincode::deserialize(read.as_slice()).expect("Failed to deserialize value.");

        if expected.is_some() {
            assert!(read.is_some());

            let read = read.unwrap();
            let expected = expected.unwrap();
            assert_eq!(expected, read);
        } else {
            assert!(read.is_none());
        }
    }

    #[test]
    fn test_read_deserialize_fail() {
        let hs = HashState::new(&log::noop());
        let read = hs.read(Vec::default());
        assert!(read.is_err());
        let read = read.unwrap_err();
        assert!(matches!(read, Error::Serialize(..)));
    }

    #[test]
    fn test_derived() {
        let cmd = Command::Insert(String::from("hello"), 1);
        let cmd_str = format!("{:?}", cmd);
        assert_eq!(cmd_str, "Insert(\"hello\", 1)");

        let cmd = Command::Remove(String::from("hello"));
        let cmd_str = format!("{:?}", cmd);
        assert_eq!(cmd_str, "Remove(\"hello\")");

        let query = Query {
            key: String::from("hello"),
        };
        let query_str = format!("{:?}", query);
        assert_eq!(query_str, "Query { key: \"hello\" }");

        let log = log::noop();
        let hs = HashState::new(&log);
        let cloned = hs.clone();
        let hs_str = format!("{:?}", cloned);
        assert_eq!(
            hs_str,
            "HashState { logger: Logger(), state: Mutex { data: {}, poisoned: false, .. } }"
        )
    }
}
