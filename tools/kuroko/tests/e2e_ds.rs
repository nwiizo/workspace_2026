//! Directory Service E2E.
//! Refs: <https://docs.aws.amazon.com/directoryservice/latest/devguide/API_Reference.html>

mod common;
use aws_sdk_directory::types::DirectorySize;
use pretty_assertions::assert_eq;

#[tokio::test]
async fn e2e_ds_create_and_describe() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let c = aws_sdk_directory::Client::new(&cfg);
    let res = c
        .create_directory()
        .name("corp.example.com")
        .password("StrongPass1!")
        .size(DirectorySize::Small)
        .send()
        .await
        .unwrap();
    let id = res.directory_id().unwrap();
    assert!(id.starts_with("d-"));
    let descs = c.describe_directories().send().await.unwrap();
    assert_eq!(descs.directory_descriptions().len(), 1);
    assert_eq!(
        descs.directory_descriptions()[0].directory_id().unwrap(),
        id
    );
}

#[tokio::test]
async fn e2e_ds_delete_unknown() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let c = aws_sdk_directory::Client::new(&cfg);
    let err = c.delete_directory().directory_id("d-deadbeef").send().await;
    assert!(err.is_err());
}

#[tokio::test]
async fn e2e_ds_create_then_delete() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let c = aws_sdk_directory::Client::new(&cfg);
    let res = c
        .create_directory()
        .name("ex.com")
        .password("Pwd123!")
        .size(DirectorySize::Small)
        .send()
        .await
        .unwrap();
    let id = res.directory_id().unwrap().to_string();
    let del = c.delete_directory().directory_id(&id).send().await.unwrap();
    assert_eq!(del.directory_id().unwrap(), id);
}
