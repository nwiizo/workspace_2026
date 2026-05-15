//! ec2 — AWS Query protocol stub.

use super::stub;
use crate::registry::Registry;
use std::sync::Arc;

static ACTIONS: &[&str] = &[
    "RunInstances",
    "TerminateInstances",
    "DescribeInstances",
    "CreateVpc",
    "DescribeVpcs",
    "CreateSubnet",
    "DescribeSubnets",
    "CreateSecurityGroup",
    "DescribeSecurityGroups",
    "AuthorizeSecurityGroupIngress",
    "DescribeAvailabilityZones",
    "DescribeRegions",
];

pub fn register(registry: &Arc<Registry>) {
    stub::register_query(registry, "ec2", "ec2", ACTIONS);
}
