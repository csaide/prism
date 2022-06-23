// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

use tonic::transport::Channel;

use crate::raft::{
    AddServerRequest, AddServerResponse, ListServerRequest, ListServerResponse, Peer,
    RemoveServerRequest, RemoveServerResponse, Result,
};

use crate::rpc::raft::RaftClient;

tonic::include_proto!("cluster");

impl AddRequest {
    pub async fn into_raft(self) -> Result<AddServerRequest<RaftClient<Channel>>> {
        let peer = RaftClient::connect(self.member.clone()).await?;
        let peer = Peer::with_client(self.member.clone(), peer);
        Ok(AddServerRequest {
            id: self.member,
            replica: self.replica,
            peer,
        })
    }

    pub fn from_raft(input: AddServerRequest<RaftClient<Channel>>) -> Result<AddRequest> {
        Ok(AddRequest {
            member: input.id,
            replica: input.replica,
        })
    }
}

impl AddResponse {
    pub fn into_raft(self) -> Result<AddServerResponse> {
        Ok(AddServerResponse {
            leader_hint: self.leader_hint,
            status: self.status,
        })
    }
    pub fn from_raft(input: AddServerResponse) -> Result<AddResponse> {
        Ok(AddResponse {
            leader_hint: input.leader_hint,
            status: input.status,
        })
    }
}

impl RemoveRequest {
    pub fn into_raft(self) -> Result<RemoveServerRequest> {
        Ok(RemoveServerRequest { id: self.member })
    }
    pub fn from_raft(input: RemoveServerRequest) -> Result<RemoveRequest> {
        Ok(RemoveRequest { member: input.id })
    }
}

impl RemoveResponse {
    pub fn into_raft(self) -> Result<RemoveServerResponse> {
        Ok(RemoveServerResponse {
            leader_hint: self.leader_hint,
            status: self.status,
        })
    }

    pub fn from_raft(input: RemoveServerResponse) -> Result<RemoveResponse> {
        Ok(RemoveResponse {
            leader_hint: input.leader_hint,
            status: input.status,
        })
    }
}

impl ListRequest {
    pub fn into_raft(self) -> Result<ListServerRequest> {
        Ok(ListServerRequest {})
    }

    pub fn from_raft(_: ListServerRequest) -> Result<ListRequest> {
        Ok(ListRequest {})
    }
}

impl ListResponse {
    pub fn into_raft(self) -> Result<ListServerResponse> {
        let term = self.term.as_slice().try_into().map(u128::from_be_bytes)?;
        Ok(ListServerResponse {
            leader: self.leader,
            replicas: self.replicas,
            voters: self.voters,
            term,
        })
    }

    pub fn from_raft(input: ListServerResponse) -> Result<ListResponse> {
        Ok(ListResponse {
            leader: input.leader,
            replicas: input.replicas,
            voters: input.voters,
            term: input.term.to_be_bytes().to_vec(),
        })
    }
}
