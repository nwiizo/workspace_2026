//! Route 53 — REST/XML protocol, mounted under `/2013-04-01/*`.
//!
//! Implements hosted zone CRUD plus ChangeResourceRecordSets (CREATE / DELETE
//! / UPSERT) and ListResourceRecordSets. Record-set values are stored as a
//! plain string vector; kuroko does not actually resolve DNS — this is API
//! shape only.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use axum::Router;
use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::{StatusCode, header};
use axum::response::Response;
use axum::routing::{get, post};
use bytes::Bytes;
use parking_lot::RwLock;
use uuid::Uuid;

use crate::aws_error::{AwsError, xml_escape};
use crate::service::{Service, ServiceContext, persistence_error};

const NS: &str = "https://route53.amazonaws.com/doc/2013-04-01/";

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
struct State_ {
    zones: HashMap<String, HostedZone>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct HostedZone {
    id: String,
    name: String,
    caller_reference: String,
    comment: Option<String>,
    private_zone: bool,
    records: Vec<ResourceRecordSet>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct ResourceRecordSet {
    name: String,
    type_: String,
    ttl: i64,
    values: Vec<String>,
}

pub struct Route53 {
    state: Arc<RwLock<State_>>,
}

impl Route53 {
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(State_::default())),
        }
    }
}

impl Default for Route53 {
    fn default() -> Self {
        Self::new()
    }
}

type R53State = Arc<RwLock<State_>>;

#[async_trait]
impl Service for Route53 {
    fn name(&self) -> &'static str {
        "route53"
    }

    fn reset(&self) {
        self.state.write().zones.clear();
    }

    fn restore(&self, ctx: &ServiceContext) -> Result<(), AwsError> {
        if let Some(snap) = &ctx.snapshot
            && let Some(data) = snap.load::<State_>("route53").map_err(persistence_error)?
        {
            *self.state.write() = data;
        }
        Ok(())
    }

    fn snapshot(&self, ctx: &ServiceContext) -> Result<(), AwsError> {
        if let Some(snap) = &ctx.snapshot {
            let data = self.state.read();
            snap.save("route53", &*data).map_err(persistence_error)?;
        }
        Ok(())
    }

    fn router(&self, _ctx: ServiceContext) -> Router {
        let state = self.state.clone();
        Router::new()
            .route(
                "/2013-04-01/hostedzone",
                post(create_hosted_zone).get(list_hosted_zones),
            )
            .route(
                "/2013-04-01/hostedzone/{id}",
                get(get_hosted_zone).delete(delete_hosted_zone),
            )
            .route(
                "/2013-04-01/hostedzone/{id}/rrset",
                post(change_record_sets).get(list_record_sets),
            )
            .with_state(state)
    }
}

async fn create_hosted_zone(State(state): State<R53State>, body: Bytes) -> Response {
    let xml = match std::str::from_utf8(&body) {
        Ok(s) => s,
        Err(_) => return rest_error(AwsError::new("InvalidInput", "body must be utf-8")),
    };
    let name = match extract_tag(xml, "Name") {
        Some(n) => normalize_zone_name(&n),
        None => return rest_error(AwsError::new("InvalidInput", "Name required")),
    };
    let caller_reference = extract_tag(xml, "CallerReference").unwrap_or_default();
    let comment = extract_tag(xml, "Comment");
    let mut s = state.write();
    let id = format!(
        "Z{}",
        Uuid::new_v4().simple().to_string()[..13].to_uppercase()
    );
    let zone = HostedZone {
        id: id.clone(),
        name: name.clone(),
        caller_reference,
        comment,
        private_zone: false,
        records: Vec::new(),
    };
    let body_xml = format!(
        r#"<CreateHostedZoneResponse xmlns="{NS}">
  <HostedZone>{zone}</HostedZone>
  <ChangeInfo><Id>/change/{cid}</Id><Status>INSYNC</Status><SubmittedAt>{ts}</SubmittedAt></ChangeInfo>
  <Location>/2013-04-01/hostedzone/{id}</Location>
</CreateHostedZoneResponse>"#,
        zone = zone_xml(&zone),
        cid = short_id(),
        ts = chrono::Utc::now().to_rfc3339(),
    );
    s.zones.insert(id.clone(), zone);
    rest_xml(StatusCode::CREATED, body_xml)
}

