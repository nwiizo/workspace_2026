//! ECR E2E tests against AWS official API spec.
//!
//! References:
//! - CreateRepository: <https://docs.aws.amazon.com/AmazonECR/latest/APIReference/API_CreateRepository.html>
//! - PutImage:         <https://docs.aws.amazon.com/AmazonECR/latest/APIReference/API_PutImage.html>
//! - BatchGetImage:    <https://docs.aws.amazon.com/AmazonECR/latest/APIReference/API_BatchGetImage.html>

mod common;

use aws_sdk_ecr::types::ImageIdentifier;
use pretty_assertions::assert_eq;

const MANIFEST: &str =
    r#"{"schemaVersion":2,"mediaType":"application/vnd.docker.distribution.manifest.v2+json"}"#;

#[tokio::test]
async fn e2e_ecr_create_repository_returns_uri() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let ecr = aws_sdk_ecr::Client::new(&cfg);

    let res = ecr
        .create_repository()
        .repository_name("apps/web")
        .send()
        .await
        .unwrap();
    let r = res.repository().unwrap();
    assert_eq!(r.repository_name(), Some("apps/web"));
    assert!(r.repository_uri().unwrap().ends_with("/apps/web"));
}

#[tokio::test]
async fn e2e_ecr_duplicate_create_fails() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let ecr = aws_sdk_ecr::Client::new(&cfg);

    ecr.create_repository()
        .repository_name("dup")
        .send()
        .await
        .unwrap();
    let err = ecr.create_repository().repository_name("dup").send().await;
    assert!(
        err.is_err(),
        "duplicate must fail with RepositoryAlreadyExistsException"
    );
}

#[tokio::test]
async fn e2e_ecr_put_image_then_list_images() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let ecr = aws_sdk_ecr::Client::new(&cfg);

    ecr.create_repository()
        .repository_name("r")
        .send()
        .await
        .unwrap();
    ecr.put_image()
        .repository_name("r")
        .image_manifest(MANIFEST)
        .image_tag("v1")
        .send()
        .await
        .unwrap();
    let res = ecr.list_images().repository_name("r").send().await.unwrap();
    assert_eq!(res.image_ids().len(), 1);
    assert_eq!(res.image_ids()[0].image_tag(), Some("v1"));
}

#[tokio::test]
async fn e2e_ecr_batch_get_image_by_tag() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let ecr = aws_sdk_ecr::Client::new(&cfg);

    ecr.create_repository()
        .repository_name("r")
        .send()
        .await
        .unwrap();
    ecr.put_image()
        .repository_name("r")
        .image_manifest(MANIFEST)
        .image_tag("latest")
        .send()
        .await
        .unwrap();
    let res = ecr
        .batch_get_image()
        .repository_name("r")
        .image_ids(ImageIdentifier::builder().image_tag("latest").build())
        .send()
        .await
        .unwrap();
    assert_eq!(res.images().len(), 1);
    assert_eq!(res.failures().len(), 0);
    assert_eq!(res.images()[0].image_manifest(), Some(MANIFEST));
}

#[tokio::test]
async fn e2e_ecr_repoint_tag_to_new_manifest() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let ecr = aws_sdk_ecr::Client::new(&cfg);

    ecr.create_repository()
        .repository_name("r")
        .send()
        .await
        .unwrap();
    ecr.put_image()
        .repository_name("r")
        .image_manifest(r#"{"v":1}"#)
        .image_tag("latest")
        .send()
        .await
        .unwrap();
    ecr.put_image()
        .repository_name("r")
        .image_manifest(r#"{"v":2}"#)
        .image_tag("latest")
        .send()
        .await
        .unwrap();
    // "latest" should now point to the v:2 manifest only.
    let res = ecr.list_images().repository_name("r").send().await.unwrap();
    let latest: Vec<_> = res
        .image_ids()
        .iter()
        .filter(|id| id.image_tag() == Some("latest"))
        .collect();
    assert_eq!(latest.len(), 1);
}

#[tokio::test]
async fn e2e_ecr_delete_non_empty_repository_fails_without_force() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let ecr = aws_sdk_ecr::Client::new(&cfg);

    ecr.create_repository()
        .repository_name("r")
        .send()
        .await
        .unwrap();
    ecr.put_image()
        .repository_name("r")
        .image_manifest(MANIFEST)
        .image_tag("v")
        .send()
        .await
        .unwrap();
    let err = ecr.delete_repository().repository_name("r").send().await;
    assert!(err.is_err(), "delete must require force when non-empty");

    ecr.delete_repository()
        .repository_name("r")
        .force(true)
        .send()
        .await
        .unwrap();
}

#[tokio::test]
async fn e2e_ecr_get_authorization_token_shape() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let ecr = aws_sdk_ecr::Client::new(&cfg);

    let res = ecr.get_authorization_token().send().await.unwrap();
    let data = &res.authorization_data()[0];
    assert!(!data.authorization_token().unwrap().is_empty());
    assert!(data.proxy_endpoint().unwrap().starts_with("https://"));
}
