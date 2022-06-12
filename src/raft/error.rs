// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

use std::result;

use thiserror::Error;
use tokio::sync::watch;
use tonic::Status;

use crate::store;

/// Custom Result wrapper to simplify usage.
pub type Result<T> = result::Result<T, Error>;

/// Represents errors interacting with a storage repository.
#[derive(Debug, Error)]
pub enum Error {
    #[error("store error: {0}")]
    Store(
        #[source]
        #[from]
        store::Error,
    ),
    #[error("tokio watch send error: {0}")]
    WatchSend(
        #[source]
        #[from]
        watch::error::SendError<()>,
    ),
    #[error("rpc error: {0}")]
    Rpc(
        #[source]
        #[from]
        Status,
    ),
    #[error("state error: invalid state for operation")]
    InvalidState,
}

impl Into<Status> for Error {
    fn into(self) -> Status {
        match self {
            Error::Rpc(status) => status,
            _ => Status::internal(self.to_string()),
        }
    }
}
