use std::io::Cursor;
use std::sync::Arc;

use arc_swap::ArcSwap;
use openraft::storage::RaftStateMachine;
use openraft::{
    Entry, EntryPayload, ErrorSubject, ErrorVerb, LogId, OptionalSend, RaftSnapshotBuilder,
    Snapshot, SnapshotMeta, StorageError, StorageIOError, StoredMembership,
};
use raft_proxy_core::{
    AppRequest, AppResponse, Node, NodeId, RouteSideState, RoutingTable, TypeConfig,
};
use thiserror::Error;
use tokio::sync::RwLock;

#[derive(Clone, Debug)]
pub struct StoredSnapshot {
    pub meta: SnapshotMeta<NodeId, Node>,
    pub data: Vec<u8>,
}

#[derive(Clone, Debug)]
pub struct StateMachineStore {
    routing: Arc<ArcSwap<RoutingTable>>,
    route_side_state: Arc<RouteSideState>,
    inner: Arc<RwLock<StateMachineInner>>,
}

#[derive(Clone, Debug, Default)]
struct StateMachineInner {
    last_applied: Option<LogId<NodeId>>,
    last_membership: StoredMembership<NodeId, Node>,
    snapshot_idx: u64,
    current_snapshot: Option<StoredSnapshot>,
}

#[derive(Debug, Error)]
enum StoreError {
    #[error("failed to serialize routing table snapshot: {0}")]
    SerializeSnapshot(#[source] serde_json::Error),

    #[error("failed to deserialize routing table snapshot: {0}")]
    DeserializeSnapshot(#[source] serde_json::Error),
}

impl StateMachineStore {
    /// Shares route-side state with the data plane; applying `DeleteRoute`
    /// removes the host's round-robin counter from that shared state.
    pub fn new(route_side_state: Arc<RouteSideState>) -> (Self, Arc<ArcSwap<RoutingTable>>) {
        let routing = Arc::new(ArcSwap::from_pointee(RoutingTable::new()));
        let me = Self {
            routing: Arc::clone(&routing),
            route_side_state,
            inner: Arc::new(RwLock::new(StateMachineInner::default())),
        };
        (me, routing)
    }
}

impl Default for StateMachineStore {
    fn default() -> Self {
        Self::new(Arc::new(RouteSideState::new())).0
    }
}

impl RaftStateMachine<TypeConfig> for StateMachineStore {
    type SnapshotBuilder = Self;

    async fn applied_state(
        &mut self,
    ) -> Result<(Option<LogId<NodeId>>, StoredMembership<NodeId, Node>), StorageError<NodeId>> {
        let inner = self.inner.read().await;
        Ok((inner.last_applied, inner.last_membership.clone()))
    }

    async fn apply<I>(&mut self, entries: I) -> Result<Vec<AppResponse>, StorageError<NodeId>>
    where
        I: IntoIterator<Item = Entry<TypeConfig>> + OptionalSend,
        I::IntoIter: OptionalSend,
    {
        let mut updated_tbl = self.routing.load_full().as_ref().clone();
        let mut responses = Vec::new();
        let mut last_applied = None;
        let mut last_membership = None;
        let mut deleted_hosts = Vec::new();
        let mut routes_changed = false;

        for entry in entries {
            match entry.payload {
                EntryPayload::Blank => {
                    responses.push(AppResponse::Ok);
                }
                EntryPayload::Normal(AppRequest::PutRoute { host, upstreams }) => {
                    updated_tbl.insert(host, upstreams);
                    routes_changed = true;
                    responses.push(AppResponse::Ok);
                }
                EntryPayload::Normal(AppRequest::DeleteRoute { host }) => {
                    let removed = updated_tbl.remove(&host);
                    if removed {
                        routes_changed = true;
                        deleted_hosts.push(host);
                    }
                    responses.push(if removed {
                        AppResponse::Ok
                    } else {
                        AppResponse::NotFound
                    });
                }
                EntryPayload::Membership(membership) => {
                    last_membership = Some(StoredMembership::new(Some(entry.log_id), membership));
                    responses.push(AppResponse::Ok);
                }
            }

            last_applied = Some(entry.log_id);
        }

        if last_applied.is_some() {
            let mut inner = self.inner.write().await;
            if routes_changed {
                self.routing.store(Arc::new(updated_tbl));
            }
            inner.last_applied = last_applied;
            if let Some(membership) = last_membership {
                inner.last_membership = membership;
            }
            for host in deleted_hosts {
                self.route_side_state.remove_host(&host);
            }
        }

        Ok(responses)
    }

    async fn get_snapshot_builder(&mut self) -> Self::SnapshotBuilder {
        Self {
            routing: Arc::clone(&self.routing),
            route_side_state: Arc::clone(&self.route_side_state),
            inner: Arc::clone(&self.inner),
        }
    }

    async fn begin_receiving_snapshot(
        &mut self,
    ) -> Result<Box<Cursor<Vec<u8>>>, StorageError<NodeId>> {
        Ok(Box::new(Cursor::new(Vec::new())))
    }

    async fn install_snapshot(
        &mut self,
        meta: &SnapshotMeta<NodeId, Node>,
        snapshot: Box<Cursor<Vec<u8>>>,
    ) -> Result<(), StorageError<NodeId>> {
        let data = (*snapshot).into_inner();
        let restored: RoutingTable = serde_json::from_slice(&data)
            .map_err(StoreError::DeserializeSnapshot)
            .map_err(|err| {
                storage_error(
                    ErrorSubject::Snapshot(Some(meta.signature())),
                    ErrorVerb::Read,
                    err,
                )
            })?;

        self.routing.store(Arc::new(restored));

        let mut inner = self.inner.write().await;
        inner.last_applied = meta.last_log_id;
        inner.last_membership = meta.last_membership.clone();
        inner.current_snapshot = Some(StoredSnapshot {
            meta: meta.clone(),
            data,
        });

        Ok(())
    }

    async fn get_current_snapshot(
        &mut self,
    ) -> Result<Option<Snapshot<TypeConfig>>, StorageError<NodeId>> {
        let inner = self.inner.read().await;
        Ok(inner.current_snapshot.clone().map(|stored| Snapshot {
            meta: stored.meta,
            snapshot: Box::new(Cursor::new(stored.data)),
        }))
    }
}

impl RaftSnapshotBuilder<TypeConfig> for StateMachineStore {
    /// Builds a snapshot while holding the state-machine lock so snapshot data
    /// and last_applied are captured atomically.
    async fn build_snapshot(&mut self) -> Result<Snapshot<TypeConfig>, StorageError<NodeId>> {
        let mut inner = self.inner.write().await;
        let routing = self.routing.load_full();
        let data = serde_json::to_vec(routing.as_ref())
            .map_err(StoreError::SerializeSnapshot)
            .map_err(|err| storage_error(ErrorSubject::Snapshot(None), ErrorVerb::Write, err))?;

        inner.snapshot_idx += 1;
        let snapshot_id = match inner.last_applied {
            Some(log_id) => format!("{}-{}", log_id.index, inner.snapshot_idx),
            None => format!("empty-{}", inner.snapshot_idx),
        };
        let meta = SnapshotMeta {
            last_log_id: inner.last_applied,
            last_membership: inner.last_membership.clone(),
            snapshot_id,
        };

        inner.current_snapshot = Some(StoredSnapshot {
            meta: meta.clone(),
            data: data.clone(),
        });

        Ok(Snapshot {
            meta,
            snapshot: Box::new(Cursor::new(data)),
        })
    }
}

fn storage_error(
    subject: ErrorSubject<NodeId>,
    verb: ErrorVerb,
    error: StoreError,
) -> StorageError<NodeId> {
    StorageError::IO {
        source: StorageIOError::new(subject, verb, &error),
    }
}

#[cfg(test)]
mod tests {
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};

    use openraft::{CommittedLeaderId, LogId};

    use super::*;
    use raft_proxy_core::Upstream;

    fn make_entry(index: u64, term: u64, req: AppRequest) -> Entry<TypeConfig> {
        Entry {
            log_id: LogId::new(CommittedLeaderId::new(term, 1), index),
            payload: EntryPayload::Normal(req),
        }
    }

    fn upstream(port: u16) -> Upstream {
        Upstream::new(SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port))
    }

