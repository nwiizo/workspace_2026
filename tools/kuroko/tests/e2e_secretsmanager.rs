//! Secrets Manager E2E tests against AWS official API spec.
//!
//! References:
//! - CreateSecret:    <https://docs.aws.amazon.com/secretsmanager/latest/apireference/API_CreateSecret.html>
//! - GetSecretValue:  <https://docs.aws.amazon.com/secretsmanager/latest/apireference/API_GetSecretValue.html>
//! - PutSecretValue:  <https://docs.aws.amazon.com/secretsmanager/latest/apireference/API_PutSecretValue.html>
//! - DescribeSecret:  <https://docs.aws.amazon.com/secretsmanager/latest/apireference/API_DescribeSecret.html>

mod common;

use pretty_assertions::assert_eq;

#[tokio::test]
async fn e2e_secrets_create_then_get_value() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let sm = aws_sdk_secretsmanager::Client::new(&cfg);

    let res = sm
        .create_secret()
        .name("db-password")
        .secret_string("hunter2")
        .send()
        .await
        .unwrap();
    let arn = res.arn().unwrap();
    assert!(arn.starts_with("arn:aws:secretsmanager:"));
    assert!(arn.contains(":secret:db-password-"));

    let got = sm
        .get_secret_value()
        .secret_id("db-password")
        .send()
        .await
        .unwrap();
    assert_eq!(got.secret_string(), Some("hunter2"));
    assert!(got.version_stages().iter().any(|s| s == "AWSCURRENT"));
}

#[tokio::test]
async fn e2e_secrets_duplicate_create_fails() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let sm = aws_sdk_secretsmanager::Client::new(&cfg);

    sm.create_secret()
        .name("dup")
        .secret_string("v1")
        .send()
        .await
        .unwrap();
    let err = sm
        .create_secret()
        .name("dup")
        .secret_string("v2")
        .send()
        .await;
    assert!(
        err.is_err(),
        "duplicate create must fail with ResourceExistsException"
    );
}

#[tokio::test]
async fn e2e_secrets_put_value_moves_previous_current_to_previous() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let sm = aws_sdk_secretsmanager::Client::new(&cfg);

    sm.create_secret()
        .name("rot")
        .secret_string("v1")
        .send()
        .await
        .unwrap();
    sm.put_secret_value()
        .secret_id("rot")
        .secret_string("v2")
        .send()
        .await
        .unwrap();

    let current = sm.get_secret_value().secret_id("rot").send().await.unwrap();
    assert_eq!(current.secret_string(), Some("v2"));

    let previous = sm
        .get_secret_value()
        .secret_id("rot")
        .version_stage("AWSPREVIOUS")
        .send()
        .await
        .unwrap();
    assert_eq!(previous.secret_string(), Some("v1"));
}

#[tokio::test]
async fn e2e_secrets_describe_returns_versions_to_stages() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let sm = aws_sdk_secretsmanager::Client::new(&cfg);

    sm.create_secret()
        .name("d")
        .secret_string("x")
        .description("e2e")
        .send()
        .await
        .unwrap();
    let desc = sm.describe_secret().secret_id("d").send().await.unwrap();
    assert_eq!(desc.description(), Some("e2e"));
    assert!(desc.version_ids_to_stages().is_some());
}

#[tokio::test]
async fn e2e_secrets_list_then_delete_force() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let sm = aws_sdk_secretsmanager::Client::new(&cfg);

    for n in ["a", "b", "c"] {
        sm.create_secret()
            .name(n)
            .secret_string("x")
            .send()
            .await
            .unwrap();
    }
    let list = sm.list_secrets().send().await.unwrap();
    let names: Vec<&str> = list.secret_list().iter().filter_map(|s| s.name()).collect();
    assert_eq!(names.len(), 3);

    sm.delete_secret()
        .secret_id("a")
        .force_delete_without_recovery(true)
        .send()
        .await
        .unwrap();
    let err = sm.describe_secret().secret_id("a").send().await;
    assert!(err.is_err(), "force-deleted secret must be gone");
}

#[tokio::test]
async fn e2e_secrets_get_by_arn_accepts_full_arn() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let sm = aws_sdk_secretsmanager::Client::new(&cfg);

    let res = sm
        .create_secret()
        .name("arn-test")
        .secret_string("v")
        .send()
        .await
        .unwrap();
    let arn = res.arn().unwrap().to_string();
    let got = sm.get_secret_value().secret_id(arn).send().await.unwrap();
    assert_eq!(got.secret_string(), Some("v"));
}
