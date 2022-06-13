// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

use std::sync::Mutex;

use crate::rpc::raft::Entry;
use crate::store::Store;

use super::{Error, Result};

#[derive(Debug)]
pub struct Log {
    store: Mutex<Store<i64, Entry>>,
}

impl Log {
    pub fn new(db: &sled::Db) -> Result<Log> {
        let store = Store::new(db, "log")?;
        Ok(Log {
            store: Mutex::new(store),
        })
    }

    pub fn len(&self) -> usize {
        self.store.lock().unwrap().len()
    }

    pub fn is_empty(&self) -> bool {
        self.store.lock().unwrap().is_empty()
    }

    pub fn last_log_idx_and_term(&self) -> Result<(i64, i64)> {
        self.store
            .lock()
            .unwrap()
            .last()?
            .map_or(Ok((0, 0)), |(idx, entry)| Ok((idx, entry.term)))
    }

    pub fn append(&self, term: i64, command: Vec<u8>) -> Result<()> {
        let store = self.store.lock().unwrap();

        let idx = store.last()?.map_or(0, |(idx, _)| idx + 1);
        store
            .compare_and_swap(idx, None, Some(Entry { term, command }))
            .map_err(Error::from)
    }

    pub fn insert(&self, idx: i64, entry: Entry) -> Result<()> {
        let store = self.store.lock().unwrap();
        store
            .compare_and_swap(idx, None, Some(entry))
            .map_err(Error::from)
    }

    pub fn idx_and_term_match(&self, idx: i64, term: i64) -> Result<bool> {
        let entry = self.get(idx)?;
        Ok(entry.term == term)
    }

    pub fn get(&self, idx: i64) -> Result<Entry> {
        let store = self.store.lock().unwrap();

        match store.get(idx)? {
            Some(entry) => Ok(entry),
            None => Err(Error::Missing),
        }
    }

    pub fn range(&self, idx: i64, count: usize) -> Result<Vec<Entry>> {
        let store = self.store.lock().unwrap();
        let end = idx + count as i64;
        store
            .range(idx..end)
            .map(|result| result.map(|(_, entry)| entry).map_err(Error::from))
            .collect()
    }

    pub fn dump(&self) -> Result<Vec<Entry>> {
        let store = self.store.lock().unwrap();
        store
            .iter()
            .map(|result| result.map(|(_, entry)| entry).map_err(Error::from))
            .collect()
    }
}
