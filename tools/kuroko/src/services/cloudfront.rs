//! CloudFront — REST/XML under `/2020-05-31/distribution`.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use axum::Router;
use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::{StatusCode, header};
use axum::response::Response;
use axum::routing::get;
use bytes::Bytes;
use parking_lot::RwLock;
use uuid::Uuid;

use crate::aws_error::{AwsError, xml_escape};
use crate::service::{Service, ServiceContext, persistence_error};

const NS: &str = "http://cloudfront.amazonaws.com/doc/2020-05-31/";

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
struct State_ {
    distributions: HashMap<String, Distribution>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct Distribution {
    id: String,
    domain_name: String,
    status: String,
    arn: String,
}

pub struct CloudFront {
    state: Arc<RwLock<State_>>,
}
impl CloudFront {
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(State_::default())),
        }
    }
}
impl Default for CloudFront {
    fn default() -> Self {
        Self::new()
    }
}

type CfState = Arc<RwLock<State_>>;

#[async_trait]
impl Service for CloudFront {
    fn name(&self) -> &'static str {
        "cloudfront"
    }
    fn reset(&self) {
        *self.state.write() = State_::default();
    }
    fn restore(&self, ctx: &ServiceContext) -> Result<(), AwsError> {
        if let Some(snap) = &ctx.snapshot
            && let Some(data) = snap
                .load::<State_>("cloudfront")
                .map_err(persistence_error)?
        {
            *self.state.write() = data;
        }
        Ok(())
    }
    fn snapshot(&self, ctx: &ServiceContext) -> Result<(), AwsError> {
        if let Some(snap) = &ctx.snapshot {
            snap.save("cloudfront", &*self.state.read())
                .map_err(persistence_error)?;
        }
        Ok(())
    }
    fn router(&self, _ctx: ServiceContext) -> Router {
        let state = self.state.clone();
        Router::new()
            .route(
                "/2020-05-31/distribution",
                axum::routing::post(create_distribution).get(list_distributions),
            )
            .route(
                "/2020-05-31/distribution/{id}",
                get(get_distribution).delete(delete_distribution),
            )
            .with_state(state)
    }
}

async fn create_distribution(State(state): State<CfState>, _body: Bytes) -> Response {
    let id = format!(
        "E{}",
        Uuid::new_v4().simple().to_string()[..14].to_uppercase()
    );
    let domain = format!("{}.cloudfront.kuroko.test", id.to_lowercase());
    let arn = format!("arn:aws:cloudfront::000000000000:distribution/{id}");
    state.write().distributions.insert(
        id.clone(),
        Distribution {
            id: id.clone(),
            domain_name: domain.clone(),
            status: "Deployed".into(),
            arn,
        },
    );
    rest_xml(
        StatusCode::CREATED,
        &format!(
            r#"<Distribution xmlns="{NS}"><Id>{id}</Id><Status>Deployed</Status><DomainName>{domain}</DomainName></Distribution>"#
        ),
    )
}

async fn get_distribution(State(state): State<CfState>, Path(id): Path<String>) -> Response {
    let s = state.read();
    match s.distributions.get(&id) {
        Some(d) => rest_xml(
            StatusCode::OK,
            &format!(
                r#"<Distribution xmlns="{NS}"><Id>{id}</Id><Status>{status}</Status><DomainName>{domain}</DomainName><ARN>{arn}</ARN></Distribution>"#,
                id = xml_escape(&d.id),
                status = xml_escape(&d.status),
                domain = xml_escape(&d.domain_name),
                arn = xml_escape(&d.arn),
            ),
        ),
        None => rest_err(StatusCode::NOT_FOUND, "NoSuchDistribution", "not found"),
    }
}

async fn list_distributions(State(state): State<CfState>) -> Response {
    let s = state.read();
    let mut items = String::new();
    for d in s.distributions.values() {
        items.push_str(&format!(
            "<DistributionSummary><Id>{id}</Id><Status>{status}</Status><DomainName>{domain}</DomainName><ARN>{arn}</ARN></DistributionSummary>",
            id = xml_escape(&d.id),
            status = xml_escape(&d.status),
            domain = xml_escape(&d.domain_name),
            arn = xml_escape(&d.arn),
        ));
    }
    let count = s.distributions.len();
    rest_xml(
        StatusCode::OK,
        &format!(
            r#"<DistributionList xmlns="{NS}"><Marker></Marker><MaxItems>100</MaxItems><IsTruncated>false</IsTruncated><Quantity>{count}</Quantity><Items>{items}</Items></DistributionList>"#
        ),
    )
}

async fn delete_distribution(State(state): State<CfState>, Path(id): Path<String>) -> Response {
    state.write().distributions.remove(&id);
    Response::builder()
        .status(StatusCode::NO_CONTENT)
        .body(Body::empty())
        .unwrap()
}

fn rest_xml(status: StatusCode, body: &str) -> Response {
    Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, "application/xml")
        .body(Body::from(body.to_string()))
        .unwrap()
}

fn rest_err(status: StatusCode, code: &str, msg: &str) -> Response {
    Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, "application/xml")
        .body(Body::from(format!(
            r#"<?xml version="1.0"?><ErrorResponse xmlns="{NS}"><Error><Type>Sender</Type><Code>{code}</Code><Message>{msg}</Message></Error></ErrorResponse>"#
        )))
        .unwrap()
}

pub fn register(registry: &Arc<crate::registry::Registry>) {
    registry.register(Arc::new(CloudFront::new()));
}
