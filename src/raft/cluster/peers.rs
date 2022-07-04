// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard};

use crate::raft::ListServerResponse;

use super::{Client, ClusterConfig, Peer, Result};

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

    pub fn lock(&self) -> LockedPeers<'_, C> {
        LockedPeers {
            id: self.id.clone(),
            voters: self.voters.lock().unwrap(),
            replicas: self.replicas.lock().unwrap(),
        }
    }
}

pub struct LockedPeers<'a, C> {
    id: String,
    voters: MutexGuard<'a, HashMap<String, Peer<C>>>,
    replicas: MutexGuard<'a, HashMap<String, Peer<C>>>,
}

impl<'a, C> LockedPeers<'a, C>
where
    C: Client + Send + Clone + 'static,
{
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

    pub fn append(&mut self, id: String, peer: Peer<C>) {
        self.voters.insert(id, peer);
    }

    pub fn remove(&mut self, id: &String) {
        self.voters.remove(id);
    }

    pub fn idx_matches(&self, idx: u128) -> bool {
        let mut matches = 1;
        for (_, peer) in self.voters.iter().filter(|(id, _)| *id != &self.id) {
            if peer.match_idx >= idx {
                matches += 1;
            }
        }
        matches > (self.voters.len() - 1) / 2
    }

    pub fn reset(&mut self, last_log_idx: u128) {
        self.voters
            .iter_mut()
            .for_each(|(_, peer)| peer.reset(last_log_idx))
    }

    pub fn to_cluster_config(&self, term: u128) -> ClusterConfig {
        ClusterConfig {
            term,
            voters: self.voters.iter().map(|(id, _)| id).cloned().collect(),
            replicas: self.replicas.iter().map(|(id, _)| id).cloned().collect(),
        }
    }

    pub fn to_list_response(&self, term: u128) -> ListServerResponse {
        ListServerResponse {
            term,
            leader: self.id.clone(),
            voters: self.voters.iter().map(|(id, _)| id).cloned().collect(),
            replicas: self.replicas.iter().map(|(id, _)| id).cloned().collect(),
        }
    }

    pub fn update(&mut self, mut cfg: ClusterConfig) -> Result<bool> {
        let mut found_self = false;

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

        *self.voters = voters;
        *self.replicas = replicas;
        Ok(found_self)
    }
}

#[cfg(test)]
mod tests {
    use crate::raft::cluster::client::mock::MockClient;

    use super::*;

    #[test]
    fn test_new() {
        let id = String::from("grpc://127.0.0.1:8080");
        let peers = Peers::<MockClient>::new(id.clone());
        assert!(peers.lock().is_empty());
    }

    #[test]
    fn test_peers() {
        let id = String::from("grpc://127.0.0.1:8080");
        let peers = Peers::<MockClient>::new(id.clone());

        let mut locked = peers.lock();
        assert_eq!(0, locked.len());
        assert!(locked.is_empty());

        let peer_id = String::from("grpc://127.0.0.1:8081");
        let peer = Peer::new(peer_id.clone());
        locked.append(peer_id.clone(), peer);
        assert!(locked.contains(&peer_id));

        let actual = locked.get(&peer_id);
        assert!(actual.is_some());

        let actual = locked.get_mut(&peer_id);
        assert!(actual.is_some());
        let actual = actual.unwrap();
        actual.reset(1);

        locked.remove(&peer_id);
        assert!(!locked.contains(&peer_id));
        assert!(locked.get(&peer_id).is_none());
    }

    #[test]
    fn test_peers_cluster_mgmt() {
        let id = String::from("grpc://127.0.0.1:8081");
        let peers = Peers::<MockClient>::new(id.clone());

        let input_cluster_cfg = ClusterConfig {
            term: 1,
            replicas: vec![
                String::from("grpc://127.0.0.1:8081"),
                String::from("grpc://127.0.0.1:8084"),
                String::from("grpc://127.0.0.1:8085"),
            ],
            voters: vec![
                String::from("grpc://127.0.0.1:8081"),
                String::from("grpc://127.0.0.1:8082"),
                String::from("grpc://127.0.0.1:8083"),
            ],
        };

        let mut locked = peers.lock();
        let found_self = locked
            .update(input_cluster_cfg)
            .expect("Failed to update Peers with new cluster config.");
        assert!(found_self);

        locked.reset(1);
        assert!(locked.idx_matches(0));
        assert!(!locked.idx_matches(1));

        let list_resp = locked.to_list_response(1);
        assert_eq!(1, list_resp.term);
        assert_eq!(3, list_resp.voters.len());
        assert_eq!(3, list_resp.replicas.len());
        assert_eq!(id, list_resp.leader);

        let actual_cluster_cfg = locked.to_cluster_config(1);
        assert_eq!(1, actual_cluster_cfg.term);
        assert_eq!(3, actual_cluster_cfg.voters.len());
        assert_eq!(3, actual_cluster_cfg.replicas.len());

        assert_eq!(2, locked.iter_mut().count());
    }
}
