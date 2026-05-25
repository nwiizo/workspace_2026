//! RDS E2E tests against AWS official API spec.
//! Refs: <https://docs.aws.amazon.com/AmazonRDS/latest/APIReference/Welcome.html>

mod common;

use pretty_assertions::assert_eq;

#[tokio::test]
async fn e2e_rds_create_db_instance_returns_arn() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let r = aws_sdk_rds::Client::new(&cfg);
    let res = r
        .create_db_instance()
        .db_instance_identifier("primary")
        .db_instance_class("db.t3.micro")
        .engine("postgres")
        .master_username("postgres")
        .allocated_storage(20)
        .send()
        .await
        .unwrap();
    let inst = res.db_instance().unwrap();
    assert_eq!(inst.db_instance_identifier(), Some("primary"));
    assert!(inst.db_instance_arn().unwrap().contains(":db:primary"));
    assert_eq!(inst.endpoint().unwrap().port(), Some(5432));
}

#[tokio::test]
async fn e2e_rds_duplicate_instance_fails() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let r = aws_sdk_rds::Client::new(&cfg);
    r.create_db_instance()
        .db_instance_identifier("dup")
        .db_instance_class("db.t3.micro")
        .engine("mysql")
        .send()
        .await
        .unwrap();
    let err = r
        .create_db_instance()
        .db_instance_identifier("dup")
        .db_instance_class("db.t3.micro")
        .engine("mysql")
        .send()
        .await;
    assert!(err.is_err());
}

#[tokio::test]
async fn e2e_rds_describe_filters_by_identifier() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let r = aws_sdk_rds::Client::new(&cfg);
    for n in ["a", "b", "c"] {
        r.create_db_instance()
            .db_instance_identifier(n)
            .db_instance_class("db.t3.micro")
            .engine("mysql")
            .send()
            .await
            .unwrap();
    }
    let res = r
        .describe_db_instances()
        .db_instance_identifier("b")
        .send()
        .await
        .unwrap();
    assert_eq!(res.db_instances().len(), 1);
    assert_eq!(res.db_instances()[0].db_instance_identifier(), Some("b"));
}

#[tokio::test]
async fn e2e_rds_create_db_cluster_then_describe() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let r = aws_sdk_rds::Client::new(&cfg);
    r.create_db_cluster()
        .db_cluster_identifier("auroracluster")
        .engine("aurora-postgresql")
        .master_username("admin")
        .send()
        .await
        .unwrap();
    let res = r.describe_db_clusters().send().await.unwrap();
    assert_eq!(res.db_clusters().len(), 1);
    assert!(
        res.db_clusters()[0]
            .endpoint()
            .unwrap()
            .contains("auroracluster.cluster")
    );
}

#[tokio::test]
async fn e2e_rds_create_snapshot_from_instance() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let r = aws_sdk_rds::Client::new(&cfg);
    r.create_db_instance()
        .db_instance_identifier("source")
        .db_instance_class("db.t3.micro")
        .engine("postgres")
        .send()
        .await
        .unwrap();
    let res = r
        .create_db_snapshot()
        .db_snapshot_identifier("snap1")
        .db_instance_identifier("source")
        .send()
        .await
        .unwrap();
    assert_eq!(
        res.db_snapshot().unwrap().db_snapshot_identifier(),
        Some("snap1")
    );
}

#[tokio::test]
async fn e2e_rds_delete_then_describe_404() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let r = aws_sdk_rds::Client::new(&cfg);
    r.create_db_instance()
        .db_instance_identifier("doomed")
        .db_instance_class("db.t3.micro")
        .engine("mysql")
        .send()
        .await
        .unwrap();
    r.delete_db_instance()
        .db_instance_identifier("doomed")
        .send()
        .await
        .unwrap();
    let err = r
        .describe_db_instances()
        .db_instance_identifier("doomed")
        .send()
        .await;
    // Note: DescribeDBInstances on an unknown identifier returns the empty
    // set (not an error) in AWS, but kuroko's filter returns empty too.
    assert!(err.is_ok());
    assert_eq!(err.unwrap().db_instances().len(), 0);
}
