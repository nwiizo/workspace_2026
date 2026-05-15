//! elasticbeanstalk — AWS Query protocol stub.

use super::stub;
use crate::registry::Registry;
use std::sync::Arc;

static ACTIONS: &[&str] = &[
    "CreateApplication",
    "DescribeApplications",
    "DeleteApplication",
];

pub fn register(registry: &Arc<Registry>) {
    stub::register_query(registry, "elasticbeanstalk", "elasticbeanstalk", ACTIONS);
}
