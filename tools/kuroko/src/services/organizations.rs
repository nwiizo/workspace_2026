//! organizations — AWS JSON protocol stub.

use super::stub;
use crate::registry::Registry;
use std::sync::Arc;

pub fn register(registry: &Arc<Registry>) {
    stub::register_json(registry, "organizations", "AWSOrganizationsV20161128");
}
