//! elbv2 — AWS Query protocol stub.

use super::stub;
use crate::registry::Registry;
use std::sync::Arc;

static ACTIONS: &[&str] = &[
    "CreateLoadBalancer",
    "DescribeLoadBalancers",
    "DeleteLoadBalancer",
    "CreateTargetGroup",
    "DescribeTargetGroups",
    "RegisterTargets",
];

pub fn register(registry: &Arc<Registry>) {
    stub::register_query(registry, "elbv2", "elasticloadbalancing", ACTIONS);
}
