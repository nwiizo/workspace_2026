//! MSK (kafka) E2E.
//! Refs: <https://docs.aws.amazon.com/msk/1.0/apireference/operations.html>

mod common;
use aws_sdk_kafka::types::{BrokerNodeGroupInfo, ClusterType};
use pretty_assertions::assert_eq;

fn broker() -> BrokerNodeGroupInfo {
    BrokerNodeGroupInfo::builder()
        .client_subnets("subnet-1")
        .instance_type("kafka.m5.large")
        .build()
}

#[tokio::test]
async fn e2e_msk_create_cluster_v1() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let c = aws_sdk_kafka::Client::new(&cfg);
    let res = c
        .create_cluster()
        .cluster_name("c1")
        .kafka_version("3.5.1")
        .number_of_broker_nodes(2)
        .broker_node_group_info(broker())
        .send()
        .await
        .unwrap();
    assert!(res.cluster_arn().unwrap().contains(":cluster/c1"));
}

#[tokio::test]
async fn e2e_msk_list_clusters_v2() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let c = aws_sdk_kafka::Client::new(&cfg);
    c.create_cluster_v2()
        .cluster_name("cv2")
        .provisioned(
            aws_sdk_kafka::types::ProvisionedRequest::builder()
                .kafka_version("3.5.1")
                .number_of_broker_nodes(2)
                .broker_node_group_info(broker())
                .build(),
        )
        .send()
        .await
        .unwrap();
    let res = c.list_clusters_v2().send().await.unwrap();
    assert_eq!(res.cluster_info_list().len(), 1);
    assert_eq!(
        res.cluster_info_list()[0].cluster_type(),
        Some(&ClusterType::Provisioned)
    );
}

#[tokio::test]
async fn e2e_msk_describe_and_delete() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let c = aws_sdk_kafka::Client::new(&cfg);
    let created = c
        .create_cluster()
        .cluster_name("c2")
        .kafka_version("3.5.1")
        .number_of_broker_nodes(2)
        .broker_node_group_info(broker())
        .send()
        .await
        .unwrap();
    let arn = created.cluster_arn().unwrap().to_string();
    let desc = c.describe_cluster().cluster_arn(&arn).send().await.unwrap();
    assert_eq!(
        desc.cluster_info().unwrap().cluster_arn(),
        Some(arn.as_str())
    );
    c.delete_cluster().cluster_arn(&arn).send().await.unwrap();
    let list = c.list_clusters().send().await.unwrap();
    assert_eq!(list.cluster_info_list().len(), 0);
}
