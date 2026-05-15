//! cloudformation — AWS Query protocol stub.

use super::stub;
use crate::registry::Registry;
use std::sync::Arc;

static ACTIONS: &[&str] = &[
    "CreateStack",
    "DeleteStack",
    "DescribeStacks",
    "UpdateStack",
    "ListStacks",
];

pub fn register(registry: &Arc<Registry>) {
    stub::register_query(registry, "cloudformation", "cloudformation", ACTIONS);
}