async fn list_hosted_zones(State(state): State<R53State>) -> Response {
    let s = state.read();
    let mut members = String::new();
    for z in s.zones.values() {
        members.push_str(&format!("<HostedZone>{}</HostedZone>", zone_xml(z)));
    }
    let body = format!(
        r#"<ListHostedZonesResponse xmlns="{NS}">
  <HostedZones>{members}</HostedZones>
  <IsTruncated>false</IsTruncated>
  <MaxItems>100</MaxItems>
</ListHostedZonesResponse>"#
    );
    rest_xml(StatusCode::OK, body)
}

async fn get_hosted_zone(State(state): State<R53State>, Path(id): Path<String>) -> Response {
    let s = state.read();
    let Some(z) = s.zones.get(&id) else {
        return rest_error(no_such_zone(&id));
    };
    let body = format!(
        r#"<GetHostedZoneResponse xmlns="{NS}">
  <HostedZone>{zone}</HostedZone>
  <DelegationSet><NameServers><NameServer>ns-kuroko.example.</NameServer></NameServers></DelegationSet>
</GetHostedZoneResponse>"#,
        zone = zone_xml(z),
    );
    rest_xml(StatusCode::OK, body)
}

async fn delete_hosted_zone(State(state): State<R53State>, Path(id): Path<String>) -> Response {
    let mut s = state.write();
    if s.zones.remove(&id).is_none() {
        return rest_error(no_such_zone(&id));
    }
    let body = format!(
        r#"<DeleteHostedZoneResponse xmlns="{NS}">
  <ChangeInfo><Id>/change/{cid}</Id><Status>INSYNC</Status><SubmittedAt>{ts}</SubmittedAt></ChangeInfo>
</DeleteHostedZoneResponse>"#,
        cid = short_id(),
        ts = chrono::Utc::now().to_rfc3339(),
    );
    rest_xml(StatusCode::OK, body)
}

async fn change_record_sets(
    State(state): State<R53State>,
    Path(id): Path<String>,
    body: Bytes,
) -> Response {
    let xml = match std::str::from_utf8(&body) {
        Ok(s) => s,
        Err(_) => return rest_error(AwsError::new("InvalidInput", "body must be utf-8")),
    };
    let mut s = state.write();
    let Some(z) = s.zones.get_mut(&id) else {
        return rest_error(no_such_zone(&id));
    };
    for change in extract_changes(xml) {
        let normalized_name = normalize_zone_name(&change.name);
        // remove any existing RR with the same (name, type)
        let mut removed = false;
        z.records.retain(|r| {
            if r.name == normalized_name && r.type_ == change.type_ {
                removed = true;
                false
            } else {
                true
            }
        });
        match change.action.as_str() {
            "CREATE" | "UPSERT" => {
                z.records.push(ResourceRecordSet {
                    name: normalized_name,
                    type_: change.type_,
                    ttl: change.ttl,
                    values: change.values,
                });
            }
            "DELETE" => {
                // Already removed above; nothing else to do.
                let _ = removed;
            }
            _ => {}
        }
    }
    let body = format!(
        r#"<ChangeResourceRecordSetsResponse xmlns="{NS}">
  <ChangeInfo><Id>/change/{cid}</Id><Status>INSYNC</Status><SubmittedAt>{ts}</SubmittedAt></ChangeInfo>
</ChangeResourceRecordSetsResponse>"#,
        cid = short_id(),
        ts = chrono::Utc::now().to_rfc3339(),
    );
    rest_xml(StatusCode::OK, body)
}

async fn list_record_sets(State(state): State<R53State>, Path(id): Path<String>) -> Response {
    let s = state.read();
    let Some(z) = s.zones.get(&id) else {
        return rest_error(no_such_zone(&id));
    };
    let mut members = String::new();
    for r in &z.records {
        let mut vals = String::new();
        for v in &r.values {
            vals.push_str(&format!(
                "<ResourceRecord><Value>{}</Value></ResourceRecord>",
                xml_escape(v)
            ));
        }
        members.push_str(&format!(
            "<ResourceRecordSet><Name>{}</Name><Type>{}</Type><TTL>{}</TTL><ResourceRecords>{}</ResourceRecords></ResourceRecordSet>",
            xml_escape(&r.name),
            xml_escape(&r.type_),
            r.ttl,
            vals,
        ));
    }
    let body = format!(
        r#"<ListResourceRecordSetsResponse xmlns="{NS}">
  <ResourceRecordSets>{members}</ResourceRecordSets>
  <IsTruncated>false</IsTruncated>
  <MaxItems>100</MaxItems>
</ListResourceRecordSetsResponse>"#
    );
    rest_xml(StatusCode::OK, body)
}

