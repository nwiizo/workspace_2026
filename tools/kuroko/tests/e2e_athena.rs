//! Athena E2E tests against AWS official API spec.
//! Refs: <https://docs.aws.amazon.com/athena/latest/APIReference/Welcome.html>

mod common;

use aws_sdk_athena::types::QueryExecutionState;
use pretty_assertions::assert_eq;

#[tokio::test]
async fn e2e_athena_start_query_returns_id_and_succeeds() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let a = aws_sdk_athena::Client::new(&cfg);
    let res = a
        .start_query_execution()
        .query_string("SELECT 1")
        .send()
        .await
        .unwrap();
    let id = res.query_execution_id().unwrap().to_string();
    assert!(!id.is_empty());
    let got = a
        .get_query_execution()
        .query_execution_id(&id)
        .send()
        .await
        .unwrap();
    assert_eq!(
        got.query_execution().unwrap().status().unwrap().state(),
        Some(&QueryExecutionState::Succeeded)
    );
}

#[tokio::test]
async fn e2e_athena_get_query_results_empty_set() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let a = aws_sdk_athena::Client::new(&cfg);
    let id = a
        .start_query_execution()
        .query_string("SELECT 1")
        .send()
        .await
        .unwrap()
        .query_execution_id()
        .unwrap()
        .to_string();
    let r = a
        .get_query_results()
        .query_execution_id(id)
        .send()
        .await
        .unwrap();
    assert_eq!(r.result_set().unwrap().rows().len(), 0);
}

#[tokio::test]
async fn e2e_athena_primary_workgroup_exists() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let a = aws_sdk_athena::Client::new(&cfg);
    let res = a.list_work_groups().send().await.unwrap();
    assert!(
        res.work_groups()
            .iter()
            .any(|w| w.name() == Some("primary"))
    );
}

#[tokio::test]
async fn e2e_athena_create_then_delete_workgroup() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let a = aws_sdk_athena::Client::new(&cfg);
    a.create_work_group()
        .name("analytics")
        .send()
        .await
        .unwrap();
    a.delete_work_group()
        .work_group("analytics")
        .send()
        .await
        .unwrap();
    let err = a.get_work_group().work_group("analytics").send().await;
    assert!(err.is_err());
}

#[tokio::test]
async fn e2e_athena_primary_workgroup_is_protected() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let a = aws_sdk_athena::Client::new(&cfg);
    let err = a.delete_work_group().work_group("primary").send().await;
    assert!(err.is_err());
}
