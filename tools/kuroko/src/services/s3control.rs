//! S3 Control — minimal named-resource stub.
use super::resource_stub::ResourceStub;
use crate::registry::Registry;
use std::sync::Arc;

pub fn register(registry: &Arc<Registry>) {
    registry.register(Arc::new(ResourceStub::new(
        "s3control",
        "/v20180820/accesspoint",
        "/v20180820/accesspoint/{name}",
        "Name",
        "AccessPointList",
    )));
}
