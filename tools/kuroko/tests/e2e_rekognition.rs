//! Rekognition E2E tests.
mod common;
use pretty_assertions::assert_eq;

#[tokio::test]
async fn e2e_rek_create_collection() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let r = aws_sdk_rekognition::Client::new(&cfg);
    let res = r
        .create_collection()
        .collection_id("people")
        .send()
        .await
        .unwrap();
    assert_eq!(res.status_code(), Some(200));
    assert!(res.collection_arn().unwrap().contains(":collection/people"));
}

#[tokio::test]
async fn e2e_rek_list_collections() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let r = aws_sdk_rekognition::Client::new(&cfg);
    for n in ["c1", "c2"] {
        r.create_collection().collection_id(n).send().await.unwrap();
    }
    let res = r.list_collections().send().await.unwrap();
    assert_eq!(res.collection_ids().len(), 2);
}

#[tokio::test]
async fn e2e_rek_describe_collection() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let r = aws_sdk_rekognition::Client::new(&cfg);
    r.create_collection()
        .collection_id("d")
        .send()
        .await
        .unwrap();
    let res = r
        .describe_collection()
        .collection_id("d")
        .send()
        .await
        .unwrap();
    assert_eq!(res.face_count(), Some(0));
}

#[tokio::test]
async fn e2e_rek_delete_collection() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let r = aws_sdk_rekognition::Client::new(&cfg);
    r.create_collection()
        .collection_id("d")
        .send()
        .await
        .unwrap();
    r.delete_collection()
        .collection_id("d")
        .send()
        .await
        .unwrap();
    let err = r.describe_collection().collection_id("d").send().await;
    assert!(err.is_err());
}
