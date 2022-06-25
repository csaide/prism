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
        let req = req.into_raft()?;
        let resp = self.h.append_entries(req).await?;
        AppendResponse::from_raft(resp)
            .map(Response::new)
            .map_err(|e| e.into())
    }

    pub async fn vote(&self, req: Request<VoteRequest>) -> Result<Response<VoteResponse>, Status> {
        let req = req.into_inner();
        let req = req.into_raft()?;
        let resp = self.h.vote_request(req).await?;
        VoteResponse::from_raft(resp)
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
