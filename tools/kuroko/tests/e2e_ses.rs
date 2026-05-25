//! SES v1 E2E tests.
//! Refs: <https://docs.aws.amazon.com/ses/latest/APIReference/Welcome.html>

mod common;

use aws_sdk_ses::types::{Body, Content, Destination, Message};
use pretty_assertions::assert_eq;

#[tokio::test]
async fn e2e_ses_verify_then_list_identity() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let s = aws_sdk_ses::Client::new(&cfg);
    s.verify_email_identity()
        .email_address("alice@kuroko.test")
        .send()
        .await
        .unwrap();
    let list = s.list_identities().send().await.unwrap();
    assert!(list.identities().contains(&"alice@kuroko.test".to_string()));
}

#[tokio::test]
async fn e2e_ses_send_email_unverified_from_fails() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let s = aws_sdk_ses::Client::new(&cfg);
    let err = s
        .send_email()
        .source("notverified@example.com")
        .destination(
            Destination::builder()
                .to_addresses("to@example.com")
                .build(),
        )
        .message(
            Message::builder()
                .subject(Content::builder().data("hi").build().unwrap())
                .body(
                    Body::builder()
                        .text(Content::builder().data("kuroko").build().unwrap())
                        .build(),
                )
                .build(),
        )
        .send()
        .await;
    assert!(err.is_err());
}

#[tokio::test]
async fn e2e_ses_send_email_verified_success() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let s = aws_sdk_ses::Client::new(&cfg);
    s.verify_email_identity()
        .email_address("verified@kuroko.test")
        .send()
        .await
        .unwrap();
    let res = s
        .send_email()
        .source("verified@kuroko.test")
        .destination(
            Destination::builder()
                .to_addresses("to@example.com")
                .build(),
        )
        .message(
            Message::builder()
                .subject(Content::builder().data("hi").build().unwrap())
                .body(
                    Body::builder()
                        .text(Content::builder().data("kuroko").build().unwrap())
                        .build(),
                )
                .build(),
        )
        .send()
        .await
        .unwrap();
    assert!(!res.message_id().is_empty());
}

#[tokio::test]
async fn e2e_ses_delete_identity_removes_it() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let s = aws_sdk_ses::Client::new(&cfg);
    s.verify_email_identity()
        .email_address("doomed@kuroko.test")
        .send()
        .await
        .unwrap();
    s.delete_identity()
        .identity("doomed@kuroko.test")
        .send()
        .await
        .unwrap();
    let list = s.list_identities().send().await.unwrap();
    assert!(
        !list
            .identities()
            .contains(&"doomed@kuroko.test".to_string())
    );
}

#[tokio::test]
async fn e2e_ses_verification_attributes() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let s = aws_sdk_ses::Client::new(&cfg);
    s.verify_email_identity()
        .email_address("v@kuroko.test")
        .send()
        .await
        .unwrap();
    let res = s
        .get_identity_verification_attributes()
        .identities("v@kuroko.test")
        .send()
        .await
        .unwrap();
    let attrs = res.verification_attributes();
    assert_eq!(
        attrs.get("v@kuroko.test").unwrap().verification_status(),
        &aws_sdk_ses::types::VerificationStatus::Success
    );
}
