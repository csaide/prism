// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

use std::mem::{align_of, size_of};

use super::{Error, Result, Valid};

/// A trait which implements a raw binary deserialization method.
/// This has a blanket implementation for all types T which are sized,
/// however this is an inherently unsafe operation.
///
/// # Safety
/// Note NEVER implement this trait manually, instead rely on the [crate::bytes::Serializable]
/// marker trait to handle auto-implementing this trait for you.
pub unsafe trait Deserialize: Valid + Sized {
    /// Handles casting raw binary data to the implementation type, this
    /// is inherently unsafe as this is a direct 0 copy cast from the binary
    /// data to the representative type. This does NOT handle endinaness nor
    /// does it handle panding in any way shape or form. Therefore the following
    /// restrictions are in place:
    /// - Size of the supplied byte slice must be greater than or equal to the size of
    ///   the representative type.
    /// - The alignment of the slice and struct must be the same.
    /// - The representative type must be inhabited.
    ///
    /// Note: This particular method is *only* implemented for little endian machines.
    /// Therefore this should only compile on targets with that endian configuration.
    ///
    /// TODO(csaide): Determine how to handle multiple endian configurations.
    #[inline]
    #[cfg(target_endian = "little")]
    fn from_raw(raw: &[u8]) -> Result<(&Self, Option<usize>)> {
        let size = size_of::<Self>();
        if raw.len() < size {
            return Err(Error::SizeMismatch {
                actual: raw.len(),
                expected: size,
            });
        }
        let bytes = &raw[..size];

        let expected = align_of::<Self>();
        let diff = (bytes.as_ptr() as usize) % expected;
        if diff != 0 {
            return Err(Error::Misaligned { diff, expected });
        }

        let out = unsafe { &*(bytes.as_ptr() as *const Self) };
        if !out.valid() {
            Err(Error::InvalidByteSequence)
        } else if raw.len() > size {
            Ok((out, Some(size)))
        } else {
            Ok((out, None))
        }
    }

    fn from_raw_owned(raw: &[u8]) -> Result<(Self, Option<usize>)> {
        let size = size_of::<Self>();
        if raw.len() < size {
            return Err(Error::SizeMismatch {
                actual: raw.len(),
                expected: size,
            });
        }
        let bytes = &raw[..size];

        let expected = align_of::<Self>();
        let diff = (bytes.as_ptr() as usize) % expected;
        if diff != 0 {
            return Err(Error::Misaligned { diff, expected });
        }

        let out = unsafe { std::ptr::read(raw.as_ptr() as *const Self) };
        if !out.valid() {
            Err(Error::InvalidByteSequence)
        } else if raw.len() > size {
            Ok((out, Some(size)))
        } else {
            Ok((out, None))
        }
    }
}
