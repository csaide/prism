// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

mod handler;
mod proto;

pub(crate) const FILE_DESCRIPTOR_SET: &[u8] =
    tonic::include_file_descriptor_set!("cluster_descriptor");

pub use handler::Handler;
pub use proto::{cluster_client::ClusterClient, cluster_server::ClusterServer};
pub use proto::{
    AddRequest, AddResponse, ListRequest, ListResponse, RemoveRequest, RemoveResponse,
};
