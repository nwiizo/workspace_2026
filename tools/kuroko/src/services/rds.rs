//! rds — AWS Query protocol stub.

use super::stub;
use crate::registry::Registry;
use std::sync::Arc;

static ACTIONS: &[&str] = &[
    "CreateDBInstance",
    "DescribeDBInstances",
    "DeleteDBInstance",
    "CreateDBCluster",
    "DescribeDBClusters",
    "DeleteDBCluster",
    "CreateDBSnapshot",
    "DescribeDBSnapshots",
];

pub fn register(registry: &Arc<Registry>) {
    stub::register_query(registry, "rds", "rds", ACTIONS);
}
