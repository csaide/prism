// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

use sled::IVec;

use crate::store::Serializeable;

#[derive(Debug, Clone)]
pub struct Entry<T> {
    pub term: i64,
    pub command: T,
}

impl<T> Entry<T>
where
    T: Serializeable,
{
    pub fn new(term: i64, command: T) -> Entry<T> {
        Entry { term, command }
    }
}

impl<T> Serializeable for Entry<T>
where
    T: Serializeable,
{
    fn from_raw(data: &[u8]) -> crate::store::Result<Self> {
        let term = data[..8].try_into().map(i64::from_be_bytes)?;
        let command = T::from_raw(&data[8..])?;
        Ok(Entry::new(term, command))
    }
    fn to_raw(&self) -> IVec {
        let term = IVec::from(&self.term.to_be_bytes());
        let cmd = self.command.to_raw();
        IVec::from([term, cmd].concat())
    }
}
