// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

use std::result;

use thiserror::Error;
use tokio::sync::{mpsc, watch};
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
    #[error("tokio mpsc send error: {0}")]
    MpscSend(
        #[source]
        #[from]
        mpsc::error::SendError<()>,
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
    #[error("state error: failed to find entry for index")]
    Missing,
    #[error("sled error: {0}")]
    Sled(
        #[source]
        #[from]
        sled::Error,
    ),
}

impl From<Error> for Status {
    fn from(input: Error) -> Self {
        match input {
            Error::Rpc(status) => status,
            _ => Status::internal(input.to_string()),
        }
    }
}
