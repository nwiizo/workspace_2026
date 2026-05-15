use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use openraft::error::{InstallSnapshotError, NetworkError, RPCError, RaftError};
use openraft::network::RPCOption;
use openraft::raft::{
    AppendEntriesRequest, AppendEntriesResponse, InstallSnapshotRequest, InstallSnapshotResponse,
    VoteRequest, VoteResponse,
};
use openraft::{RaftNetwork, RaftNetworkFactory};
use raft_proxy_core::{Node, NodeId, TypeConfig};
use reqwest::header::CONTENT_TYPE;
use serde::Serialize;
use serde::de::DeserializeOwned;
use tracing::debug;

use crate::WireError;

#[derive(Clone, Default)]
pub struct PeerRegistry {
    inner: Arc<parking_lot::RwLock<HashMap<NodeId, String>>>,
}

impl PeerRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&self, id: NodeId, base_url: String) {
        self.inner.write().insert(id, normalize_base_url(&base_url));
    }

    pub fn remove(&self, id: NodeId) {
        self.inner.write().remove(&id);
    }

    pub fn get(&self, id: NodeId) -> Option<String> {
        self.inner.read().get(&id).cloned()
    }

    pub fn snapshot(&self) -> HashMap<NodeId, String> {
        self.inner.read().clone()
    }
}

pub struct HttpNetworkFactory {
    peers: PeerRegistry,
    client: reqwest::Client,
}

impl HttpNetworkFactory {
    pub fn new(peers: PeerRegistry) -> Result<Self, WireError> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()?;
        Ok(Self { peers, client })
    }
}

impl RaftNetworkFactory<TypeConfig> for HttpNetworkFactory {
    type Network = HttpNetwork;

    async fn new_client(&mut self, target: NodeId, node: &Node) -> Self::Network {
        let base_url = if node.rpc_addr.is_empty() {
            self.peers.get(target).unwrap_or_default()
        } else {
            normalize_base_url(&node.rpc_addr)
        };

        debug!(target, base_url, "created raft http network client");

        HttpNetwork {
            target,
            base_url,
            client: self.client.clone(),
        }
    }
}

pub struct HttpNetwork {
    target: NodeId,
    base_url: String,
    client: reqwest::Client,
}

impl HttpNetwork {
    async fn post_json<Req, Resp>(&self, path: &str, rpc: &Req) -> Result<Resp, WireError>
    where
        Req: Serialize,
        Resp: DeserializeOwned,
    {
        let url = format!("{}{}", self.base_url, path);
        debug!(target = self.target, path, "sending raft rpc");
        let body = serde_json::to_vec(rpc)?;
        let response = self
            .client
            .post(url)
            .header(CONTENT_TYPE, "application/json")
            .body(body)
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            return Err(WireError::Status(status));
        }

        let body = response.bytes().await?;
        Ok(serde_json::from_slice(&body)?)
    }
}

impl RaftNetwork<TypeConfig> for HttpNetwork {
    async fn append_entries(
        &mut self,
        rpc: AppendEntriesRequest<TypeConfig>,
        _option: RPCOption,
    ) -> Result<AppendEntriesResponse<NodeId>, RPCError<NodeId, Node, RaftError<NodeId>>> {
        self.post_json("/raft/append", &rpc)
            .await
            .map_err(rpc_network_error)
    }

    async fn install_snapshot(
        &mut self,
        rpc: InstallSnapshotRequest<TypeConfig>,
        _option: RPCOption,
    ) -> Result<
        InstallSnapshotResponse<NodeId>,
        RPCError<NodeId, Node, RaftError<NodeId, InstallSnapshotError>>,
    > {
        self.post_json("/raft/snapshot", &rpc)
            .await
            .map_err(rpc_network_error)
    }

    async fn vote(
        &mut self,
        rpc: VoteRequest<NodeId>,
        _option: RPCOption,
    ) -> Result<VoteResponse<NodeId>, RPCError<NodeId, Node, RaftError<NodeId>>> {
        self.post_json("/raft/vote", &rpc)
            .await
            .map_err(rpc_network_error)
    }
}

fn rpc_network_error<E>(err: WireError) -> RPCError<NodeId, Node, RaftError<NodeId, E>>
where
    E: std::error::Error,
{
    RPCError::Network(NetworkError::new(&err))
}

pub fn normalize_base_url(base_url: &str) -> String {
    let trimmed = base_url.trim().trim_end_matches('/');
    if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        trimmed.to_owned()
    } else {
        format!("http://{trimmed}")
    }
}

#[cfg(test)]
mod tests {
    use super::PeerRegistry;

    #[tokio::test]
    async fn peer_registry_snapshot_tracks_insert_and_remove() {
        let peers = PeerRegistry::new();
        peers.insert(1, "http://127.0.0.1:9080".to_owned());
        peers.insert(2, "127.0.0.1:9081".to_owned());

        let snapshot = peers.snapshot();
        assert_eq!(snapshot.len(), 2);
        assert_eq!(snapshot.get(&1), Some(&"http://127.0.0.1:9080".to_owned()));
        assert_eq!(snapshot.get(&2), Some(&"http://127.0.0.1:9081".to_owned()));

        peers.remove(1);

        let snapshot = peers.snapshot();
        assert_eq!(snapshot.len(), 1);
        assert_eq!(snapshot.get(&2), Some(&"http://127.0.0.1:9081".to_owned()));
    }
}
