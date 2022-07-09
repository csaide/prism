// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

use core::panic;

use serde_derive::{Deserialize, Serialize};
use sled::IVec;

use super::{Error, Result};

/// A [Command] represents a state mutation command log entry for the state machine associated with a given
/// raft cluster.
#[derive(Default, Debug, Serialize, Deserialize, Clone, PartialEq, PartialOrd)]
pub struct Command {
    pub term: u64,
    #[serde(with = "serde_bytes")]
    pub data: Vec<u8>,
}

/// A [ClusterConfig] represents a clust&er configuration change log entry for the various members of a given
/// raft cluster.
#[derive(Default, Debug, Serialize, Deserialize, Clone, PartialEq, PartialOrd)]
pub struct ClusterConfig {
    pub term: u64,
    pub voters: Vec<String>,
    pub replicas: Vec<String>,
}

/// An [Entry] represents a single log entry in a given cluster members persistent log.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, PartialOrd)]
pub enum Entry {
    None,
    Command(Command),
    ClusterConfig(ClusterConfig),
    Registration(u64),
    Noop(u64),
}

impl Default for Entry {
    fn default() -> Self {
        Entry::Command(Command::default())
    }
}

impl Entry {
    pub fn term(&self) -> u64 {
        match self {
            Entry::None => panic!("called term on None entry"),
            Entry::Command(cmd) => cmd.term,
            Entry::ClusterConfig(cfg) => cfg.term,
            Entry::Registration(term) => *term,
            Entry::Noop(term) => *term,
        }
    }

    pub fn is_none(&self) -> bool {
        matches!(self, Entry::None)
    }

    pub fn is_cluster_config(&self) -> bool {
        matches!(self, Entry::ClusterConfig(..))
    }

    pub fn unwrap_cluster_config(&self) -> ClusterConfig {
        match self {
            Entry::ClusterConfig(cfg) => cfg.clone(),
            _ => panic!("called unwrap_cluster_config on non-ClusterConfig type"),
        }
    }

    pub fn is_command(&self) -> bool {
        matches!(self, Entry::Command(..))
    }

    pub fn unwrap_command(&self) -> Command {
        match self {
            Entry::Command(cmd) => cmd.clone(),
            _ => panic!("called unwrap_cluster_config on non=Command type"),
        }
    }

    pub fn is_registration(&self) -> bool {
        matches!(self, Entry::Registration(..))
    }

    pub fn unwrap_registration(&self) -> u64 {
        match self {
            Entry::Registration(term) => *term,
            _ => panic!("called unwrap_cluster_config on non-Registration type"),
        }
    }

    pub fn is_noop(&self) -> bool {
        matches!(self, Entry::Noop(_))
    }

    pub fn unwrap_noop(&self) -> u64 {
        match self {
            Entry::Noop(term) => *term,
            _ => panic!("called unwrap_noop on non-Noop type"),
        }
    }

    pub fn to_ivec(&self) -> Result<IVec> {
        if self.is_none() {
            panic!("called to_vec on None entry")
        }

        bincode::serialize(self)
            .map(IVec::from)
            .map_err(|e| Error::Serialize(e.to_string()))
    }

    pub fn from_ivec(v: IVec) -> Result<Entry> {
        bincode::deserialize(v.as_ref()).map_err(|e| Error::Serialize(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use core::fmt::Debug;

    use rstest::rstest;
    use serde::{Deserialize, Serialize};

    use super::*;

    #[rstest]
    #[case(Command::default())]
    #[case(ClusterConfig::default())]
    #[case(Entry::default())]
    fn test_models<M>(#[case] model: M)
    where
        for<'a> M: Default + Debug + Clone + PartialEq + PartialOrd + Serialize + Deserialize<'a>,
    {
        let cloned = model.clone();
        assert_eq!(cloned, model);
        assert!(cloned >= model);
        assert!(cloned <= model);
        assert!(cloned == model);
        assert_eq!(format!("{:?}", cloned), format!("{:?}", model));

        let model_serialized = bincode::serialize(&model).expect("Failed to serialize");
        let cloned_serialized = bincode::serialize(&cloned).expect("Failed to serialize");
        assert_eq!(cloned_serialized, model_serialized);

        let model_deserialized =
            bincode::deserialize::<M>(&model_serialized).expect("Failed to deserialize");
        let cloned_deserialized =
            bincode::deserialize::<M>(&cloned_serialized).expect("Failed to deserialize");

        assert_eq!(model_deserialized, model);
        assert_eq!(cloned_deserialized, cloned);
    }

    #[rstest]
    #[case::command(Entry::Command(Command::default()), 0)]
    #[case::cluster_config(Entry::ClusterConfig(ClusterConfig::default()), 0)]
    #[case::registration(Entry::Registration(1), 1)]
    #[case::noop(Entry::Noop(1), 1)]
    #[should_panic]
    #[case::none(Entry::None, 1)]
    fn test_term(#[case] input: Entry, #[case] expected: u64) {
        let term = input.term();
        assert_eq!(expected, term)
    }

    #[test]
    fn test_entry_command() {
        let expected = Command::default();
        let entry = Entry::Command(expected.clone());

        assert!(entry.is_command());
        assert!(!entry.is_cluster_config());
        assert!(!entry.is_registration());
        assert!(!entry.is_none());
        assert!(!entry.is_noop());

        let actual = entry.unwrap_command();
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_entry_config() {
        let expected = ClusterConfig::default();
        let entry = Entry::ClusterConfig(expected.clone());

        assert!(!entry.is_command());
        assert!(entry.is_cluster_config());
        assert!(!entry.is_registration());
        assert!(!entry.is_none());
        assert!(!entry.is_noop());

        let actual = entry.unwrap_cluster_config();
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_entry_registration() {
        let expected = 1;
        let entry = Entry::Registration(expected);

        assert!(!entry.is_command());
        assert!(!entry.is_cluster_config());
        assert!(entry.is_registration());
        assert!(!entry.is_none());
        assert!(!entry.is_noop());

        let actual = entry.unwrap_registration();
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_entry_noop() {
        let expected = 1;
        let entry = Entry::Noop(expected);

        assert!(!entry.is_command());
        assert!(!entry.is_cluster_config());
        assert!(!entry.is_registration());
        assert!(!entry.is_none());
        assert!(entry.is_noop());

        let actual = entry.unwrap_noop();
        assert_eq!(actual, expected);
    }

    #[test]
    #[should_panic]
    fn test_unwrap_command_panic() {
        let entry = Entry::Registration(1);
        entry.unwrap_command();
    }

    #[test]
    #[should_panic]
    fn test_unwrap_config_panic() {
        let entry = Entry::Registration(1);
        entry.unwrap_cluster_config();
    }

    #[test]
    #[should_panic]
    fn test_unwrap_registration_panic() {
        let entry = Entry::Command(Command::default());
        entry.unwrap_registration();
    }

    #[test]
    #[should_panic]
    fn test_unwrap_noop_panic() {
        let entry = Entry::Registration(1);
        entry.unwrap_noop();
    }

    #[test]
    fn test_serialization() {
        let entry = Entry::Registration(1);

        let serialized = entry
            .to_ivec()
            .expect("Should have serialized without error.");
        let deserialized =
            Entry::from_ivec(serialized).expect("Should have deserialized without error.");
        assert_eq!(entry, deserialized);

        Entry::from_ivec(IVec::from(vec![0x00])).expect_err("Should have failed to deserialize");
    }
}
