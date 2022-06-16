// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

// stdlib usings
use std::io;

// extern usings
use slog::Drain;

mod config;
mod error;
mod filter;
mod level;

pub use self::config::Config;
pub use self::error::{Error, Result};
pub use self::level::Level;

#[cfg(test)]
pub fn noop() -> slog::Logger {
    let drain = slog::Discard;
    slog::Logger::root(drain, o!())
}

/// Return a defualt logger to use for init processing before configuraiton can be
/// parsed. This default logger should only be used temporarily and then thrown away
/// in favor of a user configured logger.
///
/// # Example
/// ```
/// use slog::crit;
///
/// let logger = libprism::log::default("example", "0.1.1");
/// crit!(logger, "default logger only logs crit level logs!"; "hello" => "world!");
/// ```
pub fn default(bin: &'static str, version: &'static str) -> slog::Logger {
    let decorator = slog_term::TermDecorator::new().build();
    let drain = slog_term::FullFormat::new(decorator)
        .use_utc_timestamp()
        .build()
        .fuse();

    let drain = filter::LevelFilter {
        drain,
        level: slog::Level::Critical,
    }
    .fuse();

    let drain = slog_async::Async::new(drain).build().fuse();
    slog::Logger::root(drain, o!("binary" => bin, "version" => version))
}

/// Return a newly constructed slog::Logger based on the supplied configuration.
/// This also injects the application name and version as base key/value pairs for the
/// returned root logger.
///
/// # Example
/// ```
/// use slog::info;
///
/// let logger = libprism::log::new(
///     &libprism::log::Config {
///         level: libprism::log::Level::Info,
///         json: true,
///     },
///     "example",
///     "0.1.1",
/// );
///
/// info!(logger, "Hello world!"; "woot" => "woot");
/// ```
pub fn new(cfg: &config::Config, bin: &'static str, version: &'static str) -> slog::Logger {
    let drain: Box<dyn Drain<Ok = (), Err = slog::Never> + Send> = if cfg.json {
        Box::new(
            slog_json::Json::new(io::stdout())
                .add_default_keys()
                .build()
                .fuse(),
        )
    } else {
        let decorator = slog_term::TermDecorator::new().build();
        Box::new(
            slog_term::FullFormat::new(decorator)
                .use_utc_timestamp()
                .build()
                .fuse(),
        )
    };

    let drain = filter::LevelFilter {
        drain,
        level: cfg.level.to_slog(),
    }
    .fuse();

    let drain = slog_async::Async::new(drain).build().fuse();
    slog::Logger::root(drain, o!("binary" => bin, "version" => version))
}

#[cfg(test)]
#[cfg(not(tarpaulin_include))]
mod tests {
    use super::*;

    #[test]
    fn test_default() {
        default("test", "alpha");
    }

    #[test]
    fn test_new() {
        let cfg = Config {
            json: true,
            level: Level::Debug,
        };
        new(&cfg, "test", "alpha");
        let cfg = Config {
            json: false,
            level: Level::Debug,
        };
        new(&cfg, "test", "alpha");
    }
}
