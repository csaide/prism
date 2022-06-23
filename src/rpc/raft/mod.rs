// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

mod handler;
mod proto;

pub(crate) const FILE_DESCRIPTOR_SET: &[u8] =
    tonic::include_file_descriptor_set!("raft_descriptor");

pub use handler::Handler;
pub use proto::{entry::Payload, raft_client::RaftClient, raft_server::RaftServer};
pub use proto::{
    AppendRequest, AppendResponse, ClusterConfig, Command, Entry, VoteRequest, VoteResponse,
};
