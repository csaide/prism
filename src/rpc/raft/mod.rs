// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

mod proto {
    use sled::IVec;
    use tonic::transport::Channel;

    use crate::{
        raft::{Error, Peer, Result},
        store::Serializeable,
    };

    tonic::include_proto!("raft");

    #[tonic::async_trait]
    impl Peer for raft_service_client::RaftServiceClient<Channel> {
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

    impl Serializeable for Entry {
        fn from_raw(data: &[u8]) -> crate::store::Result<Self> {
            let term = data[..8]
                .try_into()
                .map(i64::from_be_bytes)
                .map_err(crate::store::Error::from)?;
            Ok(Self {
                term,
                command: data[8..].to_vec(),
            })
        }
        fn to_raw(&self) -> IVec {
            IVec::from([self.term.to_be_bytes().as_slice(), &self.command].concat())
        }
    }
}
mod handler;

pub(crate) const FILE_DESCRIPTOR_SET: &[u8] =
    tonic::include_file_descriptor_set!("raft_descriptor");

pub use handler::Handler;
pub use proto::{raft_service_client::RaftServiceClient, raft_service_server::RaftServiceServer};
pub use proto::{AppendRequest, AppendResponse, Entry, VoteRequest, VoteResponse};
