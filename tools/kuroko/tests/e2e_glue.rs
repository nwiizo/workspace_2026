//! Glue Data Catalog E2E tests.
//! Refs: <https://docs.aws.amazon.com/glue/latest/dg/aws-glue-api-catalog.html>

mod common;

use aws_sdk_glue::types::{CrawlerTargets, DatabaseInput, S3Target, TableInput};
use pretty_assertions::assert_eq;

#[tokio::test]
async fn e2e_glue_create_then_get_database() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let g = aws_sdk_glue::Client::new(&cfg);
    g.create_database()
        .database_input(
            DatabaseInput::builder()
                .name("warehouse")
                .description("kuroko")
                .build()
                .unwrap(),
        )
        .send()
        .await
        .unwrap();
    let got = g.get_database().name("warehouse").send().await.unwrap();
    assert_eq!(got.database().unwrap().name(), "warehouse");
}

#[tokio::test]
async fn e2e_glue_duplicate_create_fails() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let g = aws_sdk_glue::Client::new(&cfg);
    g.create_database()
        .database_input(DatabaseInput::builder().name("dup").build().unwrap())
        .send()
        .await
        .unwrap();
    let err = g
        .create_database()
        .database_input(DatabaseInput::builder().name("dup").build().unwrap())
        .send()
        .await;
    assert!(err.is_err());
}

#[tokio::test]
async fn e2e_glue_list_databases() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let g = aws_sdk_glue::Client::new(&cfg);
    for n in ["a", "b", "c"] {
        g.create_database()
            .database_input(DatabaseInput::builder().name(n).build().unwrap())
            .send()
            .await
            .unwrap();
    }
    let res = g.get_databases().send().await.unwrap();
    assert_eq!(res.database_list().len(), 3);
}

#[tokio::test]
async fn e2e_glue_create_table_in_database() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let g = aws_sdk_glue::Client::new(&cfg);
    g.create_database()
        .database_input(DatabaseInput::builder().name("dw").build().unwrap())
        .send()
        .await
        .unwrap();
    g.create_table()
        .database_name("dw")
        .table_input(TableInput::builder().name("orders").build().unwrap())
        .send()
        .await
        .unwrap();
    let res = g.get_tables().database_name("dw").send().await.unwrap();
    assert_eq!(res.table_list().len(), 1);
    assert_eq!(res.table_list()[0].name(), "orders");
}

#[tokio::test]
async fn e2e_glue_create_crawler_then_list() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let g = aws_sdk_glue::Client::new(&cfg);
    g.create_crawler()
        .name("daily")
        .role("arn:aws:iam::000000000000:role/glue")
        .targets(
            CrawlerTargets::builder()
                .s3_targets(S3Target::builder().path("s3://bucket/").build())
                .build(),
        )
        .send()
        .await
        .unwrap();
    let got = g.get_crawler().name("daily").send().await.unwrap();
    assert_eq!(got.crawler().unwrap().name(), Some("daily"));
}
