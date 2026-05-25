//! DocumentDB E2E tests.
mod common;
use pretty_assertions::assert_eq;

#[tokio::test]
async fn e2e_docdb_create_then_describe() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let d = aws_sdk_docdb::Client::new(&cfg);
    d.create_db_cluster()
        .db_cluster_identifier("docs")
        .engine("docdb")
        .send()
        .await
        .unwrap();
    let res = d.describe_db_clusters().send().await.unwrap();
    assert_eq!(res.db_clusters().len(), 1);
    assert_eq!(res.db_clusters()[0].engine(), Some("docdb"));
}

#[tokio::test]
async fn e2e_docdb_delete_cluster() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let d = aws_sdk_docdb::Client::new(&cfg);
    d.create_db_cluster()
        .db_cluster_identifier("c")
        .engine("docdb")
        .send()
        .await
        .unwrap();
    d.delete_db_cluster()
        .db_cluster_identifier("c")
        .send()
        .await
        .unwrap();
    let res = d.describe_db_clusters().send().await.unwrap();
    assert_eq!(res.db_clusters().len(), 0);
}
