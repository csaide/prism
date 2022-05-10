// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

use super::{Error, Result};

pub trait Key: Sized {
    fn to_bytes(&self) -> Vec<u8>;
    fn from_bytes(buf: &[u8]) -> Result<Self>;
}

impl Key for u64 {
    fn to_bytes(&self) -> Vec<u8> {
        self.to_be_bytes().to_vec()
    }
    fn from_bytes(buf: &[u8]) -> Result<Self> {
        buf.try_into().map_err(Error::from).map(u64::from_be_bytes)
    }
}

impl Key for String {
    fn to_bytes(&self) -> Vec<u8> {
        self.as_bytes().to_vec()
    }
    fn from_bytes(buf: &[u8]) -> Result<Self> {
        String::from_utf8(buf.to_vec()).map_err(Error::from)
    }
}

#[cfg(test)]
#[cfg(not(tarpaulin_include))]
mod tests {
    use super::*;

    #[test]
    fn test_u64() {
        let expected: u64 = 1000;
        let bytes = expected.to_bytes();
        assert_eq!(bytes.len(), 8);
        let actual = u64::from_bytes(&bytes).expect("failed to decode u64 key");
        assert_eq!(expected, actual)
    }

    #[test]
    fn test_string() {
        let expected = String::from("hello world!!!! This is a test yo!");
        let bytes = expected.to_bytes();
        assert_eq!(bytes.len(), 34);
        let actual = String::from_bytes(&bytes).expect("failed to decode string key");
        assert_eq!(expected, actual)
    }
}
