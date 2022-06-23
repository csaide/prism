// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

use super::{Command, Result};

pub trait StateMachine: Send + Sync + 'static {
    fn apply(&self, command: Command) -> Result<Vec<u8>>;
}
