// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

use std::iter::Iterator;
use std::{collections::HashMap, sync::Arc};

use tokio::sync::{mpsc, watch};

use super::{AppendEntriesRequest, AppendEntriesResponse, Client, Log, Result, State};

#[derive(Debug, Clone)]
pub struct Quorum<P> {
    logger: slog::Logger,
    state: Arc<State<P>>,
    log: Arc<Log>,
    commit_tx: Arc<watch::Sender<()>>,
}

impl<P> Quorum<P>
where
    P: Client + Send + Clone + 'static,
{
    pub fn new(
        logger: &slog::Logger,
        state: Arc<State<P>>,
        log: Arc<Log>,
        commit_tx: Arc<watch::Sender<()>>,
    ) -> Quorum<P> {
        Quorum {
            logger: logger.clone(),
            state,
            log,
            commit_tx,
        }
    }

    pub async fn exec(&self) -> bool {
        if !self.state.is_leader() || self.state.peers.lock().is_empty() {
            return false;
        }

        let saved_term = self.state.get_current_term();

        let (tx, rx) = mpsc::channel::<(usize, String, Result<AppendEntriesResponse>)>(
            self.state.peers.lock().len(),
        );

        self.send_requests(saved_term, &tx);
        drop(tx);

        self.handle_responses(saved_term, rx).await
    }

    fn send_requests(
        &self,
        saved_term: u64,
        tx: &mpsc::Sender<(usize, String, Result<AppendEntriesResponse>)>,
    ) {
        for (peer_id, peer) in self
            .state
            .peers
            .lock()
            .iter()
            .filter(|(id, _)| **id != self.state.id)
        {
            let prev_log_idx = peer.next_idx - 1;
            let mut prev_log_term = 0;
            if prev_log_idx > 0 {
                prev_log_term = match self.log.get(prev_log_idx) {
                    Ok(opt) => opt.map(|entry| entry.term()).unwrap_or(0),
                    Err(_) => continue,
                };
            }

            let entries = self
                .log
                .range(peer.next_idx..)
                .entries()
                .filter_map(|entry| entry.ok())
                .collect();
            let req = AppendEntriesRequest {
                leader_commit_idx: self.state.get_commit_idx(),
                leader_id: self.state.id.clone(),
                prev_log_idx,
                prev_log_term,
                term: saved_term,
                entries,
            };

            let mut cli = peer.clone();
            let tx = tx.clone();
            let id = peer_id.clone();
            tokio::task::spawn(async move {
                let entries = req.entries.len();
                let resp = cli.append(req).await;
                let _ = tx.send((entries, id, resp)).await;
            });
        }
    }

    async fn handle_responses(
        &self,
        saved_term: u64,
        mut rx: mpsc::Receiver<(usize, String, Result<AppendEntriesResponse>)>,
    ) -> bool {
        let mut successes = HashMap::<String, bool>::default();
        successes.insert(self.state.id.clone(), true);

        while let Some(evt) = rx.recv().await {
            let (entries, peer_id, resp) = evt;
            let resp = match resp {
                Ok(resp) => resp,
                Err(e) => {
                    error!(self.logger, "Failed to execute AppendEntries rpc."; "error" => e.to_string(), "peer" => peer_id);
                    continue;
                }
            };

            if !self.state.is_leader() {
                return false;
            }
            if resp.term > saved_term {
                self.state.transition_follower(Some(resp.term));
                return false;
            }
            if resp.term < saved_term {
                continue;
            }

            let mut locked_peers = self.state.peers.lock();
            let mut peer = match locked_peers.get_mut(&peer_id) {
                Some(peer) => peer,
                None => continue,
            };

            if !resp.success {
                peer.next_idx -= 1;
                successes.insert(peer_id.clone(), false);
                continue;
            }

            peer.next_idx += entries as u64;
            peer.match_idx = peer.next_idx - 1;

            if peer.is_voter() {
                successes.insert(peer_id.clone(), true);
            } else {
                continue;
            }

            let saved_commit = self.state.get_commit_idx();
            let start = saved_commit + 1;
            for idx in start.. {
                match self.log.get(idx) {
                    Ok(entry) => match entry {
                        Some(entry) => {
                            if entry.term() != saved_term {
                                continue;
                            }
                        }
                        None => break,
                    },
                    Err(e) => {
                        error!(self.logger, "Failed to pull entry."; "error" => e.to_string());
                        continue;
                    }
                };
                if locked_peers.idx_matches(idx) {
                    self.state.set_commit_idx(idx);
                }
            }
            if saved_commit != self.state.get_commit_idx() {
                if let Err(e) = self.commit_tx.send(()) {
                    error!(self.logger, "Failed to send commit notification."; "error" => e.to_string());
                }
                if self
                    .state
                    .matches_last_cluster_config_idx(self.state.get_commit_idx())
                    && !locked_peers.contains_key(&self.state.id)
                {
                    self.state.transition_dead();
                    return false;
                }
            }
        }
        let peers = self.state.peers.lock().len();
        peers >= 2 && successes.iter().filter(|(_, val)| **val).count() > peers / 2
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use rstest::rstest;

    use crate::{
        logging,
        raft::{Command, Entry, Error, MockClient, Mode, Peer},
    };

    use super::*;

    fn test_setup(
        peers: HashMap<String, Peer<MockClient>>,
        entries: Vec<Entry>,
        mode: Mode,
    ) -> Quorum<MockClient> {
        let logger = logging::noop();
        let db = sled::Config::new()
            .temporary(true)
            .open()
            .expect("Failed to open temp database");

        let log = Log::new(&db).expect("Failed to open log.");
        log.append_entries(0, entries).expect("Failed to seed log.");

        let log = Arc::new(log);

        let id = String::from("leader");
        let state = State::<MockClient>::new(id.clone(), peers, &db)
            .expect("Failed to init internal state.");
        state.set_mode(mode);

        let state = Arc::new(state);

        let (commit_tx, _) = watch::channel(());
        let commit_tx = Arc::new(commit_tx);

        Quorum::<MockClient>::new(&logger, state.clone(), log.clone(), commit_tx.clone())
    }

    fn mock_client_error() -> MockClient {
        let mut mock = MockClient::default();
        mock.expect_clone().once().returning(|| -> MockClient {
            let mut mock = MockClient::default();
            mock.expect_append()
                .once()
                .returning(|_| -> Result<AppendEntriesResponse> { Err(Error::Dead) });
            mock
        });
        mock
    }

    fn mock_client_success() -> MockClient {
        let mut mock = MockClient::default();
        mock.expect_clone().once().returning(|| -> MockClient {
            let mut mock = MockClient::default();
            mock.expect_append()
                .once()
                .returning(|req| -> Result<AppendEntriesResponse> {
                    Ok(AppendEntriesResponse {
                        success: true,
                        term: req.term,
                    })
                });
            mock
        });
        mock
    }

    fn mock_client_fail() -> MockClient {
        let mut mock = MockClient::default();
        mock.expect_clone().once().returning(|| -> MockClient {
            let mut mock = MockClient::default();
            mock.expect_append()
                .once()
                .returning(|req| -> Result<AppendEntriesResponse> {
                    Ok(AppendEntriesResponse {
                        success: false,
                        term: req.term,
                    })
                });
            mock
        });
        mock
    }

    fn happy_path() -> HashMap<String, Peer<MockClient>> {
        let mut peers = HashMap::default();

        let id1 = String::from("peer1");
        let peer1 = mock_client_success();
        peers.insert(id1.clone(), Peer::with_client(id1, peer1));

        let id2 = String::from("peer2");
        let peer2 = mock_client_success();
        peers.insert(id2.clone(), Peer::with_client(id2, peer2));

        let id3 = String::from("leader");
        let peer3 = MockClient::default();
        peers.insert(id3.clone(), Peer::with_client(id3, peer3));
        peers
    }

    fn single_erroring_peer() -> HashMap<String, Peer<MockClient>> {
        let mut peers = HashMap::default();

        let id1 = String::from("peer1");
        let peer1 = mock_client_success();
        peers.insert(id1.clone(), Peer::with_client(id1, peer1));

        let id2 = String::from("peer2");
        let peer2 = mock_client_error();
        peers.insert(id2.clone(), Peer::with_client(id2, peer2));

        let id3 = String::from("leader");
        let peer3 = MockClient::default();
        peers.insert(id3.clone(), Peer::with_client(id3, peer3));
        peers
    }

    fn multiple_erroring_peer() -> HashMap<String, Peer<MockClient>> {
        let mut peers = HashMap::default();

        let id1 = String::from("peer1");
        let peer1 = mock_client_error();
        peers.insert(id1.clone(), Peer::with_client(id1, peer1));

        let id2 = String::from("peer2");
        let peer2 = mock_client_error();
        peers.insert(id2.clone(), Peer::with_client(id2, peer2));

        let id3 = String::from("leader");
        let peer3 = MockClient::default();
        peers.insert(id3.clone(), Peer::with_client(id3, peer3));
        peers
    }

    fn single_non_successful_peer() -> HashMap<String, Peer<MockClient>> {
        let mut peers = HashMap::default();

        let id1 = String::from("peer1");
        let peer1 = mock_client_success();
        peers.insert(id1.clone(), Peer::with_client(id1, peer1));

        let id2 = String::from("peer2");
        let peer2 = mock_client_fail();
        peers.insert(id2.clone(), Peer::with_client(id2, peer2));

        let id3 = String::from("leader");
        let peer3 = MockClient::default();
        peers.insert(id3.clone(), Peer::with_client(id3, peer3));
        peers
    }

    fn multiple_non_successful_peer() -> HashMap<String, Peer<MockClient>> {
        let mut peers = HashMap::default();

        let id1 = String::from("peer1");
        let peer1 = mock_client_fail();
        peers.insert(id1.clone(), Peer::with_client(id1, peer1));

        let id2 = String::from("peer2");
        let peer2 = mock_client_fail();
        peers.insert(id2.clone(), Peer::with_client(id2, peer2));

        let id3 = String::from("leader");
        let peer3 = MockClient::default();
        peers.insert(id3.clone(), Peer::with_client(id3, peer3));
        peers
    }

    #[rstest]
    #[case::empty_peers(
        || -> HashMap<String, Peer<MockClient>> {
            HashMap::default()
        },
        Mode::Leader,
        Vec::default(),
        false
    )]
    #[case::not_leader(
        || -> HashMap<String, Peer<MockClient>> {
            HashMap::default()
        },
        Mode::Follower,
        Vec::default(),
        false
    )]
    #[case::happy_path(happy_path, Mode::Leader, vec![Entry::Command(Command{term:1, data:vec![0x00]}),Entry::Command(Command{term:1, data:vec![0x01]})], true)]
    #[case::single_erroring_peer(single_erroring_peer, Mode::Leader, Vec::default(), true)]
    #[case::multiple_erroring_peers(multiple_erroring_peer, Mode::Leader, Vec::default(), false)]
    #[case::single_failure(single_non_successful_peer, Mode::Leader, Vec::default(), true)]
    #[case::multiple_failure(multiple_non_successful_peer, Mode::Leader, Vec::default(), false)]
    #[tokio::test]
    async fn test_quorum<F: FnMut() -> HashMap<String, Peer<MockClient>>>(
        #[case] mut peers_factory: F,
        #[case] mode: Mode,
        #[case] entries: Vec<Entry>,
        #[case] expected: bool,
    ) {
        let peers = (peers_factory)();
        let quorum = test_setup(peers, entries, mode);

        let confirmed = quorum.exec().await;
        assert_eq!(confirmed, expected);
    }
}
