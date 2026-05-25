//! Neptune E2E tests.
mod common;
use pretty_assertions::assert_eq;

#[tokio::test]
async fn e2e_neptune_create_cluster() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let n = aws_sdk_neptune::Client::new(&cfg);
    let res = n
        .create_db_cluster()
        .db_cluster_identifier("graph")
        .engine("neptune")
        .send()
        .await
        .unwrap();
    assert_eq!(res.db_cluster().unwrap().engine(), Some("neptune"));
}

#[tokio::test]
async fn e2e_neptune_describe_clusters() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let n = aws_sdk_neptune::Client::new(&cfg);
    n.create_db_cluster()
        .db_cluster_identifier("g1")
        .engine("neptune")
        .send()
        .await
        .unwrap();
    let res = n.describe_db_clusters().send().await.unwrap();
    assert_eq!(res.db_clusters().len(), 1);
}
