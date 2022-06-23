// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

use super::{
    AppendEntriesRequest, AppendEntriesResponse, RequestVoteRequest, RequestVoteResponse, Result,
};

#[tonic::async_trait]
pub trait Client: Send + Sync + 'static + Sized {
    async fn connect(addr: String) -> Result<Self>;
    async fn vote(&mut self, req: RequestVoteRequest) -> Result<RequestVoteResponse>;
    async fn append(&mut self, req: AppendEntriesRequest) -> Result<AppendEntriesResponse>;
}
