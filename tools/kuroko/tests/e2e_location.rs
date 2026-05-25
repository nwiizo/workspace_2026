//! Amazon Location E2E. The aws-sdk-location crate uses sub-service endpoints
//! that don't honor a custom endpoint_url for every API; we exercise the REST
//! surface directly with reqwest, which is what real client SDKs ultimately
//! send.
mod common;
use pretty_assertions::assert_eq;
use serde_json::Value;

#[tokio::test]
async fn e2e_location_place_index_lifecycle() {
    let srv = common::spawn().await;
    let client = reqwest::Client::new();
    let url = format!("{}/places/v0/indexes", srv.endpoint);
    let create = client
        .post(&url)
        .json(&serde_json::json!({ "IndexName": "idx", "DataSource": "Esri" }))
        .send()
        .await
        .unwrap();
    assert_eq!(create.status(), 200);
    let body: Value = create.json().await.unwrap();
    assert_eq!(body["IndexName"], "idx");

    let list = client.get(&url).send().await.unwrap();
    let body: Value = list.json().await.unwrap();
    assert_eq!(body["Entries"].as_array().unwrap().len(), 1);

    let del = client
        .delete(format!("{}/places/v0/indexes/idx", srv.endpoint))
        .send()
        .await
        .unwrap();
    assert_eq!(del.status(), 200);
}

#[tokio::test]
async fn e2e_location_map_create_describe() {
    let srv = common::spawn().await;
    let client = reqwest::Client::new();
    client
        .post(format!("{}/maps/v0/maps", srv.endpoint))
        .json(&serde_json::json!({ "MapName": "m1" }))
        .send()
        .await
        .unwrap();
    let desc = client
        .get(format!("{}/maps/v0/maps/m1", srv.endpoint))
        .send()
        .await
        .unwrap();
    let body: Value = desc.json().await.unwrap();
    assert_eq!(body["MapName"], "m1");
}

#[tokio::test]
async fn e2e_location_geofence_collection() {
    let srv = common::spawn().await;
    let client = reqwest::Client::new();
    let res = client
        .post(format!("{}/geofencing/v0/collections", srv.endpoint))
        .json(&serde_json::json!({ "CollectionName": "c1" }))
        .send()
        .await
        .unwrap();
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["CollectionName"], "c1");
}
