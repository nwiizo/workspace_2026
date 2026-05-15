//! Lambda E2E tests against AWS official API spec.
//!
//! kuroko stores function metadata and echoes invocation payloads — it does
//! not actually execute Lambda code. Tests verify the API contract (CRUD,
//! listing, configuration update) plus echo-Invoke.
//!
//! References:
//! - CreateFunction: <https://docs.aws.amazon.com/lambda/latest/api/API_CreateFunction.html>
//! - Invoke:         <https://docs.aws.amazon.com/lambda/latest/api/API_Invoke.html>

mod common;

use aws_sdk_lambda::primitives::Blob;
use aws_sdk_lambda::types::{FunctionCode, Runtime};
use pretty_assertions::assert_eq;

fn fixture_code() -> FunctionCode {
    FunctionCode::builder()
        .zip_file(Blob::new(b"fake-zip-bytes".to_vec()))
        .build()
}

#[tokio::test]
async fn e2e_lambda_create_returns_arn_and_runtime() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let lambda = aws_sdk_lambda::Client::new(&cfg);

    let res = lambda
        .create_function()
        .function_name("hello")
        .runtime(Runtime::Providedal2)
        .role("arn:aws:iam::000000000000:role/lambda")
        .handler("bootstrap")
        .code(fixture_code())
        .send()
        .await
        .unwrap();
    assert_eq!(res.function_name(), Some("hello"));
    let arn = res.function_arn().unwrap();
    assert!(arn.starts_with("arn:aws:lambda:"));
    assert!(arn.ends_with(":function:hello"));
}

#[tokio::test]
async fn e2e_lambda_duplicate_create_fails() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let lambda = aws_sdk_lambda::Client::new(&cfg);

    lambda
        .create_function()
        .function_name("dup")
        .runtime(Runtime::Providedal2)
        .role("arn:aws:iam::000000000000:role/lambda")
        .handler("h")
        .code(fixture_code())
        .send()
        .await
        .unwrap();
    let err = lambda
        .create_function()
        .function_name("dup")
        .runtime(Runtime::Providedal2)
        .role("arn:aws:iam::000000000000:role/lambda")
        .handler("h")
        .code(fixture_code())
        .send()
        .await;
    assert!(err.is_err(), "duplicate CreateFunction must fail");
}

#[tokio::test]
async fn e2e_lambda_get_function_returns_configuration_and_code() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let lambda = aws_sdk_lambda::Client::new(&cfg);

    lambda
        .create_function()
        .function_name("f")
        .runtime(Runtime::Providedal2)
        .role("arn:aws:iam::000000000000:role/lambda")
        .handler("h")
        .code(fixture_code())
        .send()
        .await
        .unwrap();
    let res = lambda
        .get_function()
        .function_name("f")
        .send()
        .await
        .unwrap();
    assert_eq!(res.configuration().unwrap().function_name(), Some("f"));
    assert!(res.code().is_some());
}

#[tokio::test]
async fn e2e_lambda_list_functions() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let lambda = aws_sdk_lambda::Client::new(&cfg);

    for n in ["one", "two", "three"] {
        lambda
            .create_function()
            .function_name(n)
            .runtime(Runtime::Providedal2)
            .role("arn:aws:iam::000000000000:role/lambda")
            .handler("h")
            .code(fixture_code())
            .send()
            .await
            .unwrap();
    }
    let res = lambda.list_functions().send().await.unwrap();
    assert_eq!(res.functions().len(), 3);
}

#[tokio::test]
async fn e2e_lambda_update_configuration_changes_handler() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let lambda = aws_sdk_lambda::Client::new(&cfg);

    lambda
        .create_function()
        .function_name("u")
        .runtime(Runtime::Providedal2)
        .role("arn:aws:iam::000000000000:role/lambda")
        .handler("old")
        .code(fixture_code())
        .send()
        .await
        .unwrap();
    let res = lambda
        .update_function_configuration()
        .function_name("u")
        .handler("new")
        .timeout(15)
        .send()
        .await
        .unwrap();
    assert_eq!(res.handler(), Some("new"));
    assert_eq!(res.timeout(), Some(15));
}

#[tokio::test]
async fn e2e_lambda_invoke_echoes_payload() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let lambda = aws_sdk_lambda::Client::new(&cfg);

    lambda
        .create_function()
        .function_name("echo")
        .runtime(Runtime::Providedal2)
        .role("arn:aws:iam::000000000000:role/lambda")
        .handler("h")
        .code(fixture_code())
        .send()
        .await
        .unwrap();
    let res = lambda
        .invoke()
        .function_name("echo")
        .payload(Blob::new(br#"{"hello":"kuroko"}"#.to_vec()))
        .send()
        .await
        .unwrap();
    let body = res.payload().unwrap().as_ref().to_vec();
    assert_eq!(body, br#"{"hello":"kuroko"}"#);
}

#[tokio::test]
async fn e2e_lambda_delete_then_get_404() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let lambda = aws_sdk_lambda::Client::new(&cfg);

    lambda
        .create_function()
        .function_name("d")
        .runtime(Runtime::Providedal2)
        .role("arn:aws:iam::000000000000:role/lambda")
        .handler("h")
        .code(fixture_code())
        .send()
        .await
        .unwrap();
    lambda
        .delete_function()
        .function_name("d")
        .send()
        .await
        .unwrap();
    let err = lambda.get_function().function_name("d").send().await;
    assert!(err.is_err(), "GetFunction must fail after DeleteFunction");
}
