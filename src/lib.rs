// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

//! The libprism library encapsulates all logic for prismd and prismctl applications.

// macro usings
#[macro_use]
extern crate slog;

/// General logging utilities/functionality, based ontop of the [slog] ecosystem.
pub mod log;
/// Entrypoint logic for prismctl.
pub mod prismctl;
/// Entrypoint logic for prismd.
pub mod prismd;
