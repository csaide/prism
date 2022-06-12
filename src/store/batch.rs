// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

use std::marker::PhantomData;

use super::serde::Serializeable;

pub struct Batch<K, V> {
    inner: sled::Batch,
    phantom_k: PhantomData<K>,
    phantom_v: PhantomData<V>,
}

impl<K, V> Batch<K, V>
where
    K: Serializeable,
    V: Serializeable,
{
    pub fn insert(&mut self, k: K, v: V) {
        self.inner.insert(k.to_raw(), v.to_raw())
    }
    pub fn remove(&mut self, k: K) {
        self.inner.remove(k.to_raw())
    }
    pub fn into_inner(self) -> sled::Batch {
        self.inner
    }
}
