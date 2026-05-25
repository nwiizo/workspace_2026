//! Forecast E2E tests.
mod common;
use aws_sdk_forecast::types::{AttributeType, DatasetType, Domain, Schema, SchemaAttribute};
use pretty_assertions::assert_eq;

#[tokio::test]
async fn e2e_forecast_create_dataset_group() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let f = aws_sdk_forecast::Client::new(&cfg);
    let res = f
        .create_dataset_group()
        .dataset_group_name("group1")
        .domain(Domain::Retail)
        .send()
        .await
        .unwrap();
    assert!(
        res.dataset_group_arn()
            .unwrap()
            .contains(":dataset-group/group1")
    );
}

#[tokio::test]
async fn e2e_forecast_list_dataset_groups() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let f = aws_sdk_forecast::Client::new(&cfg);
    for n in ["a", "b"] {
        f.create_dataset_group()
            .dataset_group_name(n)
            .domain(Domain::Retail)
            .send()
            .await
            .unwrap();
    }
    let res = f.list_dataset_groups().send().await.unwrap();
    assert_eq!(res.dataset_groups().len(), 2);
}

#[tokio::test]
async fn e2e_forecast_create_dataset() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let f = aws_sdk_forecast::Client::new(&cfg);
    let schema = Schema::builder()
        .attributes(
            SchemaAttribute::builder()
                .attribute_name("timestamp")
                .attribute_type(AttributeType::Timestamp)
                .build(),
        )
        .build();
    let res = f
        .create_dataset()
        .dataset_name("ds1")
        .domain(Domain::Retail)
        .dataset_type(DatasetType::TargetTimeSeries)
        .schema(schema)
        .send()
        .await
        .unwrap();
    assert!(res.dataset_arn().unwrap().contains(":dataset/ds1"));
}
