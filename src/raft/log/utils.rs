// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

use std::{ops::Bound, result};

use sled::IVec;

use super::{Entry, Error, Result};

fn ivec_to_key(idx: IVec) -> u64 {
    // We have to panic here, this is _only_ called when pulling data out of the DB,
    // if we fail to deserialize.... thats a very _VERY_ bad thing. Panic and shutdown
    // is the only option as the DB has become corupted.
    idx.as_ref()
        .try_into()
        .map(u64::from_be_bytes)
        .expect("internal database corruption detected... shutting down")
}

pub fn ivec_to_entry(entry: IVec) -> Entry {
    // We have to panic here, this is _only_ called when pulling data out of the DB,
    // if we fail to deserialize.... thats a very _VERY_ bad thing. Panic and shutdown
    // is the only option as the DB has become corupted.
    Entry::from_ivec(entry).expect("internal database corruption detected... shutting down")
}

pub fn tuple_to_key_entry(tuple: (IVec, IVec)) -> (u64, Entry) {
    (ivec_to_key(tuple.0), ivec_to_entry(tuple.1))
}

pub fn result_tuple_to_key_entry(
    result: result::Result<(IVec, IVec), sled::Error>,
) -> Result<(u64, Entry)> {
    result.map(tuple_to_key_entry).map_err(Error::from)
}

pub fn result_tuple_to_key(result: result::Result<(IVec, IVec), sled::Error>) -> Result<u64> {
    result.map(|(idx, _)| ivec_to_key(idx)).map_err(Error::from)
}

pub fn result_tuple_to_entry(result: result::Result<(IVec, IVec), sled::Error>) -> Result<Entry> {
    result
        .map(|(_, entry)| ivec_to_entry(entry))
        .map_err(Error::from)
}

pub fn map_bound<T, U, F: FnOnce(T) -> U>(input: Bound<T>, f: F) -> Bound<U> {
    use Bound::*;
    match input {
        Unbounded => Unbounded,
        Included(x) => Included(f(x)),
        Excluded(x) => Excluded(f(x)),
    }
}

#[cfg(test)]
mod tests {
    use std::fmt::Debug;

    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case::included(Bound::Included(100), Bound::Included(false), |val| val > 10000)]
    #[case::excluded(Bound::Excluded(100), Bound::Excluded(false), |val| val > 10000)]
    #[case::unbounded(Bound::Unbounded, Bound::Unbounded, |val: u64| val > 10000)]
    fn test_map_bound<I, E>(
        #[case] input: Bound<I>,
        #[case] expected: Bound<E>,
        #[case] f: impl FnMut(I) -> E,
    ) where
        E: Debug + PartialEq,
    {
        let actual = map_bound(input, f);
        assert_eq!(expected, actual)
    }

    #[rstest]
    #[case::error(
        Err(sled::Error::Unsupported(String::from("hello"))),
        Err(Error::from(sled::Error::Unsupported(String::from("hello"))))
    )]
    #[case::valid(
        Ok((IVec::default(), Entry::Registration(1).to_ivec().unwrap())),
        Ok(Entry::Registration(1))
    )]
    #[should_panic]
    #[case::invalid(
        Ok((IVec::default(), IVec::default())),
        Err(Error::from(sled::Error::Unsupported(String::from("hello"))))
    )]
    fn test_result_tuple_to_entry(
        #[case] input: result::Result<(IVec, IVec), sled::Error>,
        #[case] expected: Result<Entry>,
    ) {
        let actual = result_tuple_to_entry(input);
        if expected.is_ok() {
            let actual = actual.unwrap();
            let expected = expected.unwrap();

            assert_eq!(expected, actual);
        } else {
            assert!(actual.is_err());
        }
    }

    #[rstest]
    #[case::error(
        Err(sled::Error::Unsupported(String::from("hello"))),
        Err(Error::from(sled::Error::Unsupported(String::from("hello"))))
    )]
    #[case::valid(
        Ok((IVec::from(&1u64.to_be_bytes()), IVec::default())),
        Ok(1)
    )]
    #[should_panic]
    #[case::invalid(
        Ok((IVec::default(), IVec::default())),
        Err(Error::from(sled::Error::Unsupported(String::from("hello"))))
    )]
    fn test_result_tuple_to_key(
        #[case] input: result::Result<(IVec, IVec), sled::Error>,
        #[case] expected: Result<u64>,
    ) {
        let actual = result_tuple_to_key(input);
        if expected.is_ok() {
            let actual = actual.unwrap();
            let expected = expected.unwrap();

            assert_eq!(expected, actual);
        } else {
            assert!(actual.is_err());
        }
    }

    #[rstest]
    #[case::error(
        Err(sled::Error::Unsupported(String::from("hello"))),
        Err(Error::from(sled::Error::Unsupported(String::from("hello"))))
    )]
    #[case::valid(
        Ok((IVec::from(&1u64.to_be_bytes()), Entry::Registration(1).to_ivec().unwrap())),
        Ok((1, Entry::Registration(1)))
    )]
    #[should_panic]
    #[case::invalid_entry(
        Ok((IVec::from(&1u64.to_be_bytes()), IVec::default())),
        Err(Error::from(sled::Error::Unsupported(String::from("hello"))))
    )]
    #[should_panic]
    #[case::invalid_key(
        Ok((IVec::default(), Entry::Registration(1).to_ivec().unwrap())),
        Err(Error::from(sled::Error::Unsupported(String::from("hello"))))
    )]
    fn test_result_tuple_to_key_entry(
        #[case] input: result::Result<(IVec, IVec), sled::Error>,
        #[case] expected: Result<(u64, Entry)>,
    ) {
        let actual = result_tuple_to_key_entry(input);
        if expected.is_ok() {
            let actual = actual.unwrap();
            let expected = expected.unwrap();

            assert_eq!(expected, actual);
        } else {
            assert!(actual.is_err());
        }
    }
}
