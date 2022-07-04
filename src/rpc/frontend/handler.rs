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
        self.cm
            .mutate_request(req.into())
            .await
            .map(MutateResponse::from)
            .map(Response::new)
            .map_err(|e| e.into())
    }

    async fn read_impl(&self, req: Request<ReadRequest>) -> Result<Response<ReadResponse>, Status> {
        let req = req.into_inner();
        self.cm
            .read_request(req.into())
            .await
            .map(ReadResponse::from)
            .map(Response::new)
            .map_err(|e| e.into())
    }

    async fn register_impl(
        &self,
        req: Request<RegisterRequest>,
    ) -> Result<Response<RegisterResponse>, Status> {
        let req = req.into_inner();
        self.cm
            .register_client(req.into())
            .await
            .map(RegisterResponse::from)
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

#[cfg(test)]
mod tests {
    use mockall::predicate::eq;
    use tokio_test::block_on as wait;
    use tonic::transport::Channel;

    use crate::raft::{
        MockFrontendHandler, MutateStateRequest, MutateStateResponse, ReadStateRequest,
        ReadStateResponse, RegisterClientRequest, RegisterClientResponse,
    };
    use crate::rpc::raft::RaftClient;

    use super::*;

    #[test]
    fn test_mutate() {
        let mut mock = MockFrontendHandler::<RaftClient<Channel>>::default();

        let inner = MutateRequest {
            client_id: 1u128.to_be_bytes().to_vec(),
            command: vec![0x00, 0x01],
            sequence_num: 1,
        };

        mock.expect_mutate_request()
            .with(eq(MutateStateRequest::from(inner.clone())))
            .once()
            .returning(|_| {
                Ok(MutateStateResponse {
                    leader_hint: String::from("leader"),
                    response: vec![0x00, 0x01],
                    status: String::from("OK"),
                })
            });

        let srv = Handler::new(mock);

        let req = Request::new(inner);
        let result = wait(srv.mutate(req)).expect("Should not have returned an error.");
        let result = result.into_inner();
        assert_eq!(result.leader_hint, "leader");
        assert_eq!(result.response, vec![0x00, 0x01]);
        assert_eq!(result.status, "OK");
    }

    #[test]
    fn test_read() {
        let mut mock = MockFrontendHandler::<RaftClient<Channel>>::default();

        let inner = ReadRequest {
            query: vec![0x00, 0x01],
        };

        mock.expect_read_request()
            .with(eq(ReadStateRequest::from(inner.clone())))
            .once()
            .returning(|_| {
                Ok(ReadStateResponse {
                    leader_hint: String::from("leader"),
                    response: vec![0x00, 0x01],
                    status: String::from("OK"),
                })
            });

        let srv = Handler::new(mock);

        let req = Request::new(inner);
        let result = wait(srv.read(req)).expect("Should not have returned an error.");
        let result = result.into_inner();
        assert_eq!(result.leader_hint, "leader");
        assert_eq!(result.response, vec![0x00, 0x01]);
        assert_eq!(result.status, "OK");
    }

    #[test]
    fn test_register() {
        let mut mock = MockFrontendHandler::<RaftClient<Channel>>::default();

        let inner = RegisterRequest {};

        mock.expect_register_client()
            .with(eq(RegisterClientRequest::from(inner.clone())))
            .once()
            .returning(|_| {
                Ok(RegisterClientResponse {
                    leader_hint: String::from("leader"),
                    client_id: 1u128.to_be_bytes().to_vec(),
                    status: String::from("OK"),
                })
            });

        let srv = Handler::new(mock);

        let req = Request::new(inner);
        let result = wait(srv.register(req)).expect("Should not have returned an error.");
        let result = result.into_inner();
        assert_eq!(result.leader_hint, "leader");
        assert_eq!(result.client_id, 1u128.to_be_bytes().to_vec());
        assert_eq!(result.status, "OK");
    }
}
