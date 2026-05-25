//! Comprehend E2E tests.
mod common;
use aws_sdk_comprehend::types::SentimentType;
use pretty_assertions::assert_eq;

#[tokio::test]
async fn e2e_comprehend_detect_dominant_language() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let c = aws_sdk_comprehend::Client::new(&cfg);
    let res = c
        .detect_dominant_language()
        .text("hello kuroko")
        .send()
        .await
        .unwrap();
    assert_eq!(res.languages()[0].language_code(), Some("en"));
}

#[tokio::test]
async fn e2e_comprehend_detect_sentiment_positive() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let c = aws_sdk_comprehend::Client::new(&cfg);
    let res = c
        .detect_sentiment()
        .text("kuroko is great")
        .language_code(aws_sdk_comprehend::types::LanguageCode::En)
        .send()
        .await
        .unwrap();
    assert_eq!(res.sentiment(), Some(&SentimentType::Positive));
}

#[tokio::test]
async fn e2e_comprehend_detect_entities_returns_something() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let c = aws_sdk_comprehend::Client::new(&cfg);
    let res = c
        .detect_entities()
        .text("kuroko in Tokyo")
        .language_code(aws_sdk_comprehend::types::LanguageCode::En)
        .send()
        .await
        .unwrap();
    assert!(!res.entities().is_empty());
}

#[tokio::test]
async fn e2e_comprehend_detect_key_phrases() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let c = aws_sdk_comprehend::Client::new(&cfg);
    let res = c
        .detect_key_phrases()
        .text("kuroko emulator")
        .language_code(aws_sdk_comprehend::types::LanguageCode::En)
        .send()
        .await
        .unwrap();
    assert!(!res.key_phrases().is_empty());
}
