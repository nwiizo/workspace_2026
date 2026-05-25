//! Service Quotas E2E.
//! Refs: <https://docs.aws.amazon.com/servicequotas/2019-06-24/apireference/Welcome.html>

mod common;
use pretty_assertions::assert_eq;

#[tokio::test]
async fn e2e_sq_list_services() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let c = aws_sdk_servicequotas::Client::new(&cfg);
    let res = c.list_services().send().await.unwrap();
    let codes: Vec<_> = res
        .services()
        .iter()
        .filter_map(|s| s.service_code())
        .collect();
    assert!(codes.contains(&"ec2"));
    assert!(codes.contains(&"lambda"));
}

#[tokio::test]
async fn e2e_sq_list_service_quotas() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let c = aws_sdk_servicequotas::Client::new(&cfg);
    let res = c
        .list_service_quotas()
        .service_code("lambda")
        .send()
        .await
        .unwrap();
    assert!(!res.quotas().is_empty());
}

#[tokio::test]
async fn e2e_sq_get_quota() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let c = aws_sdk_servicequotas::Client::new(&cfg);
    let res = c
        .get_service_quota()
        .service_code("ec2")
        .quota_code("L-1216C47A")
        .send()
        .await
        .unwrap();
    let q = res.quota().unwrap();
    assert_eq!(q.value(), Some(5.0));
}

#[tokio::test]
async fn e2e_sq_get_unknown_quota_fails() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let c = aws_sdk_servicequotas::Client::new(&cfg);
    let err = c
        .get_service_quota()
        .service_code("ec2")
        .quota_code("L-DEADBEEF")
        .send()
        .await;
    assert!(err.is_err());
}

#[tokio::test]
async fn e2e_sq_request_increase() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let c = aws_sdk_servicequotas::Client::new(&cfg);
    let res = c
        .request_service_quota_increase()
        .service_code("lambda")
        .quota_code("L-B99A9384")
        .desired_value(2000.0)
        .send()
        .await
        .unwrap();
    let q = res.requested_quota().unwrap();
    assert_eq!(
        q.status(),
        Some(&aws_sdk_servicequotas::types::RequestStatus::Pending)
    );
    assert_eq!(q.desired_value(), Some(2000.0));
}
