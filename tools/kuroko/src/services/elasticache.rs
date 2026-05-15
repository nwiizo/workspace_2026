//! elasticache — AWS Query protocol stub.

use super::stub;
use crate::registry::Registry;
use std::sync::Arc;

static ACTIONS: &[&str] = &[
    "CreateCacheCluster",
    "DescribeCacheClusters",
    "DeleteCacheCluster",
];

pub fn register(registry: &Arc<Registry>) {
    stub::register_query(registry, "elasticache", "elasticache", ACTIONS);
}
