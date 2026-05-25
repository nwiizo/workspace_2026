//! MemoryDB E2E tests.
mod common;
use pretty_assertions::assert_eq;

#[tokio::test]
async fn e2e_memorydb_create_then_describe() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let m = aws_sdk_memorydb::Client::new(&cfg);
    let res = m
        .create_cluster()
        .cluster_name("redis1")
        .node_type("db.t4g.small")
        .acl_name("default")
        .send()
        .await
        .unwrap();
    assert_eq!(res.cluster().unwrap().name(), Some("redis1"));
    let d = m
        .describe_clusters()
        .cluster_name("redis1")
        .send()
        .await
        .unwrap();
    assert_eq!(d.clusters().len(), 1);
}

#[tokio::test]
async fn e2e_memorydb_delete_cluster() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let m = aws_sdk_memorydb::Client::new(&cfg);
    m.create_cluster()
        .cluster_name("c")
        .node_type("db.t4g.small")
        .acl_name("default")
        .send()
        .await
        .unwrap();
    m.delete_cluster().cluster_name("c").send().await.unwrap();
    let d = m.describe_clusters().send().await.unwrap();
    assert_eq!(d.clusters().len(), 0);
}
