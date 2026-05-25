//! Cost Explorer E2E.
mod common;
use aws_sdk_costexplorer::types::{DateInterval, Granularity};
use pretty_assertions::assert_eq;

#[tokio::test]
async fn e2e_ce_get_cost_and_usage() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let c = aws_sdk_costexplorer::Client::new(&cfg);
    let res = c
        .get_cost_and_usage()
        .time_period(
            DateInterval::builder()
                .start("2026-01-01")
                .end("2026-01-31")
                .build()
                .unwrap(),
        )
        .granularity(Granularity::Monthly)
        .metrics("BlendedCost")
        .send()
        .await
        .unwrap();
    assert_eq!(res.results_by_time().len(), 1);
}

#[tokio::test]
async fn e2e_ce_get_tags_empty() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let c = aws_sdk_costexplorer::Client::new(&cfg);
    let res = c
        .get_tags()
        .time_period(
            DateInterval::builder()
                .start("2026-01-01")
                .end("2026-01-31")
                .build()
                .unwrap(),
        )
        .send()
        .await
        .unwrap();
    assert_eq!(res.total_size(), 0);
}
