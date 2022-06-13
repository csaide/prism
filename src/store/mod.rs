// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

mod backend;
mod batch;
mod error;
mod event;
mod iter;
mod serde;
mod subscriber;

#[cfg(test)]
#[cfg(not(tarpaulin_include))]
mod test_harness;

pub use backend::Store;
pub use batch::Batch;
pub use error::{Error, Result};
pub use event::Event;
pub use iter::Iter;
pub use serde::Serializeable;
pub use subscriber::Subscriber;
