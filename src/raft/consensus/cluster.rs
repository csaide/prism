// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

use std::sync::Arc;

use tokio::sync::mpsc::Sender;

use super::{
    AddServerRequest, AddServerResponse, Client, ClusterConfig, Entry, Error, ListServerRequest,
    ListServerResponse, Log, RemoveServerRequest, RemoveServerResponse, Result, State, Syncer,
    Watcher,
};

#[tonic::async_trait]
pub trait ClusterHandler<P>: Send + Sync + Clone + 'static {
    async fn add_server(&self, add_request: AddServerRequest<P>) -> Result<AddServerResponse>;
    async fn remove_server(
        &self,
        remove_request: RemoveServerRequest,
    ) -> Result<RemoveServerResponse>;
    async fn list_servers(&self, list_request: ListServerRequest) -> Result<ListServerResponse>;
}

#[derive(Debug, Clone)]
pub struct Cluster<P> {
    logger: slog::Logger,
    state: Arc<State<P>>,
    log: Arc<Log>,
    watcher: Arc<Watcher>,
    submit_tx: Sender<()>,
}

impl<P> Cluster<P>
where
    P: Client + Send + Clone + 'static,
{
    pub fn new(
        logger: &slog::Logger,
        state: Arc<State<P>>,
        log: Arc<Log>,
        watcher: Arc<Watcher>,
        submit_tx: Sender<()>,
    ) -> Cluster<P> {
        Cluster {
            logger: logger.new(o!("module" => "consensus-cluster")),
            state,
            log,
            watcher,
            submit_tx,
        }
    }

    async fn submit_cluster_config(&self, cfg: ClusterConfig) -> Result<()> {
        if self.state.is_dead() {
            return Err(Error::Dead);
        }
        if !self.state.is_leader() {
            return Err(Error::InvalidMode);
        }
        let last_cluster_config_idx = self.log.append(Entry::ClusterConfig(cfg))?;

        self.state
            .set_last_cluster_config_idx(last_cluster_config_idx);

        let rx = self
            .watcher
            .register_cluster_config_watch(last_cluster_config_idx);

        self.submit_tx.send(()).await?;
        rx.await.map_err(Error::from)
    }

    pub async fn add_member(
        &self,
        server_request: AddServerRequest<P>,
    ) -> Result<AddServerResponse> {
        if self.state.is_dead() {
            return Err(Error::Dead);
        }
        if !self.state.is_leader() {
            return Err(Error::InvalidMode);
        }

        Syncer::new(
            &self.logger,
            self.state.clone(),
            self.log.clone(),
            server_request.peer.clone(),
        )
        .exec()
        .await;

        let cfg = {
            let mut locked_peers = self.state.peers.lock();
            locked_peers.append(server_request.id, server_request.peer);

            locked_peers.to_cluster_config(self.state.get_current_term())
        };

        self.submit_cluster_config(cfg).await?;

        Ok(AddServerResponse {
            leader_hint: self.state.id.clone(),
            status: "OK".to_string(),
        })
    }

    pub async fn remove_member(
        &self,
        server_request: RemoveServerRequest,
    ) -> Result<RemoveServerResponse> {
        if self.state.is_dead() {
            return Err(Error::Dead);
        }
        if !self.state.is_leader() {
            return Err(Error::InvalidMode);
        }

        let cfg = {
            let mut locked_peers = self.state.peers.lock();
            locked_peers.remove(&server_request.id);
            locked_peers.to_cluster_config(self.state.get_current_term())
        };

        self.submit_cluster_config(cfg).await?;

        Ok(RemoveServerResponse {
            leader_hint: self.state.id.clone(),
            status: "OK".to_string(),
        })
    }

    pub async fn list_members(&self, _: ListServerRequest) -> Result<ListServerResponse> {
        if self.state.is_dead() {
            return Err(Error::Dead);
        }
        if !self.state.is_leader() {
            return Err(Error::InvalidMode);
        }

        let term = self.state.get_current_term();
        Ok(self.state.peers.lock().to_list_response(term))
    }
}

#[tonic::async_trait]
impl<P> ClusterHandler<P> for Cluster<P>
where
    P: Client + Send + Clone + 'static,
{
    async fn add_server(&self, add_request: AddServerRequest<P>) -> Result<AddServerResponse> {
        self.add_member(add_request).await
    }
    async fn remove_server(
        &self,
        remove_request: RemoveServerRequest,
    ) -> Result<RemoveServerResponse> {
        self.remove_member(remove_request).await
    }
    async fn list_servers(&self, list_request: ListServerRequest) -> Result<ListServerResponse> {
        self.list_members(list_request).await
    }
}
