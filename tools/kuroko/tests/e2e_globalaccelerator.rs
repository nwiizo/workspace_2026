//! Global Accelerator E2E tests.
mod common;
use pretty_assertions::assert_eq;

#[tokio::test]
async fn e2e_ga_create_then_describe() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let g = aws_sdk_globalaccelerator::Client::new(&cfg);
    let res = g
        .create_accelerator()
        .name("global-app")
        .idempotency_token("token")
        .send()
        .await
        .unwrap();
    let arn = res
        .accelerator()
        .unwrap()
        .accelerator_arn()
        .unwrap()
        .to_string();
    assert!(arn.contains(":accelerator/"));
    let d = g
        .describe_accelerator()
        .accelerator_arn(&arn)
        .send()
        .await
        .unwrap();
    assert_eq!(d.accelerator().unwrap().name(), Some("global-app"));
}

#[tokio::test]
async fn e2e_ga_list_accelerators() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let g = aws_sdk_globalaccelerator::Client::new(&cfg);
    for n in ["a", "b"] {
        g.create_accelerator()
            .name(n)
            .idempotency_token(n)
            .send()
            .await
            .unwrap();
    }
    let res = g.list_accelerators().send().await.unwrap();
    assert_eq!(res.accelerators().len(), 2);
}

#[tokio::test]
async fn e2e_ga_delete_accelerator() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let g = aws_sdk_globalaccelerator::Client::new(&cfg);
    let arn = g
        .create_accelerator()
        .name("d")
        .idempotency_token("d")
        .send()
        .await
        .unwrap()
        .accelerator()
        .unwrap()
        .accelerator_arn()
        .unwrap()
        .to_string();
    g.delete_accelerator()
        .accelerator_arn(&arn)
        .send()
        .await
        .unwrap();
    let err = g.describe_accelerator().accelerator_arn(&arn).send().await;
    assert!(err.is_err());
}
