// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

use tonic::{transport::Channel, Request, Response, Status};

use super::{
    proto::raft_service_server::RaftService, AppendRequest, AppendResponse, RaftServiceClient,
    VoteRequest, VoteResponse,
};
use crate::raft::ConsensusMod;

pub struct Handler {
    cm: ConsensusMod<RaftServiceClient<Channel>>,
}

impl Handler {
    pub fn new(cm: ConsensusMod<RaftServiceClient<Channel>>) -> Handler {
        Handler { cm }
    }

    pub async fn append(
        &self,
        req: Request<AppendRequest>,
    ) -> Result<Response<AppendResponse>, Status> {
        let req = req.into_inner();
        self.cm
            .append(req.into())
            .await
            .map(|op| Response::new(AppendResponse::from(op)))
            .map_err(|e| e.into())
    }

    pub async fn vote(&self, req: Request<VoteRequest>) -> Result<Response<VoteResponse>, Status> {
        let req = req.into_inner();
        self.cm
            .vote(req.into())
            .await
            .map(|resp| Response::new(VoteResponse::from(resp)))
            .map_err(|e| e.into())
    }
}

#[tonic::async_trait]
impl RaftService for Handler {
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
