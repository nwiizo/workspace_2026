//! lambda — REST-style stub (no router yet; reachable as a named service).

use super::stub;
use crate::registry::Registry;
use std::sync::Arc;

pub fn register(registry: &Arc<Registry>) {
    stub::register_bare(registry, "lambda");
}
