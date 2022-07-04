// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

use super::{AtomicIndex, AtomicLeader, AtomicMode};

#[derive(Debug, Default)]
pub struct VolatileState {
    pub mode: AtomicMode,
    pub leader: AtomicLeader,
    pub last_cluster_config_idx: AtomicIndex,
    pub last_applied_idx: AtomicIndex,
    pub commit_idx: AtomicIndex,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_volatile_state() {
        let state = VolatileState::default();
        let state_str = format!("{:?}", state);
        assert_eq!(state_str, "VolatileState { mode: AtomicMode { inner: RwLock { data: Follower, poisoned: false, .. } }, leader: AtomicLeader { inner: RwLock { data: None, poisoned: false, .. } }, last_cluster_config_idx: AtomicIndex { inner: RwLock { data: 0, poisoned: false, .. } }, last_applied_idx: AtomicIndex { inner: RwLock { data: 0, poisoned: false, .. } }, commit_idx: AtomicIndex { inner: RwLock { data: 0, poisoned: false, .. } } }");
    }
}
