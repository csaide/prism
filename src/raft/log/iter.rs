// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

use super::{
    utils::{result_tuple_to_entry, result_tuple_to_key, result_tuple_to_key_entry},
    Entry, Result,
};

pub struct Iter {
    inner: sled::Iter,
}

impl Iter {
    pub fn keys(self) -> impl DoubleEndedIterator<Item = Result<u64>> + Send + Sync {
        self.inner.map(result_tuple_to_key)
    }

    pub fn entries(self) -> impl DoubleEndedIterator<Item = Result<Entry>> + Send + Sync {
        self.inner.map(result_tuple_to_entry)
    }
}

impl From<sled::Iter> for Iter {
    fn from(inner: sled::Iter) -> Iter {
        Iter { inner }
    }
}

impl Iterator for Iter {
    type Item = Result<(u64, Entry)>;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(result_tuple_to_key_entry)
    }

    fn last(self) -> Option<Self::Item> {
        self.inner.last().map(result_tuple_to_key_entry)
    }
}

impl DoubleEndedIterator for Iter {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.inner.next_back().map(result_tuple_to_key_entry)
    }
}

#[cfg(test)]
mod tests {
    use std::ops::RangeBounds;

    use rstest::rstest;

    use crate::raft::log::Log;

    use super::*;

    #[rstest]
    #[case::empty(.., vec![], vec![])]
    #[case::happy(.., vec![Entry::Registration(1), Entry::Noop(1)], vec![1, 2])]
    fn test_iter_keys<R>(#[case] bounds: R, #[case] input: Vec<Entry>, #[case] expected: Vec<u64>)
    where
        R: RangeBounds<u64>,
    {
        let log = Log::default();

        log.append_entries(0, input.clone())
            .expect("Failed to append input entries.");

        let iter = log.range(bounds);
        let actual: Vec<u64> = iter
            .keys()
            .map(|result| result.expect("Failed to marshal key"))
            .collect();
        assert_eq!(expected, actual);
    }

    #[rstest]
    #[case::empty(.., vec![], vec![])]
    #[case::happy(1..3, vec![Entry::Registration(1), Entry::Noop(1)], vec![Entry::Registration(1), Entry::Noop(1)])]
    fn test_iter_entries<R>(
        #[case] bounds: R,
        #[case] input: Vec<Entry>,
        #[case] expected: Vec<Entry>,
    ) where
        R: RangeBounds<u64>,
    {
        let log = Log::default();

        log.append_entries(0, input.clone())
            .expect("Failed to append input entries.");

        let iter = log.range(bounds);
        let actual: Vec<Entry> = iter
            .entries()
            .map(|result| result.expect("Failed to marshal entry"))
            .collect();
        assert_eq!(expected, actual);
    }

    #[rstest]
    #[case::empty(.., vec![], None, None)]
    #[case::happy(1..=3, vec![Entry::Registration(1), Entry::Noop(1), Entry::Registration(2)], Some((1, Entry::Registration(1))), Some((3,Entry::Registration(2))))]
    fn test_iterator<R>(
        #[case] bounds: R,
        #[case] input: Vec<Entry>,
        #[case] first: Option<(u64, Entry)>,
        #[case] last: Option<(u64, Entry)>,
    ) where
        R: RangeBounds<u64>,
    {
        let log = Log::default();

        log.append_entries(0, input.clone())
            .expect("Failed to append input entries.");

        let mut iter = log.range(bounds);

        let actual_first = iter.next().transpose().expect("Failed to pop next.");
        assert_eq!(first, actual_first);

        let actual_last = iter.last().transpose().expect("Failed to pop last");
        assert_eq!(last, actual_last);
    }

    #[rstest]
    #[case::empty(.., vec![], None)]
    #[case::happy(1..=3, vec![Entry::Registration(1), Entry::Noop(1), Entry::Registration(2)], Some((3, Entry::Registration(2))))]
    fn test_double_ended_iterator<R>(
        #[case] bounds: R,
        #[case] input: Vec<Entry>,
        #[case] last: Option<(u64, Entry)>,
    ) where
        R: RangeBounds<u64>,
    {
        let log = Log::default();

        log.append_entries(0, input.clone())
            .expect("Failed to append input entries.");

        let mut iter = log.range(bounds);

        let actual_last = iter
            .next_back()
            .transpose()
            .expect("Failed to pop next back.");
        assert_eq!(last, actual_last);
    }
}
