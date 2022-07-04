// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

use tonic::transport::Channel;

use crate::raft::{
    AddServerRequest, AddServerResponse, ListServerRequest, ListServerResponse, Peer,
    RemoveServerRequest, RemoveServerResponse,
};

use crate::rpc::raft::RaftClient;

tonic::include_proto!("cluster");

impl From<AddRequest> for AddServerRequest<RaftClient<Channel>> {
    fn from(input: AddRequest) -> AddServerRequest<RaftClient<Channel>> {
        let peer = Peer::new(input.member.clone());
        AddServerRequest {
            id: input.member,
            replica: input.replica,
            peer,
        }
    }
}

impl From<AddServerRequest<RaftClient<Channel>>> for AddRequest {
    fn from(input: AddServerRequest<RaftClient<Channel>>) -> AddRequest {
        AddRequest {
            member: input.id,
            replica: input.replica,
        }
    }
}

impl From<AddResponse> for AddServerResponse {
    fn from(input: AddResponse) -> AddServerResponse {
        AddServerResponse {
            leader_hint: input.leader_hint,
            status: input.status,
        }
    }
}

impl From<AddServerResponse> for AddResponse {
    fn from(input: AddServerResponse) -> AddResponse {
        AddResponse {
            leader_hint: input.leader_hint,
            status: input.status,
        }
    }
}

impl From<RemoveRequest> for RemoveServerRequest {
    fn from(input: RemoveRequest) -> RemoveServerRequest {
        RemoveServerRequest { id: input.member }
    }
}

impl From<RemoveServerRequest> for RemoveRequest {
    fn from(input: RemoveServerRequest) -> RemoveRequest {
        RemoveRequest { member: input.id }
    }
}

impl From<RemoveResponse> for RemoveServerResponse {
    fn from(input: RemoveResponse) -> RemoveServerResponse {
        RemoveServerResponse {
            leader_hint: input.leader_hint,
            status: input.status,
        }
    }
}

impl From<RemoveServerResponse> for RemoveResponse {
    fn from(input: RemoveServerResponse) -> RemoveResponse {
        RemoveResponse {
            leader_hint: input.leader_hint,
            status: input.status,
        }
    }
}

impl From<ListRequest> for ListServerRequest {
    fn from(_: ListRequest) -> ListServerRequest {
        ListServerRequest {}
    }
}

impl From<ListServerRequest> for ListRequest {
    fn from(_: ListServerRequest) -> ListRequest {
        ListRequest {}
    }
}

impl From<ListResponse> for ListServerResponse {
    fn from(input: ListResponse) -> ListServerResponse {
        let term = input
            .term
            .as_slice()
            .try_into()
            .map(u128::from_be_bytes)
            .unwrap_or(0);

        ListServerResponse {
            leader: input.leader,
            replicas: input.replicas,
            voters: input.voters,
            term,
        }
    }
}

impl From<ListServerResponse> for ListResponse {
    fn from(input: ListServerResponse) -> ListResponse {
        ListResponse {
            leader: input.leader,
            replicas: input.replicas,
            voters: input.voters,
            term: input.term.to_be_bytes().to_vec(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_request() {
        let id = String::from("hello");
        let raft: AddServerRequest<RaftClient<Channel>> = AddServerRequest {
            id: id.clone(),
            peer: Peer::new(id.clone()),
            replica: false,
        };

        let actual = AddRequest::from(raft);
        assert_eq!(actual.member, id);
        assert!(!actual.replica);

        let actual_raft = AddServerRequest::from(actual);
        assert_eq!(actual_raft.id, id);
        assert!(!actual_raft.replica);
    }

    #[test]
    fn test_add_response() {
        let leader_hint = String::from("hello");
        let status = String::from("status");
        let raft = AddServerResponse {
            leader_hint: leader_hint.clone(),
            status: status.clone(),
        };

        let actual = AddResponse::from(raft);
        assert_eq!(actual.leader_hint, leader_hint);
        assert_eq!(actual.status, status);

        let actual_raft = AddServerResponse::from(actual);
        assert_eq!(actual_raft.leader_hint, leader_hint);
        assert_eq!(actual_raft.status, status);
    }

    #[test]
    fn test_remove_request() {
        let id = String::from("hello");
        let raft = RemoveServerRequest { id: id.clone() };

        let actual = RemoveRequest::from(raft);
        assert_eq!(actual.member, id);

        let actual_raft = RemoveServerRequest::from(actual);
        assert_eq!(actual_raft.id, id);
    }

    #[test]
    fn test_remove_response() {
        let leader_hint = String::from("hello");
        let status = String::from("status");
        let raft = RemoveServerResponse {
            leader_hint: leader_hint.clone(),
            status: status.clone(),
        };

        let actual = RemoveResponse::from(raft);
        assert_eq!(actual.leader_hint, leader_hint);
        assert_eq!(actual.status, status);

        let actual_raft = RemoveServerResponse::from(actual);
        assert_eq!(actual_raft.leader_hint, leader_hint);
        assert_eq!(actual_raft.status, status);
    }

    #[test]
    fn test_list_request() {
        let raft = ListServerRequest {};

        let actual = ListRequest::from(raft);
        let _actual_raft = ListServerRequest::from(actual);
    }

    #[test]
    fn test_list_response() {
        let voters = vec![
            String::from("hello"),
            String::from("goodbye"),
            String::from("noop"),
        ];
        let replicas = vec![String::from("repl1"), String::from("repl2")];
        let leader = String::from("hello");

        let raft = ListServerResponse {
            leader: leader.clone(),
            replicas: replicas.clone(),
            voters: voters.clone(),
            term: 1,
        };

        let actual = ListResponse::from(raft);
        assert_eq!(2, actual.replicas.len());
        assert_eq!(3, actual.voters.len());
        assert_eq!(leader, actual.leader);
        assert_eq!(1u128.to_be_bytes().to_vec(), actual.term);

        let actual_raft = ListServerResponse::from(actual);
        assert_eq!(2, actual_raft.replicas.len());
        assert_eq!(3, actual_raft.voters.len());
        assert_eq!(leader, actual_raft.leader);
        assert_eq!(1, actual_raft.term)
    }
}
