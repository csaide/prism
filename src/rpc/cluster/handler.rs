// (c) Copyright 2022 Christian Saide[]
// SPDX-License-Identifier: GPL-3.0-or-later

use tonic::{transport::Channel, Request, Response, Status};

use super::{
    proto::cluster_server::Cluster, AddRequest, AddResponse, ListRequest, ListResponse,
    RemoveRequest, RemoveResponse,
};
use crate::raft::ConcensusRepo;
use crate::rpc::raft::RaftClient;

pub struct Handler<CM> {
    cm: CM,
}

impl<CM> Handler<CM>
where
    CM: ConcensusRepo<RaftClient<Channel>>,
{
    pub fn new(cm: CM) -> Handler<CM> {
        Handler { cm }
    }

    async fn add_impl(&self, req: Request<AddRequest>) -> Result<Response<AddResponse>, Status> {
        let req = req.into_inner();
        let req = req.into_raft().await?;
        let resp = self.cm.add_server(req).await?;
        AddResponse::from_raft(resp)
            .map(Response::new)
            .map_err(|e| e.into())
    }

    async fn remove_impl(
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

    async fn list_impl(&self, req: Request<ListRequest>) -> Result<Response<ListResponse>, Status> {
        let req = req.into_inner();
        let req = req.into_raft()?;
        let resp = self.cm.list_servers(req).await?;
        ListResponse::from_raft(resp)
            .map(Response::new)
            .map_err(|e| e.into())
    }
}

#[tonic::async_trait]
impl<CM> Cluster for Handler<CM>
where
    CM: ConcensusRepo<RaftClient<Channel>>,
{
    async fn add(&self, request: Request<AddRequest>) -> Result<Response<AddResponse>, Status> {
        self.add_impl(request).await
    }
    async fn remove(
        &self,
        request: Request<RemoveRequest>,
    ) -> Result<Response<RemoveResponse>, Status> {
        self.remove_impl(request).await
    }

    async fn list(&self, request: Request<ListRequest>) -> Result<Response<ListResponse>, Status> {
        self.list_impl(request).await
    }
}
