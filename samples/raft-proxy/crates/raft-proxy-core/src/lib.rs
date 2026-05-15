use std::collections::HashMap;
use std::fmt;
use std::io::Cursor;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering};

use dashmap::DashMap;
use serde::{Deserialize, Serialize};

pub type NodeId = u64;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Upstream {
    pub addr: SocketAddr,
    #[serde(default = "default_weight")]
    pub weight: u32,
}

impl Upstream {
    pub fn new(addr: SocketAddr) -> Self {
        Self {
            addr,
            weight: default_weight(),
        }
    }

    pub fn with_weight(addr: SocketAddr, weight: u32) -> Self {
        Self { addr, weight }
    }
}

fn default_weight() -> u32 {
    1
}

/// Shared data-plane state for a route table, owned by both `ProxyService` and
/// `StateMachineStore` so Raft-side deletions can drop per-host counters.
#[derive(Debug, Default)]
pub struct RouteSideState {
    rr_counters: DashMap<String, AtomicUsize>,
}

impl RouteSideState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn next_round_robin_index(&self, host: &str, modulo: usize) -> Option<usize> {
        if modulo == 0 {
            return None;
        }

        let counter = self
            .rr_counters
            .entry(normalize_host(host))
            .or_insert_with(|| AtomicUsize::new(0));
        Some(counter.fetch_add(1, Ordering::Relaxed) % modulo)
    }

    pub fn remove_host(&self, host: &str) {
        self.rr_counters.remove(&normalize_host(host));
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct RoutingTable {
    routes: HashMap<String, Vec<Upstream>>,
}

impl RoutingTable {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(&self, host: &str) -> Option<&[Upstream]> {
        self.routes.get(&normalize_host(host)).map(Vec::as_slice)
    }

    pub fn routes(&self) -> &HashMap<String, Vec<Upstream>> {
        &self.routes
    }

    pub fn insert(&mut self, host: String, upstreams: Vec<Upstream>) {
        self.routes.insert(normalize_host(&host), upstreams);
    }

    pub fn remove(&mut self, host: &str) -> bool {
        self.routes.remove(&normalize_host(host)).is_some()
    }

    pub fn len(&self) -> usize {
        self.routes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.routes.is_empty()
    }
}

fn normalize_host(host: &str) -> String {
    let host = host.to_ascii_lowercase();

    if let Some(rest) = host.strip_prefix('[') {
        if let Some((inside_brackets, after_brackets)) = rest.split_once(']') {
            if after_brackets.is_empty()
                || after_brackets
                    .strip_prefix(':')
                    .is_some_and(|port| port.parse::<u16>().is_ok())
            {
                return inside_brackets.to_owned();
            }
        }
    }

    match host.rsplit_once(':') {
        Some((name, port)) if !name.contains(':') && port.parse::<u16>().is_ok() => name.to_owned(),
        _ => host,
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum AppRequest {
    PutRoute {
        host: String,
        upstreams: Vec<Upstream>,
    },
    DeleteRoute {
        host: String,
    },
}

impl fmt::Display for AppRequest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PutRoute { host, upstreams } => {
                write!(
                    formatter,
                    "put route {host} with {} upstreams",
                    upstreams.len()
                )
            }
            Self::DeleteRoute { host } => write!(formatter, "delete route {host}"),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum AppResponse {
    Ok,
    NotFound,
}

impl fmt::Display for AppResponse {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ok => formatter.write_str("ok"),
            Self::NotFound => formatter.write_str("not found"),
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct Node {
    pub rpc_addr: String,
}

openraft::declare_raft_types!(
    pub TypeConfig:
        D = AppRequest,
        R = AppResponse,
        NodeId = crate::NodeId,
        Node = Node,
        Entry = openraft::Entry<TypeConfig>,
        SnapshotData = Cursor<Vec<u8>>,
        AsyncRuntime = openraft::impls::TokioRuntime,
);

#[cfg(test)]
mod tests {
    use super::RoutingTable;

    #[test]
    fn normalize_host_canonicalizes_case_ports_and_ipv6_brackets() {
        let mut table = RoutingTable::new();
        table.insert("[::1]".to_string(), Vec::new());
        table.insert("Example.Test".to_string(), Vec::new());

        assert!(table.get("[::1]").is_some());
        assert!(table.get("[::1]:8080").is_some());
        assert!(table.get("Example.Test").is_some());
        assert!(table.get("example.test:8080").is_some());
        assert!(table.routes().contains_key("::1"));
        assert!(table.routes().contains_key("example.test"));
    }
}
