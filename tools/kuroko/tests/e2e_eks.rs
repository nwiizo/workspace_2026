//! EKS E2E tests.
//! Refs: <https://docs.aws.amazon.com/eks/latest/APIReference/Welcome.html>

mod common;

use aws_sdk_eks::types::NodegroupScalingConfig;
use pretty_assertions::assert_eq;

#[tokio::test]
async fn e2e_eks_create_cluster_returns_active() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let e = aws_sdk_eks::Client::new(&cfg);
    let res = e
        .create_cluster()
        .name("prod")
        .role_arn("arn:aws:iam::000000000000:role/eks")
        .send()
        .await
        .unwrap();
    let c = res.cluster().unwrap();
    assert_eq!(c.name(), Some("prod"));
    assert!(c.arn().unwrap().contains(":cluster/prod"));
}

#[tokio::test]
async fn e2e_eks_list_then_describe() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let e = aws_sdk_eks::Client::new(&cfg);
    e.create_cluster()
        .name("c1")
        .role_arn("arn:aws:iam::000000000000:role/eks")
        .send()
        .await
        .unwrap();
    let list = e.list_clusters().send().await.unwrap();
    assert!(list.clusters().iter().any(|n| n == "c1"));
    let d = e.describe_cluster().name("c1").send().await.unwrap();
    assert_eq!(d.cluster().unwrap().status().unwrap().as_str(), "ACTIVE");
}

#[tokio::test]
async fn e2e_eks_duplicate_create_fails() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let e = aws_sdk_eks::Client::new(&cfg);
    e.create_cluster()
        .name("dup")
        .role_arn("arn:aws:iam::000000000000:role/eks")
        .send()
        .await
        .unwrap();
    let err = e
        .create_cluster()
        .name("dup")
        .role_arn("arn:aws:iam::000000000000:role/eks")
        .send()
        .await;
    assert!(err.is_err());
}

#[tokio::test]
async fn e2e_eks_create_nodegroup() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let e = aws_sdk_eks::Client::new(&cfg);
    e.create_cluster()
        .name("c")
        .role_arn("arn:aws:iam::000000000000:role/eks")
        .send()
        .await
        .unwrap();
    let res = e
        .create_nodegroup()
        .cluster_name("c")
        .nodegroup_name("workers")
        .node_role("arn:aws:iam::000000000000:role/worker")
        .instance_types("t3.medium")
        .scaling_config(
            NodegroupScalingConfig::builder()
                .desired_size(3)
                .min_size(1)
                .max_size(5)
                .build(),
        )
        .send()
        .await
        .unwrap();
    assert_eq!(res.nodegroup().unwrap().nodegroup_name(), Some("workers"));
    let list = e.list_nodegroups().cluster_name("c").send().await.unwrap();
    assert_eq!(list.nodegroups().len(), 1);
}

#[tokio::test]
async fn e2e_eks_delete_cluster() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let e = aws_sdk_eks::Client::new(&cfg);
    e.create_cluster()
        .name("doomed")
        .role_arn("arn:aws:iam::000000000000:role/eks")
        .send()
        .await
        .unwrap();
    e.delete_cluster().name("doomed").send().await.unwrap();
    let err = e.describe_cluster().name("doomed").send().await;
    assert!(err.is_err());
}