    fn new_store() -> (StateMachineStore, Arc<ArcSwap<RoutingTable>>) {
        StateMachineStore::new(Arc::new(RouteSideState::new()))
    }

    #[tokio::test]
    async fn apply_put_route_swaps_arc_swap() {
        let (mut store, routing) = new_store();
        let entry = make_entry(
            1,
            1,
            AppRequest::PutRoute {
                host: "Example.COM".to_string(),
                upstreams: vec![upstream(3000)],
            },
        );

        let responses = store.apply([entry]).await.unwrap();

        assert_eq!(responses, vec![AppResponse::Ok]);
        let table = routing.load_full();
        assert!(table.get("example.com").is_some());
    }

    #[tokio::test]
    async fn apply_delete_returns_not_found_when_missing() {
        let (mut store, routing) = new_store();
        let entry = make_entry(
            1,
            1,
            AppRequest::DeleteRoute {
                host: "missing.example".to_string(),
            },
        );

        let responses = store.apply([entry]).await.unwrap();

        assert_eq!(responses, vec![AppResponse::NotFound]);
        assert!(routing.load_full().is_empty());
    }

    #[tokio::test]
    async fn snapshot_roundtrip() {
        let (mut store, _routing) = new_store();
        let entries = [
            make_entry(
                1,
                1,
                AppRequest::PutRoute {
                    host: "a.example".to_string(),
                    upstreams: vec![upstream(3000)],
                },
            ),
            make_entry(
                2,
                1,
                AppRequest::PutRoute {
                    host: "b.example".to_string(),
                    upstreams: vec![upstream(3001)],
                },
            ),
        ];
        store.apply(entries).await.unwrap();

        let snapshot = store
            .get_snapshot_builder()
            .await
            .build_snapshot()
            .await
            .unwrap();

        let (mut restored, restored_routing) = new_store();
        restored
            .install_snapshot(&snapshot.meta, snapshot.snapshot)
            .await
            .unwrap();

        let restored_table = restored_routing.load_full();
        assert_eq!(restored_table.get("a.example"), Some(&[upstream(3000)][..]));
        assert_eq!(restored_table.get("b.example"), Some(&[upstream(3001)][..]));
        assert_eq!(restored_table.len(), 2);
    }
}
