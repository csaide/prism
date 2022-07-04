// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

use serde_derive::{Deserialize, Serialize};
use sled::IVec;

use super::{Error, Peer, Result};

#[derive(Default, Debug, Clone, PartialEq, PartialOrd)]
pub enum ElectionResult {
    Success,
    #[default]
    Failed,
}

/// A [Command] represents a state mutation command log entry for the state machine associated with a given
/// raft cluster.
#[derive(Default, Debug, Serialize, Deserialize, Clone, PartialEq, PartialOrd)]
pub struct Command {
    pub term: u128,
    pub data: Vec<u8>,
}

/// A [ClusterConfig] represents a cluster configuration change log entry for the various members of a given
/// raft cluster.
#[derive(Default, Debug, Serialize, Deserialize, Clone, PartialEq, PartialOrd)]
pub struct ClusterConfig {
    pub term: u128,
    pub voters: Vec<String>,
    pub replicas: Vec<String>,
}

/// A [Registration] represents a client that is registering itself with the cluster.
#[derive(Default, Debug, Serialize, Deserialize, Clone, PartialEq, PartialOrd)]
pub struct Registration {
    pub term: u128,
}

/// An [Entry] represents a single log entry in a given cluster members persistent log.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, PartialOrd)]
pub enum Entry {
    Command(Command),
    ClusterConfig(ClusterConfig),
    Registration(Registration),
}

impl Default for Entry {
    fn default() -> Self {
        Entry::Command(Command::default())
    }
}

