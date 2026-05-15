//! Service trait — every AWS service plugged into kuroko implements this.

/// The region kuroko emulates. SDK callers usually configure their client to
/// `us-east-1`; we mirror that in ARNs and error envelopes so the shapes
/// match AWS examples.
pub const EMULATED_REGION: &str = "us-east-1";

/// The 12-digit account id baked into ARNs.
pub const EMULATED_ACCOUNT_ID: &str = "000000000000";

/// Convert a `persistence::PersistError` into an `AwsError` without leaking
/// internal filesystem paths to the client (code #4). The original error is
/// surfaced to operators via `tracing::warn!` so debuggability isn't lost.
pub(crate) fn persistence_error(
    err: crate::persistence::PersistError,
) -> crate::aws_error::AwsError {
    tracing::warn!(error = %err, "snapshot operation failed");
    crate::aws_error::AwsError::internal("snapshot unavailable")
}

use std::sync::Arc;

use async_trait::async_trait;
use axum::Router;

use crate::aws_error::AwsError;
use crate::persistence::Snapshot;

/// Shared runtime context handed to every service when it builds its router or
/// handles a dispatched protocol request.
#[derive(Debug, Clone)]
pub struct ServiceContext {
    pub snapshot: Option<Snapshot>,
}

impl ServiceContext {
    pub fn new(snapshot: Option<Snapshot>) -> Self {
        Self { snapshot }
    }
}

/// Trait shared by all services. `name` is the lowercase service identifier
/// (matching AWS SDK service ids: "s3", "sqs", "dynamodb", ...).
#[async_trait]
pub trait Service: Send + Sync {
    fn name(&self) -> &'static str;

    /// Per-service axum router merged at the top level. Most services return
    /// `Router::new()` and rely on the protocol dispatchers instead; REST-style
    /// services (S3) provide their full router here.
    fn router(&self, _ctx: ServiceContext) -> Router {
        Router::new()
    }

    /// Optional restore hook called once at startup if persistence is enabled.
    fn restore(&self, _ctx: &ServiceContext) -> Result<(), AwsError> {
        Ok(())
    }

    /// Persist current state. Called on graceful shutdown and by reset hooks
    /// where applicable. Services without resettable/persistable state leave
    /// the default no-op implementation.
    fn snapshot(&self, _ctx: &ServiceContext) -> Result<(), AwsError> {
        Ok(())
    }

    /// Drop all in-memory state. Invoked by `POST /_kuroko/reset` to give tests
    /// a clean slate.
    fn reset(&self) {}

    /// Optional `Any` projection used by cross-service wiring (e.g. SNS →
    /// SQS fanout). The default returns `None`; concrete services that need
    /// to be reached directly override this.
    fn as_any(&self) -> Option<&dyn std::any::Any> {
        None
    }
}

/// Wrapper to share services as Arc<dyn Service>.
pub type DynService = Arc<dyn Service>;

/// Marker implemented by services that speak the AWS JSON protocol (1.0/1.1)
/// dispatched via `X-Amz-Target`.
#[async_trait]
pub trait JsonProtocolService: Service {
    /// e.g. "AmazonSQS", "DynamoDB_20120810", "AWSEvents".
    fn target_prefix(&self) -> &'static str;

    async fn dispatch(
        &self,
        ctx: ServiceContext,
        action: &str,
        body: bytes::Bytes,
    ) -> Result<serde_json::Value, AwsError>;
}

pub type DynJsonService = Arc<dyn JsonProtocolService>;

/// Marker for AWS Query-protocol services (form-urlencoded body with `Action=`).
#[async_trait]
pub trait QueryProtocolService: Service {
    /// User-Agent service identifier disambiguator (e.g. "rds", "ec2").
    fn sdk_id(&self) -> &'static str;

    /// All actions this service handles — used by the dispatcher to route by
    /// `Action=` parameter.
    fn actions(&self) -> &'static [&'static str];

    async fn dispatch(
        &self,
        ctx: ServiceContext,
        action: &str,
        params: &std::collections::HashMap<String, String>,
    ) -> Result<String, AwsError>;
}

pub type DynQueryService = Arc<dyn QueryProtocolService>;

/// Marker for Smithy RPC v2 CBOR services routed via
/// `/service/{name}/operation/{op}`.
#[async_trait]
pub trait CborProtocolService: Service {
    fn smithy_service(&self) -> &'static str;

    async fn dispatch(
        &self,
        ctx: ServiceContext,
        operation: &str,
        body: bytes::Bytes,
    ) -> Result<bytes::Bytes, AwsError>;
}

pub type DynCborService = Arc<dyn CborProtocolService>;
