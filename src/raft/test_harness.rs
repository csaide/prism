// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

use std::sync::Arc;

use super::{
    AddServerRequest, AddServerResponse, AppendEntriesRequest, AppendEntriesResponse, Client,
    RemoveServerRequest, RemoveServerResponse, RequestVoteRequest, RequestVoteResponse, Result,
};

#[derive(Clone)]
pub struct MockPeer {
    pub vote_resp: Arc<Box<dyn Fn() -> Result<RequestVoteResponse> + Send + Sync>>,
    pub append_resp: Arc<Box<dyn Fn() -> Result<AppendEntriesResponse> + Send + Sync>>,
    pub add_resp: Arc<Box<dyn Fn() -> Result<AddServerResponse> + Send + Sync>>,
    pub remove_resp: Arc<Box<dyn Fn() -> Result<RemoveServerResponse> + Send + Sync>>,
}

#[tonic::async_trait]
impl Client for MockPeer {
    async fn connect(_: String) -> Result<MockPeer> {
        unimplemented!()
    }
    async fn vote(&mut self, _: RequestVoteRequest) -> Result<RequestVoteResponse> {
        (self.vote_resp)()
    }
    async fn append(&mut self, _: AppendEntriesRequest) -> Result<AppendEntriesResponse> {
        (self.append_resp)()
    }
    async fn add(&mut self, _: AddServerRequest<MockPeer>) -> Result<AddServerResponse> {
        (self.add_resp)()
    }
    async fn remove(&mut self, _: RemoveServerRequest) -> Result<RemoveServerResponse> {
        (self.remove_resp)()
    }
}
