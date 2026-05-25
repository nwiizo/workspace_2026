//! Amazon MQ E2E.
//! Refs: <https://docs.aws.amazon.com/amazon-mq/latest/api-reference/welcome.html>

mod common;
use aws_sdk_mq::types::{DeploymentMode, EngineType, User};
use pretty_assertions::assert_eq;

fn admin() -> User {
    User::builder()
        .username("admin")
        .password("StrongPassword123!")
        .build()
}

#[tokio::test]
async fn e2e_mq_create_broker() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let c = aws_sdk_mq::Client::new(&cfg);
    let res = c
        .create_broker()
        .broker_name("b1")
        .engine_type(EngineType::Activemq)
        .engine_version("5.17.6")
        .host_instance_type("mq.t3.micro")
        .deployment_mode(DeploymentMode::SingleInstance)
        .auto_minor_version_upgrade(true)
        .publicly_accessible(false)
        .users(admin())
        .send()
        .await
        .unwrap();
    assert!(res.broker_id().unwrap().starts_with("b-"));
}

#[tokio::test]
async fn e2e_mq_list_and_describe() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let c = aws_sdk_mq::Client::new(&cfg);
    let created = c
        .create_broker()
        .broker_name("b2")
        .engine_type(EngineType::Activemq)
        .engine_version("5.17.6")
        .host_instance_type("mq.t3.micro")
        .deployment_mode(DeploymentMode::SingleInstance)
        .auto_minor_version_upgrade(true)
        .publicly_accessible(false)
        .users(admin())
        .send()
        .await
        .unwrap();
    let id = created.broker_id().unwrap().to_string();
    let listed = c.list_brokers().send().await.unwrap();
    assert_eq!(listed.broker_summaries().len(), 1);
    let desc = c.describe_broker().broker_id(&id).send().await.unwrap();
    assert_eq!(desc.broker_name(), Some("b2"));
    assert_eq!(desc.engine_type(), Some(&EngineType::Activemq));
}

#[tokio::test]
async fn e2e_mq_delete() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let c = aws_sdk_mq::Client::new(&cfg);
    let created = c
        .create_broker()
        .broker_name("b3")
        .engine_type(EngineType::Rabbitmq)
        .engine_version("3.11.20")
        .host_instance_type("mq.t3.micro")
        .deployment_mode(DeploymentMode::SingleInstance)
        .auto_minor_version_upgrade(true)
        .publicly_accessible(false)
        .users(admin())
        .send()
        .await
        .unwrap();
    let id = created.broker_id().unwrap().to_string();
    c.delete_broker().broker_id(&id).send().await.unwrap();
    let listed = c.list_brokers().send().await.unwrap();
    assert_eq!(listed.broker_summaries().len(), 0);
}
