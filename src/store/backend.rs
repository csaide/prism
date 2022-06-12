// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

use std::{marker::PhantomData, ops::RangeBounds};

use sled::Tree;

use super::{serde::Serializeable, Batch, Error, Iter, Result, Subscriber};

pub struct Store<K, V> {
    tree: Tree,
    phantom_k: PhantomData<K>,
    phantom_v: PhantomData<V>,
}

impl<K, V> Store<K, V>
where
    V: Serializeable,
    K: Serializeable,
{
    pub fn new(db: &sled::Db, name: &str) -> Result<Store<K, V>> {
        let tree = db.open_tree(name)?;
        Ok(Store {
            tree,
            phantom_k: PhantomData,
            phantom_v: PhantomData,
        })
    }

    pub fn contains(&self, key: K) -> Result<bool> {
        self.tree.contains_key(key.to_raw()).map_err(Error::from)
    }

    pub fn len(&self) -> usize {
        self.tree.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tree.is_empty()
    }

    pub fn remove(&self, key: K) -> Result<Option<V>> {
        match self.tree.remove(key.to_raw())? {
            Some(ivec) => V::from_raw(ivec.as_ref()).map(Some),
            None => Ok(None),
        }
    }

    pub fn apply_batch(&self, batch: Batch<K, V>) -> Result<()> {
        self.tree
            .apply_batch(batch.into_inner())
            .map_err(Error::from)
    }

    pub fn clear(&self) -> Result<()> {
        self.tree.clear().map_err(Error::from)
    }

    pub fn compare_and_swap(&self, key: K, old: Option<V>, new: Option<V>) -> Result<()> {
        self.tree
            .compare_and_swap(
                key.to_raw(),
                old.map(|val| val.to_raw()),
                new.map(|val| val.to_raw()),
            )?
            .map_err(Error::from)
    }

    pub fn update_and_fetch<F>(&self, key: K, mut f: F) -> Result<Option<V>>
    where
        F: FnMut(Option<&[u8]>) -> Option<V>,
    {
        match self
            .tree
            .update_and_fetch(key.to_raw(), |val| f(val).map(|val| val.to_raw()))?
        {
            Some(val) => Ok(Some(V::from_raw(val.as_ref())?)),
            None => Ok(None),
        }
    }

    pub fn fetch_and_update<F>(&self, key: K, mut f: F) -> Result<Option<V>>
    where
        F: FnMut(Option<&[u8]>) -> Option<V>,
    {
        match self
            .tree
            .fetch_and_update(key.to_raw(), |val| f(val).map(|val| val.to_raw()))?
        {
            Some(val) => Ok(Some(V::from_raw(val.as_ref())?)),
            None => Ok(None),
        }
    }

    pub fn insert(&self, key: K, value: &V) -> Result<Option<V>> {
        let bytes = value.to_raw();

        match self.tree.insert(key.to_raw(), bytes.as_ref())? {
            Some(ivec) => V::from_raw(ivec.as_ref()).map(Some),
            None => Ok(None),
        }
    }

    pub fn get(&self, key: K) -> Result<Option<V>> {
        match self.tree.get(key.to_raw())? {
            Some(ivec) => V::from_raw(ivec.as_ref()).map(Some),
            None => Ok(None),
        }
    }

    pub fn iter(&self) -> Iter<K, V> {
        self.tree.iter().into()
    }

    pub fn range<R>(&self, range: R) -> Iter<K, V>
    where
        R: RangeBounds<K>,
    {
        use std::ops::Bound::*;

        let start = match range.start_bound() {
            Included(start) => Included(start.to_raw()),
            Excluded(start) => Excluded(start.to_raw()),
            Unbounded => Unbounded,
        };
        let end = match range.end_bound() {
            Included(end) => Included(end.to_raw()),
            Excluded(end) => Excluded(end.to_raw()),
            Unbounded => Unbounded,
        };
        self.tree.range((start, end)).into()
    }

    pub fn scan_prefix(&self, prefix: K) -> Iter<K, V> {
        self.tree.scan_prefix(prefix.to_raw()).into()
    }

    pub fn watch_prefix(&self, prefix: K) -> Subscriber<K, V> {
        self.tree.watch_prefix(prefix.to_raw()).into()
    }

    pub fn first(&self) -> Result<Option<(K, V)>> {
        match self.tree.first()? {
            Some((key, value)) => Ok(Some((
                K::from_raw(key.as_ref())?,
                V::from_raw(value.as_ref())?,
            ))),
            None => Ok(None),
        }
    }

    pub fn last(&self) -> Result<Option<(K, V)>> {
        match self.tree.last()? {
            Some((key, value)) => Ok(Some((
                K::from_raw(key.as_ref())?,
                V::from_raw(value.as_ref())?,
            ))),
            None => Ok(None),
        }
    }
}

