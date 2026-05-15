//! Kinesis E2E tests against AWS official API spec.
//!
//! References:
//! - CreateStream:      <https://docs.aws.amazon.com/kinesis/latest/APIReference/API_CreateStream.html>
//! - PutRecord:         <https://docs.aws.amazon.com/kinesis/latest/APIReference/API_PutRecord.html>
//! - GetShardIterator:  <https://docs.aws.amazon.com/kinesis/latest/APIReference/API_GetShardIterator.html>
//! - GetRecords:        <https://docs.aws.amazon.com/kinesis/latest/APIReference/API_GetRecords.html>

mod common;

use aws_sdk_kinesis::primitives::Blob;
use aws_sdk_kinesis::types::{PutRecordsRequestEntry, ShardIteratorType, StreamStatus};
use pretty_assertions::assert_eq;

#[tokio::test]
async fn e2e_kinesis_create_then_describe_stream() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let k = aws_sdk_kinesis::Client::new(&cfg);

    k.create_stream()
        .stream_name("logs")
        .shard_count(1)
        .send()
        .await
        .unwrap();
    let res = k
        .describe_stream()
        .stream_name("logs")
        .send()
        .await
        .unwrap();
    let desc = res.stream_description().unwrap();
    assert_eq!(desc.stream_name(), "logs");
    assert_eq!(desc.stream_status(), &StreamStatus::Active);
    assert_eq!(desc.shards().len(), 1);
}

#[tokio::test]
async fn e2e_kinesis_duplicate_create_fails() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let k = aws_sdk_kinesis::Client::new(&cfg);

    k.create_stream().stream_name("dup").send().await.unwrap();
    let err = k.create_stream().stream_name("dup").send().await;
    assert!(err.is_err(), "duplicate CreateStream must fail");
}

#[tokio::test]
async fn e2e_kinesis_list_streams_includes_created() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let k = aws_sdk_kinesis::Client::new(&cfg);

    for n in ["s1", "s2"] {
        k.create_stream().stream_name(n).send().await.unwrap();
    }
    let res = k.list_streams().send().await.unwrap();
    assert!(res.stream_names().contains(&"s1".to_string()));
    assert!(res.stream_names().contains(&"s2".to_string()));
}

#[tokio::test]
async fn e2e_kinesis_put_record_then_get_records_via_trim_horizon() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let k = aws_sdk_kinesis::Client::new(&cfg);

    k.create_stream().stream_name("s").send().await.unwrap();
    for i in 0..3 {
        k.put_record()
            .stream_name("s")
            .partition_key(format!("pk{i}"))
            .data(Blob::new(format!("event-{i}").into_bytes()))
            .send()
            .await
            .unwrap();
    }

    let it = k
        .get_shard_iterator()
        .stream_name("s")
        .shard_id("shardId-000000000000")
        .shard_iterator_type(ShardIteratorType::TrimHorizon)
        .send()
        .await
        .unwrap();
    let iter = it.shard_iterator().unwrap();
    let res = k.get_records().shard_iterator(iter).send().await.unwrap();
    let records = res.records();
    assert_eq!(records.len(), 3);
    let payloads: Vec<Vec<u8>> = records.iter().map(|r| r.data().as_ref().to_vec()).collect();
    assert_eq!(payloads[0], b"event-0");
    assert_eq!(payloads[2], b"event-2");
}

#[tokio::test]
async fn e2e_kinesis_get_shard_iterator_latest_yields_no_existing_records() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let k = aws_sdk_kinesis::Client::new(&cfg);

    k.create_stream().stream_name("s").send().await.unwrap();
    k.put_record()
        .stream_name("s")
        .partition_key("pk")
        .data(Blob::new(b"before".to_vec()))
        .send()
        .await
        .unwrap();

    // Iterator at LATEST should skip all existing records.
    let it = k
        .get_shard_iterator()
        .stream_name("s")
        .shard_id("shardId-000000000000")
        .shard_iterator_type(ShardIteratorType::Latest)
        .send()
        .await
        .unwrap();
    let res = k
        .get_records()
        .shard_iterator(it.shard_iterator().unwrap())
        .send()
        .await
        .unwrap();
    assert_eq!(res.records().len(), 0);

    // A subsequent put is visible via the next iterator.
    k.put_record()
        .stream_name("s")
        .partition_key("pk")
        .data(Blob::new(b"after".to_vec()))
        .send()
        .await
        .unwrap();
    let it2 = k
        .get_shard_iterator()
        .stream_name("s")
        .shard_id("shardId-000000000000")
        .shard_iterator_type(ShardIteratorType::TrimHorizon)
        .send()
        .await
        .unwrap();
    let res2 = k
        .get_records()
        .shard_iterator(it2.shard_iterator().unwrap())
        .send()
        .await
        .unwrap();
    assert_eq!(res2.records().len(), 2);
}

#[tokio::test]
async fn e2e_kinesis_put_records_batch() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let k = aws_sdk_kinesis::Client::new(&cfg);

    k.create_stream().stream_name("s").send().await.unwrap();
    let entries: Vec<_> = (0..3)
        .map(|i| {
            PutRecordsRequestEntry::builder()
                .partition_key(format!("k{i}"))
                .data(Blob::new(format!("d{i}").into_bytes()))
                .build()
                .unwrap()
        })
        .collect();
    let res = k
        .put_records()
        .stream_name("s")
        .set_records(Some(entries))
        .send()
        .await
        .unwrap();
    assert_eq!(res.failed_record_count(), Some(0));
    assert_eq!(res.records().len(), 3);
}

#[tokio::test]
async fn e2e_kinesis_delete_stream_then_describe_404() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let k = aws_sdk_kinesis::Client::new(&cfg);

    k.create_stream()
        .stream_name("doomed")
        .send()
        .await
        .unwrap();
    k.delete_stream()
        .stream_name("doomed")
        .send()
        .await
        .unwrap();
    let err = k.describe_stream().stream_name("doomed").send().await;
    assert!(err.is_err(), "stream must be gone after DeleteStream");
}
