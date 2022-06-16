// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

use tonic::{Request, Response, Status};

use super::{
    proto::raft_service_server::RaftService, AppendRequest, AppendResponse, VoteRequest,
    VoteResponse,
};
use crate::raft::ConcensusRepo;

pub struct Handler<CM> {
    cm: CM,
}

impl<CM> Handler<CM>
where
    CM: ConcensusRepo,
{
    pub fn new(cm: CM) -> Handler<CM> {
        Handler { cm }
    }

    pub async fn append(
        &self,
        req: Request<AppendRequest>,
    ) -> Result<Response<AppendResponse>, Status> {
        let req = req.into_inner();
        self.cm
            .append_entries(req)
            .await
            .map(Response::new)
            .map_err(|e| e.into())
    }

    pub async fn vote(&self, req: Request<VoteRequest>) -> Result<Response<VoteResponse>, Status> {
        let req = req.into_inner();
        self.cm
            .vote_request(req)
            .await
            .map(Response::new)
            .map_err(|e| e.into())
    }
}

#[tonic::async_trait]
impl<CM> RaftService for Handler<CM>
where
    CM: ConcensusRepo,
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
