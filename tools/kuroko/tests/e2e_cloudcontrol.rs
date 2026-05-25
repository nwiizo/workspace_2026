//! Cloud Control API E2E.
mod common;
use pretty_assertions::assert_eq;

#[tokio::test]
async fn e2e_cc_create_get_resource() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let c = aws_sdk_cloudcontrol::Client::new(&cfg);
    let res = c
        .create_resource()
        .type_name("AWS::S3::Bucket")
        .desired_state(r#"{"BucketName":"my-bucket"}"#)
        .send()
        .await
        .unwrap();
    let id = res
        .progress_event()
        .unwrap()
        .identifier()
        .unwrap()
        .to_string();
    let got = c
        .get_resource()
        .type_name("AWS::S3::Bucket")
        .identifier(&id)
        .send()
        .await
        .unwrap();
    assert_eq!(got.type_name(), Some("AWS::S3::Bucket"));
}

#[tokio::test]
async fn e2e_cc_list_resources() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let c = aws_sdk_cloudcontrol::Client::new(&cfg);
    for _ in 0..2 {
        c.create_resource()
            .type_name("AWS::SNS::Topic")
            .desired_state(r#"{"TopicName":"t"}"#)
            .send()
            .await
            .unwrap();
    }
    let res = c
        .list_resources()
        .type_name("AWS::SNS::Topic")
        .send()
        .await
        .unwrap();
    assert_eq!(res.resource_descriptions().len(), 2);
}

#[tokio::test]
async fn e2e_cc_get_unknown_fails() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let c = aws_sdk_cloudcontrol::Client::new(&cfg);
    let err = c
        .get_resource()
        .type_name("AWS::Lambda::Function")
        .identifier("nope")
        .send()
        .await;
    assert!(err.is_err());
}
