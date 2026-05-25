//! AWS App Mesh — minimal named-resource stub.
use super::resource_stub::ResourceStub;
use crate::registry::Registry;
use std::sync::Arc;

pub fn register(registry: &Arc<Registry>) {
    registry.register(Arc::new(ResourceStub::new(
        "appmesh",
        "/v20190125/meshes",
        "/v20190125/meshes/{name}",
        "meshName",
        "meshes",
    )));
}
