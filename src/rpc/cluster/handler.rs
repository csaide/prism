// (c) Copyright 2022 Christian Saide[]
// SPDX-License-Identifier: GPL-3.0-or-later

use tonic::{transport::Channel, Request, Response, Status};

use super::{
    proto::cluster_server::Cluster, AddRequest, AddResponse, ListRequest, ListResponse,
    RemoveRequest, RemoveResponse,
};
use crate::raft::ClusterHandler;
use crate::rpc::raft::RaftClient;

pub struct Handler<H> {
    ch: H,
}

impl<H> Handler<H>
where
    H: ClusterHandler<RaftClient<Channel>>,
{
    pub fn new(ch: H) -> Handler<H> {
        Handler { ch }
    }

    async fn add_impl(&self, req: Request<AddRequest>) -> Result<Response<AddResponse>, Status> {
        let req = req.into_inner();
        self.ch
            .add_server(req.into())
            .await
            .map(AddResponse::from)
            .map(Response::new)
            .map_err(|e| e.into())
    }

    async fn remove_impl(
        &self,
        req: Request<RemoveRequest>,
    ) -> Result<Response<RemoveResponse>, Status> {
        let req = req.into_inner();
        self.ch
            .remove_server(req.into())
            .await
            .map(RemoveResponse::from)
            .map(Response::new)
            .map_err(|e| e.into())
    }

    async fn list_impl(&self, req: Request<ListRequest>) -> Result<Response<ListResponse>, Status> {
        let req = req.into_inner();
        self.ch
            .list_servers(req.into())
            .await
            .map(ListResponse::from)
            .map(Response::new)
            .map_err(|e| e.into())
    }
}

#[tonic::async_trait]
impl<CM> Cluster for Handler<CM>
where
    CM: ClusterHandler<RaftClient<Channel>>,
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

#[cfg(test)]
mod tests {
    use tokio_test::block_on as wait;

    use crate::raft::{
        AddServerResponse, ListServerResponse, MockClusterHandler, RemoveServerResponse,
    };

    use super::*;

    #[test]
    fn test_add() {
        let mut mock = MockClusterHandler::default();
        mock.expect_add_server().once().returning(|_| {
            Ok(AddServerResponse {
                leader_hint: String::from("leader"),
                status: String::from("OK"),
            })
        });

        let srv = Handler::new(mock);

        let inner = AddRequest {
            member: String::from("member"),
            replica: false,
        };

        let req = Request::new(inner);
        let result = wait(srv.add(req)).expect("Should not have returned an error.");
        let result = result.into_inner();
        assert_eq!(result.leader_hint, "leader");
        assert_eq!(result.status, "OK");
    }

    #[test]
    fn test_remove() {
        let mut mock = MockClusterHandler::default();
        mock.expect_remove_server().once().returning(|_| {
            Ok(RemoveServerResponse {
                leader_hint: String::from("leader"),
                status: String::from("OK"),
            })
        });

        let srv = Handler::new(mock);

        let inner = RemoveRequest {
            member: String::from("member"),
        };

        let req = Request::new(inner);
        let result = wait(srv.remove(req)).expect("Should not have returned an error.");
        let result = result.into_inner();
        assert_eq!(result.leader_hint, "leader");
        assert_eq!(result.status, "OK");
    }

    #[test]
    fn test_list() {
        let mut mock = MockClusterHandler::default();
        mock.expect_list_servers().once().returning(|_| {
            Ok(ListServerResponse {
                leader: String::from("leader"),
                replicas: Vec::default(),
                voters: Vec::default(),
                term: 1,
            })
        });

        let srv = Handler::new(mock);

        let inner = ListRequest {};

        let req = Request::new(inner);
        let result = wait(srv.list(req)).expect("Should not have returned an error.");
        let result = result.into_inner();
        assert_eq!(result.leader, "leader");
        assert_eq!(result.term, 1u128.to_be_bytes().to_vec());
    }
}
