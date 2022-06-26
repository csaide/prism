// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

use std::sync::Arc;

use tokio::sync::mpsc::Sender;

use super::{
    Command, Entry, Error, Log, MutateStateRequest, MutateStateResponse, ReadStateRequest,
    ReadStateResponse, RegisterClientRequest, RegisterClientResponse, Registration, Result, State,
    Watcher, NOT_LEADER, OK,
};

#[tonic::async_trait]
pub trait FrontendHandler: Send + Sync + Clone + 'static {
    async fn mutate_request(
        &self,
        mutate_request: MutateStateRequest,
    ) -> Result<MutateStateResponse>;
    async fn read_request(&self, read_request: ReadStateRequest) -> Result<ReadStateResponse>;
    async fn register_client(
        &self,
        register_request: RegisterClientRequest,
    ) -> Result<RegisterClientResponse>;
}

#[derive(Debug, Clone)]
pub struct Frontend<P> {
    state: Arc<State<P>>,
    log: Arc<Log>,
    watcher: Arc<Watcher>,
    submit_tx: Sender<()>,
}

impl<P> Frontend<P> {
    pub fn new(
        state: Arc<State<P>>,
        log: Arc<Log>,
        watcher: Arc<Watcher>,
        submit_tx: Sender<()>,
    ) -> Frontend<P> {
        Frontend {
            state,
            log,
            watcher,
            submit_tx,
        }
    }

    async fn submit_command(&self, command: Vec<u8>) -> Result<Vec<u8>> {
        let current_term = self.state.get_current_term();
        let idx = self.log.append(Entry::Command(Command {
            term: current_term,
            data: command,
        }))?;

        let rx = self.watcher.register_command_watch(idx);
        self.submit_tx.send(()).await?;

        let res = rx.await.map_err(Error::from)?;
        res
    }

    async fn submit_registration(&self, reg: Registration) -> Result<u128> {
        let idx = self.log.append(Entry::Registration(reg))?;

        let rx = self.watcher.register_registration_watch(idx);
        self.submit_tx.send(()).await?;
        rx.await?;

        Ok(idx)
    }

    pub async fn register_client(
        &self,
        _: RegisterClientRequest,
    ) -> Result<RegisterClientResponse> {
        if self.state.is_dead() {
            return Err(Error::Dead);
        }
        if !self.state.is_leader() {
            return Ok(RegisterClientResponse {
                client_id: Vec::default(),
                leader_hint: self.state.current_leader().unwrap_or_default(),
                status: NOT_LEADER.to_string(),
            });
        }

        let term = self.state.get_current_term();
        let reg = Registration { term };

        let idx = self.submit_registration(reg).await?;

        Ok(RegisterClientResponse {
            client_id: idx.to_be_bytes().to_vec(),
            leader_hint: self.state.id.clone(),
            status: OK.to_string(),
        })
    }

    pub async fn mutate_request(
        &self,
        mutate_request: MutateStateRequest,
    ) -> Result<MutateStateResponse> {
        if self.state.is_dead() {
            return Err(Error::Dead);
        }
        if !self.state.is_leader() {
            return Ok(MutateStateResponse {
                response: Vec::default(),
                leader_hint: self.state.current_leader().unwrap_or_default(),
                status: NOT_LEADER.to_string(),
            });
        }

        let res = self.submit_command(mutate_request.command).await?;
        Ok(MutateStateResponse {
            leader_hint: self.state.id.clone(),
            response: res,
            status: OK.to_string(),
        })
    }

    pub async fn read_request(&self, _read_request: ReadStateRequest) -> Result<ReadStateResponse> {
        unimplemented!()
    }
}

#[tonic::async_trait]
impl<P: Send + Sync + Clone + 'static> FrontendHandler for Frontend<P> {
    async fn mutate_request(
        &self,
        mutate_request: MutateStateRequest,
    ) -> Result<MutateStateResponse> {
        self.mutate_request(mutate_request).await
    }
    async fn read_request(&self, read_request: ReadStateRequest) -> Result<ReadStateResponse> {
        self.read_request(read_request).await
    }
    async fn register_client(
        &self,
        register_request: RegisterClientRequest,
    ) -> Result<RegisterClientResponse> {
        self.register_client(register_request).await
    }
}
