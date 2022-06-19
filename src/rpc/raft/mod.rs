// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

mod proto {
    use tonic::transport::Channel;

    use crate::raft::{
        self, AddServerRequest, AddServerResponse, AppendEntriesRequest, AppendEntriesResponse,
        Client, Error, Peer, RemoveServerRequest, RemoveServerResponse, RequestVoteRequest,
        RequestVoteResponse, Result,
    };

    use super::RaftServiceClient;

    tonic::include_proto!("raft");

    #[tonic::async_trait]
    impl Client for raft_service_client::RaftServiceClient<Channel> {
        async fn connect(addr: String) -> Result<RaftServiceClient<Channel>> {
            RaftServiceClient::connect(addr).await.map_err(Error::from)
        }
        async fn vote(&mut self, req: RequestVoteRequest) -> Result<RequestVoteResponse> {
            let resp = self
                .request_vote(VoteRequest::from_raft(req)?)
                .await
                .map(|resp| resp.into_inner())
                .map_err(Error::from)?;
            resp.into_raft()
        }
        async fn append(&mut self, req: AppendEntriesRequest) -> Result<AppendEntriesResponse> {
            let resp = self
                .append_entries(AppendRequest::from_raft(req)?)
                .await
                .map(|resp| resp.into_inner())
                .map_err(Error::from)?;
            resp.into_raft()
        }
        async fn add(&mut self, req: AddServerRequest<Self>) -> Result<AddServerResponse> {
            let resp = self
                .add_server(AddRequest::from_raft(req)?)
                .await
                .map(|resp| resp.into_inner())?;
            resp.into_raft()
        }
        async fn remove(&mut self, req: RemoveServerRequest) -> Result<RemoveServerResponse> {
            let resp = self
                .remove_server(RemoveRequest::from_raft(req)?)
                .await
                .map(|resp| resp.into_inner())?;
            resp.into_raft()
        }
    }

    impl entry::Payload {
        pub fn into_raft(self) -> Result<raft::Entry> {
            use entry::Payload::*;
            match self {
                Command(cmd) => {
                    let term = cmd.term.as_slice().try_into().map(u128::from_be_bytes)?;
                    Ok(raft::Entry::Command(raft::Command {
                        term,
                        data: cmd.data,
                    }))
                }
                Config(cfg) => {
                    let term = cfg.term.as_slice().try_into().map(u128::from_be_bytes)?;
                    Ok(raft::Entry::ClusterConfig(raft::ClusterConfig {
                        term,
                        replicas: cfg.replicas,
                        voters: cfg.voters,
                    }))
                }
            }
        }

        pub fn from_raft(input: raft::Entry) -> Result<entry::Payload> {
            Ok(match input {
                raft::Entry::Command(cmd) => entry::Payload::Command(Command {
                    data: cmd.data,
                    term: cmd.term.to_be_bytes().to_vec(),
                }),
                raft::Entry::ClusterConfig(cfg) => entry::Payload::Config(ClusterConfig {
                    term: cfg.term.to_be_bytes().to_vec(),
                    replicas: cfg.replicas,
                    voters: cfg.voters,
                }),
            })
        }
    }

    impl Entry {
        pub fn into_raft(self) -> Result<raft::Entry> {
            if let Some(payload) = self.payload {
                payload.into_raft()
            } else {
                Err(Error::Missing)
            }
        }

        pub fn from_raft(input: raft::Entry) -> Result<Entry> {
            Ok(Entry {
                payload: Some(entry::Payload::from_raft(input)?),
            })
        }
    }

    impl AppendRequest {
        pub fn into_raft(mut self) -> Result<AppendEntriesRequest> {
            let term = self.term.as_slice().try_into().map(u128::from_be_bytes)?;
            let leader_commit_idx = self
                .leader_commit_idx
                .as_slice()
                .try_into()
                .map(u128::from_be_bytes)?;
            let prev_log_idx = self
                .prev_log_idx
                .as_slice()
                .try_into()
                .map(u128::from_be_bytes)?;
            let prev_log_term = self
                .prev_log_term
                .as_slice()
                .try_into()
                .map(u128::from_be_bytes)?;
            let entries: Result<Vec<raft::Entry>> = self
                .entries
                .drain(..)
                .map(|entry| entry.into_raft())
                .collect();
            let entries = match entries {
                Ok(entries) => entries,
                Err(e) => return Err(e),
            };

            Ok(AppendEntriesRequest {
                term,
                leader_commit_idx,
                prev_log_idx,
                prev_log_term,
                leader_id: self.leader_id,
                entries,
            })
        }

