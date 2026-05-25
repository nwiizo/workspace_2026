//! EBS direct APIs E2E via raw HTTP.
mod common;
use pretty_assertions::assert_eq;
use serde_json::Value;

#[tokio::test]
async fn e2e_ebs_start_complete_snapshot() {
    let srv = common::spawn().await;
    let client = reqwest::Client::new();
    let create = client
        .post(format!("{}/snapshots", srv.endpoint))
        .json(&serde_json::json!({ "VolumeSize": 1 }))
        .send()
        .await
        .unwrap();
    let body: Value = create.json().await.unwrap();
    let id = body["SnapshotId"].as_str().unwrap().to_string();
    let complete = client
        .post(format!("{}/snapshots/completion/{}", srv.endpoint, id))
        .send()
        .await
        .unwrap();
    let body: Value = complete.json().await.unwrap();
    assert_eq!(body["Status"], "completed");
}

#[tokio::test]
async fn e2e_ebs_put_get_block() {
    let srv = common::spawn().await;
    let client = reqwest::Client::new();
    let create = client
        .post(format!("{}/snapshots", srv.endpoint))
        .json(&serde_json::json!({ "VolumeSize": 1 }))
        .send()
        .await
        .unwrap();
    let body: Value = create.json().await.unwrap();
    let id = body["SnapshotId"].as_str().unwrap().to_string();

    client
        .put(format!("{}/snapshots/{}/blocks/0", srv.endpoint, id))
        .body("hello world".to_string())
        .send()
        .await
        .unwrap();

    let block = client
        .get(format!("{}/snapshots/{}/blocks/0", srv.endpoint, id))
        .send()
        .await
        .unwrap();
    assert_eq!(block.bytes().await.unwrap(), &b"hello world"[..]);
}
