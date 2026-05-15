//! IAM E2E tests against AWS official API spec.
//!
//! References:
//! - CreateUser:    <https://docs.aws.amazon.com/IAM/latest/APIReference/API_CreateUser.html>
//! - CreateRole:    <https://docs.aws.amazon.com/IAM/latest/APIReference/API_CreateRole.html>
//! - CreatePolicy:  <https://docs.aws.amazon.com/IAM/latest/APIReference/API_CreatePolicy.html>
//! - CreateAccessKey: <https://docs.aws.amazon.com/IAM/latest/APIReference/API_CreateAccessKey.html>

mod common;

use pretty_assertions::assert_eq;

const ASSUME_ROLE: &str = r#"{"Version":"2012-10-17","Statement":[{"Effect":"Allow","Principal":{"Service":"lambda.amazonaws.com"},"Action":"sts:AssumeRole"}]}"#;
const POLICY_DOC: &str =
    r#"{"Version":"2012-10-17","Statement":[{"Effect":"Allow","Action":"s3:*","Resource":"*"}]}"#;

#[tokio::test]
async fn e2e_iam_create_then_get_user() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let iam = aws_sdk_iam::Client::new(&cfg);

    let res = iam.create_user().user_name("alice").send().await.unwrap();
    let user = res.user().unwrap();
    assert_eq!(user.user_name(), "alice");
    assert!(user.arn().starts_with("arn:aws:iam::"));

    let got = iam.get_user().user_name("alice").send().await.unwrap();
    assert_eq!(got.user().unwrap().user_name(), "alice");
}

#[tokio::test]
async fn e2e_iam_duplicate_user_fails() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let iam = aws_sdk_iam::Client::new(&cfg);

    iam.create_user().user_name("dup").send().await.unwrap();
    let err = iam.create_user().user_name("dup").send().await;
    assert!(
        err.is_err(),
        "duplicate user must return EntityAlreadyExists"
    );
}

#[tokio::test]
async fn e2e_iam_list_users() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let iam = aws_sdk_iam::Client::new(&cfg);

    for n in ["a", "b", "c"] {
        iam.create_user().user_name(n).send().await.unwrap();
    }
    let res = iam.list_users().send().await.unwrap();
    assert_eq!(res.users().len(), 3);
}

#[tokio::test]
async fn e2e_iam_create_role_with_assume_policy() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let iam = aws_sdk_iam::Client::new(&cfg);

    let res = iam
        .create_role()
        .role_name("lambda-exec")
        .assume_role_policy_document(ASSUME_ROLE)
        .send()
        .await
        .unwrap();
    let role = res.role().unwrap();
    assert_eq!(role.role_name(), "lambda-exec");
    assert!(role.arn().ends_with(":role/lambda-exec"));
}

#[tokio::test]
async fn e2e_iam_attach_then_list_role_policies() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let iam = aws_sdk_iam::Client::new(&cfg);

    iam.create_role()
        .role_name("r")
        .assume_role_policy_document(ASSUME_ROLE)
        .send()
        .await
        .unwrap();
    let p = iam
        .create_policy()
        .policy_name("read-s3")
        .policy_document(POLICY_DOC)
        .send()
        .await
        .unwrap();
    let arn = p.policy().unwrap().arn().unwrap().to_string();

    iam.attach_role_policy()
        .role_name("r")
        .policy_arn(&arn)
        .send()
        .await
        .unwrap();
    let res = iam
        .list_attached_role_policies()
        .role_name("r")
        .send()
        .await
        .unwrap();
    assert_eq!(res.attached_policies().len(), 1);
    assert_eq!(res.attached_policies()[0].policy_name(), Some("read-s3"));
}

#[tokio::test]
async fn e2e_iam_create_access_key_then_list() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let iam = aws_sdk_iam::Client::new(&cfg);

    iam.create_user().user_name("k").send().await.unwrap();
    let res = iam.create_access_key().user_name("k").send().await.unwrap();
    let ak = res.access_key().unwrap();
    assert!(ak.access_key_id().starts_with("AKIA"));
    assert_eq!(ak.user_name(), "k");

    let list = iam.list_access_keys().user_name("k").send().await.unwrap();
    assert_eq!(list.access_key_metadata().len(), 1);
}

#[tokio::test]
async fn e2e_iam_delete_user_removes_access_keys() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let iam = aws_sdk_iam::Client::new(&cfg);

    iam.create_user().user_name("doomed").send().await.unwrap();
    iam.create_access_key()
        .user_name("doomed")
        .send()
        .await
        .unwrap();
    iam.delete_user().user_name("doomed").send().await.unwrap();
    let err = iam.get_user().user_name("doomed").send().await;
    assert!(err.is_err(), "user must be gone after DeleteUser");
}