        pub fn from_raft(mut input: AppendEntriesRequest) -> Result<AppendRequest> {
            let entries = input.entries.drain(..).map(Entry::from_raft).collect();
            let entries = match entries {
                Ok(entries) => entries,
                Err(e) => return Err(e),
            };

            Ok(AppendRequest {
                leader_id: input.leader_id,
                term: input.term.to_be_bytes().to_vec(),
                leader_commit_idx: input.leader_commit_idx.to_be_bytes().to_vec(),
                prev_log_idx: input.prev_log_idx.to_be_bytes().to_vec(),
                prev_log_term: input.prev_log_term.to_be_bytes().to_vec(),
                entries,
            })
        }
    }

    impl AppendResponse {
        pub fn into_raft(self) -> Result<AppendEntriesResponse> {
            let term = self.term.as_slice().try_into().map(u128::from_be_bytes)?;
            Ok(AppendEntriesResponse {
                term,
                success: self.success,
            })
        }

        pub fn from_raft(input: AppendEntriesResponse) -> Result<AppendResponse> {
            Ok(AppendResponse {
                term: input.term.to_be_bytes().to_vec(),
                success: input.success,
            })
        }
    }

    impl VoteRequest {
        pub fn into_raft(self) -> Result<RequestVoteRequest> {
            let term = self.term.as_slice().try_into().map(u128::from_be_bytes)?;
            let last_log_idx = self
                .last_log_idx
                .as_slice()
                .try_into()
                .map(u128::from_be_bytes)?;
            let last_log_term = self
                .last_log_term
                .as_slice()
                .try_into()
                .map(u128::from_be_bytes)?;

            Ok(RequestVoteRequest {
                candidate_id: self.candidate_id,
                last_log_idx,
                last_log_term,
                term,
            })
        }

        pub fn from_raft(input: RequestVoteRequest) -> Result<VoteRequest> {
            Ok(VoteRequest {
                candidate_id: input.candidate_id,
                last_log_idx: input.last_log_idx.to_be_bytes().to_vec(),
                last_log_term: input.last_log_term.to_be_bytes().to_vec(),
                term: input.term.to_be_bytes().to_vec(),
            })
        }
    }

    impl VoteResponse {
        pub fn into_raft(self) -> Result<RequestVoteResponse> {
            let term = self.term.as_slice().try_into().map(u128::from_be_bytes)?;
            Ok(RequestVoteResponse {
                term,
                vote_granted: self.vote_granted,
            })
        }

        pub fn from_raft(input: RequestVoteResponse) -> Result<VoteResponse> {
            Ok(VoteResponse {
                term: input.term.to_be_bytes().to_vec(),
                vote_granted: input.vote_granted,
            })
        }
    }

    impl AddRequest {
        pub async fn into_raft(self) -> Result<AddServerRequest<RaftServiceClient<Channel>>> {
            let peer = RaftServiceClient::connect(self.member.clone()).await?;
            let peer = Peer::with_client(peer);
            Ok(AddServerRequest {
                id: self.member,
                peer,
            })
        }

        pub fn from_raft(
            input: AddServerRequest<RaftServiceClient<Channel>>,
        ) -> Result<AddRequest> {
            Ok(AddRequest { member: input.id })
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
}
mod handler;

pub(crate) const FILE_DESCRIPTOR_SET: &[u8] =
    tonic::include_file_descriptor_set!("raft_descriptor");

pub use handler::Handler;
pub use proto::{
    entry::Payload, raft_service_client::RaftServiceClient, raft_service_server::RaftServiceServer,
};
pub use proto::{
    AddRequest, AddResponse, AppendRequest, AppendResponse, ClusterConfig, Command, Entry,
    RemoveRequest, RemoveResponse, VoteRequest, VoteResponse,
};
