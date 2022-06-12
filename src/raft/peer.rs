// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

use super::Result;
use crate::rpc::raft::{AppendRequest, AppendResponse, VoteRequest, VoteResponse};

#[tonic::async_trait]
pub trait Peer {
    async fn vote(&mut self, req: VoteRequest) -> Result<VoteResponse>;
    async fn append(&mut self, req: AppendRequest) -> Result<AppendResponse>;
}
