//! EMR Serverless — minimal named-resource stub.
use super::resource_stub::ResourceStub;
use crate::registry::Registry;
use std::sync::Arc;

pub fn register(registry: &Arc<Registry>) {
    registry.register(Arc::new(ResourceStub::new(
        "emrserverless",
        "/applications",
        "/applications/{name}",
        "name",
        "applications",
    )));
}
