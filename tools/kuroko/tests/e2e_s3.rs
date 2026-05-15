//! S3 E2E tests verifying behavior matches the AWS official API spec.
//!
//! References:
//! - CreateBucket: <https://docs.aws.amazon.com/AmazonS3/latest/API/API_CreateBucket.html>
//! - DeleteBucket: <https://docs.aws.amazon.com/AmazonS3/latest/API/API_DeleteBucket.html>
//! - HeadBucket:   <https://docs.aws.amazon.com/AmazonS3/latest/API/API_HeadBucket.html>
//! - PutObject:    <https://docs.aws.amazon.com/AmazonS3/latest/API/API_PutObject.html>
//! - GetObject:    <https://docs.aws.amazon.com/AmazonS3/latest/API/API_GetObject.html>
//! - DeleteObject: <https://docs.aws.amazon.com/AmazonS3/latest/API/API_DeleteObject.html>
//! - ListObjectsV2:<https://docs.aws.amazon.com/AmazonS3/latest/API/API_ListObjectsV2.html>
//! - ListBuckets:  <https://docs.aws.amazon.com/AmazonS3/latest/API/API_ListBuckets.html>

mod common;

use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::types::{Delete, ObjectIdentifier};

#[tokio::test]
async fn e2e_s3_create_then_list_buckets() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let s3 = common::s3_client(&cfg);

    s3.create_bucket().bucket("alpha").send().await.unwrap();
    s3.create_bucket().bucket("beta").send().await.unwrap();

    let res = s3.list_buckets().send().await.unwrap();
    let names: Vec<&str> = res.buckets().iter().filter_map(|b| b.name()).collect();
    assert!(
        names.contains(&"alpha"),
        "ListBuckets must include 'alpha': {names:?}"
    );
    assert!(
        names.contains(&"beta"),
        "ListBuckets must include 'beta': {names:?}"
    );
}

#[tokio::test]
async fn e2e_s3_head_bucket_404_when_missing() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let s3 = common::s3_client(&cfg);

    // AWS spec: HeadBucket returns 404 NotFound when the bucket does not exist.
    let err = s3.head_bucket().bucket("nope").send().await;
    assert!(err.is_err(), "HeadBucket on missing bucket must fail");
}

#[tokio::test]
async fn e2e_s3_head_bucket_200_when_exists() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let s3 = common::s3_client(&cfg);

    s3.create_bucket().bucket("hb").send().await.unwrap();
    s3.head_bucket().bucket("hb").send().await.unwrap();
}

#[tokio::test]
async fn e2e_s3_delete_bucket_204() {
    // AWS spec: DeleteBucket returns 204 No Content on success.
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let s3 = common::s3_client(&cfg);

    s3.create_bucket().bucket("d").send().await.unwrap();
    s3.delete_bucket().bucket("d").send().await.unwrap();
    let err = s3.head_bucket().bucket("d").send().await;
    assert!(err.is_err(), "bucket must be gone after DeleteBucket");
}

#[tokio::test]
async fn e2e_s3_delete_bucket_not_empty_fails() {
    // AWS spec: all objects must be deleted before the bucket itself.
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let s3 = common::s3_client(&cfg);

    s3.create_bucket().bucket("d2").send().await.unwrap();
    s3.put_object()
        .bucket("d2")
        .key("k")
        .body(ByteStream::from_static(b"x"))
        .send()
        .await
        .unwrap();
    let err = s3.delete_bucket().bucket("d2").send().await;
    assert!(
        err.is_err(),
        "non-empty bucket delete must fail (BucketNotEmpty)"
    );
}

