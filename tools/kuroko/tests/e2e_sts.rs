//! STS E2E tests against AWS official API spec.
//!
//! References:
//! - GetCallerIdentity: <https://docs.aws.amazon.com/STS/latest/APIReference/API_GetCallerIdentity.html>
//! - AssumeRole:        <https://docs.aws.amazon.com/STS/latest/APIReference/API_AssumeRole.html>
//! - GetSessionToken:   <https://docs.aws.amazon.com/STS/latest/APIReference/API_GetSessionToken.html>

mod common;

use pretty_assertions::assert_eq;

#[tokio::test]
async fn e2e_sts_get_caller_identity_returns_account_and_arn() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let sts = aws_sdk_sts::Client::new(&cfg);

    let res = sts.get_caller_identity().send().await.unwrap();
    let account = res.account().unwrap();
    assert_eq!(account, "000000000000");
    let arn = res.arn().unwrap();
    assert!(arn.starts_with("arn:aws:iam::"));
}

#[tokio::test]
async fn e2e_sts_assume_role_returns_credentials() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let sts = aws_sdk_sts::Client::new(&cfg);

    let res = sts
        .assume_role()
        .role_arn("arn:aws:iam::000000000000:role/test")
        .role_session_name("kuroko-session")
        .send()
        .await
        .unwrap();
    let creds = res.credentials().unwrap();
    assert!(creds.access_key_id().starts_with("ASIA"));
    assert!(!creds.secret_access_key().is_empty());
    assert!(!creds.session_token().is_empty());
}

#[tokio::test]
async fn e2e_sts_get_session_token_returns_credentials() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let sts = aws_sdk_sts::Client::new(&cfg);

    let res = sts.get_session_token().send().await.unwrap();
    let creds = res.credentials().unwrap();
    assert!(creds.access_key_id().starts_with("ASIA"));
    assert!(!creds.session_token().is_empty());
}
