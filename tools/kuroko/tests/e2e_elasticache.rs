//! ElastiCache E2E tests.
mod common;
use pretty_assertions::assert_eq;

#[tokio::test]
async fn e2e_ec_create_cluster() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let ec = aws_sdk_elasticache::Client::new(&cfg);
    let res = ec
        .create_cache_cluster()
        .cache_cluster_id("redis1")
        .engine("redis")
        .cache_node_type("cache.t3.micro")
        .num_cache_nodes(1)
        .send()
        .await
        .unwrap();
    let c = res.cache_cluster().unwrap();
    assert_eq!(c.cache_cluster_id(), Some("redis1"));
    assert_eq!(c.engine(), Some("redis"));
}

#[tokio::test]
async fn e2e_ec_describe_clusters() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let ec = aws_sdk_elasticache::Client::new(&cfg);
    ec.create_cache_cluster()
        .cache_cluster_id("c1")
        .engine("redis")
        .cache_node_type("cache.t3.micro")
        .num_cache_nodes(1)
        .send()
        .await
        .unwrap();
    let res = ec.describe_cache_clusters().send().await.unwrap();
    assert_eq!(res.cache_clusters().len(), 1);
}

#[tokio::test]
async fn e2e_ec_delete_cluster() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let ec = aws_sdk_elasticache::Client::new(&cfg);
    ec.create_cache_cluster()
        .cache_cluster_id("d")
        .engine("redis")
        .cache_node_type("cache.t3.micro")
        .num_cache_nodes(1)
        .send()
        .await
        .unwrap();
    ec.delete_cache_cluster()
        .cache_cluster_id("d")
        .send()
        .await
        .unwrap();
    let res = ec.describe_cache_clusters().send().await.unwrap();
    assert_eq!(res.cache_clusters().len(), 0);
}
