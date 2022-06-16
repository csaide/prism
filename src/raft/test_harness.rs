// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

use std::sync::Arc;

use super::{Client, Result};
use crate::rpc::raft::{AppendRequest, AppendResponse, VoteRequest, VoteResponse};

#[derive(Clone)]
pub struct MockPeer {
    pub vote_resp: Arc<Box<dyn Fn() -> Result<VoteResponse> + Send + Sync>>,
    pub append_resp: Arc<Box<dyn Fn() -> Result<AppendResponse> + Send + Sync>>,
}

#[tonic::async_trait]
impl Client for MockPeer {
    async fn vote(&mut self, _: VoteRequest) -> Result<VoteResponse> {
        (self.vote_resp)()
    }
    async fn append(&mut self, _: AppendRequest) -> Result<AppendResponse> {
        (self.append_resp)()
    }
}
