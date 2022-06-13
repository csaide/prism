// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

use std::result;

use thiserror::Error;

/// A [std::result::Result] that is wrapped using the custom [Error] implemented here as
/// the error case.
pub type Result<T> = result::Result<T, Error>;

/// A rift serialize error which represents the various errors that can occur when marshalling
/// [u8] arrays to/from structs.
#[derive(Error, Clone, Debug, PartialEq, PartialOrd)]
pub enum Error {
    /// This occurs during marshalling of a given struct/byte array combination, specifically
    /// when size of the combination is mismatched.
    #[error("failed to de/serialize struct from/to bytes due to size mismatch")]
    SizeMismatch {
        /// The expected size of the byte array, based on the size of the struct.
        expected: usize,
        /// The actual size of the byte array used for the conversion.
        actual: usize,
    },
    /// This occurs during marshalling of a given struct/byte array combination, specifcally
    /// when the alignment of the byte array differs from the alignment of the internal struct
    /// data representation.
    #[error("failed to de/serialize struct from/to bytes due to alignment error")]
    Misaligned {
        /// The difference in alignment observed.
        diff: usize,
        /// The expected alignment in bytes for this struct/byte array combination.
        expected: usize,
    },
    /// This occurs when the supplied byte array has an invalid byte sequence for the struct that it
    /// is supposed to represent.
    #[error("failed to handle parsing the struct from bytes due to invalid byte sequence")]
    InvalidByteSequence,
}
