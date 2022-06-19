// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

use std::sync::Arc;

use tokio::sync::mpsc;

use super::{
    Client, ElectionResult, Log, Metadata, RequestVoteRequest, RequestVoteResponse, Result,
};

pub struct Candidate<P> {
    logger: slog::Logger,
    metadata: Arc<Metadata<P>>,
    log: Arc<Log>,
    saved_term: u128,
}

impl<P> Candidate<P>
where
    P: Client + Send + Clone + 'static,
{
    pub fn new(
        logger: &slog::Logger,
        metadata: Arc<Metadata<P>>,
        log: Arc<Log>,
        saved_term: u128,
    ) -> Candidate<P> {
        Candidate {
            logger: logger.new(o!("module" => "candidate")),
            metadata,
            log,
            saved_term,
        }
    }

    pub async fn exec(&self) -> ElectionResult {
        let (tx, rx) = mpsc::channel::<Result<RequestVoteResponse>>(self.metadata.peers.len());

        if let Err(e) = self.send_requests(tx) {
            error!(self.logger, "Failed to send vote requests."; "error" => e.to_string());
            return ElectionResult::Failed;
        };
        self.handle_responses(rx).await
    }

    pub fn send_requests(&self, tx: mpsc::Sender<Result<RequestVoteResponse>>) -> Result<()> {
        let (last_log_idx, last_log_term) = self.log.last_log_idx_and_term()?;

        let request = RequestVoteRequest {
            candidate_id: self.metadata.id.clone(),
            last_log_idx,
            last_log_term,
            term: self.saved_term,
        };

        for (_, peer) in self.metadata.peers.lock().iter() {
            let mut cli = peer.clone();
            let req = request.clone();
            let tx = tx.clone();
            tokio::task::spawn(async move {
                let resp = cli.vote(req).await;
                let _ = tx.send(resp).await;
            });
        }
        Ok(())
    }

    pub async fn handle_responses(
        &self,
        mut rx: mpsc::Receiver<Result<RequestVoteResponse>>,
    ) -> ElectionResult {
        let mut votes: usize = 1;
        while let Some(resp) = rx.recv().await {
            if !self.metadata.is_candidate() {
                return ElectionResult::Failed;
            }

            let resp = match resp {
                Ok(resp) => resp,
                Err(e) => {
                    error!(self.logger, "Failed to execute VoteRequest rpc."; "error" => e.to_string());
                    continue;
                }
            };

            if resp.term > self.saved_term {
                debug!(
                    self.logger,
                    "Encountered response with newer term, bailing out."
                );
                self.metadata.transition_follower(Some(resp.term));
                return ElectionResult::Failed;
            }
            if resp.term < self.saved_term {
                debug!(
                    self.logger,
                    "Encountered response with older term, ignoring."
                );
                continue;
            }
            if resp.vote_granted {
                votes += 1;
                if votes * 2 > self.metadata.peers.len() {
                    return ElectionResult::Success;
                }
            }
        }
        self.metadata.transition_follower(None);
        ElectionResult::Failed
    }
}

#[cfg(test)]
#[cfg(not(tarpaulin_include))]
mod tests {
    use std::collections::HashMap;

    use crate::{
        log,
        raft::{
            test_harness::MockPeer, AddServerResponse, AppendEntriesResponse, Error, Peer,
            RemoveServerResponse,
        },
    };

    use super::*;

    #[test]
    fn test_candidate_step_down() {
        let db = sled::Config::new()
            .temporary(true)
            .open()
            .expect("Failed to open temp database");
        let logger = log::noop();

        let log = Log::new(&db).expect("Failed to open log.");
        let log = Arc::new(log);

        let peer1 = MockPeer {
            append_resp: Arc::new(Box::new(|| -> Result<AppendEntriesResponse> {
                unimplemented!()
            })),
            vote_resp: Arc::new(Box::new(|| -> Result<RequestVoteResponse> {
                Ok(RequestVoteResponse {
                    term: 0,
                    vote_granted: false,
                })
            })),
            add_resp: Arc::new(Box::new(|| -> Result<AddServerResponse> {
                unimplemented!()
            })),
            remove_resp: Arc::new(Box::new(|| -> Result<RemoveServerResponse> {
                unimplemented!()
            })),
        };
        let mut peers = HashMap::default();
        peers.insert(
            "grpc://localhost:12345".to_string(),
            Peer::with_client(peer1),
        );
        let metadata = Arc::new(
            Metadata::new(String::from("testing"), peers, &db)
                .expect("Failed to create metadata instance."),
        );

        let candidate = Candidate::new(&logger, metadata.clone(), log, 0);
        let result = tokio_test::block_on(candidate.exec());
        assert_eq!(result, ElectionResult::Failed);
    }

