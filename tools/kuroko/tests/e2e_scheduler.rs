//! EventBridge Scheduler E2E tests.
//! Refs: <https://docs.aws.amazon.com/scheduler/latest/APIReference/Welcome.html>

mod common;

use aws_sdk_scheduler::types::Target;
use pretty_assertions::assert_eq;

#[tokio::test]
async fn e2e_scheduler_default_group_exists() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let s = aws_sdk_scheduler::Client::new(&cfg);
    let res = s.list_schedule_groups().send().await.unwrap();
    assert!(
        res.schedule_groups()
            .iter()
            .any(|g| g.name() == Some("default"))
    );
}

#[tokio::test]
async fn e2e_scheduler_create_schedule() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let s = aws_sdk_scheduler::Client::new(&cfg);
    let target = Target::builder()
        .arn("arn:aws:lambda:us-east-1:000000000000:function:test")
        .role_arn("arn:aws:iam::000000000000:role/sched")
        .build()
        .unwrap();
    let res = s
        .create_schedule()
        .name("nightly")
        .schedule_expression("cron(0 0 * * ? *)")
        .target(target)
        .flexible_time_window(
            aws_sdk_scheduler::types::FlexibleTimeWindow::builder()
                .mode(aws_sdk_scheduler::types::FlexibleTimeWindowMode::Off)
                .build()
                .unwrap(),
        )
        .send()
        .await
        .unwrap();
    assert!(res.schedule_arn().contains(":schedule/default/nightly"));
}

#[tokio::test]
async fn e2e_scheduler_create_then_get_schedule() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let s = aws_sdk_scheduler::Client::new(&cfg);
    let target = Target::builder()
        .arn("arn:aws:lambda:us-east-1:000000000000:function:f")
        .role_arn("arn:aws:iam::000000000000:role/r")
        .build()
        .unwrap();
    s.create_schedule()
        .name("hourly")
        .schedule_expression("rate(1 hour)")
        .target(target)
        .flexible_time_window(
            aws_sdk_scheduler::types::FlexibleTimeWindow::builder()
                .mode(aws_sdk_scheduler::types::FlexibleTimeWindowMode::Off)
                .build()
                .unwrap(),
        )
        .send()
        .await
        .unwrap();
    let g = s.get_schedule().name("hourly").send().await.unwrap();
    assert_eq!(g.name(), Some("hourly"));
}

#[tokio::test]
async fn e2e_scheduler_create_group_then_delete() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let s = aws_sdk_scheduler::Client::new(&cfg);
    s.create_schedule_group()
        .name("custom")
        .send()
        .await
        .unwrap();
    s.delete_schedule_group()
        .name("custom")
        .send()
        .await
        .unwrap();
}

#[tokio::test]
async fn e2e_scheduler_cannot_delete_default_group() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let s = aws_sdk_scheduler::Client::new(&cfg);
    let err = s.delete_schedule_group().name("default").send().await;
    assert!(err.is_err());
}
