// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

//! The libprism library encapsulates all logic for prismd and prismctl applications.

#![cfg_attr(coverage_nightly, feature(no_coverage))]

#[macro_use]
extern crate slog;

/// Base level binary init functionality.
pub mod base;
/// An example state machine implementation based on an in memory hashmap.
pub mod hash;
/// General logging utilities/functionality, based ontop of the [slog] ecosystem.
pub mod log;
/// Entrypoint logic for prismctl.
pub mod prismctl;
/// Entrypoint logic for prismd.
pub mod prismd;
/// Raft implementation.
pub mod raft;
/// gRPC implementation details.
pub mod rpc;
