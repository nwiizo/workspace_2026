//! Sanity E2E for all minimal "resource_stub" services. We exercise the create
//! / list / describe / delete shape via raw HTTP since these are control-plane
//! stubs without a fully-modeled SDK surface.
mod common;
use pretty_assertions::assert_eq;
use serde_json::Value;

async fn lifecycle(
    service: &str,
    list_path: &str,
    item_path_for: impl Fn(&str) -> String,
    name: &str,
    name_field: &str,
    list_field: &str,
) {
    let srv = common::spawn().await;
    let client = reqwest::Client::new();

    let create = client
        .post(format!("{}{}", srv.endpoint, list_path))
        .json(&serde_json::json!({ name_field: name }))
        .send()
        .await
        .unwrap();
    assert_eq!(create.status(), 200, "{service}: create");

    let list = client
        .get(format!("{}{}", srv.endpoint, list_path))
        .send()
        .await
        .unwrap();
    assert_eq!(list.status(), 200, "{service}: list");
    let body: Value = list.json().await.unwrap();
    assert!(
        body[list_field].as_array().is_some_and(|a| !a.is_empty()),
        "{service}: list field {list_field} not populated: {body}"
    );

    let item = client
        .get(format!("{}{}", srv.endpoint, item_path_for(name)))
        .send()
        .await
        .unwrap();
    assert_eq!(item.status(), 200, "{service}: describe");

    let del = client
        .delete(format!("{}{}", srv.endpoint, item_path_for(name)))
        .send()
        .await
        .unwrap();
    assert_eq!(del.status(), 200, "{service}: delete");
}

#[tokio::test]
async fn pipes_lifecycle() {
    lifecycle(
        "pipes",
        "/v1/pipes",
        |n| format!("/v1/pipes/{n}"),
        "p1",
        "Name",
        "Pipes",
    )
    .await;
}

#[tokio::test]
async fn appmesh_lifecycle() {
    lifecycle(
        "appmesh",
        "/v20190125/meshes",
        |n| format!("/v20190125/meshes/{n}"),
        "m1",
        "meshName",
        "meshes",
    )
    .await;
}

#[tokio::test]
async fn appsync_lifecycle() {
    lifecycle(
        "appsync",
        "/v1/apis",
        |n| format!("/v1/apis/{n}"),
        "api1",
        "name",
        "graphqlApis",
    )
    .await;
}

#[tokio::test]
async fn dataexchange_lifecycle() {
    lifecycle(
        "dataexchange",
        "/v1/data-sets",
        |n| format!("/v1/data-sets/{n}"),
        "ds1",
        "Name",
        "DataSets",
    )
    .await;
}

#[tokio::test]
async fn macie2_lifecycle() {
    lifecycle(
        "macie2",
        "/jobs",
        |n| format!("/jobs/{n}"),
        "j1",
        "name",
        "items",
    )
    .await;
}

#[tokio::test]
async fn finspace_lifecycle() {
    lifecycle(
        "finspace",
        "/environment",
        |n| format!("/environment/{n}"),
        "e1",
        "name",
        "environments",
    )
    .await;
}

#[tokio::test]
async fn resiliencehub_lifecycle() {
    let srv = common::spawn().await;
    let client = reqwest::Client::new();
    let res = client
        .post(format!("{}/create-app", srv.endpoint))
        .json(&serde_json::json!({ "name": "app1" }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
    let desc = client
        .get(format!("{}/describe-app/app1", srv.endpoint))
        .send()
        .await
        .unwrap();
    assert_eq!(desc.status(), 200);
}

#[tokio::test]
async fn emrserverless_lifecycle() {
    lifecycle(
        "emrserverless",
        "/applications",
        |n| format!("/applications/{n}"),
        "app1",
        "name",
        "applications",
    )
    .await;
}

#[tokio::test]
async fn entityresolution_lifecycle() {
    lifecycle(
        "entityresolution",
        "/matchingworkflows",
        |n| format!("/matchingworkflows/{n}"),
        "w1",
        "workflowName",
        "workflowSummaries",
    )
    .await;
}

#[tokio::test]
async fn securitylake_lifecycle() {
    lifecycle(
        "securitylake",
        "/v1/datalakes",
        |n| format!("/v1/datalakes/{n}"),
        "us-east-1",
        "region",
        "dataLakes",
    )
    .await;
}

#[tokio::test]
async fn s3tables_lifecycle() {
    lifecycle(
        "s3tables",
        "/buckets",
        |n| format!("/buckets/{n}"),
        "b1",
        "name",
        "tableBuckets",
    )
    .await;
}

#[tokio::test]
async fn s3control_lifecycle() {
    lifecycle(
        "s3control",
        "/v20180820/accesspoint",
        |n| format!("/v20180820/accesspoint/{n}"),
        "ap1",
        "Name",
        "AccessPointList",
    )
    .await;
}

#[tokio::test]
async fn codeguruprofiler_lifecycle() {
    lifecycle(
        "codeguruprofiler",
        "/profilingGroups",
        |n| format!("/profilingGroups/{n}"),
        "g1",
        "profilingGroupName",
        "profilingGroupNames",
    )
    .await;
}

#[tokio::test]
async fn codegurureviewer_lifecycle() {
    lifecycle(
        "codegurureviewer",
        "/associations",
        |n| format!("/associations/{n}"),
        "a1",
        "Name",
        "RepositoryAssociationSummaries",
    )
    .await;
}
