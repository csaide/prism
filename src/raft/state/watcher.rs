// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

use std::{collections::HashMap, sync::Mutex};

use tokio::sync::oneshot;

use super::Result;

type LockedUintMap<V> = Mutex<HashMap<u64, V>>;
type Bytes = Vec<u8>;

#[derive(Debug, Default)]
pub struct Watcher {
    command_watches: LockedUintMap<oneshot::Sender<Result<Bytes>>>,
    cluster_config_watches: LockedUintMap<oneshot::Sender<()>>,
    registration_watches: LockedUintMap<oneshot::Sender<()>>,
}

impl Watcher {
    pub fn register_command_watch(&self, idx: u64) -> oneshot::Receiver<Result<Bytes>> {
        let (tx, rx) = oneshot::channel();
        self.command_watches.lock().unwrap().insert(idx, tx);
        rx
    }

    pub fn command_applied(&self, idx: u64, result: Result<Bytes>) {
        if let Some(submit_tx) = self.command_watches.lock().unwrap().remove(&idx) {
            // If we have an error here its because the receiver hung up.
            // In that case there is nothing for us to do anyway, so just return.
            let _ = submit_tx.send(result);
        }
    }

    pub fn register_registration_watch(&self, idx: u64) -> oneshot::Receiver<()> {
        let (tx, rx) = oneshot::channel();
        self.registration_watches.lock().unwrap().insert(idx, tx);
        rx
    }

    pub fn registration_applied(&self, idx: u64) {
        if let Some(submit_tx) = self.registration_watches.lock().unwrap().remove(&idx) {
            // If we have an error here its because the receiver hung up.
            // In that case there is nothing for us to do anyway, so just return.
            let _ = submit_tx.send(());
        }
    }

    pub fn register_cluster_config_watch(&self, idx: u64) -> oneshot::Receiver<()> {
        let (tx, rx) = oneshot::channel();
        self.cluster_config_watches.lock().unwrap().insert(idx, tx);
        rx
    }

    pub fn cluster_config_applied(&self, idx: u64) {
        if let Some(submit_tx) = self.cluster_config_watches.lock().unwrap().remove(&idx) {
            // If we have an error here its because the receiver hung up.
            // In that case there is nothing for us to do anyway, so just return.
            let _ = submit_tx.send(());
        }
    }
}

#[cfg(test)]
mod tests {
    use tokio_test::block_on as wait;

    use super::*;

    #[test]
    fn test_register_command() {
        let watch = Watcher::default();

        let rx = watch.register_command_watch(1);
        watch.command_applied(1, Ok(Vec::default()));
        watch.command_applied(2, Ok(Vec::default()));

        let resp = wait(rx);
        assert!(resp.is_ok());
    }

    #[test]
    fn test_register_registration() {
        let watch = Watcher::default();

        let rx = watch.register_registration_watch(1);
        watch.registration_applied(1);
        watch.registration_applied(2);

        let resp = wait(rx);
        assert!(resp.is_ok());
    }

    #[test]
    fn test_register_cluster_cfg() {
        let watch = Watcher::default();

        let rx = watch.register_cluster_config_watch(1);
        watch.cluster_config_applied(1);
        watch.cluster_config_applied(2);

        let resp = wait(rx);
        assert!(resp.is_ok());
    }
}
