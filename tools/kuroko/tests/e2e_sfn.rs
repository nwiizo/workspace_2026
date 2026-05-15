//! Step Functions E2E tests against AWS official API spec.
//!
//! References:
//! - CreateStateMachine: <https://docs.aws.amazon.com/step-functions/latest/apireference/API_CreateStateMachine.html>
//! - StartExecution:     <https://docs.aws.amazon.com/step-functions/latest/apireference/API_StartExecution.html>
//! - DescribeExecution:  <https://docs.aws.amazon.com/step-functions/latest/apireference/API_DescribeExecution.html>
//!
//! kuroko marks every started execution as SUCCEEDED with the input echoed
//! as the output (no ASL interpreter), so tests assert that contract.

mod common;

use aws_sdk_sfn::types::ExecutionStatus;
use pretty_assertions::assert_eq;

const DEFINITION: &str = r#"{"StartAt":"Done","States":{"Done":{"Type":"Pass","End":true}}}"#;

#[tokio::test]
async fn e2e_sfn_create_state_machine_returns_arn() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let sfn = aws_sdk_sfn::Client::new(&cfg);

    let res = sfn
        .create_state_machine()
        .name("wf")
        .definition(DEFINITION)
        .role_arn("arn:aws:iam::000000000000:role/sfn")
        .send()
        .await
        .unwrap();
    let arn = res.state_machine_arn();
    assert!(arn.starts_with("arn:aws:states:"));
    assert!(arn.ends_with(":stateMachine:wf"));
}

#[tokio::test]
async fn e2e_sfn_create_invalid_definition_fails() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let sfn = aws_sdk_sfn::Client::new(&cfg);

    let err = sfn
        .create_state_machine()
        .name("bad")
        .definition("not json")
        .role_arn("arn:aws:iam::000000000000:role/sfn")
        .send()
        .await;
    assert!(err.is_err(), "non-JSON definition must fail");
}

#[tokio::test]
async fn e2e_sfn_list_then_describe_state_machine() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let sfn = aws_sdk_sfn::Client::new(&cfg);

    let arn = sfn
        .create_state_machine()
        .name("wf2")
        .definition(DEFINITION)
        .role_arn("arn:aws:iam::000000000000:role/sfn")
        .send()
        .await
        .unwrap()
        .state_machine_arn()
        .to_string();

    let list = sfn.list_state_machines().send().await.unwrap();
    assert!(list.state_machines().iter().any(|m| m.name() == "wf2"));

    let desc = sfn
        .describe_state_machine()
        .state_machine_arn(&arn)
        .send()
        .await
        .unwrap();
    assert_eq!(desc.name(), "wf2");
    assert_eq!(desc.definition(), DEFINITION);
}

#[tokio::test]
async fn e2e_sfn_start_execution_succeeds_with_echoed_output() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let sfn = aws_sdk_sfn::Client::new(&cfg);

    let arn = sfn
        .create_state_machine()
        .name("wf3")
        .definition(DEFINITION)
        .role_arn("arn:aws:iam::000000000000:role/sfn")
        .send()
        .await
        .unwrap()
        .state_machine_arn()
        .to_string();

    let start = sfn
        .start_execution()
        .state_machine_arn(&arn)
        .input(r#"{"k":"v"}"#)
        .send()
        .await
        .unwrap();
    let exec_arn = start.execution_arn().to_string();

    let desc = sfn
        .describe_execution()
        .execution_arn(&exec_arn)
        .send()
        .await
        .unwrap();
    assert_eq!(desc.status(), &ExecutionStatus::Succeeded);
    assert_eq!(desc.input(), Some(r#"{"k":"v"}"#));
    assert_eq!(desc.output(), Some(r#"{"k":"v"}"#));
}

#[tokio::test]
async fn e2e_sfn_list_executions_filters_by_state_machine() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let sfn = aws_sdk_sfn::Client::new(&cfg);

    let arn = sfn
        .create_state_machine()
        .name("listed")
        .definition(DEFINITION)
        .role_arn("arn:aws:iam::000000000000:role/sfn")
        .send()
        .await
        .unwrap()
        .state_machine_arn()
        .to_string();
    for _ in 0..2 {
        sfn.start_execution()
            .state_machine_arn(&arn)
            .input("{}")
            .send()
            .await
            .unwrap();
    }
    let res = sfn
        .list_executions()
        .state_machine_arn(&arn)
        .send()
        .await
        .unwrap();
    assert_eq!(res.executions().len(), 2);
}

#[tokio::test]
async fn e2e_sfn_get_execution_history_returns_synthetic_events() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let sfn = aws_sdk_sfn::Client::new(&cfg);

    let arn = sfn
        .create_state_machine()
        .name("hist")
        .definition(DEFINITION)
        .role_arn("arn:aws:iam::000000000000:role/sfn")
        .send()
        .await
        .unwrap()
        .state_machine_arn()
        .to_string();
    let exec_arn = sfn
        .start_execution()
        .state_machine_arn(&arn)
        .input("{}")
        .send()
        .await
        .unwrap()
        .execution_arn()
        .to_string();
    let hist = sfn
        .get_execution_history()
        .execution_arn(&exec_arn)
        .send()
        .await
        .unwrap();
    // kuroko synthesizes a Started + Succeeded pair.
    assert_eq!(hist.events().len(), 2);
}

#[tokio::test]
async fn e2e_sfn_delete_state_machine_then_describe_404() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let sfn = aws_sdk_sfn::Client::new(&cfg);

    let arn = sfn
        .create_state_machine()
        .name("doomed")
        .definition(DEFINITION)
        .role_arn("arn:aws:iam::000000000000:role/sfn")
        .send()
        .await
        .unwrap()
        .state_machine_arn()
        .to_string();
    sfn.delete_state_machine()
        .state_machine_arn(&arn)
        .send()
        .await
        .unwrap();
    let err = sfn
        .describe_state_machine()
        .state_machine_arn(&arn)
        .send()
        .await;
    assert!(
        err.is_err(),
        "state machine must be gone after DeleteStateMachine"
    );
}
