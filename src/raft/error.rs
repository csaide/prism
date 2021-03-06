// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

use std::{array::TryFromSliceError, result};

use sled::{transaction::TransactionError, CompareAndSwapError};
use thiserror::Error;
use tokio::sync::{mpsc, oneshot, watch};
use tonic::Status;

/// Custom Result wrapper to simplify usage.
pub type Result<T> = result::Result<T, Error>;

/// Represents errors interacting with a storage repository.
#[derive(Debug, Error)]
pub enum Error {
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
    #[error("tokio oneshot recv error: {0}")]
    Oneshot(
        #[source]
        #[from]
        oneshot::error::RecvError,
    ),
    #[error("rpc error: {0}")]
    Rpc(
        #[source]
        #[from]
        Status,
    ),
    #[error("state error: invalid state for operation")]
    InvalidMode,
    #[error("state error: failed to find entry for index")]
    Missing,
    #[error("sled error: {0}")]
    Sled(
        #[source]
        #[from]
        sled::Error,
    ),
    #[error("sled transaction error: {0}")]
    Transaction(
        #[source]
        #[from]
        sled::transaction::TransactionError,
    ),
    #[error("internal cast error: {0}")]
    Cast(
        #[source]
        #[from]
        TryFromSliceError,
    ),
    #[error("serialization error: {0}")]
    Serialize(String),
    #[error("failed to swap values: {0}")]
    CompareAndSwap(
        #[source]
        #[from]
        CompareAndSwapError,
    ),
    #[error("transport error: {0}")]
    Transport(
        #[source]
        #[from]
        tonic::transport::Error,
    ),
    #[error("server has transitioned to dead")]
    Dead,
    #[error("unable to achieve quorum")]
    NoQuorum,
}

impl From<Error> for Status {
    fn from(input: Error) -> Self {
        match input {
            Error::Rpc(status) => status,
            _ => Status::internal(input.to_string()),
        }
    }
}

impl From<TransactionError<Error>> for Error {
    fn from(t: TransactionError<Error>) -> Self {
        t.into()
    }
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use tonic::{Code, Status};

    #[test]
    fn test_derived() {
        let err = super::Error::Dead;
        let err_str = format!("{:?}", err);
        assert_eq!(err_str, "Dead");
        let source = err.source();
        assert!(source.is_none());
    }

    #[test]
    fn test_from() {
        let err = super::Error::Dead;
        let status = Status::from(err);
        assert_eq!(status.code(), Code::Internal);
        assert_eq!(status.message(), "server has transitioned to dead");

        let status = Status::new(Code::Aborted, "hello");
        let err = super::Error::Rpc(status);
        let source = Status::from(err);
        assert_eq!(source.code(), Code::Aborted);
        assert_eq!(source.message(), "hello");
    }
}
