// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

// stdlib usings
use std::result;

// extern usings
use slog::Drain;

/// Wraps a standard slog Drain so that we can filter the messages
/// logged by the defined log handler.
pub struct LevelFilter<D> {
    pub drain: D,
    pub level: slog::Level,
}

impl<D> Drain for LevelFilter<D>
where
    D: Drain,
{
    type Err = Option<D::Err>;
    type Ok = Option<D::Ok>;

    /// Handles actually filtering the log messages.
    fn log(
        &self,
        record: &slog::Record,
        values: &slog::OwnedKVList,
    ) -> result::Result<Self::Ok, Self::Err> {
        if record.level().is_at_least(self.level) {
            self.drain.log(record, values).map(Some).map_err(Some)
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
#[cfg(not(tarpaulin_include))]
mod tests {
    use super::*;

    #[test]
    fn test_filter() {
        let drain = slog::Discard {};
        let filter = LevelFilter {
            drain,
            level: slog::Level::Info,
        }
        .fuse();
        let logger = slog::Logger::root(filter, o!());

        info!(&logger, "Info");
        debug!(&logger, "Debug");
    }
}
