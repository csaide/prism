// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd)]
pub enum Mode {
    Leader,
    Follower,
    Candidate,
    Dead,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mode() {
        let mode = Mode::Leader;
        let mode_str = format!("{:?}", mode);
        assert_eq!(mode_str, "Leader");

        let cloned = mode.clone();
        assert_eq!(cloned, mode);

        let dead = Mode::Dead;
        assert!(dead > mode);
    }
}
