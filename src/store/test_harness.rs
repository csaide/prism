// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

use rkyv::{Archive, Deserialize, Serialize};
use sled::Db;

use super::{Error, Result};

pub fn new_test_db() -> Result<Db> {
    let cfg = sled::Config::default().temporary(true);
    cfg.open().map_err(Error::from)
}

#[derive(Archive, Serialize, Deserialize, Debug, PartialEq, PartialOrd)]
#[archive(compare(PartialEq, PartialOrd))]
#[archive_attr(derive(Debug), repr(C))]
pub struct TestStruct {
    pub a: u64,
    pub b: u64,
}

impl TestStruct {
    pub fn new(a: u64, b: u64) -> TestStruct {
        TestStruct { a, b }
    }
}
