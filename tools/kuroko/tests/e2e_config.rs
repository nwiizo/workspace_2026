//! AWS Config E2E.
//! Refs: <https://docs.aws.amazon.com/config/latest/APIReference/Welcome.html>

mod common;
use aws_sdk_config::types::{ConfigRule, Owner, Source};
use pretty_assertions::assert_eq;

fn rule(name: &str) -> ConfigRule {
    ConfigRule::builder()
        .config_rule_name(name)
        .source(
            Source::builder()
                .owner(Owner::Aws)
                .source_identifier("S3_BUCKET_PUBLIC_READ_PROHIBITED")
                .build()
                .unwrap(),
        )
        .build()
}

#[tokio::test]
async fn e2e_config_put_describe_rule() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let c = aws_sdk_config::Client::new(&cfg);
    c.put_config_rule()
        .config_rule(rule("r1"))
        .send()
        .await
        .unwrap();
    let res = c.describe_config_rules().send().await.unwrap();
    assert_eq!(res.config_rules().len(), 1);
    assert_eq!(res.config_rules()[0].config_rule_name(), Some("r1"));
}

#[tokio::test]
async fn e2e_config_delete_rule() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let c = aws_sdk_config::Client::new(&cfg);
    c.put_config_rule()
        .config_rule(rule("r2"))
        .send()
        .await
        .unwrap();
    c.delete_config_rule()
        .config_rule_name("r2")
        .send()
        .await
        .unwrap();
    let res = c.describe_config_rules().send().await.unwrap();
    assert_eq!(res.config_rules().len(), 0);
}

#[tokio::test]
async fn e2e_config_delete_missing_fails() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let c = aws_sdk_config::Client::new(&cfg);
    let err = c.delete_config_rule().config_rule_name("nope").send().await;
    assert!(err.is_err());
}