impl Entry {
    pub fn term(&self) -> u128 {
        match self {
            Entry::Command(cmd) => cmd.term,
            Entry::ClusterConfig(cfg) => cfg.term,
            Entry::Registration(reg) => reg.term,
        }
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
    pub fn unwrap_registration(&self) -> Registration {
        match self {
            Entry::Registration(reg) => reg.clone(),
            _ => panic!("called unwrap_cluster_config on non-Registration type"),
        }
    }

    pub fn to_ivec(&self) -> Result<IVec> {
        Ok(IVec::from(
            bincode::serialize(self).map_err(|e| Error::Serialize(e.to_string()))?,
        ))
    }
    pub fn from_ivec(v: IVec) -> Result<Entry> {
        bincode::deserialize(v.as_ref()).map_err(|e| Error::Serialize(e.to_string()))
    }
}

/// An [AppendEntriesRequest] handles both heartbeating (empty entries field), or replicating logs between the
/// current cluster leader and its followers.
#[derive(Default, Debug, Clone, PartialEq, PartialOrd)]
pub struct AppendEntriesRequest {
    pub term: u128,
    pub leader_id: String,
    pub leader_commit_idx: u128,
    pub prev_log_idx: u128,
    pub prev_log_term: u128,
    pub entries: Vec<Entry>,
}

/// An [AppendEntriesResponse] handles informing a given leader of this followers acceptance of its hearbeat/replication request.
#[derive(Default, Debug, Clone, PartialEq, PartialOrd)]
pub struct AppendEntriesResponse {
    pub term: u128,
    pub success: bool,
}

/// A [RequestVoteRequest] handles requesting votes during a given candidates election.
#[derive(Default, Debug, Clone, PartialEq, PartialOrd)]
pub struct RequestVoteRequest {
    pub term: u128,
    pub candidate_id: String,
    pub last_log_idx: u128,
    pub last_log_term: u128,
}

/// A [RequestVoteResponse] handles informing the candidate requesting votes, whether or not this meember
/// granted its vote or not.
#[derive(Default, Debug, Clone, PartialEq, PartialOrd)]
pub struct RequestVoteResponse {
    pub term: u128,
    pub vote_granted: bool,
}

#[derive(Default, Debug, Clone)]
pub struct AddServerRequest<P> {
    pub id: String,
    pub replica: bool,
    pub peer: Peer<P>,
}

#[derive(Default, Debug, Clone, PartialEq, PartialOrd)]
pub struct AddServerResponse {
    pub status: String,
    pub leader_hint: String,
}

#[derive(Default, Debug, Clone, PartialEq, PartialOrd)]
pub struct RemoveServerRequest {
    pub id: String,
}

#[derive(Default, Debug, Clone, PartialEq, PartialOrd)]
pub struct RemoveServerResponse {
    pub status: String,
    pub leader_hint: String,
}

#[derive(Default, Debug, Clone, PartialEq, PartialOrd)]
pub struct ListServerRequest {}

#[derive(Default, Debug, Clone, PartialEq, PartialOrd)]
pub struct ListServerResponse {
    pub term: u128,
    pub voters: Vec<String>,
    pub replicas: Vec<String>,
    pub leader: String,
}

#[derive(Default, Debug, Clone, PartialEq, PartialOrd)]
pub struct MutateStateRequest {
    pub client_id: Vec<u8>,
    pub sequence_num: u64,
    pub command: Vec<u8>,
}

#[derive(Default, Debug, Clone, PartialEq, PartialOrd)]
pub struct MutateStateResponse {
    pub status: String,
    pub response: Vec<u8>,
    pub leader_hint: String,
}

#[derive(Default, Debug, Clone, PartialEq, PartialOrd)]
pub struct ReadStateRequest {
    pub query: Vec<u8>,
}

#[derive(Default, Debug, Clone, PartialEq, PartialOrd)]
pub struct ReadStateResponse {
    pub status: String,
    pub response: Vec<u8>,
    pub leader_hint: String,
}

#[derive(Default, Debug, Clone, PartialEq, PartialOrd)]
pub struct RegisterClientRequest {}

#[derive(Default, Debug, Clone, PartialEq, PartialOrd)]
pub struct RegisterClientResponse {
    pub status: String,
    pub client_id: Vec<u8>,
    pub leader_hint: String,
}

#[cfg(test)]
mod tests {
    use core::fmt::Debug;

    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case(RegisterClientResponse::default())]
    #[case(RegisterClientRequest::default())]
    #[case(ElectionResult::default())]
    #[case(Command::default())]
    #[case(ClusterConfig::default())]
    #[case(Registration::default())]
    #[case(Entry::default())]
    #[case(AppendEntriesRequest::default())]
    #[case(AppendEntriesResponse::default())]
    #[case(RequestVoteRequest::default())]
    #[case(RequestVoteResponse::default())]
    #[case(AddServerResponse::default())]
    #[case(RemoveServerRequest::default())]
    #[case(RemoveServerResponse::default())]
    #[case(ListServerRequest::default())]
    #[case(ListServerResponse::default())]
    #[case(MutateStateRequest::default())]
    #[case(MutateStateResponse::default())]
    #[case(ReadStateRequest::default())]
    #[case(ReadStateResponse::default())]
    fn test_models<M>(#[case] model: M)
    where
        M: Default + Debug + Clone + PartialEq + PartialOrd,
    {
        let cloned = model.clone();
        assert_eq!(cloned, model);
        assert!(cloned >= model);
        assert!(cloned <= model);
        assert!(cloned == model);
        assert_eq!(format!("{:?}", cloned), format!("{:?}", model));
    }

    #[test]
    fn test_entry_command() {
        let expected = Command::default();
        let entry = Entry::Command(expected.clone());

        assert!(entry.is_command());
        assert!(!entry.is_cluster_config());
        assert!(!entry.is_registration());

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

        let actual = entry.unwrap_cluster_config();
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_entry_registration() {
        let expected = Registration::default();
        let entry = Entry::Registration(expected.clone());

        assert!(!entry.is_command());
        assert!(!entry.is_cluster_config());
        assert!(entry.is_registration());

        let actual = entry.unwrap_registration();
        assert_eq!(actual, expected);
    }

    #[test]
    #[should_panic]
    fn test_unwrap_command_panic() {
        let entry = Entry::Registration(Registration::default());
        entry.unwrap_command();
    }

    #[test]
    #[should_panic]
    fn test_unwrap_config_panic() {
        let entry = Entry::Registration(Registration::default());
        entry.unwrap_cluster_config();
    }

    #[test]
    #[should_panic]
    fn test_unwrap_registration_panic() {
        let entry = Entry::Command(Command::default());
        entry.unwrap_registration();
    }

    #[test]
    fn test_serialization() {
        let entry = Entry::Registration(Registration::default());

        let serialized = entry
            .to_ivec()
            .expect("Should have serialized without error.");
        let deserialized =
            Entry::from_ivec(serialized).expect("Should have deserialized without error.");
        assert_eq!(entry, deserialized);

        Entry::from_ivec(IVec::from(vec![0x00])).expect_err("Should have failed to deserialize");
    }
}
