// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

use sled::{Db, IVec};

use super::{serde::Serializeable, Error, Result};

pub fn new_test_db() -> Result<Db> {
    let cfg = sled::Config::default().temporary(true);
    cfg.open().map_err(Error::from)
}

#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub struct TestStruct {
    pub a: u64,
    pub b: u64,
}

impl TestStruct {
    pub fn new(a: u64, b: u64) -> TestStruct {
        TestStruct { a, b }
    }
}

impl Serializeable for TestStruct {
    fn to_raw(&self) -> IVec {
        [self.a.to_raw(), self.b.to_raw()].concat().into()
    }

    fn from_raw(data: &[u8]) -> Result<Self> {
        let a = u64::from_raw(&data[..8])?;
        let b = u64::from_raw(&data[8..])?;
        Ok(TestStruct { a, b })
    }
}
