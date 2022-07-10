// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

use std::sync::RwLock;

#[derive(Debug)]
pub struct AtomicIndex {
    inner: RwLock<u64>,
}

impl AtomicIndex {
    pub fn new() -> AtomicIndex {
        AtomicIndex {
            inner: RwLock::new(0),
        }
    }

    pub fn get(&self) -> u64 {
        *self.inner.read().unwrap()
    }
    pub fn set(&self, idx: u64) {
        *self.inner.write().unwrap() = idx
    }
}

impl Default for AtomicIndex {
    fn default() -> AtomicIndex {
        AtomicIndex::new()
    }
}

impl From<u64> for AtomicIndex {
    fn from(inner: u64) -> AtomicIndex {
        AtomicIndex {
            inner: RwLock::new(inner),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_atomic_index() {
        let index = AtomicIndex::from(2);
        assert_eq!(2, index.get());
        index.set(20);
        assert_eq!(20, index.get());
    }
}
