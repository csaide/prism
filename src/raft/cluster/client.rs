// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

use super::{
    AddServerRequest, AddServerResponse, AppendEntriesRequest, AppendEntriesResponse,
    RemoveServerRequest, RemoveServerResponse, RequestVoteRequest, RequestVoteResponse, Result,
};

#[tonic::async_trait]
pub trait Client: Send + Sync + 'static + Sized {
    async fn connect(addr: String) -> Result<Self>;
    async fn vote(&mut self, req: RequestVoteRequest) -> Result<RequestVoteResponse>;
    async fn append(&mut self, req: AppendEntriesRequest) -> Result<AppendEntriesResponse>;
    async fn add(&mut self, req: AddServerRequest<Self>) -> Result<AddServerResponse>;
    async fn remove(&mut self, req: RemoveServerRequest) -> Result<RemoveServerResponse>;
}