    #[test]
    fn test_candidate_success() {
        let db = sled::Config::new()
            .temporary(true)
            .open()
            .expect("Failed to open temp database");
        let logger = log::noop();

        let log = Log::new(&db).expect("Failed to open log.");
        let log = Arc::new(log);

        let peer1 = MockPeer {
            append_resp: Arc::new(Box::new(|| -> Result<AppendEntriesResponse> {
                unimplemented!()
            })),
            vote_resp: Arc::new(Box::new(|| -> Result<RequestVoteResponse> {
                Ok(RequestVoteResponse {
                    term: 0,
                    vote_granted: false,
                })
            })),
            add_resp: Arc::new(Box::new(|| -> Result<AddServerResponse> {
                unimplemented!()
            })),
            remove_resp: Arc::new(Box::new(|| -> Result<RemoveServerResponse> {
                unimplemented!()
            })),
        };
        let peer2 = MockPeer {
            append_resp: Arc::new(Box::new(|| -> Result<AppendEntriesResponse> {
                unimplemented!()
            })),
            vote_resp: Arc::new(Box::new(|| -> Result<RequestVoteResponse> {
                Ok(RequestVoteResponse {
                    term: 1,
                    vote_granted: true,
                })
            })),
            add_resp: Arc::new(Box::new(|| -> Result<AddServerResponse> {
                unimplemented!()
            })),
            remove_resp: Arc::new(Box::new(|| -> Result<RemoveServerResponse> {
                unimplemented!()
            })),
        };
        let peer3 = MockPeer {
            append_resp: Arc::new(Box::new(|| -> Result<AppendEntriesResponse> {
                unimplemented!()
            })),
            vote_resp: Arc::new(Box::new(|| -> Result<RequestVoteResponse> {
                Ok(RequestVoteResponse {
                    term: 1,
                    vote_granted: true,
                })
            })),
            add_resp: Arc::new(Box::new(|| -> Result<AddServerResponse> {
                unimplemented!()
            })),
            remove_resp: Arc::new(Box::new(|| -> Result<RemoveServerResponse> {
                unimplemented!()
            })),
        };

        let mut peers = HashMap::default();
        peers.insert(
            "grpc://localhost:12345".to_string(),
            Peer::with_client(peer1),
        );
        peers.insert(
            "grpc://localhost:12346".to_string(),
            Peer::with_client(peer2),
        );
        peers.insert(
            "grpc://localhost:12347".to_string(),
            Peer::with_client(peer3),
        );

        let metadata = Arc::new(
            Metadata::new(String::from("testing"), peers, &db)
                .expect("Failed to create metadata instance."),
        );
        let saved_term = metadata.transition_candidate();

        let candidate = Candidate::new(&logger, metadata.clone(), log, saved_term);
        let result = tokio_test::block_on(candidate.exec());
        assert_eq!(result, ElectionResult::Success);
    }

    #[test]
    fn test_candidate_reply_gt_term() {
        let db = sled::Config::new()
            .temporary(true)
            .open()
            .expect("Failed to open temp database");
        let logger = log::noop();

        let log = Log::new(&db).expect("Failed to open log.");
        let log = Arc::new(log);

        let peer1 = MockPeer {
            append_resp: Arc::new(Box::new(|| -> Result<AppendEntriesResponse> {
                unimplemented!()
            })),
            vote_resp: Arc::new(Box::new(|| -> Result<RequestVoteResponse> {
                Ok(RequestVoteResponse {
                    term: 1000,
                    vote_granted: false,
                })
            })),
            add_resp: Arc::new(Box::new(|| -> Result<AddServerResponse> {
                unimplemented!()
            })),
            remove_resp: Arc::new(Box::new(|| -> Result<RemoveServerResponse> {
                unimplemented!()
            })),
        };
        let mut peers = HashMap::default();
        peers.insert(
            "grpc://localhost:12345".to_string(),
            Peer::with_client(peer1),
        );

        let metadata = Arc::new(
            Metadata::new(String::from("testing"), peers, &db)
                .expect("Failed to generate metadata instance."),
        );
        let saved_term = metadata.transition_candidate();

        let candidate = Candidate::new(&logger, metadata.clone(), log, saved_term);
        let result = tokio_test::block_on(candidate.exec());

        assert_eq!(result, ElectionResult::Failed);
        assert!(metadata.is_follower())
    }

    #[test]
    fn test_candidate_rpc_failure() {
        let db = sled::Config::new()
            .temporary(true)
            .open()
            .expect("Failed to open temp database");
        let logger = log::noop();

        let log = Log::new(&db).expect("Failed to open log.");
        let log = Arc::new(log);

        let peer1 = MockPeer {
            append_resp: Arc::new(Box::new(|| -> Result<AppendEntriesResponse> {
                unimplemented!()
            })),
            vote_resp: Arc::new(Box::new(|| -> Result<RequestVoteResponse> {
                Err(Error::Missing)
            })),
            add_resp: Arc::new(Box::new(|| -> Result<AddServerResponse> {
                unimplemented!()
            })),
            remove_resp: Arc::new(Box::new(|| -> Result<RemoveServerResponse> {
                unimplemented!()
            })),
        };
        let mut peers = HashMap::default();
        peers.insert(
            "grpc://localhost:12345".to_string(),
            Peer::with_client(peer1),
        );

        let metadata = Arc::new(
            Metadata::new(String::from("testing"), peers, &db)
                .expect("Failed to generate metadata instance"),
        );

        let saved_term = 10;
        let candidate = Candidate::new(&logger, metadata, log, saved_term);
        let result = tokio_test::block_on(candidate.exec());
        assert_eq!(result, ElectionResult::Failed);
    }
}
