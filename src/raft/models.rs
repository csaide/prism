// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

use serde_derive::{Deserialize, Serialize};
use sled::IVec;

use super::{Error, Result};

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum ElectionResult {
    Success,
    Failed,
}

/// A [Command] represents a state mutation command log entry for the state machine associated with a given
/// raft cluster.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, PartialOrd)]
pub struct Command {
    pub term: u128,
    pub data: Vec<u8>,
}

/// A [ClusterConfig] represents a cluster configuration change log entry for the various members of a given
/// raft cluster.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, PartialOrd)]
pub struct ClusterConfig {
    pub term: u128,
    pub voters: Vec<String>,
    pub replicas: Vec<String>,
}

/// An [Entry] represents a single log entry in a given cluster members persistent log.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, PartialOrd)]
pub enum Entry {
    Command(Command),
    ClusterConfig(ClusterConfig),
}

impl Entry {
    pub fn term(&self) -> u128 {
        match self {
            Entry::Command(cmd) => cmd.term,
            Entry::ClusterConfig(cfg) => cfg.term,
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
#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct AppendEntriesRequest {
    pub term: u128,
    pub leader_id: String,
    pub leader_commit_idx: u128,
    pub prev_log_idx: u128,
    pub prev_log_term: u128,
    pub entries: Vec<Entry>,
}

/// An [AppendEntriesResponse] handles informing a given leader of this followers acceptance of its hearbeat/replication request.
#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct AppendEntriesResponse {
    pub term: u128,
    pub success: bool,
}

/// A [RequestVoteRequest] handles requesting votes during a given candidates election.
#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct RequestVoteRequest {
    pub term: u128,
    pub candidate_id: String,
    pub last_log_idx: u128,
    pub last_log_term: u128,
}

/// A [RequestVoteResponse] handles informing the candidate requesting votes, whether or not this meember
/// granted its vote or not.
#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct RequestVoteResponse {
    pub term: u128,
    pub vote_granted: bool,
}
