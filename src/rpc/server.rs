// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

use std::net::SocketAddr;

use tonic::transport::{Channel, Server};

use crate::raft::ConcensusRepo;

use super::{
    interceptor,
    raft::{self, RaftServiceClient},
};

pub async fn serve<CM>(
    addr: SocketAddr,
    cm: CM,
    logger: slog::Logger,
) -> Result<(), tonic::transport::Error>
where
    CM: ConcensusRepo<RaftServiceClient<Channel>>,
{
    let (mut health_reporter, health_service) = tonic_health::server::health_reporter();
    health_reporter
        .set_service_status("", tonic_health::ServingStatus::Serving)
        .await;
    health_reporter
        .set_service_status("raft", tonic_health::ServingStatus::Serving)
        .await;

    let reflection = tonic_reflection::server::Builder::configure()
        .register_encoded_file_descriptor_set(raft::FILE_DESCRIPTOR_SET)
        .register_encoded_file_descriptor_set(
            tonic_health::proto::GRPC_HEALTH_V1_FILE_DESCRIPTOR_SET,
        )
        .build()
        .unwrap();

    let interceptor = interceptor::RaftInterceptor::new(&logger);
    let raft_impl = raft::Handler::new(cm);

    info!(logger, "Listening for gRPC requests."; "addr" => addr.to_string());
    Server::builder()
        .add_service(raft::RaftServiceServer::with_interceptor(
            raft_impl,
            interceptor,
        ))
        .add_service(reflection)
        .add_service(health_service)
        .serve(addr)
        .await
}
