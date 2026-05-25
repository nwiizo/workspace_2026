//! ECS E2E tests against AWS official API spec.
//! Refs: <https://docs.aws.amazon.com/AmazonECS/latest/APIReference/Welcome.html>

mod common;

use aws_sdk_ecs::types::{ContainerDefinition, DesiredStatus, LaunchType, NetworkMode};
use pretty_assertions::assert_eq;

#[tokio::test]
async fn e2e_ecs_create_then_describe_cluster() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let e = aws_sdk_ecs::Client::new(&cfg);
    let res = e
        .create_cluster()
        .cluster_name("prod")
        .send()
        .await
        .unwrap();
    let c = res.cluster().unwrap();
    assert_eq!(c.cluster_name(), Some("prod"));
    let desc = e.describe_clusters().clusters("prod").send().await.unwrap();
    assert_eq!(desc.clusters().len(), 1);
}

#[tokio::test]
async fn e2e_ecs_register_task_definition_increments_revision() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let e = aws_sdk_ecs::Client::new(&cfg);
    let container = ContainerDefinition::builder()
        .name("app")
        .image("nginx:latest")
        .build();
    let r1 = e
        .register_task_definition()
        .family("web")
        .network_mode(NetworkMode::Awsvpc)
        .container_definitions(container.clone())
        .send()
        .await
        .unwrap();
    let r2 = e
        .register_task_definition()
        .family("web")
        .container_definitions(container)
        .send()
        .await
        .unwrap();
    assert_eq!(r1.task_definition().unwrap().revision(), 1);
    assert_eq!(r2.task_definition().unwrap().revision(), 2);
}

#[tokio::test]
async fn e2e_ecs_create_service() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let e = aws_sdk_ecs::Client::new(&cfg);
    e.create_cluster().cluster_name("c").send().await.unwrap();
    e.register_task_definition()
        .family("web")
        .container_definitions(
            ContainerDefinition::builder()
                .name("app")
                .image("nginx")
                .build(),
        )
        .send()
        .await
        .unwrap();
    let res = e
        .create_service()
        .cluster("c")
        .service_name("api")
        .task_definition("web")
        .desired_count(3)
        .send()
        .await
        .unwrap();
    let svc = res.service().unwrap();
    assert_eq!(svc.service_name(), Some("api"));
    assert_eq!(svc.desired_count(), 3);
}

#[tokio::test]
async fn e2e_ecs_run_task_immediately_running() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let e = aws_sdk_ecs::Client::new(&cfg);
    e.create_cluster().cluster_name("c").send().await.unwrap();
    e.register_task_definition()
        .family("worker")
        .container_definitions(
            ContainerDefinition::builder()
                .name("w")
                .image("busybox")
                .build(),
        )
        .send()
        .await
        .unwrap();
    let res = e
        .run_task()
        .cluster("c")
        .task_definition("worker")
        .launch_type(LaunchType::Fargate)
        .count(2)
        .send()
        .await
        .unwrap();
    assert_eq!(res.tasks().len(), 2);
    assert_eq!(res.tasks()[0].last_status(), Some("RUNNING"));
}

#[tokio::test]
async fn e2e_ecs_stop_task_transitions_to_stopped() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let e = aws_sdk_ecs::Client::new(&cfg);
    e.create_cluster().cluster_name("c").send().await.unwrap();
    e.register_task_definition()
        .family("w")
        .container_definitions(ContainerDefinition::builder().name("c").image("x").build())
        .send()
        .await
        .unwrap();
    let task_arn = e
        .run_task()
        .cluster("c")
        .task_definition("w")
        .send()
        .await
        .unwrap()
        .tasks()[0]
        .task_arn()
        .unwrap()
        .to_string();
    let res = e
        .stop_task()
        .cluster("c")
        .task(&task_arn)
        .send()
        .await
        .unwrap();
    assert_eq!(res.task().unwrap().desired_status(), Some("STOPPED"));
    let _ = DesiredStatus::Stopped; // unused-import guard
}
