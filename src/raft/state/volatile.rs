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