#[derive(Debug)]
struct ChangeReq {
    action: String,
    name: String,
    type_: String,
    ttl: i64,
    values: Vec<String>,
}

/// Crude tag extractor: returns the inner text of the first `<tag>...</tag>`.
fn extract_tag(xml: &str, tag: &str) -> Option<String> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let start = xml.find(&open)? + open.len();
    let end = xml[start..].find(&close)? + start;
    Some(xml[start..end].to_string())
}

fn extract_all_inner(xml: &str, tag: &str) -> Vec<String> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let mut out = Vec::new();
    let mut cursor = 0;
    while let Some(rel_start) = xml[cursor..].find(&open) {
        let inner_start = cursor + rel_start + open.len();
        let Some(rel_end) = xml[inner_start..].find(&close) else {
            break;
        };
        let inner_end = inner_start + rel_end;
        out.push(xml[inner_start..inner_end].to_string());
        cursor = inner_end + close.len();
    }
    out
}

fn extract_changes(xml: &str) -> Vec<ChangeReq> {
    let mut out = Vec::new();
    for change_xml in extract_all_inner(xml, "Change") {
        let action = extract_tag(&change_xml, "Action").unwrap_or_default();
        let Some(rrset) = extract_tag(&change_xml, "ResourceRecordSet") else {
            continue;
        };
        let name = extract_tag(&rrset, "Name").unwrap_or_default();
        let type_ = extract_tag(&rrset, "Type").unwrap_or_default();
        let ttl = extract_tag(&rrset, "TTL")
            .and_then(|t| t.parse().ok())
            .unwrap_or(300);
        let values = extract_all_inner(&rrset, "Value");
        out.push(ChangeReq {
            action,
            name,
            type_,
            ttl,
            values,
        });
    }
    out
}

fn normalize_zone_name(name: &str) -> String {
    // Route 53 stores zone / record names with a trailing dot.
    if name.ends_with('.') {
        name.to_string()
    } else {
        format!("{name}.")
    }
}

fn zone_xml(z: &HostedZone) -> String {
    let comment = z
        .comment
        .as_deref()
        .map(|c| format!("<Config><Comment>{}</Comment></Config>", xml_escape(c)))
        .unwrap_or_default();
    format!(
        "<Id>/hostedzone/{id}</Id><Name>{name}</Name><CallerReference>{cref}</CallerReference>{comment}<ResourceRecordSetCount>{count}</ResourceRecordSetCount>",
        id = z.id,
        name = xml_escape(&z.name),
        cref = xml_escape(&z.caller_reference),
        count = z.records.len(),
    )
}

fn no_such_zone(id: &str) -> AwsError {
    AwsError::new(
        "NoSuchHostedZone",
        format!("hosted zone '{id}' does not exist"),
    )
    .status(StatusCode::NOT_FOUND)
}

fn short_id() -> String {
    Uuid::new_v4().simple().to_string()[..14].to_uppercase()
}

fn rest_xml(status: StatusCode, body: String) -> Response {
    Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, "application/xml")
        .body(Body::from(body))
        .unwrap()
}

fn rest_error(err: AwsError) -> Response {
    let xml = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?><ErrorResponse xmlns=\"{NS}\"><Error><Type>Sender</Type><Code>{code}</Code><Message>{msg}</Message></Error><RequestId>{rid}</RequestId></ErrorResponse>",
        code = xml_escape(&err.code),
        msg = xml_escape(&err.message),
        rid = err.request_id(),
    );
    Response::builder()
        .status(err.status)
        .header(header::CONTENT_TYPE, "application/xml")
        .body(Body::from(xml))
        .unwrap()
}

pub fn register(registry: &Arc<crate::registry::Registry>) {
    registry.register(Arc::new(Route53::new()));
}
