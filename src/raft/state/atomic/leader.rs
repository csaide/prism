// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

use std::sync::RwLock;

#[derive(Debug)]
pub struct AtomicLeader {
    inner: RwLock<Option<String>>,
}

impl AtomicLeader {
    pub fn new() -> AtomicLeader {
        AtomicLeader {
            inner: RwLock::new(None),
        }
    }

    pub fn saw_leader(&self, leader_id: String) {
        *self.inner.write().unwrap() = Some(leader_id);
    }

    pub fn lost_leader(&self) {
        *self.inner.write().unwrap() = None;
    }

    pub fn have_leader(&self) -> bool {
        self.inner.read().unwrap().is_some()
    }

    pub fn leader(&self) -> Option<String> {
        self.inner.read().unwrap().clone()
    }
}

impl Default for AtomicLeader {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_atomic_leader() {
        let leader = AtomicLeader::default();

        assert!(!leader.have_leader());
        assert!(matches!(leader.leader(), None));

        leader.saw_leader(String::from("leader"));
        assert!(leader.have_leader());
        assert!(matches!(leader.leader(), Some(..)));
        assert_eq!(leader.leader().unwrap(), "leader")
    }
}
