// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard};

use super::{
    AppendEntriesRequest, AppendEntriesResponse, RequestVoteRequest, RequestVoteResponse, Result,
};

#[tonic::async_trait]
pub trait Client: Send + Sync + 'static {
    async fn vote(&mut self, req: RequestVoteRequest) -> Result<RequestVoteResponse>;
    async fn append(&mut self, req: AppendEntriesRequest) -> Result<AppendEntriesResponse>;
}

#[derive(Debug, Clone)]
pub struct Peer<C> {
    client: C,
    pub next_idx: u128,
    pub match_idx: u128,
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

    pub fn reset(&mut self, last_log_idx: u128) {
        self.next_idx = last_log_idx + 1;
        self.match_idx = 0;
    }

    pub async fn vote(&mut self, req: RequestVoteRequest) -> Result<RequestVoteResponse> {
        self.client.vote(req).await
    }

    pub async fn append(&mut self, req: AppendEntriesRequest) -> Result<AppendEntriesResponse> {
        self.client.append(req).await
    }
}

#[derive(Debug)]
pub struct Peers<C> {
    peers: Mutex<HashMap<String, Peer<C>>>,
}

impl<C> Peers<C>
where
    C: Client + Send + Clone + 'static,
{
    pub fn new() -> Peers<C> {
        let peers = Mutex::new(HashMap::default());
        Peers { peers }
    }

    pub fn bootstrap(initial_peers: HashMap<String, Peer<C>>) -> Peers<C> {
        let peers = Mutex::new(initial_peers);
        Peers { peers }
    }

    pub fn len(&self) -> usize {
        self.peers.lock().unwrap().len()
    }

    pub fn is_empty(&self) -> bool {
        self.peers.lock().unwrap().is_empty()
    }

    pub fn append(&self, id: String, peer: Peer<C>) {
        self.peers.lock().unwrap().insert(id, peer);
    }

    pub fn reset(&self, last_log_idx: u128) {
        self.peers
            .lock()
            .unwrap()
            .iter_mut()
            .for_each(|(_, peer)| peer.reset(last_log_idx))
    }

    pub fn lock(&self) -> LockedPeers<'_, C> {
        LockedPeers {
            items: self.peers.lock().unwrap(),
        }
    }
}

impl<C> Default for Peers<C>
where
    C: Client + Send + Clone + 'static,
{
    fn default() -> Self {
        Self::new()
    }
}

pub struct LockedPeers<'a, C> {
    items: MutexGuard<'a, HashMap<String, Peer<C>>>,
}

impl<'a, C> LockedPeers<'a, C> {
    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &Peer<C>)> {
        self.items.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&String, &mut Peer<C>)> {
        self.items.iter_mut()
    }

    pub fn get(&self, id: &String) -> Option<&Peer<C>> {
        self.items.get(id)
    }

    pub fn get_mut(&mut self, id: &String) -> Option<&mut Peer<C>> {
        self.items.get_mut(id)
    }

    pub fn idx_matches(&self, idx: u128) -> bool {
        let mut matches = 1;
        for (_, peer) in self.items.iter() {
            if peer.match_idx >= idx {
                matches += 1;
            }
        }
        matches * 2 > self.len()
    }
}
