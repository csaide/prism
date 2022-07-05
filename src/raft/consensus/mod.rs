// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

use super::*;

const OK: &str = "OK";
const NOT_LEADER: &str = "NOT_LEADER";

mod cluster;
mod frontend;
mod module;
mod quorum;
mod raft;
mod repo;

pub use cluster::{Cluster, ClusterHandler};
pub use frontend::{Frontend, FrontendHandler};
pub use module::Module;
pub use quorum::Quorum;
pub use raft::{Raft, RaftHandler};
pub use repo::Repo;

#[cfg(test)]
pub use cluster::mock::MockClusterHandler;
#[cfg(test)]
pub use frontend::mock::MockFrontendHandler;
#[cfg(test)]
pub use raft::mock::MockRaftHandler;
