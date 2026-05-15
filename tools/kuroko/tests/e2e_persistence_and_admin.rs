//! Cross-cutting E2E tests for kuroko-specific behavior:
//! - JSON-snapshot persistence end-to-end (write, snapshot, restart, restore)
//! - `/_kuroko/reset` clears all service state
//! - `/_kuroko/info`, `/_kuroko/health`, `/_kuroko/services` introspection

mod common;

use aws_sdk_s3::primitives::ByteStream;

#[tokio::test]
async fn e2e_persistence_s3_survives_restart() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().to_path_buf();

    // Phase 1: write state through SDK, then snapshot to disk.
    {
        let srv = common::spawn_with_data_dir(path.clone()).await;
        let cfg = common::aws_config(&srv.endpoint).await;
        let s3 = common::s3_client(&cfg);
        s3.create_bucket().bucket("survive").send().await.unwrap();
        s3.put_object()
            .bucket("survive")
            .key("k")
            .body(ByteStream::from_static(b"persisted"))
            .send()
            .await
            .unwrap();
        srv.snapshot_all();
    }

    // Snapshot files must exist now.
    assert!(
        path.join("s3.json").exists(),
        "s3 snapshot must exist on disk"
    );

    // Phase 2: spawn a fresh instance pointing at the same data dir and verify
    // the bucket+object are restored.
    {
        let srv = common::spawn_with_data_dir(path.clone()).await;
        let cfg = common::aws_config(&srv.endpoint).await;
        let s3 = common::s3_client(&cfg);
        let got = s3
            .get_object()
            .bucket("survive")
            .key("k")
            .send()
            .await
            .expect("object must be restored from snapshot");
        let body = got.body.collect().await.unwrap().into_bytes();
        assert_eq!(&body[..], b"persisted");
    }
}

#[tokio::test]
async fn e2e_persistence_snapshot_file_contains_bucket_name() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().to_path_buf();

    let srv = common::spawn_with_data_dir(path.clone()).await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let s3 = common::s3_client(&cfg);
    s3.create_bucket()
        .bucket("inspect-me")
        .send()
        .await
        .unwrap();
    srv.snapshot_all();

    let raw = std::fs::read_to_string(path.join("s3.json")).unwrap();
    assert!(
        raw.contains("inspect-me"),
        "snapshot must contain the bucket name; got: {raw}"
    );
}

#[tokio::test]
async fn e2e_persistence_sqs_survives_restart() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().to_path_buf();
    let queue_url;

    {
        let srv = common::spawn_with_data_dir(path.clone()).await;
        let cfg = common::aws_config(&srv.endpoint).await;
        let sqs = aws_sdk_sqs::Client::new(&cfg);
        let q = sqs
            .create_queue()
            .queue_name("durable")
            .send()
            .await
            .unwrap();
        queue_url = q.queue_url().unwrap().to_string();
        sqs.send_message()
            .queue_url(&queue_url)
            .message_body("durable-msg")
            .send()
            .await
            .unwrap();
        srv.snapshot_all();
    }

    {
        let srv = common::spawn_with_data_dir(path.clone()).await;
        let cfg = common::aws_config(&srv.endpoint).await;
        let sqs = aws_sdk_sqs::Client::new(&cfg);
        // Re-fetch the URL: it is reconstructed from the queue name on restore.
        let url = sqs
            .get_queue_url()
            .queue_name("durable")
            .send()
            .await
            .unwrap()
            .queue_url()
            .unwrap()
            .to_string();
        let r = sqs.receive_message().queue_url(&url).send().await.unwrap();
        assert_eq!(r.messages().len(), 1);
        assert_eq!(r.messages()[0].body(), Some("durable-msg"));
    }
}

#[tokio::test]
async fn e2e_persistence_dynamodb_survives_restart() {
    use aws_sdk_dynamodb::types::{
        AttributeDefinition, AttributeValue, KeySchemaElement, KeyType, ScalarAttributeType,
    };

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().to_path_buf();

    {
        let srv = common::spawn_with_data_dir(path.clone()).await;
        let cfg = common::aws_config(&srv.endpoint).await;
        let ddb = aws_sdk_dynamodb::Client::new(&cfg);
        ddb.create_table()
            .table_name("dur")
            .key_schema(
                KeySchemaElement::builder()
                    .attribute_name("pk")
                    .key_type(KeyType::Hash)
                    .build()
                    .unwrap(),
            )
            .attribute_definitions(
                AttributeDefinition::builder()
                    .attribute_name("pk")
                    .attribute_type(ScalarAttributeType::S)
                    .build()
                    .unwrap(),
            )
            .send()
            .await
            .unwrap();
        ddb.put_item()
            .table_name("dur")
            .item("pk", AttributeValue::S("a".into()))
            .item("v", AttributeValue::S("survived".into()))
            .send()
            .await
            .unwrap();
        srv.snapshot_all();
    }

    {
        let srv = common::spawn_with_data_dir(path.clone()).await;
        let cfg = common::aws_config(&srv.endpoint).await;
        let ddb = aws_sdk_dynamodb::Client::new(&cfg);
        let got = ddb
            .get_item()
            .table_name("dur")
            .key("pk", AttributeValue::S("a".into()))
            .send()
            .await
            .unwrap();
        assert_eq!(
            got.item().unwrap().get("v").unwrap().as_s().unwrap(),
            "survived"
        );
    }
}

#[tokio::test]
async fn e2e_admin_reset_clears_all_state() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let s3 = common::s3_client(&cfg);

    s3.create_bucket().bucket("ephemeral").send().await.unwrap();

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{}/_kuroko/reset", srv.endpoint))
        .send()
        .await
        .unwrap();
    assert!(resp.status().is_success());
    let body: serde_json::Value = resp.json().await.unwrap();
    assert!(body["reset"].as_u64().unwrap() >= 1);

    let err = s3.head_bucket().bucket("ephemeral").send().await;
    assert!(err.is_err(), "bucket must be gone after /_kuroko/reset");
}

#[tokio::test]
async fn e2e_admin_health_endpoint() {
    let srv = common::spawn().await;
    let client = reqwest::Client::new();
    let r = client
        .get(format!("{}/_kuroko/health", srv.endpoint))
        .send()
        .await
        .unwrap();
    assert!(r.status().is_success());
    let body: serde_json::Value = r.json().await.unwrap();
    assert_eq!(body["status"], "ok");
}

#[tokio::test]
async fn e2e_admin_services_endpoint_lists_at_least_76() {
    let srv = common::spawn().await;
    let client = reqwest::Client::new();
    let r = client
        .get(format!("{}/_kuroko/services", srv.endpoint))
        .send()
        .await
        .unwrap();
    let body: serde_json::Value = r.json().await.unwrap();
    let count = body["count"].as_u64().unwrap();
    assert!(count >= 76, "expected at least 76 services, got {count}");

    let names: Vec<&str> = body["services"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|v| v.as_str())
        .collect();
    for required in ["s3", "sqs", "dynamodb"] {
        assert!(
            names.contains(&required),
            "services list must include {required}"
        );
    }
}

#[tokio::test]
async fn e2e_admin_info_returns_version() {
    let srv = common::spawn().await;
    let client = reqwest::Client::new();
    let r = client
        .get(format!("{}/_kuroko/info", srv.endpoint))
        .send()
        .await
        .unwrap();
    let body: serde_json::Value = r.json().await.unwrap();
    assert_eq!(body["name"], "kuroko");
    assert!(body["version"].as_str().is_some());
}
