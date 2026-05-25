//! DLM E2E — uses raw HTTP since SDK uses sub-service endpoints.
mod common;
use pretty_assertions::assert_eq;
use serde_json::Value;

#[tokio::test]
async fn e2e_dlm_create_policy() {
    let srv = common::spawn().await;
    let client = reqwest::Client::new();
    let res = client
        .post(format!("{}/policies", srv.endpoint))
        .json(&serde_json::json!({
            "Description": "daily-snap",
            "ExecutionRoleArn": "arn:aws:iam::000000000000:role/dlm",
            "PolicyDetails": { "PolicyType": "EBS_SNAPSHOT_MANAGEMENT" }
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
    let body: Value = res.json().await.unwrap();
    assert!(body["PolicyId"].as_str().unwrap().starts_with("policy-"));
}

#[tokio::test]
async fn e2e_dlm_list_policies() {
    let srv = common::spawn().await;
    let client = reqwest::Client::new();
    for _ in 0..2 {
        client
            .post(format!("{}/policies", srv.endpoint))
            .json(&serde_json::json!({ "Description": "x" }))
            .send()
            .await
            .unwrap();
    }
    let res = client
        .get(format!("{}/policies", srv.endpoint))
        .send()
        .await
        .unwrap();
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["Policies"].as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn e2e_dlm_get_unknown_404() {
    let srv = common::spawn().await;
    let client = reqwest::Client::new();
    let res = client
        .get(format!("{}/policies/policy-nope", srv.endpoint))
        .send()
        .await
        .unwrap();
    assert!(!res.status().is_success());
}
