//! Shared scaffolding for stub services.
//!
//! Each stub service is a thin newtype that implements the appropriate
//! protocol trait and always returns `UnsupportedOperation`. The point is
//! routing: SDK requests find their service and get a structured 501 back,
//! which gives us a clean growth path — implementations slot in by replacing
//! the stub registration with a real one.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use bytes::Bytes;

use crate::aws_error::AwsError;
use crate::registry::Registry;
use crate::service::{
    CborProtocolService, JsonProtocolService, QueryProtocolService, Service, ServiceContext,
};

/// JSON-protocol stub.
pub struct JsonStub {
    pub name: &'static str,
    pub target_prefix: &'static str,
}

#[async_trait]
impl Service for JsonStub {
    fn name(&self) -> &'static str {
        self.name
    }
}

#[async_trait]
impl JsonProtocolService for JsonStub {
    fn target_prefix(&self) -> &'static str {
        self.target_prefix
    }

    async fn dispatch(
        &self,
        _ctx: ServiceContext,
        action: &str,
        _body: Bytes,
    ) -> Result<serde_json::Value, AwsError> {
        Err(AwsError::unsupported(format!("{}::{action}", self.name)))
    }
}

/// Query-protocol stub. Accepts a flat list of actions because we don't know
/// the per-service action set until kuroko grows real coverage.
pub struct QueryStub {
    pub name: &'static str,
    pub sdk_id: &'static str,
    pub actions: &'static [&'static str],
}

#[async_trait]
impl Service for QueryStub {
    fn name(&self) -> &'static str {
        self.name
    }
}

#[async_trait]
impl QueryProtocolService for QueryStub {
    fn sdk_id(&self) -> &'static str {
        self.sdk_id
    }

    fn actions(&self) -> &'static [&'static str] {
        self.actions
    }

    async fn dispatch(
        &self,
        _ctx: ServiceContext,
        action: &str,
        _params: &HashMap<String, String>,
    ) -> Result<String, AwsError> {
        Err(AwsError::unsupported(format!("{}::{action}", self.name)))
    }
}

/// CBOR-protocol stub.
pub struct CborStub {
    pub name: &'static str,
    pub smithy_service: &'static str,
}

#[async_trait]
impl Service for CborStub {
    fn name(&self) -> &'static str {
        self.name
    }
}

#[async_trait]
impl CborProtocolService for CborStub {
    fn smithy_service(&self) -> &'static str {
        self.smithy_service
    }

    async fn dispatch(
        &self,
        _ctx: ServiceContext,
        operation: &str,
        _body: Bytes,
    ) -> Result<Bytes, AwsError> {
        Err(AwsError::unsupported(format!("{}::{operation}", self.name)))
    }
}

pub fn register_json(registry: &Arc<Registry>, name: &'static str, target_prefix: &'static str) {
    registry.register_json(Arc::new(JsonStub {
        name,
        target_prefix,
    }));
}

pub fn register_query(
    registry: &Arc<Registry>,
    name: &'static str,
    sdk_id: &'static str,
    actions: &'static [&'static str],
) {
    registry.register_query(Arc::new(QueryStub {
        name,
        sdk_id,
        actions,
    }));
}

pub fn register_cbor(registry: &Arc<Registry>, name: &'static str, smithy_service: &'static str) {
    registry.register_cbor(Arc::new(CborStub {
        name,
        smithy_service,
    }));
}

/// REST-style stubs don't need to participate in the unified dispatcher because
/// they're routed by URL; just register the bare Service so /_kuroko/services
/// lists them.
pub struct BareStub {
    pub name: &'static str,
}

#[async_trait]
impl Service for BareStub {
    fn name(&self) -> &'static str {
        self.name
    }
}

pub fn register_bare(registry: &Arc<Registry>, name: &'static str) {
    registry.register(Arc::new(BareStub { name }));
}
