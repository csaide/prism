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
        *self.inner.write().unwrap() = mode
    }
}

impl Default for AtomicMode {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case::candidate(Mode::Candidate)]
    #[case::dead(Mode::Dead)]
    #[case::follower(Mode::Follower)]
    #[case::leader(Mode::Leader)]
    fn test_atomic_mode(#[case] input: Mode) {
        let mode = AtomicMode::default();
        assert!(mode.is_follower());

        mode.set(input.clone());
        match input {
            Mode::Candidate => assert!(mode.is_candidate()),
            Mode::Dead => assert!(mode.is_dead()),
            Mode::Follower => assert!(mode.is_follower()),
            Mode::Leader => assert!(mode.is_leader()),
        }
    }
}
