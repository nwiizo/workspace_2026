//! CloudFront E2E tests.
mod common;
use aws_sdk_cloudfront::types::DistributionConfig;
use pretty_assertions::assert_eq;

fn empty_distribution_config() -> DistributionConfig {
    use aws_sdk_cloudfront::types::{DefaultCacheBehavior, Origin, Origins, ViewerProtocolPolicy};
    DistributionConfig::builder()
        .caller_reference("kuroko-1")
        .comment("kuroko test")
        .enabled(true)
        .origins(
            Origins::builder()
                .quantity(1)
                .items(
                    Origin::builder()
                        .id("orig1")
                        .domain_name("example.com")
                        .build()
                        .unwrap(),
                )
                .build()
                .unwrap(),
        )
        .default_cache_behavior(
            DefaultCacheBehavior::builder()
                .target_origin_id("orig1")
                .viewer_protocol_policy(ViewerProtocolPolicy::RedirectToHttps)
                .build()
                .unwrap(),
        )
        .build()
        .unwrap()
}

#[tokio::test]
async fn e2e_cf_create_distribution() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let c = aws_sdk_cloudfront::Client::new(&cfg);
    let res = c
        .create_distribution()
        .distribution_config(empty_distribution_config())
        .send()
        .await
        .unwrap();
    assert!(res.distribution().unwrap().id().starts_with("E"));
}

#[tokio::test]
async fn e2e_cf_list_distributions() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let c = aws_sdk_cloudfront::Client::new(&cfg);
    for _ in 0..2 {
        c.create_distribution()
            .distribution_config(empty_distribution_config())
            .send()
            .await
            .unwrap();
    }
    let res = c.list_distributions().send().await.unwrap();
    assert_eq!(res.distribution_list().unwrap().quantity(), 2);
}
