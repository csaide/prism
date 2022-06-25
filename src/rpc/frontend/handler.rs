// (c) Copyright 2022 Christian Saide[]
// SPDX-License-Identifier: GPL-3.0-or-later

use tonic::{Request, Response, Status};

use crate::raft::FrontendHandler;

use super::{
    proto::frontend_server::Frontend, MutateRequest, MutateResponse, ReadRequest, ReadResponse,
    RegisterRequest, RegisterResponse,
};

pub struct Handler<H> {
    cm: H,
}

impl<H> Handler<H>
where
    H: FrontendHandler,
{
    pub fn new(cm: H) -> Handler<H> {
        Handler { cm }
    }

    async fn mutate_impl(
        &self,
        req: Request<MutateRequest>,
    ) -> Result<Response<MutateResponse>, Status> {
        let req = req.into_inner();
        let req = req.into_raft()?;
        let resp = self.cm.mutate_request(req).await?;
        MutateResponse::from_raft(resp)
            .map(Response::new)
            .map_err(|e| e.into())
    }

    async fn read_impl(&self, req: Request<ReadRequest>) -> Result<Response<ReadResponse>, Status> {
        let req = req.into_inner();
        let req = req.into_raft()?;
        let resp = self.cm.read_request(req).await?;
        ReadResponse::from_raft(resp)
            .map(Response::new)
            .map_err(|e| e.into())
    }

    async fn register_impl(
        &self,
        req: Request<RegisterRequest>,
    ) -> Result<Response<RegisterResponse>, Status> {
        let req = req.into_inner();
        let req = req.into_raft()?;
        let resp = self.cm.register_client(req).await?;
        RegisterResponse::from_raft(resp)
            .map(Response::new)
            .map_err(|e| e.into())
    }
}

#[tonic::async_trait]
impl<H> Frontend for Handler<H>
where
    H: FrontendHandler,
{
    async fn mutate(
        &self,
        request: Request<MutateRequest>,
    ) -> Result<Response<MutateResponse>, Status> {
        self.mutate_impl(request).await
    }
    async fn read(&self, request: Request<ReadRequest>) -> Result<Response<ReadResponse>, Status> {
        self.read_impl(request).await
    }
    async fn register(
        &self,
        request: Request<RegisterRequest>,
    ) -> Result<Response<RegisterResponse>, Status> {
        self.register_impl(request).await
    }
}
