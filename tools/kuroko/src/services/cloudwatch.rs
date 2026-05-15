//! cloudwatch — AWS Query protocol stub.

use super::stub;
use crate::registry::Registry;
use std::sync::Arc;

static ACTIONS: &[&str] = &[
    "PutMetricData",
    "GetMetricStatistics",
    "ListMetrics",
    "PutMetricAlarm",
    "DescribeAlarms",
    "DeleteAlarms",
];

pub fn register(registry: &Arc<Registry>) {
    stub::register_query(registry, "cloudwatch", "monitoring", ACTIONS);
}
