// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard};

use super::{
    AddServerRequest, AddServerResponse, AppendEntriesRequest, AppendEntriesResponse,
    ClusterConfig, RemoveServerRequest, RemoveServerResponse, RequestVoteRequest,
    RequestVoteResponse, Result,
};

#[tonic::async_trait]
pub trait Client: Send + Sync + 'static + Sized {
    async fn connect(addr: String) -> Result<Self>;
    async fn vote(&mut self, req: RequestVoteRequest) -> Result<RequestVoteResponse>;
    async fn append(&mut self, req: AppendEntriesRequest) -> Result<AppendEntriesResponse>;
    async fn add(&mut self, req: AddServerRequest<Self>) -> Result<AddServerResponse>;
    async fn remove(&mut self, req: RemoveServerRequest) -> Result<RemoveServerResponse>;
}

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

    pub fn with_client(client: C) -> Peer<C> {
        Peer {
            id: String::new(),
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

#[derive(Debug)]
pub struct Peers<C> {
    id: String,
    voters: Mutex<HashMap<String, Peer<C>>>,
    replicas: Mutex<HashMap<String, Peer<C>>>,
}

impl<C> Peers<C>
where
    C: Client + Send + Clone + 'static,
{
    pub fn new(id: String) -> Peers<C> {
        let voters = Mutex::new(HashMap::default());
        let replicas = Mutex::new(HashMap::default());
        Peers {
            id,
            voters,
            replicas,
        }
    }

    pub fn bootstrap(
        id: String,
        voters: HashMap<String, Peer<C>>,
        replicas: Option<HashMap<String, Peer<C>>>,
    ) -> Peers<C> {
        let voters = Mutex::new(voters);
        let replicas = Mutex::new(replicas.unwrap_or_default());
        Peers {
            id,
            voters,
            replicas,
        }
    }

    pub fn len(&self) -> usize {
        self.voters.lock().unwrap().len()
    }

    pub fn is_empty(&self) -> bool {
        self.voters.lock().unwrap().is_empty()
    }

    pub fn append(&self, id: String, peer: Peer<C>) {
        self.voters.lock().unwrap().insert(id, peer);
    }

    pub fn remove(&self, id: &String) {
        self.voters.lock().unwrap().remove(id);
    }

    pub fn contains(&self, id: &String) -> bool {
        self.voters.lock().unwrap().contains_key(id)
    }
    pub async fn update(&self, mut cfg: ClusterConfig) -> Result<bool> {
        let mut found_self = false;
        let mut locked = self.lock();
        let mut voters = HashMap::with_capacity(cfg.voters.len());
        for voter in cfg.voters.drain(..) {
            if voter == self.id {
                found_self = true;
            }

            let cli = Peer::new(voter.clone());
            voters.insert(voter, cli);
        }
        let mut replicas = HashMap::with_capacity(cfg.replicas.len());
        for replica in cfg.replicas.drain(..) {
            if replica == self.id {
                found_self = true;
            }

            let cli = Peer::new(replica.clone());
            replicas.insert(replica, cli);
        }
        *locked.voters = voters;
        *locked.replicas = replicas;
        Ok(found_self)
    }

    pub fn reset(&self, last_log_idx: u128) {
        self.voters
            .lock()
            .unwrap()
            .iter_mut()
            .for_each(|(_, peer)| peer.reset(last_log_idx))
    }

    pub fn lock(&self) -> LockedPeers<'_, C> {
        LockedPeers {
            id: self.id.clone(),
            voters: self.voters.lock().unwrap(),
            replicas: self.replicas.lock().unwrap(),
        }
    }

    pub fn to_cluster_config(&self) -> ClusterConfig {
        ClusterConfig {
            term: 0,
            voters: self
                .voters
                .lock()
                .unwrap()
                .iter()
                .map(|(id, _)| id)
                .cloned()
                .collect(),
            replicas: self
                .replicas
                .lock()
                .unwrap()
                .iter()
                .map(|(id, _)| id)
                .cloned()
                .collect(),
        }
    }
}

pub struct LockedPeers<'a, C> {
    id: String,
    voters: MutexGuard<'a, HashMap<String, Peer<C>>>,
    replicas: MutexGuard<'a, HashMap<String, Peer<C>>>,
}

impl<'a, C> LockedPeers<'a, C> {
    pub fn len(&self) -> usize {
        self.voters.len()
    }

    pub fn is_empty(&self) -> bool {
        self.voters.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &Peer<C>)> {
        self.voters.iter().filter(|(id, _)| **id != self.id)
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&String, &mut Peer<C>)> {
        self.voters.iter_mut().filter(|(id, _)| **id != self.id)
    }

    pub fn contains(&self, id: &String) -> bool {
        self.voters.contains_key(id)
    }

    pub fn get(&self, id: &String) -> Option<&Peer<C>> {
        self.voters.get(id)
    }

    pub fn get_mut(&mut self, id: &String) -> Option<&mut Peer<C>> {
        self.voters.get_mut(id)
    }

    pub fn idx_matches(&self, idx: u128) -> bool {
        let mut matches = 1;
        for (_, peer) in self.voters.iter().filter(|(id, _)| **id != self.id) {
            if peer.match_idx >= idx {
                matches += 1;
            }
        }
        matches > (self.voters.len() - 1) / 2
    }
}
