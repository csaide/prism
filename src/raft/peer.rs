// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

use super::Result;
use crate::rpc::raft::{AppendRequest, AppendResponse, VoteRequest, VoteResponse};

#[tonic::async_trait]
pub trait Client: Send + Sync + 'static {
    async fn vote(&mut self, req: VoteRequest) -> Result<VoteResponse>;
    async fn append(&mut self, req: AppendRequest) -> Result<AppendResponse>;
}

#[derive(Debug, Clone)]
pub struct Peer<C> {
    client: C,
    pub next_idx: i64,
    pub match_idx: i64,
}

impl<C> Peer<C>
where
    C: Client + Send + Clone + 'static,
{
    pub fn new(client: C) -> Peer<C> {
        Peer {
            client,
            next_idx: 0,
            match_idx: 0,
        }
    }

    pub fn reset(&mut self, last_log_idx: i64) {
        self.next_idx = last_log_idx + 1;
        self.match_idx = 0;
    }

    pub async fn vote(&mut self, req: VoteRequest) -> Result<VoteResponse> {
        self.client.vote(req).await
    }

    pub async fn append(&mut self, req: AppendRequest) -> Result<AppendResponse> {
        self.client.append(req).await
    }
}
