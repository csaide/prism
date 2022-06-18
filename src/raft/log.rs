// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

use sled::IVec;

use crate::rpc::raft::{Entry, Payload};

use super::{Error, Result};

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

    pub fn last_log_idx_and_term(&self) -> Result<(i64, i64)> {
        let (idx, entry) = match self.store.last()? {
            Some(tuple) => tuple,
            None => return Ok((-1, -1)),
        };

        let idx = idx.as_ref().try_into().map(i64::from_be_bytes)?;
        use Payload::*;
        match Payload::from_ivec(entry)? {
            Command(cmd) => Ok((idx, cmd.term)),
            Config(cfg) => Ok((idx, cfg.term)),
            Snapshot(snap) => Ok((snap.last_included_idx, snap.last_included_term)),
        }
    }

    pub fn append(&self, payload: Payload) -> Result<()> {
        let idx = match self.store.last()? {
            Some((idx, _)) => idx.as_ref().try_into().map(i64::from_be_bytes)? + 1,
            None => 0,
        };
        let idx = IVec::from(&idx.to_be_bytes());
        self.store
            .compare_and_swap::<IVec, IVec, IVec>(idx, None, Some(payload.to_ivec()?))?
            .map_err(Error::from)
    }

    pub fn insert(&self, idx: i64, payload: Payload) -> Result<()> {
        self.store
            .compare_and_swap::<IVec, IVec, IVec>(
                IVec::from(&idx.to_be_bytes()),
                None,
                Some(payload.to_ivec()?),
            )?
            .map_err(Error::from)
    }

    pub fn idx_and_term_match(&self, idx: i64, term: i64) -> Result<bool> {
        let entry = self.get(idx)?;
        use Payload::*;
        match entry {
            Command(cmd) => Ok(cmd.term == term),
            Config(cfg) => Ok(cfg.term == term),
            Snapshot(snap) => Ok(snap.last_included_term == term),
        }
    }

    pub fn get(&self, idx: i64) -> Result<Payload> {
        match self.store.get(idx.to_be_bytes())? {
            Some(entry) => Payload::from_ivec(entry),
            None => Err(Error::Missing),
        }
    }

    pub fn range(&self, start: i64, end: i64) -> Result<Vec<Payload>> {
        let mut payloads = Vec::default();
        let start = start.to_be_bytes();
        let end = end.to_be_bytes();
        for result in self
            .store
            .range(start..end)
            .map(|result| result.map(|(_, entry)| entry).map_err(Error::from))
        {
            match result {
                Ok(v) => payloads.push(Payload::from_ivec(v)?),
                Err(e) => return Err(e),
            };
        }
        Ok(payloads)
    }

    pub fn dump(&self) -> Result<Vec<Payload>> {
        let mut payloads = Vec::with_capacity(self.len());
        for result in self
            .store
            .iter()
            .map(|result| result.map(|(_, entry)| entry).map_err(Error::from))
        {
            match result {
                Ok(v) => payloads.push(Payload::from_ivec(v)?),
                Err(e) => return Err(e),
            };
        }
        Ok(payloads)
    }

    pub fn append_entries(&self, prev_log_idx: i64, mut entries: Vec<Entry>) -> Result<()> {
        let mut log_insert_index = prev_log_idx + 1;
        let mut entries_insert_index: i64 = 0;
        loop {
            if log_insert_index >= self.len() as i64 || entries_insert_index >= entries.len() as i64
            {
                break;
            }
            if !self.idx_and_term_match(
                log_insert_index,
                entries[entries_insert_index as usize].term(),
            )? {
                break;
            }
            log_insert_index += 1;
            entries_insert_index += 1;
        }

        for entry in entries.drain(entries_insert_index as usize..) {
            if entry.payload.is_none() {
                continue;
            }

            self.insert(log_insert_index, entry.payload.unwrap())?;
            log_insert_index += 1;
        }
        Ok(())
    }
}

#[cfg(test)]
#[cfg(not(tarpaulin_include))]
mod tests {
    use super::*;
    use crate::rpc::raft::Config;

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

        assert_eq!(-1, last_log_idx);
        assert_eq!(-1, last_log_term);

        let payload = Payload::Config(Config {
            term: 2,
            peers: vec![
                String::from("grpc://example.com:12345"),
                String::from("grpc://example.com:23456"),
                String::from("grpc://example.com:34567"),
            ],
        });

        log.append(payload.clone())
            .expect("Failed to append payload to empty log.");

        let (last_log_idx, last_log_term) = log
            .last_log_idx_and_term()
            .expect("Failed to retrieve last log idx/term.");

        assert_eq!(0, last_log_idx);
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
