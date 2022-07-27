// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
    sync::{Mutex, MutexGuard},
};

use super::{Client, ClusterConfig, ListServerResponse, Peer};

#[derive(Debug)]
pub struct ClusterSet<C> {
    id: String,
    members: Mutex<HashMap<String, Peer<C>>>,
}

impl<C> ClusterSet<C> {
    pub fn new(id: String, bootstrap: Option<HashMap<String, Peer<C>>>) -> ClusterSet<C> {
        let members = Mutex::new(bootstrap.unwrap_or_default());
        ClusterSet { id, members }
    }

    pub fn lock(&self) -> ClusterSetGuard<'_, C> {
        ClusterSetGuard {
            id: self.id.clone(),
            members: self.members.lock().unwrap(),
        }
    }
}

pub struct ClusterSetGuard<'a, C> {
    id: String,
    members: MutexGuard<'a, HashMap<String, Peer<C>>>,
}

impl<'a, C> ClusterSetGuard<'a, C>
where
    C: Client + Send + Clone + 'static,
{
    pub fn quorum(&self, count: usize) -> bool {
        count > self.voters().count() / 2
    }

    pub fn voters(&self) -> impl Iterator<Item = (&String, &Peer<C>)> {
        self.members.iter().filter(|(_, peer)| peer.is_voter())
    }

    pub fn replicas(&self) -> impl Iterator<Item = (&String, &Peer<C>)> {
        self.members.iter().filter(|(_, peer)| !peer.is_voter())
    }

    pub fn idx_matches(&self, idx: u64) -> bool {
        let mut matches = 1;

        let voters = self.voters();
        for (_, peer) in voters {
            if peer.match_idx >= idx {
                matches += 1;
            }
        }
        self.quorum(matches)
    }

    pub fn reset(&mut self, last_log_idx: u64) {
        self.members
            .iter_mut()
            .filter(|(_, peer)| peer.is_voter())
            .for_each(|(_, peer)| peer.reset(last_log_idx))
    }

    pub fn to_cluster_config(&self, term: u64) -> ClusterConfig {
        ClusterConfig {
            term,
            voters: self.voters().map(|(id, _)| id).cloned().collect(),
            replicas: self.replicas().map(|(id, _)| id).cloned().collect(),
        }
    }

    pub fn to_list_response(&self, term: u64) -> ListServerResponse {
        ListServerResponse {
            term,
            leader: self.id.clone(),
            voters: self.voters().map(|(id, _)| id).cloned().collect(),
            replicas: self.replicas().map(|(id, _)| id).cloned().collect(),
        }
    }

    pub fn update(&mut self, cfg: ClusterConfig) -> bool {
        let mut found_self = false;

        let mut members = HashMap::with_capacity(cfg.voters.len() + cfg.replicas.len());
        for voter in cfg.voters {
            if voter == self.id {
                found_self = true;
            }
            members.insert(voter.clone(), Peer::voter(voter));
        }

        for replica in cfg.replicas {
            if replica == self.id {
                found_self = true;
            }
            members.insert(replica.clone(), Peer::replica(replica));
        }

        *self.members = members;
        found_self
    }
}

impl<'a, C> Deref for ClusterSetGuard<'a, C> {
    type Target = HashMap<String, Peer<C>>;

    fn deref(&self) -> &Self::Target {
        &self.members
    }
}

impl<'a, C> DerefMut for ClusterSetGuard<'a, C> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.members
    }
}

#[cfg(test)]
mod tests {
    use crate::raft::MockClient;

    use super::*;

    #[test]
    fn test_cluster_set() {
        let id = String::from("leader");
        let bootstrap = HashMap::from_iter(vec![
            (
                String::from("peer1"),
                Peer::<MockClient>::voter(String::from("peer1")),
            ),
            (
                String::from("peer2"),
                Peer::<MockClient>::voter(String::from("peer2")),
            ),
            (
                String::from("peer3"),
                Peer::<MockClient>::voter(String::from("peer3")),
            ),
            (
                String::from("replica1"),
                Peer::<MockClient>::replica(String::from("replica1")),
            ),
            (
                String::from("replica2"),
                Peer::<MockClient>::replica(String::from("replica2")),
            ),
        ]);

        let cluster_set = ClusterSet::new(id, Some(bootstrap));
        let mut locked = cluster_set.lock();

        assert!(locked.quorum(2));
        assert!(!locked.quorum(1));

        assert_eq!(3, locked.voters().count());
        assert_eq!(2, locked.replicas().count());

        assert!(locked.idx_matches(0));
        locked.reset(10);

        let expected = ClusterConfig {
            term: 1,
            voters: vec![
                String::from("peer1"),
                String::from("peer2"),
                String::from("peer3"),
            ],
            replicas: vec![String::from("replica1"), String::from("replica2")],
        };
        let actual = locked.to_cluster_config(1);

        assert_eq!(expected.term, actual.term);
        assert_eq!(expected.voters.len(), actual.voters.len());
        for entry in expected.voters {
            assert!(actual.voters.contains(&entry))
        }
        assert_eq!(expected.replicas.len(), actual.replicas.len());
        for entry in expected.replicas {
            assert!(actual.replicas.contains(&entry))
        }

        let expected = ListServerResponse {
            leader: String::from("leader"),
            term: 1,
            voters: vec![
                String::from("peer1"),
                String::from("peer2"),
                String::from("peer3"),
            ],
            replicas: vec![String::from("replica1"), String::from("replica2")],
        };
        let actual = locked.to_list_response(1);

        assert_eq!(expected.leader, actual.leader);
        assert_eq!(expected.term, actual.term);
        assert_eq!(expected.voters.len(), actual.voters.len());
        for entry in expected.voters {
            assert!(actual.voters.contains(&entry))
        }
        assert_eq!(expected.replicas.len(), actual.replicas.len());
        for entry in expected.replicas {
            assert!(actual.replicas.contains(&entry))
        }

        let new_config = ClusterConfig {
            term: 2,
            voters: vec![
                String::from("peer1"),
                String::from("peer2"),
                String::from("peer3"),
                String::from("peer4"),
            ],
            replicas: vec![String::from("replica1"), String::from("replica2")],
        };
        locked.update(new_config.clone());
        let actual = locked.to_cluster_config(2);

        assert_eq!(new_config.term, actual.term);
        assert_eq!(new_config.voters.len(), actual.voters.len());
        for entry in new_config.voters {
            assert!(actual.voters.contains(&entry))
        }
        assert_eq!(new_config.replicas.len(), actual.replicas.len());
        for entry in new_config.replicas {
            assert!(actual.replicas.contains(&entry))
        }
    }
}
