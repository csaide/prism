// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

use super::{
    AppendEntriesRequest, AppendEntriesResponse, Client, RequestVoteRequest, RequestVoteResponse,
    Result,
};

#[derive(Default, Debug, Clone)]
pub struct Peer<C> {
    client: Option<C>,
    pub id: String,
    pub next_idx: u64,
    pub match_idx: u64,
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

    pub fn reset(&mut self, last_log_idx: u64) {
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
}

#[cfg(test)]
mod tests {
    use mockall::predicate::eq;
    use tokio_test::block_on as wait;

    use crate::raft::cluster::client::mock::{get_lock, MockClient, MTX};

    use super::*;

    fn mock_client_vote_factory() -> MockClient {
        let mut mock_peer = MockClient::new();
        mock_peer.expect_clone().returning(mock_client_vote_factory);
        mock_peer.expect_vote().once().returning(|_| {
            Ok(RequestVoteResponse {
                term: 1,
                vote_granted: true,
            })
        });
        mock_peer
    }

    fn mock_client_append_factory() -> MockClient {
        let mut mock_peer = MockClient::new();
        mock_peer
            .expect_clone()
            .returning(mock_client_append_factory);
        mock_peer.expect_append().once().returning(|_| {
            Ok(AppendEntriesResponse {
                term: 1,
                success: true,
            })
        });
        mock_peer
    }

    #[test]
    fn test_peer_vote() {
        let _m = get_lock(&MTX);

        let addr = String::from("grpc://127.0.0.1:8080");
        let ctx = MockClient::connect_context();
        ctx.expect()
            .once()
            .with(eq(addr.clone()))
            .returning(|_| Ok(mock_client_vote_factory()));

        let mut peer = Peer::<MockClient>::new(addr.clone());

        let last_log_idx = 1;
        peer.reset(last_log_idx);

        assert_eq!(peer.next_idx, last_log_idx + 1);
        assert_eq!(peer.match_idx, 0);

        let resp = wait(peer.vote(RequestVoteRequest {
            term: 1,
            candidate_id: addr.clone(),
            last_log_idx: 1,
            last_log_term: 1,
        }));
        assert!(resp.is_ok());
        let resp = resp.unwrap();
        assert!(resp.vote_granted);
    }

    #[test]
    fn test_peer_append() {
        let _m = get_lock(&MTX);

        let addr = String::from("grpc://127.0.0.1:8081");
        let ctx = MockClient::connect_context();
        ctx.expect()
            .once()
            .with(eq(addr.clone()))
            .returning(|_| Ok(mock_client_append_factory()));

        let mut peer = Peer::<MockClient>::new(addr.clone());

        let last_log_idx = 1;
        peer.reset(last_log_idx);

        assert_eq!(peer.next_idx, last_log_idx + 1);
        assert_eq!(peer.match_idx, 0);

        let resp = wait(peer.append(AppendEntriesRequest {
            term: 1,
            entries: Vec::default(),
            leader_commit_idx: 1,
            leader_id: addr.clone(),
            prev_log_idx: 1,
            prev_log_term: 1,
        }));
        assert!(resp.is_ok());
        let resp = resp.unwrap();
        assert!(resp.success);
    }
}
