//! CloudWatch (metrics) E2E tests against AWS official API spec.
//!
//! References:
//! - PutMetricData:         <https://docs.aws.amazon.com/AmazonCloudWatch/latest/APIReference/API_PutMetricData.html>
//! - GetMetricStatistics:   <https://docs.aws.amazon.com/AmazonCloudWatch/latest/APIReference/API_GetMetricStatistics.html>
//! - PutMetricAlarm:        <https://docs.aws.amazon.com/AmazonCloudWatch/latest/APIReference/API_PutMetricAlarm.html>

mod common;

use aws_sdk_cloudwatch::primitives::DateTime;
use aws_sdk_cloudwatch::types::{
    ComparisonOperator, Dimension, MetricDatum, StandardUnit, StateValue, Statistic,
};
use pretty_assertions::assert_eq;

#[tokio::test]
async fn e2e_cw_put_metric_data_then_get_statistics() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let cw = aws_sdk_cloudwatch::Client::new(&cfg);

    let now_secs = chrono::Utc::now().timestamp();
    let ts = DateTime::from_secs(now_secs);
    let datum = MetricDatum::builder()
        .metric_name("Latency")
        .value(42.0)
        .unit(StandardUnit::Milliseconds)
        .timestamp(ts)
        .dimensions(Dimension::builder().name("Service").value("api").build())
        .build();
    cw.put_metric_data()
        .namespace("kuroko/api")
        .metric_data(datum)
        .send()
        .await
        .unwrap();

    let stats = cw
        .get_metric_statistics()
        .namespace("kuroko/api")
        .metric_name("Latency")
        .dimensions(Dimension::builder().name("Service").value("api").build())
        .start_time(DateTime::from_secs(now_secs - 300))
        .end_time(DateTime::from_secs(now_secs + 300))
        .period(60)
        .statistics(Statistic::Sum)
        .statistics(Statistic::Average)
        .send()
        .await
        .unwrap();
    let dps = stats.datapoints();
    assert_eq!(dps.len(), 1);
    assert_eq!(dps[0].sum(), Some(42.0));
    assert_eq!(dps[0].average(), Some(42.0));
    assert_eq!(dps[0].sample_count(), Some(1.0));
}

#[tokio::test]
async fn e2e_cw_multiple_datapoints_aggregate() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let cw = aws_sdk_cloudwatch::Client::new(&cfg);

    let now_secs = chrono::Utc::now().timestamp();
    for v in [10.0, 20.0, 30.0] {
        cw.put_metric_data()
            .namespace("kuroko/api")
            .metric_data(
                MetricDatum::builder()
                    .metric_name("Hits")
                    .value(v)
                    .timestamp(DateTime::from_secs(now_secs))
                    .build(),
            )
            .send()
            .await
            .unwrap();
    }
    let stats = cw
        .get_metric_statistics()
        .namespace("kuroko/api")
        .metric_name("Hits")
        .start_time(DateTime::from_secs(now_secs - 300))
        .end_time(DateTime::from_secs(now_secs + 300))
        .period(60)
        .statistics(Statistic::Sum)
        .send()
        .await
        .unwrap();
    let dps = stats.datapoints();
    assert_eq!(dps.len(), 1);
    assert_eq!(dps[0].sum(), Some(60.0));
    assert_eq!(dps[0].minimum(), Some(10.0));
    assert_eq!(dps[0].maximum(), Some(30.0));
    assert_eq!(dps[0].sample_count(), Some(3.0));
}

#[tokio::test]
async fn e2e_cw_list_metrics_filters_by_namespace() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let cw = aws_sdk_cloudwatch::Client::new(&cfg);

    for (ns, name) in [
        ("kuroko/api", "Latency"),
        ("kuroko/api", "Errors"),
        ("kuroko/db", "Connections"),
    ] {
        cw.put_metric_data()
            .namespace(ns)
            .metric_data(MetricDatum::builder().metric_name(name).value(1.0).build())
            .send()
            .await
            .unwrap();
    }
    let res = cw
        .list_metrics()
        .namespace("kuroko/api")
        .send()
        .await
        .unwrap();
    assert_eq!(res.metrics().len(), 2);
}

#[tokio::test]
async fn e2e_cw_put_then_describe_alarm() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let cw = aws_sdk_cloudwatch::Client::new(&cfg);

    cw.put_metric_alarm()
        .alarm_name("high-latency")
        .namespace("kuroko/api")
        .metric_name("Latency")
        .statistic(Statistic::Average)
        .threshold(500.0)
        .comparison_operator(ComparisonOperator::GreaterThanThreshold)
        .period(60)
        .evaluation_periods(2)
        .send()
        .await
        .unwrap();

    let res = cw
        .describe_alarms()
        .alarm_names("high-latency")
        .send()
        .await
        .unwrap();
    assert_eq!(res.metric_alarms().len(), 1);
    let a = &res.metric_alarms()[0];
    assert_eq!(a.alarm_name(), Some("high-latency"));
    assert_eq!(a.state_value(), Some(&StateValue::InsufficientData));
    assert_eq!(a.threshold(), Some(500.0));
}

#[tokio::test]
async fn e2e_cw_delete_alarms() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let cw = aws_sdk_cloudwatch::Client::new(&cfg);

    for n in ["a1", "a2"] {
        cw.put_metric_alarm()
            .alarm_name(n)
            .namespace("kuroko")
            .metric_name("x")
            .statistic(Statistic::Sum)
            .threshold(1.0)
            .comparison_operator(ComparisonOperator::GreaterThanThreshold)
            .period(60)
            .evaluation_periods(1)
            .send()
            .await
            .unwrap();
    }
    cw.delete_alarms().alarm_names("a1").send().await.unwrap();
    let res = cw.describe_alarms().send().await.unwrap();
    assert_eq!(res.metric_alarms().len(), 1);
    assert_eq!(res.metric_alarms()[0].alarm_name(), Some("a2"));
}
