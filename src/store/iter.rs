// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

use std::marker::PhantomData;

use rkyv::{de::deserializers::SharedDeserializeMap, Archive, Deserialize};
use sled::Iter as SledIter;

use super::{helpers::from_ivec_tuple, Error, Result};

pub struct Iter<T> {
    inner: SledIter,
    phantom: PhantomData<T>,
}

impl<T> From<SledIter> for Iter<T> {
    fn from(inner: SledIter) -> Self {
        Self {
            inner,
            phantom: PhantomData,
        }
    }
}

impl<T> Iterator for Iter<T>
where
    T: Archive,
    <T as Archive>::Archived: Deserialize<T, SharedDeserializeMap>,
{
    type Item = Result<(u64, T)>;

    fn next(&mut self) -> Option<Self::Item> {
        let next = match self.inner.next()? {
            Ok(next) => next,
            Err(e) => return Some(Err(Error::from(e))),
        };
        Some(from_ivec_tuple(next))
    }
}

impl<T> DoubleEndedIterator for Iter<T>
where
    T: Archive,
    <T as Archive>::Archived: Deserialize<T, SharedDeserializeMap>,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        let next = match self.inner.next_back()? {
            Ok(next) => next,
            Err(e) => return Some(Err(Error::from(e))),
        };
        Some(from_ivec_tuple(next))
    }
}

#[cfg(test)]
#[cfg(not(tarpaulin_include))]
mod tests {
    use crate::store::{
        helpers::to_bytes,
        test_harness::{new_test_db, TestStruct},
    };

    use super::*;

    #[test]
    fn test_iterator() {
        let db = new_test_db().expect("failed to open temporary db.");
        for idx in 0..5 as u64 {
            let data = db
                .insert(
                    idx.to_be_bytes(),
                    to_bytes::<TestStruct, 1024>(&TestStruct::new(idx + 1, idx + 2))
                        .expect("failed to serialize value")
                        .as_ref(),
                )
                .expect("failed to insert new value");
            assert!(data.is_none());
        }

        let mut iter = Iter::<TestStruct>::from(db.iter());
        for idx in 0..5 as u64 {
            let item = iter
                .next()
                .expect("missing data in iter")
                .expect("failed to read data");
            assert_eq!(item.0, idx);
            assert_eq!(item.1.a, idx + 1);
        }
        assert!(matches!(iter.next(), None))
    }

    #[test]
    fn test_double_ended_iterator() {
        let db = new_test_db().expect("failed to open temporary db.");
        for idx in 0..5 as u64 {
            let data = db
                .insert(
                    idx.to_be_bytes(),
                    to_bytes::<TestStruct, 1024>(&TestStruct::new(idx + 1, idx + 2))
                        .expect("failed to serialize value")
                        .as_ref(),
                )
                .expect("failed to insert new value");
            assert!(data.is_none());
        }

        let mut iter = Iter::<TestStruct>::from(db.iter());
        let offset: u64 = 4;
        for cursor in 0..5 as u64 {
            let idx = offset - cursor;
            let item = iter
                .next_back()
                .expect("missing data in iter")
                .expect("failed to read data");
            assert_eq!(item.0, idx);
            assert_eq!(item.1.a, idx + 1);
        }
        assert!(matches!(iter.next(), None))
    }
}
