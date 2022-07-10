// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

use std::ffi::OsString;

use structopt::{
    clap::{crate_version, ErrorKind},
    StructOpt,
};

use super::logging;

pub fn config<T>(args: Vec<OsString>, bin: &'static str) -> Result<T, (exitcode::ExitCode, String)>
where
    T: StructOpt,
{
    let setup_logger = logging::default(bin, crate_version!());
    T::from_iter_safe(args).map_err(|err| {
        if err.kind == ErrorKind::HelpDisplayed || err.kind == ErrorKind::VersionDisplayed {
            (exitcode::USAGE, err.message)
        } else {
            crit!(setup_logger, "Failed to parse provided configuration."; "error" => err.to_string());
            (exitcode::CONFIG, String::default())
        }
    })
}

#[cfg(test)]
mod tests {
    use rstest::rstest;
    use structopt::clap::{crate_version, AppSettings};
    use structopt::StructOpt;

    use super::*;

    const BIN: &'static str = "bin";

    #[derive(Debug, StructOpt, PartialEq)]
    #[structopt(
        global_settings = &[AppSettings::DeriveDisplayOrder],
        author = "Christian Saide <me@csaide.dev>",
        about = "Run an instance of primsd.",
        version = crate_version!()
    )]
    struct TestConfig {
        #[structopt(long = "field", short = "f")]
        field: bool,
        #[structopt(long = "number", short = "n", default_value = "0")]
        number: usize,
    }

    #[rstest]
    #[case(Vec::default(), Ok(TestConfig{field: false, number: 0}))]
    #[case(vec![OsString::from("test"), OsString::from("-f")], Ok(TestConfig{field: true, number: 0}))]
    #[case(vec![OsString::from("test"), OsString::from("-h")], Err((exitcode::USAGE, String::default())))]
    #[case(vec![OsString::from("test"), OsString::from("-V")], Err((exitcode::USAGE, String::default())))]
    #[case(vec![OsString::from("test"), OsString::from("-n"), OsString::from("nope")], Err((exitcode::CONFIG, String::default())))]
    fn test_config(
        #[case] args: Vec<OsString>,
        #[case] expected: Result<TestConfig, (exitcode::ExitCode, String)>,
    ) {
        let result = config::<TestConfig>(args, BIN);
        if expected.is_ok() {
            assert!(result.is_ok());
            let result = result.unwrap();
            let expected = expected.unwrap();
            assert_eq!(result, expected);
            assert_eq!(format!("{:?}", result), format!("{:?}", expected))
        } else {
            assert!(result.is_err());
            let result = result.unwrap_err();
            let expected = expected.unwrap_err();
            assert_eq!(result.0, expected.0);
        }
    }
}
