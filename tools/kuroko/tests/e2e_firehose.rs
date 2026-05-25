//! Firehose E2E tests against AWS official API spec.
//! Refs: <https://docs.aws.amazon.com/firehose/latest/APIReference/Welcome.html>

mod common;

use aws_sdk_firehose::primitives::Blob;
use aws_sdk_firehose::types::Record;
use pretty_assertions::assert_eq;

#[tokio::test]
async fn e2e_firehose_create_then_describe() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let fh = aws_sdk_firehose::Client::new(&cfg);
    let res = fh
        .create_delivery_stream()
        .delivery_stream_name("logs")
        .send()
        .await
        .unwrap();
    assert!(
        res.delivery_stream_arn()
            .unwrap()
            .contains(":deliverystream/logs")
    );
    let d = fh
        .describe_delivery_stream()
        .delivery_stream_name("logs")
        .send()
        .await
        .unwrap();
    let desc = d.delivery_stream_description().unwrap();
    assert_eq!(desc.delivery_stream_name(), "logs");
}

#[tokio::test]
async fn e2e_firehose_duplicate_create_fails() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let fh = aws_sdk_firehose::Client::new(&cfg);
    fh.create_delivery_stream()
        .delivery_stream_name("dup")
        .send()
        .await
        .unwrap();
    let err = fh
        .create_delivery_stream()
        .delivery_stream_name("dup")
        .send()
        .await;
    assert!(err.is_err());
}

#[tokio::test]
async fn e2e_firehose_list_then_delete() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let fh = aws_sdk_firehose::Client::new(&cfg);
    for n in ["a", "b"] {
        fh.create_delivery_stream()
            .delivery_stream_name(n)
            .send()
            .await
            .unwrap();
    }
    let list = fh.list_delivery_streams().send().await.unwrap();
    assert_eq!(list.delivery_stream_names().len(), 2);
    fh.delete_delivery_stream()
        .delivery_stream_name("a")
        .send()
        .await
        .unwrap();
    let list2 = fh.list_delivery_streams().send().await.unwrap();
    assert_eq!(list2.delivery_stream_names().len(), 1);
}

#[tokio::test]
async fn e2e_firehose_put_record_returns_id() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let fh = aws_sdk_firehose::Client::new(&cfg);
    fh.create_delivery_stream()
        .delivery_stream_name("s")
        .send()
        .await
        .unwrap();
    let res = fh
        .put_record()
        .delivery_stream_name("s")
        .record(
            Record::builder()
                .data(Blob::new(b"hello".to_vec()))
                .build()
                .unwrap(),
        )
        .send()
        .await
        .unwrap();
    assert!(!res.record_id().is_empty());
}

#[tokio::test]
async fn e2e_firehose_put_record_batch() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let fh = aws_sdk_firehose::Client::new(&cfg);
    fh.create_delivery_stream()
        .delivery_stream_name("s")
        .send()
        .await
        .unwrap();
    let recs = (0..3)
        .map(|i| {
            Record::builder()
                .data(Blob::new(format!("rec-{i}").into_bytes()))
                .build()
                .unwrap()
        })
        .collect::<Vec<_>>();
    let res = fh
        .put_record_batch()
        .delivery_stream_name("s")
        .set_records(Some(recs))
        .send()
        .await
        .unwrap();
    assert_eq!(res.failed_put_count(), 0);
    assert_eq!(res.request_responses().len(), 3);
}
