// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

// stdlib usings
use std::result;

// extern usings
use thiserror::Error;

/// Custom Result wrapper to simplify usage.
pub type Result<T> = result::Result<T, Error>;

/// Represents logging errors based on user configuration or OS
/// errors while attempting to configure log handlers.
#[derive(Error, Debug, Clone, PartialEq, PartialOrd)]
pub enum Error {
    /// Handles errors for undefined or invalid log level conversions.
    #[error("invalid level specified: {level}")]
    InvalidLevel {
        /// level represents the level that was configued but unimplemented.
        level: String,
    },
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    #[test]
    fn test_error() {
        let err = super::Error::InvalidLevel {
            level: String::from("level"),
        };
        let cloned = err.clone();
        assert_eq!(cloned, err);
        assert!(cloned >= err);
        assert!(cloned <= err);
        assert!(cloned == err);
        assert_eq!(format!("{:?}", cloned), format!("{:?}", err));
        assert!(err.source().is_none());
    }
}
