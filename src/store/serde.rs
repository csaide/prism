// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

use sled::IVec;

use super::{Error, Result};

pub trait Serializeable: Sized {
    fn to_raw(&self) -> IVec;
    fn from_raw(data: &[u8]) -> Result<Self>;
}

impl Serializeable for u64 {
    fn to_raw(&self) -> IVec {
        IVec::from(&self.to_be_bytes())
    }

    fn from_raw(data: &[u8]) -> Result<u64> {
        data.try_into().map_err(Error::from).map(u64::from_be_bytes)
    }
}

impl Serializeable for i64 {
    fn to_raw(&self) -> IVec {
        IVec::from(&self.to_be_bytes())
    }
    fn from_raw(data: &[u8]) -> Result<Self> {
        data.try_into().map_err(Error::from).map(i64::from_be_bytes)
    }
}

impl Serializeable for u128 {
    fn to_raw(&self) -> IVec {
        IVec::from(&self.to_be_bytes())
    }

    fn from_raw(data: &[u8]) -> Result<Self> {
        data.try_into()
            .map_err(Error::from)
            .map(u128::from_be_bytes)
    }
}

impl Serializeable for String {
    fn to_raw(&self) -> IVec {
        IVec::from(self.as_bytes())
    }

    fn from_raw(data: &[u8]) -> Result<Self> {
        String::from_utf8(data.to_vec()).map_err(Error::from)
    }
}
