// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

use super::{serde::Serializeable, Result};

pub enum Event<K, V> {
    Insert { key: K, value: V },
    Remove { key: K },
}

impl<K, V> Event<K, V>
where
    K: Serializeable,
    V: Serializeable,
{
    pub fn from_sled(ev: sled::Event) -> Result<Event<K, V>> {
        use sled::Event::*;
        match ev {
            Insert { key, value } => Ok(Event::Insert {
                key: K::from_raw(key.as_ref())?,
                value: V::from_raw(value.as_ref())?,
            }),
            Remove { key } => Ok(Event::Remove {
                key: K::from_raw(key.as_ref())?,
            }),
        }
    }

    pub fn key(&self) -> &K {
        use self::Event::*;
        match self {
            Insert { key, .. } => key,
            Remove { key } => key,
        }
    }
}
