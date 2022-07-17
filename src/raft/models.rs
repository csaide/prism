// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

use super::{Entry, Peer};

#[derive(Default, Debug, Clone, PartialEq, Eq, PartialOrd)]
pub enum ElectionResult {
    Success,
    #[default]
    Failed,
}

/// An [AppendEntriesRequest] handles both heartbeating (empty entries field), or replicating logs between the
/// current cluster leader and its followers.
#[derive(Default, Debug, Clone, PartialEq, Eq, PartialOrd)]
pub struct AppendEntriesRequest {
    pub term: u64,
    pub leader_id: String,
    pub leader_commit_idx: u64,
    pub prev_log_idx: u64,
    pub prev_log_term: u64,
    pub entries: Vec<Entry>,
}

/// An [AppendEntriesResponse] handles informing a given leader of this followers acceptance of its hearbeat/replication request.
#[derive(Default, Debug, Clone, PartialEq, Eq, PartialOrd)]
pub struct AppendEntriesResponse {
    pub term: u64,
    pub success: bool,
}

/// A [RequestVoteRequest] handles requesting votes during a given candidates election.
#[derive(Default, Debug, Clone, PartialEq, Eq, PartialOrd)]
pub struct RequestVoteRequest {
    pub term: u64,
    pub candidate_id: String,
    pub last_log_idx: u64,
    pub last_log_term: u64,
}

/// A [RequestVoteResponse] handles informing the candidate requesting votes, whether or not this meember
/// granted its vote or not.
#[derive(Default, Debug, Clone, PartialEq, Eq, PartialOrd)]
pub struct RequestVoteResponse {
    pub term: u64,
    pub vote_granted: bool,
}

#[derive(Default, Debug, Clone)]
pub struct AddServerRequest<P> {
    pub id: String,
    pub replica: bool,
    pub peer: Peer<P>,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, PartialOrd)]
pub struct AddServerResponse {
    pub status: String,
    pub leader_hint: String,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, PartialOrd)]
pub struct RemoveServerRequest {
    pub id: String,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, PartialOrd)]
pub struct RemoveServerResponse {
    pub status: String,
    pub leader_hint: String,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, PartialOrd)]
pub struct ListServerRequest {}

#[derive(Default, Debug, Clone, PartialEq, Eq, PartialOrd)]
pub struct ListServerResponse {
    pub term: u64,
    pub voters: Vec<String>,
    pub replicas: Vec<String>,
    pub leader: String,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, PartialOrd)]
pub struct MutateStateRequest {
    pub client_id: Vec<u8>,
    pub sequence_num: u64,
    pub command: Vec<u8>,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, PartialOrd)]
pub struct MutateStateResponse {
    pub status: String,
    pub response: Vec<u8>,
    pub leader_hint: String,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, PartialOrd)]
pub struct ReadStateRequest {
    pub query: Vec<u8>,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, PartialOrd)]
pub struct ReadStateResponse {
    pub status: String,
    pub response: Vec<u8>,
    pub leader_hint: String,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, PartialOrd)]
pub struct RegisterClientRequest {}

#[derive(Default, Debug, Clone, PartialEq, Eq, PartialOrd)]
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
}
