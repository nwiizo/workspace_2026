#![allow(
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_used,
    reason = "tests keep assertions and fixture setup direct"
)]

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use rust_types_as_walls::order_service;
use serde_json::{Value, json};
use tower::ServiceExt;

async fn app() -> Router {
    order_service::app().await.expect("app should build")
}

fn post_json(uri: &str, body: Value) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .expect("request should build")
}

fn get_request(uri: &str) -> Request<Body> {
    Request::builder()
        .method("GET")
        .uri(uri)
        .body(Body::empty())
        .expect("request should build")
}

async fn read_json(response: axum::response::Response) -> Value {
    let bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body should be readable");
    serde_json::from_slice(&bytes).expect("response should be valid JSON")
}

fn valid_order_request() -> Value {
    json!({
        "customer_id": 42,
        "email": "buyer@example.com",
        "payment_method": "card",
        "items": [
            {"sku": "BOOK-001", "quantity": 2},
            {"sku": "PEN-001", "quantity": 1}
        ]
    })
}

#[tokio::test]
async fn post_orders_creates_a_paid_order() {
    let app = app().await;
    let response = app
        .oneshot(post_json("/orders", valid_order_request()))
        .await
        .expect("request should succeed");

    assert_eq!(response.status(), StatusCode::CREATED);
    let body = read_json(response).await;

    assert_eq!(body["id"], 1);
    assert_eq!(body["status"], "paid");
    assert_eq!(body["payment_method"], "card");
    assert_eq!(body["total_cents"], 3300);
    assert_eq!(body["payment_reference"], "pay_1");
}

#[tokio::test]
async fn get_orders_returns_the_saved_order() {
    let app = app().await;
    let create_response = app
        .clone()
        .oneshot(post_json("/orders", valid_order_request()))
        .await
        .expect("create should succeed");
    let created = read_json(create_response).await;
    let order_id = created["id"].as_u64().expect("id should be a number");

    let response = app
        .oneshot(get_request(&format!("/orders/{order_id}")))
        .await
        .expect("get should succeed");

    assert_eq!(response.status(), StatusCode::OK);
    let body = read_json(response).await;
    assert_eq!(body["id"], order_id);
    assert_eq!(body["status"], "paid");
    assert_eq!(body["items"].as_array().map(Vec::len), Some(2));
}

#[tokio::test]
async fn ship_endpoint_transitions_paid_order_to_shipped() {
    let app = app().await;
    let create_response = app
        .clone()
        .oneshot(post_json("/orders", valid_order_request()))
        .await
        .expect("create should succeed");
    let created = read_json(create_response).await;
    let order_id = created["id"].as_u64().expect("id should be a number");

    let response = app
        .clone()
        .oneshot(post_json(&format!("/orders/{order_id}/ship"), json!({})))
        .await
        .expect("ship should succeed");

    assert_eq!(response.status(), StatusCode::OK);
    let body = read_json(response).await;
    assert_eq!(body["status"], "shipped");
    assert!(body["shipped_at"].as_str().is_some());

    let get_response = app
        .oneshot(get_request(&format!("/orders/{order_id}")))
        .await
        .expect("get should succeed");
    let get_body = read_json(get_response).await;
    assert_eq!(get_body["status"], "shipped");
}

#[tokio::test]
async fn post_orders_rejects_invalid_email() {
    let app = app().await;
    let request = json!({
        "customer_id": 42,
        "email": "invalid-email",
        "payment_method": "card",
        "items": [{"sku": "BOOK-001", "quantity": 1}]
    });

    let response = app
        .oneshot(post_json("/orders", request))
        .await
        .expect("request should succeed");

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = read_json(response).await;
    assert!(
        body["error"]
            .as_str()
            .is_some_and(|message| message.contains("メールアドレス"))
    );
}

#[tokio::test]
async fn post_orders_rejects_empty_item_list() {
    let app = app().await;
    let request = json!({
        "customer_id": 42,
        "email": "buyer@example.com",
        "payment_method": "card",
        "items": []
    });

    let response = app
        .oneshot(post_json("/orders", request))
        .await
        .expect("request should succeed");

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = read_json(response).await;
    assert_eq!(body["error"], "注文には 1 件以上の商品が必要です");
}

#[tokio::test]
async fn get_orders_returns_404_for_missing_ids() {
    let app = app().await;
    let response = app
        .oneshot(get_request("/orders/999"))
        .await
        .expect("request should succeed");

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let body = read_json(response).await;
    assert_eq!(body["error"], "注文が見つかりません: 999");
}

#[tokio::test]
async fn ship_endpoint_rejects_already_shipped_orders() {
    let app = app().await;
    let create_response = app
        .clone()
        .oneshot(post_json("/orders", valid_order_request()))
        .await
        .expect("create should succeed");
    let created = read_json(create_response).await;
    let order_id = created["id"].as_u64().expect("id should be a number");

    let first_ship = app
        .clone()
        .oneshot(post_json(&format!("/orders/{order_id}/ship"), json!({})))
        .await
        .expect("first ship should succeed");
    assert_eq!(first_ship.status(), StatusCode::OK);

    let second_ship = app
        .oneshot(post_json(&format!("/orders/{order_id}/ship"), json!({})))
        .await
        .expect("second ship should return conflict");

    assert_eq!(second_ship.status(), StatusCode::CONFLICT);
    let body = read_json(second_ship).await;
    assert_eq!(body["error"], "注文はすでに出荷済みです");
}
