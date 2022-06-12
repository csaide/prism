// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

use std::{mem::size_of, slice};

use sled::IVec;

/// A trait which implements a from raw binary serialization method.
/// This has a blanket implementation for all types T which are sized,
/// however this is an inherently unsafe operation.
///
/// Note NEVER implement this trait manually, instead rely on the [ByteSerializable]
/// marker trait to handle auto-implementing this trait for you.
pub unsafe trait Serialize: Sized {
    /// Handles casting the implementation type to a raw byte slice, this
    /// is inherently unsafe as this is a direct zero copy cast from the
    /// representative type to the raw bytes.
    ///
    /// Note: This particular method is *only* implemented for little endian machines.
    /// Therefore this should only compile on targets with that endian configuration.
    ///
    /// TODO(csaide): Determine how to handle big endian platforms.
    #[inline]
    #[cfg(target_endian = "little")]
    fn to_raw(&self) -> &[u8] {
        unsafe { slice::from_raw_parts((self as *const Self) as *const u8, size_of::<Self>()) }
    }

    fn to_ivec(&self) -> IVec {
        let bytes = self.to_raw();
        IVec::from(bytes)
    }
}
