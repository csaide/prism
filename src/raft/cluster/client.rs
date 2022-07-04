// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

use super::{
    AppendEntriesRequest, AppendEntriesResponse, RequestVoteRequest, RequestVoteResponse, Result,
};

#[tonic::async_trait]
pub trait Client: Send + Sync + 'static + Sized {
    async fn connect(addr: String) -> Result<Self>;
    async fn vote(&mut self, req: RequestVoteRequest) -> Result<RequestVoteResponse>;
    async fn append(&mut self, req: AppendEntriesRequest) -> Result<AppendEntriesResponse>;
}

#[cfg(test)]
pub mod mock {
    use lazy_static::lazy_static;
    use mockall::mock;
    use std::sync::{Mutex, MutexGuard};

    lazy_static! {
        pub static ref MTX: Mutex<()> = Mutex::new(());
    }

    #[cfg_attr(coverage_nightly, no_coverage)]
    pub fn get_lock(m: &'static Mutex<()>) -> MutexGuard<'static, ()> {
        match m.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        }
    }

    mock! {
        #[derive(Debug)]
        pub Client { }

        #[tonic::async_trait]
        impl super::Client for Client {
            async fn connect(addr: String) -> super::Result<Self>;
            async fn vote(&mut self, req: super::RequestVoteRequest) -> super::Result<super::RequestVoteResponse>;
            async fn append(&mut self, req: super::AppendEntriesRequest) -> super::Result<super::AppendEntriesResponse>;
        }

        impl Clone for Client {
            fn clone(&self) -> Self;
        }
    }
}
