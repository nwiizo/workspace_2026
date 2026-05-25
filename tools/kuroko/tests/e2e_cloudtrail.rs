//! CloudTrail E2E tests.
//! Refs: <https://docs.aws.amazon.com/awscloudtrail/latest/APIReference/Welcome.html>

mod common;

use pretty_assertions::assert_eq;

#[tokio::test]
async fn e2e_ct_create_then_get_trail() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let ct = aws_sdk_cloudtrail::Client::new(&cfg);
    ct.create_trail()
        .name("audit")
        .s3_bucket_name("audit-bucket")
        .send()
        .await
        .unwrap();
    let got = ct.get_trail().name("audit").send().await.unwrap();
    assert_eq!(got.trail().unwrap().name(), Some("audit"));
    assert!(
        got.trail()
            .unwrap()
            .trail_arn()
            .unwrap()
            .contains(":trail/audit")
    );
}

#[tokio::test]
async fn e2e_ct_duplicate_create_fails() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let ct = aws_sdk_cloudtrail::Client::new(&cfg);
    ct.create_trail()
        .name("dup")
        .s3_bucket_name("b")
        .send()
        .await
        .unwrap();
    let err = ct
        .create_trail()
        .name("dup")
        .s3_bucket_name("b")
        .send()
        .await;
    assert!(err.is_err());
}

#[tokio::test]
async fn e2e_ct_start_stop_logging_status() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let ct = aws_sdk_cloudtrail::Client::new(&cfg);
    ct.create_trail()
        .name("t")
        .s3_bucket_name("b")
        .send()
        .await
        .unwrap();
    ct.start_logging().name("t").send().await.unwrap();
    let st1 = ct.get_trail_status().name("t").send().await.unwrap();
    assert_eq!(st1.is_logging(), Some(true));
    ct.stop_logging().name("t").send().await.unwrap();
    let st2 = ct.get_trail_status().name("t").send().await.unwrap();
    assert_eq!(st2.is_logging(), Some(false));
}

#[tokio::test]
async fn e2e_ct_describe_trails_returns_all() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let ct = aws_sdk_cloudtrail::Client::new(&cfg);
    for n in ["a", "b"] {
        ct.create_trail()
            .name(n)
            .s3_bucket_name("b")
            .send()
            .await
            .unwrap();
    }
    let res = ct.describe_trails().send().await.unwrap();
    assert_eq!(res.trail_list().len(), 2);
}

#[tokio::test]
async fn e2e_ct_delete_then_404() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let ct = aws_sdk_cloudtrail::Client::new(&cfg);
    ct.create_trail()
        .name("doomed")
        .s3_bucket_name("b")
        .send()
        .await
        .unwrap();
    ct.delete_trail().name("doomed").send().await.unwrap();
    let err = ct.get_trail().name("doomed").send().await;
    assert!(err.is_err());
}
