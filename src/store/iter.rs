// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

use std::marker::PhantomData;

use sled::Iter as SledIter;

use super::{serde::Serializeable, Error, Result};

pub struct Iter<K, V> {
    inner: SledIter,
    phantom_k: PhantomData<K>,
    phantom_v: PhantomData<V>,
}

impl<K, V> From<SledIter> for Iter<K, V> {
    fn from(inner: SledIter) -> Self {
        Self {
            inner,
            phantom_k: PhantomData,
            phantom_v: PhantomData,
        }
    }
}

impl<K, V> Iterator for Iter<K, V>
where
    K: Serializeable,
    V: Serializeable,
{
    type Item = Result<(K, V)>;

    fn next(&mut self) -> Option<Self::Item> {
        let next = match self.inner.next()? {
            Ok(next) => next,
            Err(e) => return Some(Err(Error::from(e))),
        };
        let key = match K::from_raw(next.0.as_ref()) {
            Ok(key) => key,
            Err(e) => return Some(Err(e)),
        };
        let value = match V::from_raw(next.1.as_ref()) {
            Ok(value) => value,
            Err(e) => return Some(Err(e)),
        };
        Some(Ok((key, value)))
    }
}

impl<K, V> DoubleEndedIterator for Iter<K, V>
where
    K: Serializeable,
    V: Serializeable,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        let next = match self.inner.next_back()? {
            Ok(next) => next,
            Err(e) => return Some(Err(Error::from(e))),
        };
        let key = match K::from_raw(next.0.as_ref()) {
            Ok(key) => key,
            Err(e) => return Some(Err(e)),
        };
        let value = match V::from_raw(next.1.as_ref()) {
            Ok(value) => value,
            Err(e) => return Some(Err(e)),
        };
        Some(Ok((key, value)))
    }
}

#[cfg(test)]
#[cfg(not(tarpaulin_include))]
mod tests {
    use crate::store::test_harness::{new_test_db, TestStruct};

    use super::*;

    #[test]
    fn test_iterator() {
        let db = new_test_db().expect("failed to open temporary db.");
        for idx in 0..5 as u64 {
            let data = db
                .insert(
                    idx.to_be_bytes(),
                    TestStruct::new(idx + 1, idx + 2).to_raw(),
                )
                .expect("failed to insert new value");
            assert!(data.is_none());
        }

        let mut iter = Iter::<u64, TestStruct>::from(db.iter());
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
                    TestStruct::new(idx + 1, idx + 2).to_raw(),
                )
                .expect("failed to insert new value");
            assert!(data.is_none());
        }

        let mut iter = Iter::<u64, TestStruct>::from(db.iter());
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
