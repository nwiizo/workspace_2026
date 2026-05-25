//! AWS Security Lake — minimal named-resource stub.
use super::resource_stub::ResourceStub;
use crate::registry::Registry;
use std::sync::Arc;

pub fn register(registry: &Arc<Registry>) {
    registry.register(Arc::new(ResourceStub::new(
        "securitylake",
        "/v1/datalakes",
        "/v1/datalakes/{name}",
        "region",
        "dataLakes",
    )));
}
