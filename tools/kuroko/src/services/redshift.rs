//! redshift — AWS Query protocol stub.

use super::stub;
use crate::registry::Registry;
use std::sync::Arc;

static ACTIONS: &[&str] = &["CreateCluster", "DescribeClusters", "DeleteCluster"];

pub fn register(registry: &Arc<Registry>) {
    stub::register_query(registry, "redshift", "redshift", ACTIONS);
}
