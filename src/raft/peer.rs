// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

use std::sync::{Mutex, MutexGuard};

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

#[derive(Debug)]
pub struct Peers<C> {
    peers: Mutex<Vec<Peer<C>>>,
}

impl<C> Peers<C>
where
    C: Client + Send + Clone + 'static,
{
    pub fn new() -> Peers<C> {
        let peers = Mutex::new(Vec::default());
        Peers { peers }
    }

    pub fn bootstrap(initial_peers: Vec<Peer<C>>) -> Peers<C> {
        let peers = Mutex::new(initial_peers);
        Peers { peers }
    }

    pub fn len(&self) -> usize {
        self.peers.lock().unwrap().len()
    }

    pub fn is_empty(&self) -> bool {
        self.peers.lock().unwrap().is_empty()
    }

    pub fn append(&self, peer: Peer<C>) {
        self.peers.lock().unwrap().push(peer)
    }

    pub fn reset(&self, last_log_idx: i64) {
        self.peers
            .lock()
            .unwrap()
            .iter_mut()
            .for_each(|peer| peer.reset(last_log_idx))
    }

    pub fn lock(&self) -> LockedPeers<'_, C> {
        LockedPeers {
            items: self.peers.lock().unwrap(),
        }
    }
}

pub struct LockedPeers<'a, C> {
    items: MutexGuard<'a, Vec<Peer<C>>>,
}

impl<'a, C> LockedPeers<'a, C> {
    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Peer<C>> {
        self.items.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Peer<C>> {
        self.items.iter_mut()
    }

    pub fn get(&self, index: usize) -> &Peer<C> {
        &self.items[index]
    }

    pub fn get_mut(&mut self, index: usize) -> &mut Peer<C> {
        &mut self.items[index]
    }
}
