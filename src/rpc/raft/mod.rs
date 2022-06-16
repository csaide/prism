// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

mod proto {
    use sled::IVec;
    use tonic::transport::Channel;

    use crate::raft::{Client, Error, Result};

    use super::Payload;

    tonic::include_proto!("raft");

    #[tonic::async_trait]
    impl Client for raft_service_client::RaftServiceClient<Channel> {
        async fn vote(&mut self, req: VoteRequest) -> Result<VoteResponse> {
            self.request_vote(req)
                .await
                .map(|resp| resp.into_inner())
                .map_err(Error::from)
        }
        async fn append(&mut self, req: AppendRequest) -> Result<AppendResponse> {
            self.append_entries(req)
                .await
                .map(|resp| resp.into_inner())
                .map_err(Error::from)
        }
    }

    impl Entry {
        pub fn term(&self) -> i64 {
            let payload = match self.payload.as_ref() {
                Some(payload) => payload,
                None => return -1,
            };
            payload.term()
        }
    }

    impl entry::Payload {
        pub fn term(&self) -> i64 {
            use entry::Payload::*;
            match self {
                Config(cfg) => cfg.term,
                Command(cmd) => cmd.term,
                Snapshot(snap) => snap.last_included_term,
            }
        }

        pub fn to_ivec(&self) -> Result<IVec> {
            Ok(IVec::from(
                bincode::serialize(self).map_err(|e| Error::Serialize(e.to_string()))?,
            ))
        }
        pub fn from_ivec(v: IVec) -> Result<Payload> {
            bincode::deserialize(v.as_ref()).map_err(|e| Error::Serialize(e.to_string()))
        }
    }
}
mod handler;

pub(crate) const FILE_DESCRIPTOR_SET: &[u8] =
    tonic::include_file_descriptor_set!("raft_descriptor");

pub use handler::Handler;
pub use proto::{
    entry::Payload, AppendRequest, AppendResponse, Command, Config, Entry, Snapshot, VoteRequest,
    VoteResponse,
};
pub use proto::{raft_service_client::RaftServiceClient, raft_service_server::RaftServiceServer};
