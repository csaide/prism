// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

use std::sync::Arc;

use tokio::sync::mpsc;

use super::{Client, ElectionResult, Log, RequestVoteRequest, RequestVoteResponse, Result, State};

#[derive(Debug)]
pub struct Candidate<P> {
    logger: slog::Logger,
    state: Arc<State<P>>,
    log: Arc<Log>,
}

impl<P> Candidate<P>
where
    P: Client + Send + Clone + 'static,
{
    pub fn new(logger: &slog::Logger, state: Arc<State<P>>, log: Arc<Log>) -> Candidate<P> {
        Candidate {
            logger: logger.new(o!("module" => "candidate")),
            state,
            log,
        }
    }

    pub async fn exec(&self, saved_term: u64) -> ElectionResult {
        let (tx, rx) = mpsc::channel::<Result<RequestVoteResponse>>(self.state.peers.lock().len());

        if let Err(e) = self.send_requests(saved_term, tx) {
            error!(self.logger, "Failed to send vote requests."; "error" => e.to_string());
            return ElectionResult::Failed;
        };
        self.handle_responses(saved_term, rx).await
    }

    pub fn send_requests(
        &self,
        saved_term: u64,
        tx: mpsc::Sender<Result<RequestVoteResponse>>,
    ) -> Result<()> {
        let (last_log_idx, last_log_term) = self.log.last_log_idx_and_term()?;

        let request = RequestVoteRequest {
            candidate_id: self.state.id.clone(),
            last_log_idx,
            last_log_term,
            term: saved_term,
        };

        for (_, peer) in self
            .state
            .peers
            .lock()
            .voters()
            .filter(|(id, _)| self.state.id != **id)
        {
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
        saved_term: u64,
        mut rx: mpsc::Receiver<Result<RequestVoteResponse>>,
    ) -> ElectionResult {
        let mut votes: usize = 1;
        while let Some(resp) = rx.recv().await {
            if !self.state.is_candidate() {
                return ElectionResult::Failed;
            }

            let resp = match resp {
                Ok(resp) => resp,
                Err(e) => {
                    error!(self.logger, "Failed to execute VoteRequest rpc."; "error" => e.to_string());
                    continue;
                }
            };

            if resp.term > saved_term {
                debug!(
                    self.logger,
                    "Encountered response with newer term, bailing out."
                );
                self.state.transition_follower(Some(resp.term));
                return ElectionResult::Failed;
            }
            if resp.term < saved_term {
                debug!(
                    self.logger,
                    "Encountered response with older term, ignoring."
                );
                continue;
            }
            if resp.vote_granted {
                votes += 1;
                if self.state.peers.lock().quorum(votes) {
                    return ElectionResult::Success;
                }
            }
        }
        self.state.transition_follower(None);
        ElectionResult::Failed
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::{
        logging,
        raft::{Error, MockClient, Peer},
    };

    use super::*;

    fn mock_client_vote_failed() -> MockClient {
        let mut mock = MockClient::default();
        mock.expect_clone().once().returning(move || {
            let mut mock = MockClient::default();
            mock.expect_vote()
                .once()
                .returning(move |_| Err(Error::Dead));
            mock
        });
        mock
    }

    fn mock_client_vote_false(term: u64) -> MockClient {
        let mut mock = MockClient::default();
        mock.expect_clone().once().returning(move || {
            let mut mock = MockClient::default();
            mock.expect_vote().once().returning(move |_| {
                Ok(RequestVoteResponse {
                    term,
                    vote_granted: false,
                })
            });
            mock
        });
        mock
    }

    fn mock_client_vote_true() -> MockClient {
        let mut mock = MockClient::default();
        mock.expect_clone().once().returning(move || {
            let mut mock = MockClient::default();
            mock.expect_vote().once().returning(|_| {
                Ok(RequestVoteResponse {
                    term: 1,
                    vote_granted: true,
                })
            });
            mock
        });
        mock
    }

    #[test]
    fn test_candidate_step_down() {
        let db = sled::Config::new()
            .temporary(true)
            .open()
            .expect("Failed to open temp database");
        let logger = logging::noop();

        let log = Log::new(&db).expect("Failed to open log.");
        let log = Arc::new(log);

        let peer1 = mock_client_vote_false(1);

        let mut peers = HashMap::default();
        let key = "grpc://localhost:12345".to_string();
        peers.insert(key.clone(), Peer::with_client(key, peer1));
        let state = Arc::new(
            State::new(String::from("testing"), peers, &db)
                .expect("Failed to create state instance."),
        );

        let candidate = Candidate::new(&logger, state.clone(), log);
        let result = tokio_test::block_on(candidate.exec(0));
        assert_eq!(result, ElectionResult::Failed);
    }

    #[test]
    fn test_candidate_success() {
        let db = sled::Config::new()
            .temporary(true)
            .open()
            .expect("Failed to open temp database");
        let logger = logging::noop();

        let log = Log::new(&db).expect("Failed to open log.");
        let log = Arc::new(log);

        let peer1 = mock_client_vote_false(1);
        let peer2 = mock_client_vote_true();
        let peer3 = mock_client_vote_true();

        let mut peers = HashMap::default();
        let key = "grpc://localhost:12345".to_string();
        peers.insert(key.clone(), Peer::with_client(key, peer1));
        let key = "grpc://localhost:12346".to_string();
        peers.insert(key.clone(), Peer::with_client(key, peer2));
        let key = "grpc://localhost:12347".to_string();
        peers.insert(key.clone(), Peer::with_client(key, peer3));

        let state = Arc::new(
            State::new(String::from("testing"), peers, &db)
                .expect("Failed to create state instance."),
        );
        let saved_term = state.transition_candidate();

        let candidate = Candidate::new(&logger, state.clone(), log);
        let result = tokio_test::block_on(candidate.exec(saved_term));
        assert_eq!(result, ElectionResult::Success);
    }

    #[test]
    fn test_candidate_reply_gt_term() {
        let db = sled::Config::new()
            .temporary(true)
            .open()
            .expect("Failed to open temp database");
        let logger = logging::noop();

        let log = Log::new(&db).expect("Failed to open log.");
        let log = Arc::new(log);

        let peer1 = mock_client_vote_false(1000);
        let mut peers = HashMap::default();
        let key = "grpc://localhost:12345".to_string();
        peers.insert(key.clone(), Peer::with_client(key, peer1));

        let state = Arc::new(
            State::new(String::from("testing"), peers, &db)
                .expect("Failed to generate state instance."),
        );
        let saved_term = state.transition_candidate();

        let candidate = Candidate::new(&logger, state.clone(), log);
        let result = tokio_test::block_on(candidate.exec(saved_term));

        assert_eq!(result, ElectionResult::Failed);
        assert!(state.is_follower())
    }

    #[test]
    fn test_candidate_rpc_failure() {
        let db = sled::Config::new()
            .temporary(true)
            .open()
            .expect("Failed to open temp database");
        let logger = logging::noop();

        let log = Log::new(&db).expect("Failed to open log.");
        let log = Arc::new(log);

        let peer1 = mock_client_vote_failed();
        let mut peers = HashMap::default();
        let key = "grpc://localhost:12345".to_string();
        peers.insert(key.clone(), Peer::with_client(key, peer1));

        let state = Arc::new(
            State::new(String::from("testing"), peers, &db)
                .expect("Failed to generate state instance"),
        );

        let saved_term = 10;
        let candidate = Candidate::new(&logger, state, log);
        let result = tokio_test::block_on(candidate.exec(saved_term));
        assert_eq!(result, ElectionResult::Failed);
    }
}
