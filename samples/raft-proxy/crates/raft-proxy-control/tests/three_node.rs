use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use axum::serve;
use raft_proxy_control::ProxyApp;
use raft_proxy_core::NodeId;
use raft_proxy_network::PeerRegistry;
use reqwest::{Client, StatusCode};
use serde_json::{Value, json};
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;
use tokio::time::{Instant, sleep};

struct TestNode {
    id: NodeId,
    addr: SocketAddr,
    app: Arc<ProxyApp>,
    shutdown: oneshot::Sender<()>,
    server: JoinHandle<()>,
}

async fn spawn_node(id: NodeId, peers: &PeerRegistry) -> TestNode {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind test node listener");
    let addr = listener.local_addr().expect("read test node local address");
    let base_url = format!("http://{addr}");
    let app = Arc::new(
        ProxyApp::bootstrap(id, base_url, peers.clone())
            .await
            .expect("bootstrap proxy app"),
    );
    let router = Arc::clone(&app).router();
    let (shutdown, shutdown_rx) = oneshot::channel::<()>();
    let server = tokio::spawn(async move {
        serve(listener, router)
            .with_graceful_shutdown(async {
                let _ = shutdown_rx.await;
            })
            .await
            .expect("test node server");
    });

    TestNode {
        id,
        addr,
        app,
        shutdown,
        server,
    }
}

async fn find_leader(nodes: &[TestNode], deadline: Instant) -> NodeId {
    let client = Client::new();
    let mut last_metrics = json!({});

    while Instant::now() < deadline {
        last_metrics = dump_metrics(&client, nodes).await;

        for node in nodes {
            let Some(metrics) = last_metrics.get(node.id.to_string()) else {
                continue;
            };

            if metrics.get("current_leader") == Some(&json!(node.id))
                && metrics.get("state") == Some(&json!("Leader"))
            {
                return node.id;
            }
        }

        sleep(Duration::from_millis(100)).await;
    }

    panic!("leader not elected before deadline; metrics: {last_metrics}");
}

async fn wait_for_routes(client: &Client, nodes: &[TestNode], hosts: &[&str], deadline: Instant) {
    let mut last_routes = json!({});

    while Instant::now() < deadline {
        let mut routes_by_node = serde_json::Map::new();
        let mut all_visible = true;

        for node in nodes {
            let routes = get_routes(client, &format!("http://{}", node.addr)).await;

            for host in hosts {
                if routes.get("routes").and_then(|r| r.get(host)).is_none() {
                    all_visible = false;
                }
            }

            routes_by_node.insert(node.id.to_string(), routes);
        }

        last_routes = Value::Object(routes_by_node);

        if all_visible {
            return;
        }

        sleep(Duration::from_millis(50)).await;
    }

    panic!("routes {hosts:?} not visible before deadline; routes: {last_routes}");
}

async fn put_route(
    client: &Client,
    base: &str,
    host: &str,
    upstream: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let response = client
        .put(format!("{base}/admin/routes"))
        .json(&json!({
            "host": host,
            "upstreams": [
                { "addr": upstream, "weight": 1 }
            ]
        }))
        .send()
        .await?;

    assert_eq!(
        response.status(),
        StatusCode::OK,
        "PUT route failed: {}",
        response.text().await?
    );

    Ok(())
}

async fn get_routes(client: &Client, base: &str) -> Value {
    let response = client
        .get(format!("{base}/admin/routes"))
        .send()
        .await
        .expect("GET /admin/routes response");
    let status = response.status();
    let body = response
        .text()
        .await
        .expect("GET /admin/routes response body");

    assert_eq!(status, StatusCode::OK, "GET /admin/routes failed: {body}");

    serde_json::from_str(&body).expect("GET /admin/routes JSON")
}

async fn dump_metrics(client: &Client, nodes: &[TestNode]) -> Value {
    let mut metrics_by_node = serde_json::Map::new();

    for node in nodes {
        let key = node.id.to_string();
        let metrics = match client
            .get(format!("http://{}/cluster/metrics", node.addr))
            .send()
            .await
        {
            Ok(response) => match response.json::<Value>().await {
                Ok(metrics) => metrics,
                Err(err) => json!({ "error": err.to_string() }),
            },
            Err(err) => json!({ "error": err.to_string() }),
        };

        metrics_by_node.insert(key, metrics);
    }

    Value::Object(metrics_by_node)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn three_node_replicates_and_survives_leader_kill() -> Result<(), Box<dyn std::error::Error>>
{
    let peers = PeerRegistry::new();
    let mut nodes = vec![
        spawn_node(1, &peers).await,
        spawn_node(2, &peers).await,
        spawn_node(3, &peers).await,
    ];
    let client = Client::builder()
        .redirect(reqwest::redirect::Policy::limited(3))
        .build()?;

    let init_body = json!({
        "members": [
            { "id": 1, "rpc_addr": format!("http://{}", nodes[0].addr) },
            { "id": 2, "rpc_addr": format!("http://{}", nodes[1].addr) },
            { "id": 3, "rpc_addr": format!("http://{}", nodes[2].addr) },
        ]
    });
    let init = client
        .post(format!("http://{}/cluster/init", nodes[0].addr))
        .json(&init_body)
        .send()
        .await?;
    assert_eq!(
        init.status(),
        StatusCode::OK,
        "cluster init failed: {}",
        init.text().await?
    );

    let leader_id = find_leader(&nodes, Instant::now() + Duration::from_secs(5)).await;
    let follower_addr = nodes
        .iter()
        .find(|node| node.id != leader_id)
        .expect("follower node")
        .addr;

    put_route(
        &client,
        &format!("http://{follower_addr}"),
        "alpha.test",
        "10.0.0.1:80",
    )
    .await?;

    wait_for_routes(
        &client,
        &nodes,
        &["alpha.test"],
        Instant::now() + Duration::from_secs(3),
    )
    .await;

    for node in &nodes {
        let routes = get_routes(&client, &format!("http://{}", node.addr)).await;
        assert!(
            routes
                .get("routes")
                .and_then(|entries| entries.get("alpha.test"))
                .is_some(),
            "missing alpha on node {}",
            node.id
        );
    }

    let leader_idx = nodes
        .iter()
        .position(|node| node.id == leader_id)
        .expect("leader node position");
    let leader = nodes.swap_remove(leader_idx);
    let _ = leader.shutdown.send(());
    leader.server.await.expect("leader server task");
    drop(leader.app);

    let new_leader = find_leader(&nodes, Instant::now() + Duration::from_secs(5)).await;
    assert_ne!(new_leader, leader_id);

    put_route(
        &client,
        &format!("http://{}", nodes[0].addr),
        "beta.test",
        "10.0.0.2:80",
    )
    .await?;

    wait_for_routes(
        &client,
        &nodes,
        &["alpha.test", "beta.test"],
        Instant::now() + Duration::from_secs(3),
    )
    .await;

    for node in &nodes {
        let routes = get_routes(&client, &format!("http://{}", node.addr)).await;
        assert!(
            routes
                .get("routes")
                .and_then(|entries| entries.get("alpha.test"))
                .is_some(),
            "lost alpha on node {}: {routes}",
            node.id
        );
        assert!(
            routes
                .get("routes")
                .and_then(|entries| entries.get("beta.test"))
                .is_some(),
            "missing beta on node {}: {routes}",
            node.id
        );
    }

    for node in nodes {
        let _ = node.shutdown.send(());
        node.server.await.expect("test node server task");
    }

    Ok(())
}
