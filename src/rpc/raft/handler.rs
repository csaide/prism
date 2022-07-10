// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

use tonic::{Request, Response, Status};

use super::{proto::raft_server::Raft, AppendRequest, AppendResponse, VoteRequest, VoteResponse};
use crate::raft::RaftHandler;

pub struct Handler<H> {
    h: H,
}

impl<H> Handler<H>
where
    H: RaftHandler,
{
    pub fn new(h: H) -> Handler<H> {
        Handler { h }
    }

    pub async fn append(
        &self,
        req: Request<AppendRequest>,
    ) -> Result<Response<AppendResponse>, Status> {
        let req = req.into_inner();
        self.h
            .append_entries(req.into())
            .await
            .map(AppendResponse::from)
            .map(Response::new)
            .map_err(|e| e.into())
    }

    pub async fn vote(&self, req: Request<VoteRequest>) -> Result<Response<VoteResponse>, Status> {
        let req = req.into_inner();
        self.h
            .vote_request(req.into())
            .await
            .map(VoteResponse::from)
            .map(Response::new)
            .map_err(|e| e.into())
    }
}

#[tonic::async_trait]
impl<CM> Raft for Handler<CM>
where
    CM: RaftHandler,
{
    /// AppendEntries implements the heartbeat and log replication algorithms from the raft protocol.
    async fn append_entries(
        &self,
        request: Request<AppendRequest>,
    ) -> Result<Response<AppendResponse>, Status> {
        self.append(request).await
    }

    /// RequestVote implements the voting algorithm from the raft protocol.
    async fn request_vote(
        &self,
        request: Request<VoteRequest>,
    ) -> Result<Response<VoteResponse>, Status> {
        self.vote(request).await
    }
}

#[cfg(test)]
mod tests {
    use mockall::predicate::eq;
    use tokio_test::block_on as wait;
    use tonic::transport::Channel;

    use crate::raft::{
        AppendEntriesRequest, AppendEntriesResponse, MockRaftHandler, RequestVoteRequest,
        RequestVoteResponse,
    };
    use crate::rpc::raft::RaftClient;

    use super::*;

    #[test]
    fn test_append() {
        let mut mock = MockRaftHandler::<RaftClient<Channel>>::default();

        let inner = AppendRequest {
            leader_id: String::from("leader"),
            entries: Vec::default(),
            leader_commit_idx: 1u64,
            prev_log_idx: 1u64,
            prev_log_term: 1u64,
            term: 1u64,
        };

        mock.expect_append_entries()
            .with(eq(AppendEntriesRequest::from(inner.clone())))
            .once()
            .returning(|req| {
                Ok(AppendEntriesResponse {
                    success: true,
                    term: req.term,
                })
            });

        let srv = Handler::new(mock);

        let req = Request::new(inner);
        let result = wait(srv.append_entries(req)).expect("should not have returned an error.");
        let result = result.into_inner();
        assert!(result.success);
        assert_eq!(result.term, 1u64);
    }

    #[test]
    fn test_vote() {
        let mut mock = MockRaftHandler::<RaftClient<Channel>>::default();

        let inner = VoteRequest {
            candidate_id: String::from("leader"),
            last_log_idx: 1u64,
            last_log_term: 1u64,
            term: 1u64,
        };

        mock.expect_vote_request()
            .with(eq(RequestVoteRequest::from(inner.clone())))
            .once()
            .returning(|req| {
                Ok(RequestVoteResponse {
                    term: req.term,
                    vote_granted: true,
                })
            });

        let srv = Handler::new(mock);

        let req = Request::new(inner);
        let result = wait(srv.request_vote(req)).expect("should not have returned an error.");
        let result = result.into_inner();
        assert!(result.vote_granted);
        assert_eq!(result.term, 1u64)
    }
}
