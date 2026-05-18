//! CloudFormation E2E tests against AWS official API spec.
//!
//! References:
//! - CreateStack:    <https://docs.aws.amazon.com/AWSCloudFormation/latest/APIReference/API_CreateStack.html>
//! - DescribeStacks: <https://docs.aws.amazon.com/AWSCloudFormation/latest/APIReference/API_DescribeStacks.html>
//! - DeleteStack:    <https://docs.aws.amazon.com/AWSCloudFormation/latest/APIReference/API_DeleteStack.html>

mod common;

use aws_sdk_cloudformation::types::{Parameter, StackStatus, Tag};
use pretty_assertions::assert_eq;

const TEMPLATE: &str = r#"{"AWSTemplateFormatVersion":"2010-09-09","Resources":{}}"#;

#[tokio::test]
async fn e2e_cfn_create_stack_returns_stack_id() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let cfn = aws_sdk_cloudformation::Client::new(&cfg);

    let res = cfn
        .create_stack()
        .stack_name("net")
        .template_body(TEMPLATE)
        .send()
        .await
        .unwrap();
    let id = res.stack_id().unwrap();
    assert!(id.starts_with("arn:aws:cloudformation:"));
    assert!(id.contains(":stack/net/"));
}

#[tokio::test]
async fn e2e_cfn_duplicate_create_fails() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let cfn = aws_sdk_cloudformation::Client::new(&cfg);

    cfn.create_stack()
        .stack_name("dup")
        .template_body(TEMPLATE)
        .send()
        .await
        .unwrap();
    let err = cfn
        .create_stack()
        .stack_name("dup")
        .template_body(TEMPLATE)
        .send()
        .await;
    assert!(
        err.is_err(),
        "duplicate stack must fail with AlreadyExistsException"
    );
}

#[tokio::test]
async fn e2e_cfn_describe_stack_returns_complete_status() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let cfn = aws_sdk_cloudformation::Client::new(&cfg);

    cfn.create_stack()
        .stack_name("desc")
        .template_body(TEMPLATE)
        .parameters(
            Parameter::builder()
                .parameter_key("Env")
                .parameter_value("prod")
                .build(),
        )
        .tags(Tag::builder().key("Owner").value("kuroko").build())
        .send()
        .await
        .unwrap();
    let res = cfn
        .describe_stacks()
        .stack_name("desc")
        .send()
        .await
        .unwrap();
    let stack = &res.stacks()[0];
    assert_eq!(stack.stack_name(), Some("desc"));
    assert_eq!(stack.stack_status(), Some(&StackStatus::CreateComplete));
    assert!(
        stack
            .parameters()
            .iter()
            .any(|p| p.parameter_key() == Some("Env") && p.parameter_value() == Some("prod"))
    );
    assert!(
        stack
            .tags()
            .iter()
            .any(|t| t.key() == Some("Owner") && t.value() == Some("kuroko"))
    );
}

#[tokio::test]
async fn e2e_cfn_describe_stack_missing_fails() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let cfn = aws_sdk_cloudformation::Client::new(&cfg);

    let err = cfn.describe_stacks().stack_name("nope").send().await;
    assert!(err.is_err(), "missing stack must return ValidationError");
}

#[tokio::test]
async fn e2e_cfn_update_stack_transitions_to_update_complete() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let cfn = aws_sdk_cloudformation::Client::new(&cfg);

    cfn.create_stack()
        .stack_name("up")
        .template_body(TEMPLATE)
        .send()
        .await
        .unwrap();
    cfn.update_stack()
        .stack_name("up")
        .template_body(TEMPLATE)
        .send()
        .await
        .unwrap();
    let res = cfn.describe_stacks().stack_name("up").send().await.unwrap();
    assert_eq!(
        res.stacks()[0].stack_status(),
        Some(&StackStatus::UpdateComplete)
    );
}

#[tokio::test]
async fn e2e_cfn_list_stacks() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let cfn = aws_sdk_cloudformation::Client::new(&cfg);

    for n in ["a", "b", "c"] {
        cfn.create_stack()
            .stack_name(n)
            .template_body(TEMPLATE)
            .send()
            .await
            .unwrap();
    }
    let res = cfn.list_stacks().send().await.unwrap();
    assert_eq!(res.stack_summaries().len(), 3);
}

#[tokio::test]
async fn e2e_cfn_delete_stack_then_describe_fails() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let cfn = aws_sdk_cloudformation::Client::new(&cfg);

    cfn.create_stack()
        .stack_name("doomed")
        .template_body(TEMPLATE)
        .send()
        .await
        .unwrap();
    cfn.delete_stack()
        .stack_name("doomed")
        .send()
        .await
        .unwrap();
    let err = cfn.describe_stacks().stack_name("doomed").send().await;
    assert!(err.is_err(), "stack must be gone");
}
