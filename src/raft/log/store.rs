// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

use std::ops::RangeBounds;

use sled::{transaction::abort, IVec};

use super::{
    iter::Iter,
    utils::{ivec_to_entry, map_bound, result_tuple_to_key, tuple_to_key_entry},
    ClusterConfig, Entry, Error, Result,
};

const TREE: &str = "log";

#[derive(Debug, Clone)]
pub struct Log {
    tree: sled::Tree,
}

impl Log {
    pub fn new(db: &sled::Db) -> Result<Log> {
        db.open_tree(TREE).map(Log::from).map_err(Error::from)
    }

    pub fn last_log_idx_and_term(&self) -> Result<(u64, u64)> {
        Ok(self
            .tree
            .last()?
            .map(tuple_to_key_entry)
            .map(|(idx, entry)| (idx, entry.term()))
            .unwrap_or((0, 0)))
    }

    pub fn append(&self, entry: Entry) -> Result<u64> {
        let entry = entry.to_ivec()?;
        // Run a CAS loop until we successfully append this entry to our log.
        loop {
            // First grab the current last index and increment by one.
            let mut idx = self
                .tree
                .last()
                .transpose()
                .map(result_tuple_to_key)
                .unwrap_or(Ok(0))?;
            idx += 1;

            // Now attempt to insert the entry at the specified index, failing
            // if there is already a different entry.
            match self.tree.compare_and_swap::<[u8; 8], IVec, IVec>(
                idx.to_be_bytes(),
                None,
                Some(entry.clone()),
            )? {
                Ok(_) => return Ok(idx), // CAS succeeded return.
                Err(_) => continue,      // CAS failed retry.
            }
        }
    }

    pub fn append_entries(
        &self,
        prev_log_idx: u64,
        entries: Vec<Entry>,
    ) -> Result<Option<ClusterConfig>> {
        self.tree
            .transaction(move |tx| {
                let mut log_insert_index = prev_log_idx + 1;
                let mut res = None;
                let mut check_entry = true;

                let mut batch = sled::Batch::default();
                for entry in entries.iter() {
                    let key = log_insert_index.to_be_bytes();
                    if check_entry
                        && tx
                            .get(&key)?
                            .map(ivec_to_entry)
                            .map(|current| current.term() != entry.term())
                            .unwrap_or(false)
                    {
                        check_entry = false;
                        tx.remove(&key)?;
                        let mut rm_idx = log_insert_index + 1;
                        while tx.remove(&rm_idx.to_be_bytes())?.is_some() {
                            rm_idx += 1;
                        }
                    }

                    if entry.is_cluster_config() {
                        res = Some(entry.unwrap_cluster_config());
                    }

                    let entry = match entry.to_ivec() {
                        Ok(entry) => entry,
                        Err(e) => return abort::<Option<ClusterConfig>, Error>(e),
                    };
                    batch.insert(&key, entry);
                    log_insert_index += 1;
                }

                tx.apply_batch(&batch)?;
                tx.flush();
                Ok(res)
            })
            .map_err(Error::from)
    }

    pub fn get(&self, idx: u64) -> Result<Option<Entry>> {
        self.tree
            .get(idx.to_be_bytes())?
            .map(Entry::from_ivec)
            .transpose()
    }

    pub fn idx_and_term_match(&self, idx: u64, term: u64) -> Result<bool> {
        Ok(self.get(idx)?.map_or(false, |entry| entry.term() == term))
    }

    pub fn range<R>(&self, bounds: R) -> Iter
    where
        R: RangeBounds<u64>,
    {
        let start = map_bound(bounds.start_bound(), |start| start.to_be_bytes());
        let end = map_bound(bounds.end_bound(), |end| end.to_be_bytes());
        self.tree.range((start, end)).into()
    }

    pub fn dump(&self) -> impl DoubleEndedIterator<Item = Result<Entry>> {
        self.range(..).entries()
    }

    pub async fn flush(&self) -> Result<()> {
        self.tree
            .flush_async()
            .await
            .map(|_| ())
            .map_err(Error::from)
    }
}

impl Default for Log {
    fn default() -> Log {
        let db = sled::Config::new()
            .temporary(true)
            .open()
            .expect("failed to open temporary database");
        Log::new(&db).expect("failed to open tree in temporary database")
    }
}

impl From<sled::Tree> for Log {
    fn from(tree: sled::Tree) -> Log {
        Log { tree }
    }
}

