//! Pinpoint SMS Voice V2 E2E.
mod common;
use pretty_assertions::assert_eq;

#[tokio::test]
async fn e2e_pp_send_text() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let c = aws_sdk_pinpointsmsvoicev2::Client::new(&cfg);
    let res = c
        .send_text_message()
        .destination_phone_number("+15551234567")
        .message_body("hello")
        .send()
        .await
        .unwrap();
    assert!(res.message_id().is_some());
}

#[tokio::test]
async fn e2e_pp_describe_pools_empty() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let c = aws_sdk_pinpointsmsvoicev2::Client::new(&cfg);
    let res = c.describe_pools().send().await.unwrap();
    assert_eq!(res.pools().len(), 0);
}
