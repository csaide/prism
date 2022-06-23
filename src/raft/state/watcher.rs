// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

use std::{collections::HashMap, sync::Mutex};

use tokio::sync::oneshot;

use super::Result;

type LockedUintMap<V> = Mutex<HashMap<u128, V>>;
type Bytes = Vec<u8>;

#[derive(Debug, Default)]
pub struct Watcher {
    command_watches: LockedUintMap<oneshot::Sender<Result<Bytes>>>,
    cluster_config_watches: LockedUintMap<oneshot::Sender<()>>,
    registration_watches: LockedUintMap<oneshot::Sender<()>>,
}

impl Watcher {
    pub fn register_command_watch(&self, idx: u128) -> oneshot::Receiver<Result<Bytes>> {
        let (tx, rx) = oneshot::channel();
        self.command_watches.lock().unwrap().insert(idx, tx);
        rx
    }

    pub fn command_applied(&self, idx: u128, result: Result<Bytes>) {
        match self.command_watches.lock().unwrap().remove(&idx) {
            Some(submit_tx) => {
                // If we have an error here its because the receiver hung up.
                // In that case there is nothing for us to do anyway, so just retun.
                let _ = submit_tx.send(result);
            }
            None => {}
        }
    }

    pub fn register_registration_watch(&self, idx: u128) -> oneshot::Receiver<()> {
        let (tx, rx) = oneshot::channel();
        self.registration_watches.lock().unwrap().insert(idx, tx);
        rx
    }

    pub fn registration_applied(&self, idx: u128) {
        match self.registration_watches.lock().unwrap().remove(&idx) {
            Some(submit_tx) => {
                let _ = submit_tx.send(());
            }
            None => {}
        }
    }

    pub fn register_cluster_config_watch(&self, idx: u128) -> oneshot::Receiver<()> {
        let (tx, rx) = oneshot::channel();
        self.cluster_config_watches.lock().unwrap().insert(idx, tx);
        rx
    }

    pub fn cluster_config_applied(&self, idx: u128) {
        match self.cluster_config_watches.lock().unwrap().remove(&idx) {
            Some(submit_tx) => {
                let _ = submit_tx.send(());
            }
            None => {}
        }
    }
}
