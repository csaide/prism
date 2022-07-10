// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

// crate usings
use super::Level;

// extern usings
use structopt::StructOpt;

#[derive(Debug, Clone, StructOpt, PartialEq, PartialOrd)]
/// Prism logging configuration.
pub struct Config {
    #[structopt(
        long = "log-level",
        short = "l",
        env = "PRISM_LOG_LEVEL",
        help = "The logging level to use.",
        long_help = "Selects the maximum logging level to log for all application logs.",
        default_value = "info",
        possible_values = &["critical", "error", "warn", "info", "debug"],
        takes_value = true
    )]
    /// Define the logging level to use.
    pub level: Level,

    #[structopt(
        long = "log-json",
        short = "j",
        env = "PRISM_LOG_JSON",
        help = "Whether or not to log in JSON format.",
        long_help = "Sets whether or not to log in JSON format, when the `stdout` log handler is in use.",
        takes_value = false
    )]
    /// Define whether or not to log in json format.
    pub json: bool,
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;

    use super::*;

    #[test]
    fn test_config() {
        let cfg = Config::from_iter_safe(vec![OsString::from("test"), OsString::from("-j")])
            .expect("Shouldn't have errored.");
        let cloned = cfg.clone();
        assert_eq!(cloned, cfg);
        assert!(cloned >= cfg);
        assert!(cloned <= cfg);
        assert!(cloned == cfg);
        assert_eq!(format!("{:?}", cloned), format!("{:?}", cfg))
    }
}
