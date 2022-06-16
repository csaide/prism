// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

mod candidate;
mod commiter;
mod consensus;
mod error;
mod follower;
mod leader;
mod log;
mod metadata;
mod models;
mod peer;
mod state_machine;
#[cfg(test)]
#[cfg(not(tarpaulin_include))]
mod test_harness;

pub use candidate::Candidate;
pub use commiter::Commiter;
pub use consensus::{ConcensusRepo, ConsensusMod};
pub use error::{Error, Result};
pub use follower::Follower;
pub use leader::Leader;
pub use log::Log;
pub use metadata::{Metadata, State};
pub use models::*;
pub use peer::{Client, Peer};
pub use state_machine::StateMachine;
