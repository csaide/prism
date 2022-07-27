// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

use std::result;

use thiserror::Error;

pub type Result<T> = result::Result<T, Error>;

#[derive(Debug, Error, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Error {
    #[error("mmap operation failed: {0}")]
    Mmap(String),
    #[error("file operation failed: {0}")]
    Io(String),
}

impl Error {
    pub fn mmap(e: std::io::Error) -> Error {
        Error::Mmap(e.to_string())
    }

    pub fn file(e: std::io::Error) -> Error {
        Error::Io(e.to_string())
    }
}
