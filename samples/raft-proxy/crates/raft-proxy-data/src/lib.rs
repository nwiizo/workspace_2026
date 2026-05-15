use std::sync::Arc;

use arc_swap::ArcSwap;
use async_trait::async_trait;
use http::header::HOST;
use pingora::proxy::{ProxyHttp, Session, http_proxy_service};
use pingora::server::Server;
use pingora::server::configuration::ServerConf;
use pingora::upstreams::peer::HttpPeer;
use pingora::{Error, Result};
use raft_proxy_core::{RouteSideState, RoutingTable, Upstream};

const MAX_UPSTREAM_WEIGHT: u32 = 256;

pub struct ProxyService {
    routing: Arc<ArcSwap<RoutingTable>>,
    route_side_state: Arc<RouteSideState>,
}

impl ProxyService {
    /// Shares per-host round-robin counters with the state machine so
    /// `DeleteRoute` can remove data-plane state when a route is deleted.
    pub fn new(routing: Arc<ArcSwap<RoutingTable>>, route_side_state: Arc<RouteSideState>) -> Self {
        Self {
            routing,
            route_side_state,
        }
    }

    /// Expands upstreams into weighted tickets on each call instead of caching;
    /// this learning sample keeps route lists small and avoids cache invalidation
    /// bugs. Each weight is capped at 256, and weight=0 excludes an upstream.
    pub(crate) fn pick_upstream(&self, host: &str) -> Option<Upstream> {
        let table = self.routing.load();
        let upstreams = table.get(host)?;
        let tickets = weighted_tickets(upstreams);
        if tickets.is_empty() {
            return None;
        }

        let idx = self
            .route_side_state
            .next_round_robin_index(host, tickets.len())?;
        Some(tickets[idx].clone())
    }
}

#[async_trait]
impl ProxyHttp for ProxyService {
    type CTX = ();

    fn new_ctx(&self) -> Self::CTX {}

    async fn upstream_peer(
        &self,
        session: &mut Session,
        _ctx: &mut Self::CTX,
    ) -> Result<Box<HttpPeer>> {
        let host = session
            .req_header()
            .headers
            .get(HOST)
            .ok_or_else(|| Error::new_str("missing host header"))?
            .to_str()
            .map_err(|_| Error::new_str("invalid host header"))?;
        let host = strip_port(host);
        let upstream = self
            .pick_upstream(host)
            .ok_or_else(|| Error::new_str("no route for host"))?;

        Ok(Box::new(HttpPeer::new(upstream.addr, false, String::new())))
    }
}

pub fn build_server(
    routing: Arc<ArcSwap<RoutingTable>>,
    route_side_state: Arc<RouteSideState>,
    listen_addr: &str,
) -> Server {
    let mut server = Server::new_with_opt_and_conf(None, ServerConf::default());
    server.bootstrap();

    let service = ProxyService::new(routing, route_side_state);
    let mut proxy = http_proxy_service(&server.configuration, service);
    proxy.add_tcp(listen_addr);
    server.add_service(proxy);
    server
}

fn weighted_tickets(upstreams: &[Upstream]) -> Vec<Upstream> {
    let ticket_count = upstreams
        .iter()
        .map(|upstream| upstream.weight.min(MAX_UPSTREAM_WEIGHT) as usize)
        .sum::<usize>();
    let mut tickets = Vec::with_capacity(ticket_count);

    for upstream in upstreams {
        for _ in 0..upstream.weight.min(MAX_UPSTREAM_WEIGHT) {
            tickets.push(upstream.clone());
        }
    }

    tickets
}

fn strip_port(host: &str) -> &str {
    if let Some(rest) = host.strip_prefix('[') {
        if let Some((inside_brackets, after_brackets)) = rest.split_once(']') {
            if after_brackets.is_empty()
                || after_brackets
                    .strip_prefix(':')
                    .is_some_and(|port| port.parse::<u16>().is_ok())
            {
                return inside_brackets;
            }
        }
    }

    match host.rsplit_once(':') {
        Some((name, port)) if !name.contains(':') && port.parse::<u16>().is_ok() => name,
        _ => host,
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use arc_swap::ArcSwap;
    use raft_proxy_core::{RouteSideState, RoutingTable, Upstream};

    use super::ProxyService;

    fn service_with_route(host: &str, upstreams: Vec<Upstream>) -> ProxyService {
        let mut table = RoutingTable::new();
        table.insert(host.to_string(), upstreams);
        ProxyService::new(
            Arc::new(ArcSwap::from_pointee(table)),
            Arc::new(RouteSideState::new()),
        )
    }

    fn upstream(addr: &str) -> Upstream {
        Upstream::new(addr.parse().unwrap())
    }

    #[test]
    fn no_route_returns_none() {
        let service = ProxyService::new(
            Arc::new(ArcSwap::from_pointee(RoutingTable::new())),
            Arc::new(RouteSideState::new()),
        );

        assert_eq!(service.pick_upstream("missing.test"), None);
    }

    #[test]
    fn single_upstream_returns_it() {
        let expected = upstream("127.0.0.1:9001");
        let service = service_with_route("x.test", vec![expected.clone()]);

        assert_eq!(service.pick_upstream("x.test"), Some(expected));
    }

    #[test]
    fn round_robin_cycles_through_upstreams() {
        let upstreams = vec![
            upstream("127.0.0.1:9001"),
            upstream("127.0.0.1:9002"),
            upstream("127.0.0.1:9003"),
        ];
        let service = service_with_route("x.test", upstreams.clone());

        let picked = (0..6)
            .map(|_| service.pick_upstream("x.test").unwrap())
            .collect::<Vec<_>>();

        assert_eq!(
            picked,
            vec![
                upstreams[0].clone(),
                upstreams[1].clone(),
                upstreams[2].clone(),
                upstreams[0].clone(),
                upstreams[1].clone(),
                upstreams[2].clone(),
            ]
        );
    }

    #[test]
    fn weighted_round_robin_respects_weights() {
        let a = Upstream::with_weight("127.0.0.1:9001".parse().unwrap(), 3);
        let b = Upstream::with_weight("127.0.0.1:9002".parse().unwrap(), 1);
        let service = service_with_route("x.test", vec![a.clone(), b.clone()]);

        let picked = (0..8)
            .map(|_| service.pick_upstream("x.test").unwrap())
            .collect::<Vec<_>>();

        assert_eq!(picked.iter().filter(|upstream| **upstream == a).count(), 6);
        assert_eq!(picked.iter().filter(|upstream| **upstream == b).count(), 2);
    }
}
