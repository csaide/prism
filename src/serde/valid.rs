// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

/// A trait which implements a valid handler for conversion purposes, there is a default
/// implementation which blindly assumes that the type is valid after marshalling.
pub trait Valid {
    /// Returns whether or not this paricular type is Valid. This is here to ensure handling
    /// of marshalling values outside the range of possible enum values as well as other
    /// invalid representations after marshalling has been completed.
    fn valid(&self) -> bool;
}
