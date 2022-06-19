// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

use super::{
    AddServerRequest, AddServerResponse, AppendEntriesRequest, AppendEntriesResponse, Client,
    RemoveServerRequest, RemoveServerResponse, RequestVoteRequest, RequestVoteResponse, Result,
};

#[derive(Debug, Clone)]
pub struct Peer<C> {
    client: Option<C>,
    pub id: String,
    pub next_idx: u128,
    pub match_idx: u128,
}

impl<C> Peer<C>
where
    C: Client + Send + Clone + 'static,
{
    pub fn new(id: String) -> Peer<C> {
        Peer {
            id,
            client: None,
            next_idx: 1,
            match_idx: 0,
        }
    }

    pub fn with_client(id: String, client: C) -> Peer<C> {
        Peer {
            id,
            client: Some(client),
            next_idx: 1,
            match_idx: 0,
        }
    }

    pub fn reset(&mut self, last_log_idx: u128) {
        self.next_idx = last_log_idx + 1;
        self.match_idx = 0;
    }

    pub async fn vote(&mut self, req: RequestVoteRequest) -> Result<RequestVoteResponse> {
        if self.client.is_none() {
            let client = C::connect(self.id.clone()).await?;
            self.client = Some(client);
        }
        self.client.as_mut().unwrap().vote(req).await
    }

    pub async fn append(&mut self, req: AppendEntriesRequest) -> Result<AppendEntriesResponse> {
        if self.client.is_none() {
            let client = C::connect(self.id.clone()).await?;
            self.client = Some(client);
        }
        self.client.as_mut().unwrap().append(req).await
    }

    pub async fn add(&mut self, req: AddServerRequest<C>) -> Result<AddServerResponse> {
        if self.client.is_none() {
            let client = C::connect(self.id.clone()).await?;
            self.client = Some(client);
        }
        self.client.as_mut().unwrap().add(req).await
    }

    pub async fn remove(&mut self, req: RemoveServerRequest) -> Result<RemoveServerResponse> {
        if self.client.is_none() {
            let client = C::connect(self.id.clone()).await?;
            self.client = Some(client);
        }
        self.client.as_mut().unwrap().remove(req).await
    }
}
