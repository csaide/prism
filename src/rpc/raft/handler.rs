// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

use tonic::{transport::Channel, Request, Response, Status};

use super::{
    proto::raft_service_server::RaftService, AddRequest, AddResponse, AppendRequest,
    AppendResponse, RaftServiceClient, RemoveRequest, RemoveResponse, VoteRequest, VoteResponse,
};
use crate::raft::ConcensusRepo;

pub struct Handler<CM> {
    cm: CM,
}

impl<CM> Handler<CM>
where
    CM: ConcensusRepo<RaftServiceClient<Channel>>,
{
    pub fn new(cm: CM) -> Handler<CM> {
        Handler { cm }
    }

    pub async fn append(
        &self,
        req: Request<AppendRequest>,
    ) -> Result<Response<AppendResponse>, Status> {
        let req = req.into_inner();
        let req = req.into_raft()?;
        let resp = self.cm.append_entries(req).await?;
        AppendResponse::from_raft(resp)
            .map(Response::new)
            .map_err(|e| e.into())
    }

    pub async fn vote(&self, req: Request<VoteRequest>) -> Result<Response<VoteResponse>, Status> {
        let req = req.into_inner();
        let req = req.into_raft()?;
        let resp = self.cm.vote_request(req).await?;
        VoteResponse::from_raft(resp)
            .map(Response::new)
            .map_err(|e| e.into())
    }

    pub async fn add(&self, req: Request<AddRequest>) -> Result<Response<AddResponse>, Status> {
        let req = req.into_inner();
        let req = req.into_raft().await?;
        let resp = self.cm.add_server(req).await?;
        AddResponse::from_raft(resp)
            .map(Response::new)
            .map_err(|e| e.into())
    }

    pub async fn remove(
        &self,
        req: Request<RemoveRequest>,
    ) -> Result<Response<RemoveResponse>, Status> {
        let req = req.into_inner();
        let req = req.into_raft()?;
        let resp = self.cm.remove_server(req).await?;
        RemoveResponse::from_raft(resp)
            .map(Response::new)
            .map_err(|e| e.into())
    }
}

#[tonic::async_trait]
impl<CM> RaftService for Handler<CM>
where
    CM: ConcensusRepo<RaftServiceClient<Channel>>,
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
    async fn add_server(
        &self,
        request: Request<AddRequest>,
    ) -> Result<Response<AddResponse>, Status> {
        self.add(request).await
    }
    async fn remove_server(
        &self,
        request: Request<RemoveRequest>,
    ) -> Result<Response<RemoveResponse>, Status> {
        self.remove(request).await
    }
}
