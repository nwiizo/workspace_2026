//! documentdb — AWS Query protocol stub.

use super::stub;
use crate::registry::Registry;
use std::sync::Arc;

static ACTIONS: &[&str] = &["DescribeDBClusters", "CreateDBCluster", "DeleteDBCluster"];

pub fn register(registry: &Arc<Registry>) {
    stub::register_query(registry, "documentdb", "docdb", ACTIONS);
}
