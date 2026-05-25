//! Organizations E2E tests.
mod common;
use pretty_assertions::assert_eq;

#[tokio::test]
async fn e2e_org_create_then_describe() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let o = aws_sdk_organizations::Client::new(&cfg);
    let res = o.create_organization().send().await.unwrap();
    let org = res.organization().unwrap();
    assert!(org.id().unwrap().starts_with("o-"));
    let desc = o.describe_organization().send().await.unwrap();
    assert_eq!(desc.organization().unwrap().id(), org.id());
}

#[tokio::test]
async fn e2e_org_duplicate_create_fails() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let o = aws_sdk_organizations::Client::new(&cfg);
    o.create_organization().send().await.unwrap();
    let err = o.create_organization().send().await;
    assert!(err.is_err());
}

#[tokio::test]
async fn e2e_org_create_ou() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let o = aws_sdk_organizations::Client::new(&cfg);
    o.create_organization().send().await.unwrap();
    let res = o
        .create_organizational_unit()
        .name("dev")
        .parent_id("r-root")
        .send()
        .await
        .unwrap();
    assert_eq!(res.organizational_unit().unwrap().name(), Some("dev"));
}

#[tokio::test]
async fn e2e_org_list_accounts() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let o = aws_sdk_organizations::Client::new(&cfg);
    o.create_organization().send().await.unwrap();
    o.create_account()
        .email("acct@kuroko.test")
        .account_name("acct")
        .send()
        .await
        .unwrap();
    let res = o.list_accounts().send().await.unwrap();
    assert_eq!(res.accounts().len(), 1);
}
