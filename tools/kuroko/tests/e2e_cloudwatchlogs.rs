//! CloudWatch Logs E2E tests against AWS official API spec.
//!
//! References:
//! - CreateLogGroup:  <https://docs.aws.amazon.com/AmazonCloudWatchLogs/latest/APIReference/API_CreateLogGroup.html>
//! - PutLogEvents:    <https://docs.aws.amazon.com/AmazonCloudWatchLogs/latest/APIReference/API_PutLogEvents.html>
//! - GetLogEvents:    <https://docs.aws.amazon.com/AmazonCloudWatchLogs/latest/APIReference/API_GetLogEvents.html>
//! - FilterLogEvents: <https://docs.aws.amazon.com/AmazonCloudWatchLogs/latest/APIReference/API_FilterLogEvents.html>

mod common;

use aws_sdk_cloudwatchlogs::types::InputLogEvent;
use pretty_assertions::assert_eq;

async fn make_group_and_stream(client: &aws_sdk_cloudwatchlogs::Client, group: &str, stream: &str) {
    client
        .create_log_group()
        .log_group_name(group)
        .send()
        .await
        .unwrap();
    client
        .create_log_stream()
        .log_group_name(group)
        .log_stream_name(stream)
        .send()
        .await
        .unwrap();
}

#[tokio::test]
async fn e2e_logs_create_group_and_describe() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let logs = aws_sdk_cloudwatchlogs::Client::new(&cfg);

    logs.create_log_group()
        .log_group_name("g1")
        .send()
        .await
        .unwrap();
    let res = logs.describe_log_groups().send().await.unwrap();
    let names: Vec<&str> = res
        .log_groups()
        .iter()
        .filter_map(|g| g.log_group_name())
        .collect();
    assert!(names.contains(&"g1"));
}

#[tokio::test]
async fn e2e_logs_create_group_duplicate_fails() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let logs = aws_sdk_cloudwatchlogs::Client::new(&cfg);
    logs.create_log_group()
        .log_group_name("dup")
        .send()
        .await
        .unwrap();
    let err = logs.create_log_group().log_group_name("dup").send().await;
    assert!(err.is_err(), "duplicate CreateLogGroup must fail");
}

#[tokio::test]
async fn e2e_logs_put_and_get_events() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let logs = aws_sdk_cloudwatchlogs::Client::new(&cfg);
    make_group_and_stream(&logs, "g", "s").await;

    let ts = chrono::Utc::now().timestamp_millis();
    let events = (0..3)
        .map(|i| {
            InputLogEvent::builder()
                .timestamp(ts + i)
                .message(format!("msg-{i}"))
                .build()
                .unwrap()
        })
        .collect::<Vec<_>>();
    logs.put_log_events()
        .log_group_name("g")
        .log_stream_name("s")
        .set_log_events(Some(events))
        .send()
        .await
        .unwrap();

    let got = logs
        .get_log_events()
        .log_group_name("g")
        .log_stream_name("s")
        .send()
        .await
        .unwrap();
    let messages: Vec<&str> = got.events().iter().filter_map(|e| e.message()).collect();
    assert_eq!(messages, vec!["msg-0", "msg-1", "msg-2"]);
}

#[tokio::test]
async fn e2e_logs_filter_log_events_by_substring() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let logs = aws_sdk_cloudwatchlogs::Client::new(&cfg);
    make_group_and_stream(&logs, "g", "s").await;

    let ts = chrono::Utc::now().timestamp_millis();
    let events: Vec<InputLogEvent> = ["ERROR something", "INFO normal", "ERROR another"]
        .iter()
        .enumerate()
        .map(|(i, m)| {
            InputLogEvent::builder()
                .timestamp(ts + i as i64)
                .message(*m)
                .build()
                .unwrap()
        })
        .collect();
    logs.put_log_events()
        .log_group_name("g")
        .log_stream_name("s")
        .set_log_events(Some(events))
        .send()
        .await
        .unwrap();

    let res = logs
        .filter_log_events()
        .log_group_name("g")
        .filter_pattern("ERROR")
        .send()
        .await
        .unwrap();
    assert_eq!(res.events().len(), 2);
}

#[tokio::test]
async fn e2e_logs_describe_streams_in_group() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let logs = aws_sdk_cloudwatchlogs::Client::new(&cfg);
    make_group_and_stream(&logs, "g", "alpha").await;
    logs.create_log_stream()
        .log_group_name("g")
        .log_stream_name("beta")
        .send()
        .await
        .unwrap();
    let res = logs
        .describe_log_streams()
        .log_group_name("g")
        .send()
        .await
        .unwrap();
    let names: Vec<&str> = res
        .log_streams()
        .iter()
        .filter_map(|s| s.log_stream_name())
        .collect();
    assert_eq!(names.len(), 2);
}
