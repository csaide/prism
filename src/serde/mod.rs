// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

mod deserialize;
mod error;
mod padding;
mod serialize;
mod valid;

pub use self::deserialize::Deserialize;
pub use self::error::{Error, Result};
pub use self::padding::Padding;
pub use self::serialize::Serialize;
pub use self::valid::Valid;

/// This is a marker trait for auto-implementing the [Deserialize] and
/// [Serialize] traits for the representative type.
///
/// # Safety
/// Ensure you implemenet [Valid] properly for this type.
pub unsafe trait Serializable: Sized + Valid {}

unsafe impl<T> Deserialize for T where T: Serializable {}
unsafe impl<T> Serialize for T where T: Serializable {}