#[tokio::test]
async fn e2e_s3_put_get_object_with_etag_and_content_type() {
    // PutObject must return an ETag header equal to the MD5 of the body, and
    // GetObject must return body + ETag + Content-Type.
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let s3 = common::s3_client(&cfg);

    s3.create_bucket().bucket("b").send().await.unwrap();
    let put = s3
        .put_object()
        .bucket("b")
        .key("hello.txt")
        .content_type("text/plain")
        .body(ByteStream::from_static(b"hello"))
        .send()
        .await
        .unwrap();
    // MD5("hello") = 5d41402abc4b2a76b9719d911017c592
    let etag = put.e_tag().unwrap().trim_matches('"');
    assert_eq!(etag, "5d41402abc4b2a76b9719d911017c592");

    let got = s3
        .get_object()
        .bucket("b")
        .key("hello.txt")
        .send()
        .await
        .unwrap();
    assert_eq!(got.content_type(), Some("text/plain"));
    let body = got.body.collect().await.unwrap().into_bytes();
    assert_eq!(&body[..], b"hello");
}

#[tokio::test]
async fn e2e_s3_get_object_missing_returns_no_such_key() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let s3 = common::s3_client(&cfg);

    s3.create_bucket().bucket("b").send().await.unwrap();
    let err = s3.get_object().bucket("b").key("absent").send().await;
    assert!(
        err.is_err(),
        "GetObject on missing key must fail (NoSuchKey)"
    );
}

#[tokio::test]
async fn e2e_s3_delete_object_then_list_is_empty() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let s3 = common::s3_client(&cfg);

    s3.create_bucket().bucket("b").send().await.unwrap();
    s3.put_object()
        .bucket("b")
        .key("a")
        .body(ByteStream::from_static(b"x"))
        .send()
        .await
        .unwrap();
    s3.delete_object()
        .bucket("b")
        .key("a")
        .send()
        .await
        .unwrap();

    let list = s3.list_objects_v2().bucket("b").send().await.unwrap();
    assert_eq!(list.contents().len(), 0);
    assert_eq!(list.key_count(), Some(0));
}

#[tokio::test]
async fn e2e_s3_list_objects_v2_with_prefix() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let s3 = common::s3_client(&cfg);

    s3.create_bucket().bucket("b").send().await.unwrap();
    for k in ["a/1", "a/2", "b/1"] {
        s3.put_object()
            .bucket("b")
            .key(k)
            .body(ByteStream::from_static(b"x"))
            .send()
            .await
            .unwrap();
    }

    let list = s3
        .list_objects_v2()
        .bucket("b")
        .prefix("a/")
        .send()
        .await
        .unwrap();
    let keys: Vec<&str> = list.contents().iter().filter_map(|o| o.key()).collect();
    assert_eq!(keys.len(), 2);
    assert!(keys.iter().all(|k| k.starts_with("a/")));
}

#[tokio::test]
async fn e2e_s3_user_metadata_roundtrip() {
    // AWS: x-amz-meta-<name> headers on PUT round-trip on GET / HEAD.
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let s3 = common::s3_client(&cfg);

    s3.create_bucket().bucket("b").send().await.unwrap();
    s3.put_object()
        .bucket("b")
        .key("k")
        .metadata("author", "kuroko")
        .body(ByteStream::from_static(b"x"))
        .send()
        .await
        .unwrap();

    let got = s3.head_object().bucket("b").key("k").send().await.unwrap();
    assert_eq!(
        got.metadata()
            .and_then(|m| m.get("author"))
            .map(String::as_str),
        Some("kuroko")
    );
}

#[tokio::test]
async fn e2e_s3_delete_objects_batch() {
    // DeleteObjects is not yet implemented; this test documents the gap. We
    // expect an error today and replace it with a positive assertion when the
    // operation lands.
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let s3 = common::s3_client(&cfg);

    s3.create_bucket().bucket("b").send().await.unwrap();
    s3.put_object()
        .bucket("b")
        .key("k")
        .body(ByteStream::from_static(b"x"))
        .send()
        .await
        .unwrap();
    let id = ObjectIdentifier::builder().key("k").build().unwrap();
    let del = Delete::builder().objects(id).build().unwrap();
    let res = s3.delete_objects().bucket("b").delete(del).send().await;
    assert!(res.is_err(), "DeleteObjects is not implemented yet");
}
