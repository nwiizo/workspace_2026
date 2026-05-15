//! KMS E2E tests against AWS official API spec.
//!
//! References:
//! - CreateKey:    <https://docs.aws.amazon.com/kms/latest/APIReference/API_CreateKey.html>
//! - Encrypt:      <https://docs.aws.amazon.com/kms/latest/APIReference/API_Encrypt.html>
//! - Decrypt:      <https://docs.aws.amazon.com/kms/latest/APIReference/API_Decrypt.html>
//! - DescribeKey:  <https://docs.aws.amazon.com/kms/latest/APIReference/API_DescribeKey.html>
//! - CreateAlias:  <https://docs.aws.amazon.com/kms/latest/APIReference/API_CreateAlias.html>

mod common;

use aws_sdk_kms::primitives::Blob;
use pretty_assertions::assert_eq;

#[tokio::test]
async fn e2e_kms_create_key_returns_active_metadata_and_arn() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let kms = aws_sdk_kms::Client::new(&cfg);

    let res = kms
        .create_key()
        .description("e2e-key")
        .send()
        .await
        .unwrap();
    let md = res.key_metadata().unwrap();
    let arn = md.arn().unwrap();
    assert!(arn.starts_with("arn:aws:kms:"));
    assert!(arn.contains(":key/"));
    assert!(md.enabled());
}

#[tokio::test]
async fn e2e_kms_encrypt_decrypt_roundtrip() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let kms = aws_sdk_kms::Client::new(&cfg);

    let created = kms.create_key().send().await.unwrap();
    let key_id = created.key_metadata().unwrap().key_id().to_string();

    let enc = kms
        .encrypt()
        .key_id(&key_id)
        .plaintext(Blob::new(b"hello kuroko".to_vec()))
        .send()
        .await
        .unwrap();
    let ciphertext = enc.ciphertext_blob().unwrap().clone();

    let dec = kms
        .decrypt()
        .ciphertext_blob(ciphertext)
        .send()
        .await
        .unwrap();
    let plain = dec.plaintext().unwrap().as_ref().to_vec();
    assert_eq!(plain, b"hello kuroko");
}

#[tokio::test]
async fn e2e_kms_describe_key_returns_arn() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let kms = aws_sdk_kms::Client::new(&cfg);
    let created = kms.create_key().send().await.unwrap();
    let key_id = created.key_metadata().unwrap().key_id().to_string();
    let desc = kms.describe_key().key_id(&key_id).send().await.unwrap();
    assert_eq!(desc.key_metadata().unwrap().key_id(), key_id);
}

#[tokio::test]
async fn e2e_kms_create_alias_then_encrypt_by_alias() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let kms = aws_sdk_kms::Client::new(&cfg);
    let created = kms.create_key().send().await.unwrap();
    let key_id = created.key_metadata().unwrap().key_id().to_string();
    kms.create_alias()
        .alias_name("alias/myapp")
        .target_key_id(&key_id)
        .send()
        .await
        .unwrap();
    let enc = kms
        .encrypt()
        .key_id("alias/myapp")
        .plaintext(Blob::new(b"data".to_vec()))
        .send()
        .await
        .unwrap();
    let blob = enc.ciphertext_blob().unwrap().clone();
    let dec = kms.decrypt().ciphertext_blob(blob).send().await.unwrap();
    assert_eq!(dec.plaintext().unwrap().as_ref(), b"data");
}

#[tokio::test]
async fn e2e_kms_generate_data_key_returns_plaintext_and_ciphertext() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let kms = aws_sdk_kms::Client::new(&cfg);
    let created = kms.create_key().send().await.unwrap();
    let key_id = created.key_metadata().unwrap().key_id().to_string();
    let dk = kms
        .generate_data_key()
        .key_id(&key_id)
        .key_spec(aws_sdk_kms::types::DataKeySpec::Aes256)
        .send()
        .await
        .unwrap();
    assert_eq!(dk.plaintext().unwrap().as_ref().len(), 32);
    assert!(!dk.ciphertext_blob().unwrap().as_ref().is_empty());
}

#[tokio::test]
async fn e2e_kms_disable_key_then_encrypt_fails() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let kms = aws_sdk_kms::Client::new(&cfg);
    let created = kms.create_key().send().await.unwrap();
    let key_id = created.key_metadata().unwrap().key_id().to_string();
    kms.disable_key().key_id(&key_id).send().await.unwrap();
    let res = kms
        .encrypt()
        .key_id(&key_id)
        .plaintext(Blob::new(b"x".to_vec()))
        .send()
        .await;
    assert!(res.is_err(), "encrypt on disabled key must fail");
}
