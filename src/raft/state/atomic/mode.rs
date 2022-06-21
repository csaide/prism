// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

use std::sync::RwLock;

use super::Mode;

#[derive(Debug)]
pub struct AtomicMode {
    inner: RwLock<Mode>,
}

impl AtomicMode {
    pub fn new() -> AtomicMode {
        AtomicMode {
            inner: RwLock::new(Mode::Follower),
        }
    }

    pub fn is_leader(&self) -> bool {
        matches!(*self.inner.read().unwrap(), Mode::Leader)
    }

    pub fn is_candidate(&self) -> bool {
        matches!(*self.inner.read().unwrap(), Mode::Candidate)
    }

    pub fn is_follower(&self) -> bool {
        matches!(*self.inner.read().unwrap(), Mode::Follower)
    }

    pub fn is_dead(&self) -> bool {
        matches!(*self.inner.read().unwrap(), Mode::Dead)
    }

    pub fn set(&self, mode: Mode) {
        let mut val = self.inner.write().unwrap();
        *val = mode
    }
}

impl Default for AtomicMode {
    fn default() -> Self {
        Self::new()
    }
}
