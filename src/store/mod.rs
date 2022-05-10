// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

mod backend;
mod error;
mod helpers;
mod iter;
mod key;

#[cfg(test)]
#[cfg(not(tarpaulin_include))]
mod test_harness;

pub use backend::Store;
pub use error::{Error, Result};
pub use iter::Iter;
pub use key::Key;