#[cfg(test)]
#[cfg(not(tarpaulin_include))]
mod tests {
    use crate::store::test_harness::{new_test_db, TestStruct};

    use super::*;

    #[test]
    pub fn test_crud() {
        let tree = "store-test";
        let db = new_test_db().expect("failed to create test database");

        let store: Store<u64, TestStruct> =
            Store::new(&db, tree).expect("failed to create new store");

        let first = TestStruct { a: 1, b: 1 };
        let second = TestStruct { a: 2, b: 2 };

        let first_insert = store.insert(1, &first).expect("failed to insert");
        assert!(matches!(first_insert, None));

        let first_get = store.get(1).expect("failed to get");
        assert!(matches!(first_get, Some(..)));
        let first_get = first_get.unwrap();
        assert_eq!(first.a, first_get.a);
        assert_eq!(first.b, first_get.b);

        assert!(store.contains(1).expect("failed to check db for existence"));

        let second_insert = store
            .insert(1, &second)
            .expect("failed to overwrite existing key");
        assert!(matches!(second_insert, Some(..)));
        let second_insert = second_insert.unwrap();
        assert_eq!(first.a, second_insert.a);
        assert_eq!(first.b, second_insert.b);

        let first_del = store.remove(1).expect("failed to remove key");
        assert!(matches!(first_del, Some(..)));
        let first_del = first_del.unwrap();
        assert_eq!(second.a, first_del.a);
        assert_eq!(second.b, first_del.b);

        assert!(!store.contains(1).expect("failed to check db for existence"));

        assert_eq!(store.len(), 0);
        assert!(store.is_empty());
    }

    #[test]
    pub fn test_iteration() {
        let tree = "store-test";
        let db = new_test_db().expect("failed to create test database");

        let store: Store<u64, TestStruct> =
            Store::new(&db, tree).expect("failed to create new store");

        let first = TestStruct { a: 1, b: 1 };
        let second = TestStruct { a: 2, b: 2 };
        let third = TestStruct { a: 3, b: 3 };

        assert!(store
            .insert(0, &first)
            .expect("failed to insert value")
            .is_none());
        assert!(store
            .insert(1, &second)
            .expect("failed to insert value")
            .is_none());
        assert!(store
            .insert(2, &third)
            .expect("failed to insert value")
            .is_none());

        assert_eq!(store.len(), 3);
        let mut iter = store.iter();

        let current = iter.next().expect("failed to retrieve next value").unwrap();
        assert_eq!(current.0, 0);
        assert_eq!(current.1.a, 1);
        assert_eq!(current.1.b, 1);

        let current = iter.next().expect("failed to retrieve next value").unwrap();
        assert_eq!(current.0, 1);
        assert_eq!(current.1.a, 2);
        assert_eq!(current.1.b, 2);

        let current = iter.next().expect("failed to retrieve next value").unwrap();
        assert_eq!(current.0, 2);
        assert_eq!(current.1.a, 3);
        assert_eq!(current.1.b, 3);
    }

    #[test]
    pub fn test_range() {
        let tree = "store-test";
        let db = new_test_db().expect("failed to create test database");

        let store: Store<u64, TestStruct> =
            Store::new(&db, tree).expect("failed to create new store");

        let first = TestStruct { a: 1, b: 1 };
        let second = TestStruct { a: 2, b: 2 };
        let third = TestStruct { a: 3, b: 3 };

        assert!(store
            .insert(0, &first)
            .expect("failed to insert value")
            .is_none());
        assert!(store
            .insert(1, &second)
            .expect("failed to insert value")
            .is_none());
        assert!(store
            .insert(2, &third)
            .expect("failed to insert value")
            .is_none());

        assert_eq!(store.len(), 3);
        let mut iter = store.range(1..3);

        let current = iter.next().expect("failed to retrieve next value").unwrap();
        assert_eq!(current.0, 1);
        assert_eq!(current.1.a, 2);
        assert_eq!(current.1.b, 2);

        let current = iter.next().expect("failed to retrieve next value").unwrap();
        assert_eq!(current.0, 2);
        assert_eq!(current.1.a, 3);
        assert_eq!(current.1.b, 3);
    }
}
