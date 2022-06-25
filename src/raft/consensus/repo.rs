// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

use super::{Cluster, Frontend, Raft};

pub trait Repository<P>: Send + Sync + 'static {
    fn get_raft(&self) -> Raft<P>;
    fn get_frontend(&self) -> Frontend<P>;
    fn get_cluster(&self) -> Cluster<P>;
}

#[derive(Debug)]
pub struct Repo<P> {
    raft: Raft<P>,
    frontend: Frontend<P>,
    cluster: Cluster<P>,
}

impl<P> Repo<P>
where
    P: Clone,
{
    pub fn new(raft: Raft<P>, frontend: Frontend<P>, cluster: Cluster<P>) -> Repo<P> {
        Repo {
            raft,
            frontend,
            cluster,
        }
    }

    pub fn get_raft(&self) -> Raft<P> {
        self.raft.clone()
    }
    pub fn get_frontend(&self) -> Frontend<P> {
        self.frontend.clone()
    }
    pub fn get_cluster(&self) -> Cluster<P> {
        self.cluster.clone()
    }
}

impl<P> Repository<P> for Repo<P>
where
    P: Send + Clone + Sync + 'static,
{
    fn get_raft(&self) -> Raft<P> {
        self.raft.clone()
    }
    fn get_frontend(&self) -> Frontend<P> {
        self.frontend.clone()
    }
    fn get_cluster(&self) -> Cluster<P> {
        self.cluster.clone()
    }
}