#[cfg(test)]
mod tests {
    use std::{thread, time::Duration};

    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case::single(vec![Entry::Noop(1)], 1, 1)]
    #[case::multiple_same_term(vec![Entry::Noop(1), Entry::Registration(1)], 2, 1)]
    #[case::multiple_diff_term(vec![Entry::Noop(1), Entry::Registration(2)], 2, 2)]
    fn test_last_log_idx_and_term(
        #[case] input: Vec<Entry>,
        #[case] expected_last_idx: u64,
        #[case] expected_last_term: u64,
    ) {
        let log = Log::default();

        let (initial_last_idx, initial_last_term) = log
            .last_log_idx_and_term()
            .expect("Failed to retrieve last log idx and term on empty log");
        assert_eq!(0, initial_last_idx);
        assert_eq!(0, initial_last_term);

        for entry in input {
            log.append(entry).expect("Failed to append entry.");
        }
        let (actual_last_idx, actual_last_term) = log
            .last_log_idx_and_term()
            .expect("Failed to retrieve last log idx and term");
        assert_eq!(expected_last_idx, actual_last_idx);
        assert_eq!(expected_last_term, actual_last_term);
    }

    #[rstest]
    #[case::simple(
        vec![
            vec![Entry::Noop(1)],
            vec![Entry::Noop(1)],
        ],
        2, 1,
    )]
    #[case::dual_same_term(
        vec![
            vec![Entry::Noop(1), Entry::Noop(1), Entry::Noop(1), Entry::Noop(1), Entry::Noop(1), Entry::Noop(1)],
            vec![Entry::Noop(1), Entry::Noop(1), Entry::Noop(1), Entry::Noop(1), Entry::Noop(1), Entry::Noop(1)],
        ],
        12, 1,
    )]
    #[case::triple_same_term(
        vec![
            vec![Entry::Noop(1), Entry::Noop(1), Entry::Noop(1), Entry::Noop(1), Entry::Noop(1), Entry::Noop(1)],
            vec![Entry::Noop(1), Entry::Noop(1), Entry::Noop(1), Entry::Noop(1), Entry::Noop(1), Entry::Noop(1)],
            vec![Entry::Noop(1), Entry::Noop(1), Entry::Noop(1), Entry::Noop(1), Entry::Noop(1), Entry::Noop(1)],
        ],
        18, 1,
    )]
    fn test_append_multiple_writers(
        #[case] writer_lists: Vec<Vec<Entry>>,
        #[case] expected_last_idx: u64,
        #[case] expected_last_term: u64,
    ) {
        let log = Log::default();
        let mut handles = Vec::with_capacity(writer_lists.len());

        for list in writer_lists {
            let list = list.clone();
            let log = log.clone();
            let handle = thread::spawn(move || {
                for entry in list {
                    log.append(entry).expect("Failed to append entry");
                }
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().expect("Thread failed to complete.");
        }

        let (actual_last_idx, actual_last_term) = log
            .last_log_idx_and_term()
            .expect("Failed to retrieve last log idx and term");
        assert_eq!(expected_last_idx, actual_last_idx);
        assert_eq!(expected_last_term, actual_last_term);

        let matches = log
            .idx_and_term_match(actual_last_idx, actual_last_term)
            .expect("Failed to match idx to term");
        assert!(matches);

        let matches = log
            .idx_and_term_match(10000000000, 1)
            .expect("Failed to match idx to term");
        assert!(!matches);
    }

    #[rstest]
    #[case::simple(
        vec![
            (0, vec![Entry::Noop(1), Entry::Noop(2), Entry::Noop(3), Entry::Noop(4), Entry::Noop(5), Entry::Noop(6)]),
            (6, vec![Entry::Registration(7), Entry::Registration(8), Entry::Registration(9), Entry::Registration(10), Entry::Registration(11), Entry::Registration(12)]),
        ],
        vec![Entry::Noop(1), Entry::Noop(2), Entry::Noop(3), Entry::Noop(4), Entry::Noop(5), Entry::Noop(6), Entry::Registration(7), Entry::Registration(8), Entry::Registration(9), Entry::Registration(10), Entry::Registration(11), Entry::Registration(12)]
    )]
    #[case::overwrite(
        vec![
            (0, vec![Entry::Noop(1), Entry::Noop(2), Entry::Noop(3), Entry::Noop(4), Entry::Noop(5), Entry::Noop(6), Entry::Noop(1), Entry::Noop(2), Entry::Noop(3), Entry::Noop(4), Entry::Noop(5), Entry::Noop(6)]),
            (3, vec![Entry::Registration(7), Entry::Registration(8), Entry::Registration(9), Entry::Registration(10), Entry::Registration(11), Entry::Registration(12)]),
        ],
        vec![Entry::Noop(1), Entry::Noop(2), Entry::Noop(3), Entry::Registration(7), Entry::Registration(8), Entry::Registration(9), Entry::Registration(10), Entry::Registration(11), Entry::Registration(12)]
    )]
    fn test_append_entries(
        #[case] writer_lists: Vec<(u64, Vec<Entry>)>,
        #[case] expected: Vec<Entry>,
    ) {
        let log = Log::default();
        let mut handles = Vec::with_capacity(writer_lists.len());

        for (prev_log_idx, list) in writer_lists {
            let list = list.clone();
            let log = log.clone();
            let handle = thread::spawn(move || {
                log.append_entries(prev_log_idx, list)
                    .expect("Failed to append entries");
            });
            handles.push(handle);
            // Have to schedule the thread, or we run into ooo issues.
            // TODO(csaide): Does this actually pose a problem?
            thread::sleep(Duration::from_nanos(1));
        }

        for handle in handles {
            handle.join().expect("Thread failed to complete.");
        }

        let actual: Vec<Entry> = log
            .dump()
            .map(|r| r.expect("Failed to retrieve entry"))
            .collect();
        assert_eq!(expected, actual)
    }
}
