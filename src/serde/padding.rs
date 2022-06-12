// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

use super::Valid;

/// The constant padding at the end of a serializable struct to handle alignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct Padding<const N: usize>([u8; N]);

impl<const N: usize> Padding<N> {
    /// Generate a new zeroed padding struct.
    pub const fn zero() -> Self {
        Self([0x00; N])
    }
}

impl<const N: usize> Default for Padding<N> {
    fn default() -> Self {
        Self::zero()
    }
}

impl<const N: usize> Valid for Padding<N> {
    fn valid(&self) -> bool {
        self.0.iter().all(|&byte| byte == 0x00)
    }
}
