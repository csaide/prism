// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

use std::array::TryFromSliceError;
use std::result;
use std::string::FromUtf8Error;

use rkyv::de::deserializers::SharedDeserializeMapError;
use rkyv::ser::serializers::{
    AllocScratchError, CompositeSerializerError, SharedSerializeMapError,
};
use thiserror::Error;

/// Custom Result wrapper to simplify usage.
pub type Result<T> = result::Result<T, Error>;

type SerializerError =
    CompositeSerializerError<std::convert::Infallible, AllocScratchError, SharedSerializeMapError>;

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
    #[error("failed to serialize object: {0}")]
    Serialize(
        #[source]
        #[from]
        SerializerError,
    ),
    #[error("failed to deserialize object: {0}")]
    Deserialize(
        #[source]
        #[from]
        SharedDeserializeMapError,
    ),
    #[error("failed to handle string data: {0}")]
    UTF8(
        #[source]
        #[from]
        FromUtf8Error,
    ),
}
