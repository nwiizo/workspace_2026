use std::sync::Arc;

use arc_swap::ArcSwap;
use axum::Router;
use axum::routing::{delete, get, post};
use openraft::{Config, Raft};
use raft_proxy_core::{NodeId, RouteSideState, RoutingTable, TypeConfig};
use raft_proxy_network::{HttpNetworkFactory, PeerRegistry, normalize_base_url};
use raft_proxy_store::{MemLogStore, StateMachineStore};

use crate::error::ControlError;
use crate::{admin, cluster, raft_rpc};

pub struct ProxyApp {
    pub node_id: NodeId,
    pub raft: Raft<TypeConfig>,
    pub peers: PeerRegistry,
    pub routing: Arc<ArcSwap<RoutingTable>>,
    pub route_side_state: Arc<RouteSideState>,
    pub(crate) self_rpc_addr: String,
}

impl ProxyApp {
    pub async fn bootstrap(
        node_id: NodeId,
        rpc_addr: String,
        peers: PeerRegistry,
    ) -> Result<Self, ControlError> {
        let config = Arc::new(
            Config {
                heartbeat_interval: 250,
                election_timeout_min: 800,
                election_timeout_max: 1200,
                install_snapshot_timeout: 4000,
                max_in_snapshot_log_to_keep: 1000,
                purge_batch_size: 256,
                ..Config::default()
            }
            .validate()
            .map_err(|err| ControlError::RaftBuild(err.to_string()))?,
        );
        let log_store = MemLogStore::new();
        let route_side_state = Arc::new(RouteSideState::new());
        let (state_machine, routing) = StateMachineStore::new(Arc::clone(&route_side_state));
        let self_rpc_addr = normalize_base_url(&rpc_addr);
        peers.insert(node_id, self_rpc_addr.clone());
        let network_factory = HttpNetworkFactory::new(peers.clone())
            .map_err(|err| ControlError::RaftBuild(err.to_string()))?;
        let raft = Raft::new(
            node_id,
            Arc::clone(&config),
            network_factory,
            log_store,
            state_machine,
        )
        .await
        .map_err(|err| ControlError::RaftBuild(err.to_string()))?;

        Ok(Self {
            node_id,
            raft,
            peers,
            routing,
            route_side_state,
            self_rpc_addr,
        })
    }

    pub fn router(self: Arc<Self>) -> Router {
        Router::new()
            .route("/raft/vote", post(raft_rpc::vote))
            .route("/raft/append", post(raft_rpc::append))
            .route("/raft/snapshot", post(raft_rpc::snapshot))
            .route("/admin/routes", get(admin::list).put(admin::put))
            .route("/admin/routes/{host}", delete(admin::delete))
            .route("/cluster/init", post(cluster::init))
            .route("/cluster/add-learner", post(cluster::add_learner))
            .route(
                "/cluster/change-membership",
                post(cluster::change_membership),
            )
            .route("/cluster/metrics", get(cluster::metrics))
            .with_state(self)
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use axum::serve;
    use raft_proxy_network::PeerRegistry;
    use reqwest::StatusCode;
    use serde_json::{Value, json};
    use tokio::net::TcpListener;
    use tokio::sync::oneshot;
    use tokio::time::{sleep, timeout};

    use super::*;

    #[tokio::test]
    async fn single_node_route_crud() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let base_url = format!("http://{addr}");
        let app = Arc::new(
            ProxyApp::bootstrap(1, base_url.clone(), PeerRegistry::new())
                .await
                .unwrap(),
        );
        let router = Arc::clone(&app).router();
        let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
        let server = tokio::spawn(async move {
            serve(listener, router)
                .with_graceful_shutdown(async {
                    let _ = shutdown_rx.await;
                })
                .await
                .unwrap();
        });
        let client = reqwest::Client::new();

        let init = client
            .post(format!("{base_url}/cluster/init"))
            .json(&json!({
                "members": [
                    { "id": 1, "rpc_addr": base_url }
                ]
            }))
            .send()
            .await
            .unwrap();
        assert_eq!(init.status(), StatusCode::OK);

        timeout(Duration::from_secs(3), async {
            loop {
                let metrics: Value = client
                    .get(format!("{base_url}/cluster/metrics"))
                    .send()
                    .await
                    .unwrap()
                    .json()
                    .await
                    .unwrap();

                if metrics.get("current_leader") == Some(&json!(1)) {
                    break;
                }

                sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        .unwrap();

        let put = client
            .put(format!("{base_url}/admin/routes"))
            .json(&json!({
                "host": "x.test",
                "upstreams": [
                    { "addr": "127.0.0.1:9000", "weight": 1 }
                ]
            }))
            .send()
            .await
            .unwrap();
        assert_eq!(put.status(), StatusCode::OK);

        let routes: Value = client
            .get(format!("{base_url}/admin/routes"))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        assert_eq!(
            routes["routes"]["x.test"][0],
            json!({ "addr": "127.0.0.1:9000", "weight": 1 })
        );

        let delete: Value = client
            .delete(format!("{base_url}/admin/routes/x.test"))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        assert_eq!(delete, json!({ "found": true }));

        let routes: Value = client
            .get(format!("{base_url}/admin/routes"))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        assert_eq!(routes["routes"], json!({}));

        shutdown_tx.send(()).unwrap();
        server.await.unwrap();
    }
}
