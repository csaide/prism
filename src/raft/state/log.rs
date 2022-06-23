// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

use sled::IVec;

use super::{ClusterConfig, Entry, Error, Result};

#[derive(Debug)]
pub struct Log {
    store: sled::Tree,
}

impl Log {
    pub fn new(db: &sled::Db) -> Result<Log> {
        let store = db.open_tree("log")?;
        Ok(Log { store })
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.store.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.store.is_empty()
    }

    pub fn last_log_idx_and_term(&self) -> Result<(u128, u128)> {
        let (idx, entry) = match self.store.last()? {
            Some(tuple) => tuple,
            None => return Ok((0, 0)),
        };

        let idx = idx.as_ref().try_into().map(u128::from_be_bytes)?;
        use Entry::*;
        match Entry::from_ivec(entry)? {
            Command(cmd) => Ok((idx, cmd.term)),
            ClusterConfig(cfg) => Ok((idx, cfg.term)),
            Registration(reg) => Ok((idx, reg.term)),
        }
    }

    pub fn append(&self, entry: Entry) -> Result<u128> {
        let idx = match self.store.last()? {
            Some((idx, _)) => idx.as_ref().try_into().map(u128::from_be_bytes)? + 1,
            None => 1,
        };
        let key = IVec::from(&idx.to_be_bytes());
        self.store
            .compare_and_swap::<IVec, IVec, IVec>(key, None, Some(entry.to_ivec()?))?
            .map_err(Error::from)?;
        Ok(idx)
    }

    pub fn insert(&self, idx: u128, entry: Entry) -> Result<()> {
        self.store
            .compare_and_swap::<IVec, IVec, IVec>(
                IVec::from(&idx.to_be_bytes()),
                None,
                Some(entry.to_ivec()?),
            )?
            .map_err(Error::from)
    }

    pub fn idx_and_term_match(&self, idx: u128, term: u128) -> Result<bool> {
        let entry = self.get(idx)?;
        use Entry::*;
        match entry {
            Command(cmd) => Ok(cmd.term == term),
            ClusterConfig(cfg) => Ok(cfg.term == term),
            Registration(reg) => Ok(reg.term == term),
        }
    }

    pub fn get(&self, idx: u128) -> Result<Entry> {
        match self.store.get(idx.to_be_bytes())? {
            Some(entry) => Entry::from_ivec(entry),
            None => Err(Error::Missing),
        }
    }

    pub fn range(&self, start: u128, end: u128) -> Result<Vec<(u128, Entry)>> {
        let mut payloads = Vec::default();
        let start = start.to_be_bytes();
        let end = end.to_be_bytes();
        for result in self
            .store
            .range(start..end)
            .map(|result| result.map_err(Error::from))
        {
            match result {
                Ok((id, entry)) => payloads.push((
                    id.as_ref().try_into().map(u128::from_be_bytes)?,
                    Entry::from_ivec(entry)?,
                )),
                Err(e) => return Err(e),
            };
        }
        Ok(payloads)
    }

    pub fn dump(&self) -> Result<Vec<Entry>> {
        let mut payloads = Vec::with_capacity(self.len());
        for result in self
            .store
            .iter()
            .map(|result| result.map(|(_, entry)| entry).map_err(Error::from))
        {
            match result {
                Ok(v) => payloads.push(Entry::from_ivec(v)?),
                Err(e) => return Err(e),
            };
        }
        Ok(payloads)
    }

    pub fn append_entries(
        &self,
        prev_log_idx: u128,
        mut entries: Vec<Entry>,
    ) -> Result<Option<ClusterConfig>> {
        let mut log_insert_index = prev_log_idx + 1;
        let mut entries_insert_index: usize = 0;
        loop {
            if log_insert_index > self.len() as u128 || entries_insert_index >= entries.len() {
                break;
            }
            if !self.idx_and_term_match(log_insert_index, entries[entries_insert_index].term())? {
                break;
            }
            log_insert_index += 1;
            entries_insert_index += 1;
        }

        let mut res = None;
        for entry in entries.drain(entries_insert_index..) {
            if entry.is_cluster_config() {
                res = Some(entry.unwrap_cluster_config());
            }
            self.insert(log_insert_index, entry)?;
            log_insert_index += 1;
        }
        Ok(res)
    }

    pub async fn flush(&self) -> Result<()> {
        self.store
            .flush_async()
            .await
            .map(|_| ())
            .map_err(Error::from)
    }
}

#[cfg(test)]
#[cfg(not(tarpaulin_include))]
mod tests {
    use super::*;
    use crate::raft::ClusterConfig;

    #[test]
    fn test_log() {
        let db = sled::Config::new()
            .temporary(true)
            .open()
            .expect("Failed to open temp database.");

        let log = Log::new(&db).expect("Failed to create new persistent log.");

        let (last_log_idx, last_log_term) = log
            .last_log_idx_and_term()
            .expect("Failed to retrieve last log idx/term.");

        assert_eq!(0, last_log_idx);
        assert_eq!(0, last_log_term);

        let payload = Entry::ClusterConfig(ClusterConfig {
            term: 2,
            voters: vec![
                String::from("grpc://example.com:12345"),
                String::from("grpc://example.com:23456"),
                String::from("grpc://example.com:34567"),
            ],
            replicas: vec![],
        });

        log.append(payload.clone())
            .expect("Failed to append payload to empty log.");

        let (last_log_idx, last_log_term) = log
            .last_log_idx_and_term()
            .expect("Failed to retrieve last log idx/term.");

        assert_eq!(1, last_log_idx);
        assert_eq!(2, last_log_term);
        assert!(log
            .idx_and_term_match(last_log_idx, last_log_term)
            .expect("Failed to test idx/term equality"));

        let actual = log
            .get(last_log_idx)
            .expect("Failed to retrieve existing log payload.");
        assert_eq!(actual, payload)
    }
}
