//! Redshift E2E tests.
mod common;
use pretty_assertions::assert_eq;

#[tokio::test]
async fn e2e_rs_create_cluster() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let r = aws_sdk_redshift::Client::new(&cfg);
    let res = r
        .create_cluster()
        .cluster_identifier("dw")
        .node_type("dc2.large")
        .master_username("admin")
        .send()
        .await
        .unwrap();
    let c = res.cluster().unwrap();
    assert_eq!(c.cluster_identifier(), Some("dw"));
    assert_eq!(c.endpoint().unwrap().port(), Some(5439));
}

#[tokio::test]
async fn e2e_rs_describe_clusters() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let r = aws_sdk_redshift::Client::new(&cfg);
    for n in ["a", "b"] {
        r.create_cluster()
            .cluster_identifier(n)
            .node_type("dc2.large")
            .send()
            .await
            .unwrap();
    }
    let res = r.describe_clusters().send().await.unwrap();
    assert_eq!(res.clusters().len(), 2);
}

#[tokio::test]
async fn e2e_rs_delete_cluster() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let r = aws_sdk_redshift::Client::new(&cfg);
    r.create_cluster()
        .cluster_identifier("c")
        .node_type("dc2.large")
        .send()
        .await
        .unwrap();
    r.delete_cluster()
        .cluster_identifier("c")
        .send()
        .await
        .unwrap();
    let res = r.describe_clusters().send().await.unwrap();
    assert_eq!(res.clusters().len(), 0);
}
