// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum Mode {
    Leader,
    Follower,
    Candidate,
    Dead,
}
