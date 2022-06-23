// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

mod handler;
mod proto;

pub(crate) const FILE_DESCRIPTOR_SET: &[u8] =
    tonic::include_file_descriptor_set!("frontend_descriptor");

pub use handler::Handler;
pub use proto::{frontend_client::FrontendClient, frontend_server::FrontendServer};
pub use proto::{
    MutateRequest, MutateResponse, ReadRequest, ReadResponse, RegisterRequest, RegisterResponse,
};
