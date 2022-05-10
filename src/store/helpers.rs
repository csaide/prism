// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

use rkyv::{
    de::deserializers::SharedDeserializeMap, ser::serializers::AllocSerializer, ser::Serializer,
    AlignedVec, Archive, Deserialize, Serialize,
};
use sled::IVec;

use super::{Error, Key, Result};

pub fn to_bytes<T, const N: usize>(value: &T) -> Result<AlignedVec>
where
    T: Archive,
    T: Serialize<AllocSerializer<N>>,
{
    let mut serializer = AllocSerializer::<N>::default();
    serializer.serialize_value(value).map_err(Error::from)?;
    Ok(serializer.into_serializer().into_inner())
}

pub fn from_ivec<T>(ivec: IVec) -> Result<T>
where
    T: Archive,
    <T as Archive>::Archived: Deserialize<T, SharedDeserializeMap>,
{
    let bytes = ivec.as_ref();
    let mut aligned = AlignedVec::with_capacity(bytes.len());
    aligned.extend_from_slice(bytes);

    let archived = unsafe { rkyv::archived_root::<T>(&aligned) };
    let deserialized = archived
        .deserialize(&mut SharedDeserializeMap::default())
        .map_err(Error::from)?;
    Ok(deserialized)
}

pub fn from_ivec_tuple<K, V>(tuple: (IVec, IVec)) -> Result<(K, V)>
where
    V: Archive,
    <V as Archive>::Archived: Deserialize<V, SharedDeserializeMap>,
    K: Key,
{
    let key = K::from_bytes(&tuple.0)?;
    match from_ivec(tuple.1) {
        Ok(value) => Ok((key, value)),
        Err(e) => Err(e),
    }
}
