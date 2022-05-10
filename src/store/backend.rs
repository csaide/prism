// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

use std::{marker::PhantomData, ops::RangeBounds};

use rkyv::{
    de::deserializers::SharedDeserializeMap, ser::serializers::AllocSerializer, Archive,
    Deserialize,
};
use sled::Tree;

use super::{
    helpers::{from_ivec, to_bytes},
    Error, Iter, Key, Result,
};

pub struct Store<K, V, const N: usize> {
    tree: Tree,
    phantom_k: PhantomData<K>,
    phantom_v: PhantomData<V>,
}

impl<K, V, const N: usize> Store<K, V, N>
where
    V: Archive,
    <V as Archive>::Archived: Deserialize<V, SharedDeserializeMap>,
    V: rkyv::Serialize<AllocSerializer<N>>,
    K: Key,
{
    pub fn new(db: &sled::Db, name: &str) -> Result<Store<K, V, N>> {
        let tree = db.open_tree(name).map_err(Error::from)?;
        Ok(Store {
            tree,
            phantom_v: PhantomData,
            phantom_k: PhantomData,
        })
    }

    pub fn contains(&self, key: K) -> Result<bool> {
        self.tree.contains_key(key.to_bytes()).map_err(Error::from)
    }

    pub fn len(&self) -> usize {
        self.tree.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tree.is_empty()
    }

    pub fn remove(&self, key: K) -> Result<Option<V>> {
        match self.tree.remove(key.to_bytes()).map_err(Error::from)? {
            Some(ivec) => from_ivec(ivec).map(Some),
            None => Ok(None),
        }
    }

    pub fn insert(&self, key: K, value: &V) -> Result<Option<V>> {
        let bytes = to_bytes(value)?;

        match self
            .tree
            .insert(key.to_bytes(), bytes.as_ref())
            .map_err(Error::from)?
        {
            Some(ivec) => from_ivec(ivec).map(Some),
            None => Ok(None),
        }
    }

    pub fn get(&self, key: K) -> Result<Option<V>> {
        match self.tree.get(key.to_bytes()).map_err(Error::from)? {
            Some(ivec) => from_ivec(ivec).map(Some),
            None => Ok(None),
        }
    }

    pub fn iter(&self) -> Iter<V> {
        self.tree.iter().into()
    }

    pub fn range<R>(&self, range: R) -> Iter<V>
    where
        R: RangeBounds<K>,
    {
        use std::ops::Bound::*;

        let start = match range.start_bound() {
            Included(start) => Included(start.to_bytes()),
            Excluded(start) => Excluded(start.to_bytes()),
            Unbounded => Unbounded,
        };
        let end = match range.end_bound() {
            Included(end) => Included(end.to_bytes()),
            Excluded(end) => Excluded(end.to_bytes()),
            Unbounded => Unbounded,
        };
        self.tree.range((start, end)).into()
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

        let store: Store<u64, TestStruct, 16> =
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

        let store: Store<u64, TestStruct, 16> =
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

        let store: Store<u64, TestStruct, 16> =
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
