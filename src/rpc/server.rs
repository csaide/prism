// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

use std::future::Future;
use std::net::SocketAddr;

use tonic::transport::{Channel, Server};

use crate::{hash::HashState, raft::Repo};

use super::{
    cluster, frontend, interceptor,
    raft::{self, RaftClient},
};

pub async fn serve(
    addr: SocketAddr,
    cm: Repo<RaftClient<Channel>, HashState>,
    logger: slog::Logger,
    shutdown: impl Future<Output = ()>,
) -> Result<(), tonic::transport::Error> {
    let (mut health_reporter, health_service) = tonic_health::server::health_reporter();
    health_reporter
        .set_service_status("", tonic_health::ServingStatus::Serving)
        .await;
    health_reporter
        .set_service_status("raft", tonic_health::ServingStatus::Serving)
        .await;
    health_reporter
        .set_service_status("cluster", tonic_health::ServingStatus::Serving)
        .await;
    health_reporter
        .set_service_status("frontend", tonic_health::ServingStatus::Serving)
        .await;

    let reflection = tonic_reflection::server::Builder::configure()
        .register_encoded_file_descriptor_set(raft::FILE_DESCRIPTOR_SET)
        .register_encoded_file_descriptor_set(cluster::FILE_DESCRIPTOR_SET)
        .register_encoded_file_descriptor_set(frontend::FILE_DESCRIPTOR_SET)
        .register_encoded_file_descriptor_set(
            tonic_health::proto::GRPC_HEALTH_V1_FILE_DESCRIPTOR_SET,
        )
        .build()
        .unwrap();

    let interceptor = interceptor::RaftInterceptor::new(&logger);
    let raft_impl = raft::Handler::new(cm.get_raft());
    let cluster_impl = cluster::Handler::new(cm.get_cluster());
    let frontend_impl = frontend::Handler::new(cm.get_frontend());

    info!(logger, "Listening for gRPC requests."; "addr" => addr.to_string());
    Server::builder()
        .add_service(raft::RaftServer::with_interceptor(
            raft_impl,
            interceptor.clone(),
        ))
        .add_service(cluster::ClusterServer::with_interceptor(
            cluster_impl,
            interceptor.clone(),
        ))
        .add_service(frontend::FrontendServer::with_interceptor(
            frontend_impl,
            interceptor.clone(),
        ))
        .add_service(reflection)
        .add_service(health_service)
        .serve_with_shutdown(addr, shutdown)
        .await
}
