//! SES v2 E2E tests.

mod common;

use aws_sdk_sesv2::types::{Body, Content, Destination, EmailContent, Message};
use pretty_assertions::assert_eq;

#[tokio::test]
async fn e2e_sesv2_create_then_get_identity() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let s = aws_sdk_sesv2::Client::new(&cfg);
    s.create_email_identity()
        .email_identity("alice@kuroko.test")
        .send()
        .await
        .unwrap();
    let res = s
        .get_email_identity()
        .email_identity("alice@kuroko.test")
        .send()
        .await
        .unwrap();
    assert_eq!(
        res.identity_type(),
        Some(&aws_sdk_sesv2::types::IdentityType::EmailAddress)
    );
}

#[tokio::test]
async fn e2e_sesv2_list_identities() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let s = aws_sdk_sesv2::Client::new(&cfg);
    for n in ["a@k.test", "b@k.test"] {
        s.create_email_identity()
            .email_identity(n)
            .send()
            .await
            .unwrap();
    }
    let res = s.list_email_identities().send().await.unwrap();
    assert_eq!(res.email_identities().len(), 2);
}

#[tokio::test]
async fn e2e_sesv2_send_email_returns_message_id() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let s = aws_sdk_sesv2::Client::new(&cfg);
    s.create_email_identity()
        .email_identity("from@kuroko.test")
        .send()
        .await
        .unwrap();
    let res = s
        .send_email()
        .from_email_address("from@kuroko.test")
        .destination(
            Destination::builder()
                .to_addresses("to@example.com")
                .build(),
        )
        .content(
            EmailContent::builder()
                .simple(
                    Message::builder()
                        .subject(Content::builder().data("hi").build().unwrap())
                        .body(
                            Body::builder()
                                .text(Content::builder().data("kuroko").build().unwrap())
                                .build(),
                        )
                        .build(),
                )
                .build(),
        )
        .send()
        .await
        .unwrap();
    assert!(res.message_id().is_some());
}

#[tokio::test]
async fn e2e_sesv2_delete_identity() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let s = aws_sdk_sesv2::Client::new(&cfg);
    s.create_email_identity()
        .email_identity("doomed@k.test")
        .send()
        .await
        .unwrap();
    s.delete_email_identity()
        .email_identity("doomed@k.test")
        .send()
        .await
        .unwrap();
    let err = s
        .get_email_identity()
        .email_identity("doomed@k.test")
        .send()
        .await;
    assert!(err.is_err());
}
