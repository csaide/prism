// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

//! The libprism library encapsulates all logic for prismd and prismctl applications.

#[macro_use]
extern crate slog;

/// An example state machine implementation based on a in memory hashmap.
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

use structopt::{
    clap::{crate_version, ErrorKind},
    StructOpt,
};

pub fn base_config<T>(bin: &'static str) -> Result<T, exitcode::ExitCode>
where
    T: StructOpt,
{
    let setup_logger = log::default(bin, crate_version!());
    T::from_args_safe().map_err(|err| {
        if err.kind == ErrorKind::HelpDisplayed || err.kind == ErrorKind::VersionDisplayed        {
            println!("{}", err.message);
            exitcode::USAGE
        } else {
            crit!(setup_logger, "Failed to parse provided configuration."; "error" => err.to_string());
            exitcode::CONFIG
        }
    })
}
