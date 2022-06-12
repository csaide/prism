// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

use std::array::TryFromSliceError;
use std::result;
use std::string::FromUtf8Error;

use sled::CompareAndSwapError;
use thiserror::Error;

/// Custom Result wrapper to simplify usage.
pub type Result<T> = result::Result<T, Error>;

/// Represents errors interacting with a storage repository.
#[derive(Debug, Error)]
pub enum Error {
    #[error("internal sled database error: {0}")]
    Sled(
        #[source]
        #[from]
        sled::Error,
    ),
    #[error("internal cast error: {0}")]
    Cast(
        #[source]
        #[from]
        TryFromSliceError,
    ),
    #[error("failed to handle string data: {0}")]
    UTF8(
        #[source]
        #[from]
        FromUtf8Error,
    ),
    #[error("failed to swap values")]
    CompareAndSwap(
        #[source]
        #[from]
        CompareAndSwapError,
    ),
}
