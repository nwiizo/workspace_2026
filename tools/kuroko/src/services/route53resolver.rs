//! route53resolver — AWS JSON protocol stub.

use super::stub;
use crate::registry::Registry;
use std::sync::Arc;

pub fn register(registry: &Arc<Registry>) {
    stub::register_json(registry, "route53resolver", "Route53Resolver");
}
