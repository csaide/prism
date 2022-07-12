// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

use std::sync::Arc;

use tokio::sync::mpsc::Sender;

use super::{
    AddServerRequest, AddServerResponse, Client, ClusterConfig, Entry, Error, ListServerRequest,
    ListServerResponse, Log, RemoveServerRequest, RemoveServerResponse, Result, State, Syncer,
    Watcher, NOT_LEADER, OK,
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
            return Ok(AddServerResponse {
                leader_hint: self.state.current_leader().unwrap_or_default(),
                status: NOT_LEADER.to_string(),
            });
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
            locked_peers.insert(server_request.id, server_request.peer);

            locked_peers.to_cluster_config(self.state.get_current_term())
        };

        self.submit_cluster_config(cfg).await?;

        Ok(AddServerResponse {
            leader_hint: self.state.id.clone(),
            status: OK.to_string(),
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
            return Ok(RemoveServerResponse {
                leader_hint: self.state.current_leader().unwrap_or_default(),
                status: NOT_LEADER.to_string(),
            });
        }

        let cfg = {
            let mut locked_peers = self.state.peers.lock();
            locked_peers.remove(&server_request.id);
            locked_peers.to_cluster_config(self.state.get_current_term())
        };

        self.submit_cluster_config(cfg).await?;

        Ok(RemoveServerResponse {
            leader_hint: self.state.id.clone(),
            status: OK.to_string(),
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

#[cfg(test)]
pub mod mock {
    use mockall::mock;

    mock! {
        #[derive(Debug)]
        pub ClusterHandler<P> {}

        #[tonic::async_trait]
        impl<P> super::ClusterHandler<P> for ClusterHandler<P>
        where
            P: super::Client + Send + Clone + 'static,
        {
            async fn add_server(&self, add_request: super::AddServerRequest<P>) -> super::Result<super::AddServerResponse>;
            async fn remove_server(
                &self,
                remove_request: super::RemoveServerRequest,
            ) -> super::Result<super::RemoveServerResponse>;
            async fn list_servers(&self, list_request: super::ListServerRequest) -> super::Result<super::ListServerResponse>;
        }

        impl<P> Clone for ClusterHandler<P>
        where
            P: Clone
        {
            fn clone(&self) -> Self;
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, time::Duration};

    use rstest::rstest;
    use tokio::sync::mpsc;
    use tokio_test::block_on as wait;

    use crate::{
        logging,
        raft::{
            cluster::{get_lock, MockClient, MTX},
            AppendEntriesResponse, Mode, Peer,
        },
    };

    use super::*;

    fn mock_client_append_factory() -> MockClient {
        let mut mock_peer = MockClient::new();
        mock_peer
            .expect_clone()
            .returning(mock_client_append_factory);
        mock_peer.expect_append().once().returning(|_| {
            Ok(AppendEntriesResponse {
                term: 1,
                success: true,
            })
        });
        mock_peer
    }

    #[rstest]
    #[case::happy_path(
        Mode::Leader,
        vec![
            (AddServerRequest {
                id: String::from("leader"),
                peer: Peer::voter(String::from("leader")),
                replica: false,
            }, Ok(AddServerResponse {
                leader_hint: String::from("leader"),
                status: OK.to_string(),
            })),
            (AddServerRequest {
                id: String::from("follower"),
                peer: Peer::voter(String::from("follower")),
                replica: false,
            }, Ok(AddServerResponse {
                leader_hint: String::from("leader"),
                status: OK.to_string(),
            }))
        ],
        vec![
            (RemoveServerRequest {
                id: String::from("follower"),
            }, Ok(RemoveServerResponse {
                leader_hint: String::from("leader"),
                status: OK.to_string(),
            }))
        ],
        Some(ListServerResponse{
            leader: String::from("leader"),
            replicas: Vec::default(),
            voters: Vec::default(),
            term: 1,
        }),
        2
    )]
    #[case::dead(
        Mode::Dead,
        vec![
            (AddServerRequest {
                id: String::from("leader"),
                peer: Peer::voter(String::from("leader")),
                replica: false,
            }, Err(Error::Dead))
        ],
        vec![
            (RemoveServerRequest {
                id: String::from("follower"),
            },  Err(Error::Dead))
        ],
        None,
        0,
    )]
    #[case::follower(
        Mode::Follower,
        vec![
            (AddServerRequest {
                id: String::from("leader"),
                peer: Peer::voter(String::from("leader")),
                replica: false,
            }, Ok(AddServerResponse{
                leader_hint: String::from("leader"),
                status: NOT_LEADER.to_string(),
            }))
        ],
        vec![
            (RemoveServerRequest {
                id: String::from("follower"),
            }, Ok(RemoveServerResponse {
                leader_hint: String::from("leader"),
                status: NOT_LEADER.to_string(),
            }))
        ],
        None,
        0,
    )]
    fn test_cluster(
        #[case] mode: Mode,
        #[case] add_reqs: Vec<(AddServerRequest<MockClient>, Result<AddServerResponse>)>,
        #[case] rm_reqs: Vec<(RemoveServerRequest, Result<RemoveServerResponse>)>,
        #[case] expected_list_resp: Option<ListServerResponse>,
        #[case] expected_connects: usize,
    ) {
        let _m = get_lock(&MTX);

        let ctx = MockClient::connect_context();
        ctx.expect()
            .times(expected_connects)
            .returning(|_| Ok(mock_client_append_factory()));

        let logger = logging::noop();
        let id = String::from("leader");
        let db = sled::Config::new()
            .temporary(true)
            .open()
            .expect("Failed to open temp database.");
        let state = Arc::new(
            State::<MockClient>::new(id.clone(), HashMap::default(), &db)
                .expect("Failed to create new State object."),
        );
        state.set_mode(mode);

        let log = Arc::new(Log::new(&db).expect("Failed to create new persistent Log."));
        let watcher = Arc::new(Watcher::default());
        let (tx, _rx) = mpsc::channel(10);
        let handler = Cluster::new(&logger, state, log, watcher.clone(), tx);

        let mut idx: u64 = 1;
        for add_req in add_reqs {
            let watch = watcher.clone();
            std::thread::spawn(move || {
                std::thread::sleep(Duration::from_millis(10));
                watch.cluster_config_applied(idx);
            });
            let resp = wait(handler.add_server(add_req.0));
            let expected = add_req.1;
            if expected.is_ok() {
                assert!(resp.is_ok());
                let resp = resp.unwrap();
                let expected = expected.unwrap();
                assert_eq!(resp.status, expected.status);
            } else {
                assert!(resp.is_err());
            }

            idx += 1;
        }

        for rm_req in rm_reqs {
            let watch = watcher.clone();
            std::thread::spawn(move || {
                std::thread::sleep(Duration::from_millis(10));
                watch.cluster_config_applied(idx);
            });
            let resp = wait(handler.remove_server(rm_req.0));
            let expected = rm_req.1;
            if expected.is_ok() {
                assert!(resp.is_ok());
                let resp = resp.unwrap();
                let expected = expected.unwrap();
                assert_eq!(resp.status, expected.status);
            } else {
                assert!(resp.is_err());
            }
            idx += 1;
        }

        let actual_list_resp = wait(handler.list_servers(ListServerRequest {}));
        assert!(actual_list_resp.is_ok() == expected_list_resp.is_some());
        if actual_list_resp.is_ok() {
            let actual_list_resp = actual_list_resp.unwrap();
            let expected_list_resp = expected_list_resp.unwrap();

            assert_eq!(actual_list_resp.term, expected_list_resp.term);
        }
    }
}
