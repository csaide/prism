// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

// Super usings
use super::error::{Error, Result};

// Standard usings
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq)]
/// Set the verbosity of logs printed to the defined handler.
pub enum Level {
    /// Only print critical errors.
    Crit,
    /// Print critical and standard level errors.
    Error,
    /// Print critical, error, and warning level logs.
    Warn,
    /// Print critical, error, warning, and informational level logs.
    Info,
    /// Print critical, error, warning, informational, and debugging level logs.
    Debug,
}

impl FromStr for Level {
    type Err = Error;

    /// Handles converting the supplied static &str to a Level. In the event
    /// the supplied static &str is not defined, an Error::InvalidLevel is returned.
    ///
    /// ```
    /// use std::str::FromStr;
    /// let x = libprism::log::Level::from_str("critical");
    /// assert_eq!(x.is_ok(), true);
    /// assert_eq!(x.unwrap(), libprism::log::Level::Crit);
    /// ```
    fn from_str(t: &str) -> Result<Level> {
        match t {
            "critical" => Ok(Level::Crit),
            "error" => Ok(Level::Error),
            "warn" => Ok(Level::Warn),
            "info" => Ok(Level::Info),
            "debug" => Ok(Level::Debug),
            _ => Err(Error::InvalidLevel {
                level: t.to_owned(),
            }),
        }
    }
}

impl Level {
    /// Handles converting the internal  log level to the lower level slog representation
    /// of log levels for consumption.
    ///
    /// ```
    /// use std::str::FromStr;
    /// let x = libprism::log::Level::Crit;
    /// assert_eq!(x.to_slog(), slog::Level::Critical);
    /// ```
    pub fn to_slog(&self) -> slog::Level {
        match self {
            Level::Crit => slog::Level::Critical,
            Level::Error => slog::Level::Error,
            Level::Warn => slog::Level::Warning,
            Level::Info => slog::Level::Info,
            Level::Debug => slog::Level::Debug,
        }
    }
}

#[cfg(test)]
#[cfg(not(tarpaulin_include))]
mod tests {
    use super::*;

    #[test]
    fn test_from_str() {
        assert_eq!(Level::Crit, Level::from_str("critical").unwrap());
        assert_eq!(Level::Error, Level::from_str("error").unwrap());
        assert_eq!(Level::Warn, Level::from_str("warn").unwrap());
        assert_eq!(Level::Info, Level::from_str("info").unwrap());
        assert_eq!(Level::Debug, Level::from_str("debug").unwrap());

        let res = Level::from_str("nope");
        assert_eq!(true, res.is_err());
        let err = res.unwrap_err();

        match err {
            Error::InvalidLevel { ref level } => {
                assert_eq!(&String::from("nope"), level);
            }
        }
    }

    #[test]
    fn test_to_slog() {
        let level = Level::Crit;
        assert_eq!(slog::Level::Critical, level.to_slog());
        let level = Level::Error;
        assert_eq!(slog::Level::Error, level.to_slog());
        let level = Level::Warn;
        assert_eq!(slog::Level::Warning, level.to_slog());
        let level = Level::Info;
        assert_eq!(slog::Level::Info, level.to_slog());
        let level = Level::Debug;
        assert_eq!(slog::Level::Debug, level.to_slog());
    }
}
