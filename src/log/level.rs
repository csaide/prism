// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

// Super usings
use super::error::{Error, Result};

// Standard usings
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq, PartialOrd)]
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
    /// # use std::str::FromStr;
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
mod tests {
    use rstest::rstest;

    use super::*;

    #[test]
    fn test_derived() {
        let level = Level::Info;
        let level_str = format!("{:?}", level);
        assert_eq!(level_str, "Info");
        let cloned = level.clone();
        assert_eq!(cloned, level);
    }

    #[rstest]
    #[case::crit("critical", Ok(Level::Crit))]
    #[case::error("error", Ok(Level::Error))]
    #[case::warn("warn", Ok(Level::Warn))]
    #[case::info("info", Ok(Level::Info))]
    #[case::debug("debug", Ok(Level::Debug))]
    #[case::fail("fail", Err(Error::InvalidLevel { level: String::from("fail") }))]
    fn test_from_str(#[case] input: &str, #[case] expected: Result<Level>) {
        let result = Level::from_str(input);
        if expected.is_err() {
            assert!(result.is_err());

            let result = result.unwrap_err();
            let expected_level = String::from(input);
            match result {
                Error::InvalidLevel { level } => assert_eq!(level, expected_level),
            }
        } else {
            assert!(result.is_ok());
            assert_eq!(expected.unwrap(), result.unwrap())
        }
    }

    #[rstest]
    #[case::crit(Level::Crit, slog::Level::Critical)]
    #[case::error(Level::Error, slog::Level::Error)]
    #[case::warn(Level::Warn, slog::Level::Warning)]
    #[case::info(Level::Info, slog::Level::Info)]
    #[case::debug(Level::Debug, slog::Level::Debug)]
    fn test_to_slog(#[case] input: Level, #[case] expected: slog::Level) {
        let actual = input.to_slog();
        assert_eq!(expected, actual)
    }
}
