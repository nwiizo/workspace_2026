//! S3 — REST/XML protocol.
//!
//! Supports the core object-store API surface used by most CI/dev workflows:
//! bucket CRUD, object PUT/GET/HEAD/DELETE, and ListObjectsV2 with prefix and
//! delimiter. Both path-style (`/bucket/key`) and virtual-hosted-style
//! (`bucket.s3.localhost/key`) addressing are accepted. CopyObject and the
//! multipart upload API are not yet implemented.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use axum::Router;
use axum::body::Body;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::Response;
use axum::routing::{get, put};
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use bytes::Bytes;
use chrono::{DateTime, Utc};
use md5::{Digest, Md5};
use parking_lot::RwLock;

use crate::aws_error::{AwsError, xml_escape};
use crate::service::{Service, ServiceContext};

#[derive(Debug, Default)]
struct State_ {
    buckets: HashMap<String, Bucket>,
}

#[derive(Debug, Default, Clone)]
struct Bucket {
    objects: HashMap<String, Object>,
    created: DateTime<Utc>,
}

#[derive(Debug, Clone)]
struct Object {
    data: Bytes,
    etag: String,
    content_type: String,
    last_modified: DateTime<Utc>,
    metadata: HashMap<String, String>,
}

// JSON-snapshot shape. Objects are base64-encoded so the snapshot file stays
// human-readable.
#[derive(serde::Serialize, serde::Deserialize, Default)]
struct PersistedState {
    buckets: HashMap<String, PersistedBucket>,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct PersistedBucket {
    objects: HashMap<String, PersistedObject>,
    created: DateTime<Utc>,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct PersistedObject {
    data_b64: String,
    etag: String,
    content_type: String,
    last_modified: DateTime<Utc>,
    metadata: HashMap<String, String>,
}

impl From<&State_> for PersistedState {
    fn from(s: &State_) -> Self {
        Self {
            buckets: s
                .buckets
                .iter()
                .map(|(name, b)| {
                    (
                        name.clone(),
                        PersistedBucket {
                            created: b.created,
                            objects: b
                                .objects
                                .iter()
                                .map(|(k, o)| {
                                    (
                                        k.clone(),
                                        PersistedObject {
                                            data_b64: BASE64.encode(&o.data),
                                            etag: o.etag.clone(),
                                            content_type: o.content_type.clone(),
                                            last_modified: o.last_modified,
                                            metadata: o.metadata.clone(),
                                        },
                                    )
                                })
                                .collect(),
                        },
                    )
                })
                .collect(),
        }
    }
}

impl From<PersistedState> for State_ {
    fn from(p: PersistedState) -> Self {
        Self {
            buckets: p
                .buckets
                .into_iter()
                .map(|(name, b)| {
                    (
                        name,
                        Bucket {
                            created: b.created,
                            objects: b
                                .objects
                                .into_iter()
                                .map(|(k, o)| {
                                    let data = BASE64
                                        .decode(o.data_b64.as_bytes())
                                        .map(Bytes::from)
                                        .unwrap_or_default();
                                    (
                                        k,
                                        Object {
                                            data,
                                            etag: o.etag,
                                            content_type: o.content_type,
                                            last_modified: o.last_modified,
                                            metadata: o.metadata,
                                        },
                                    )
                                })
                                .collect(),
                        },
                    )
                })
                .collect(),
        }
    }
}

pub struct S3 {
    state: Arc<RwLock<State_>>,
}

impl S3 {
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(State_::default())),
        }
    }
}

impl Default for S3 {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Service for S3 {
    fn name(&self) -> &'static str {
        "s3"
    }

    fn reset(&self) {
        self.state.write().buckets.clear();
    }

    fn restore(&self, ctx: &ServiceContext) -> Result<(), AwsError> {
        if let Some(snap) = &ctx.snapshot
            && let Some(data) = snap
                .load::<PersistedState>("s3")
                .map_err(crate::service::persistence_error)?
        {
            *self.state.write() = data.into();
        }
        Ok(())
    }

    fn snapshot(&self, ctx: &ServiceContext) -> Result<(), AwsError> {
        if let Some(snap) = &ctx.snapshot {
            let data = PersistedState::from(&*self.state.read());
            snap.save("s3", &data)
                .map_err(crate::service::persistence_error)?;
        }
        Ok(())
    }

    fn router(&self, _ctx: ServiceContext) -> Router {
        let state = self.state.clone();
        Router::new()
            // Path-style ListBuckets is `GET /`. POST `/` is owned by the
            // unified protocol dispatcher (JSON/Query), so the two coexist on
            // the same path via method-based routing.
            .route("/", get(list_buckets))
            // Path-style routes. The SDK occasionally appends a trailing slash
            // for bucket-scoped operations, so both shapes need every method.
            .route(
                "/{bucket}",
                put(put_bucket)
                    .delete(delete_bucket)
                    .get(get_bucket)
                    .head(head_bucket),
            )
            .route(
                "/{bucket}/",
                put(put_bucket)
                    .delete(delete_bucket)
                    .get(get_bucket)
                    .head(head_bucket),
            )
            .route(
                "/{bucket}/{*key}",
                put(put_object)
                    .get(get_object)
                    .head(head_object_path)
                    .delete(delete_object),
            )
            .layer(axum::extract::DefaultBodyLimit::max(100 * 1024 * 1024))
            .with_state(state)
    }
}

type S3State = Arc<RwLock<State_>>;

// === Bucket operations ===

async fn list_buckets(State(state): State<S3State>) -> Response {
    let buckets = state.read();
    let mut xml = String::from(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<ListAllMyBucketsResult xmlns="http://s3.amazonaws.com/doc/2006-03-01/">
<Owner><ID>kuroko</ID><DisplayName>kuroko</DisplayName></Owner>
<Buckets>"#,
    );
    for (name, b) in &buckets.buckets {
        xml.push_str(&format!(
            "<Bucket><Name>{}</Name><CreationDate>{}</CreationDate></Bucket>",
            xml_escape(name),
            b.created.format("%Y-%m-%dT%H:%M:%S%.3fZ"),
        ));
    }
    xml.push_str("</Buckets></ListAllMyBucketsResult>");
    xml_response(StatusCode::OK, xml)
}

async fn put_bucket(State(state): State<S3State>, Path(bucket): Path<String>) -> Response {
    let mut s = state.write();
    if s.buckets.contains_key(&bucket) {
        return AwsError::new("BucketAlreadyOwnedByYou", "bucket already exists")
            .status(StatusCode::CONFLICT)
            .to_rest_xml_response();
    }
    s.buckets.insert(
        bucket.clone(),
        Bucket {
            created: Utc::now(),
            ..Default::default()
        },
    );
    Response::builder()
        .status(StatusCode::OK)
        .header(header::LOCATION, format!("/{bucket}"))
        .body(Body::empty())
        .unwrap()
}

async fn delete_bucket(State(state): State<S3State>, Path(bucket): Path<String>) -> Response {
    let mut s = state.write();
    match s.buckets.get(&bucket) {
        Some(b) if !b.objects.is_empty() => AwsError::new("BucketNotEmpty", "bucket not empty")
            .status(StatusCode::CONFLICT)
            .to_rest_xml_response(),
        Some(_) => {
            s.buckets.remove(&bucket);
            Response::builder()
                .status(StatusCode::NO_CONTENT)
                .body(Body::empty())
                .unwrap()
        }
        None => bucket_not_found(),
    }
}

async fn head_bucket(State(state): State<S3State>, Path(bucket): Path<String>) -> Response {
    if state.read().buckets.contains_key(&bucket) {
        Response::builder()
            .status(StatusCode::OK)
            .body(Body::empty())
            .unwrap()
    } else {
        Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::empty())
            .unwrap()
    }
}

/// GET on `/bucket` is ListObjectsV2 (with `list-type=2` query param) or
/// ListObjects v1 (without). We always answer in V2 format which all modern
/// SDKs handle, but include common-prefix support.
async fn get_bucket(
    State(state): State<S3State>,
    Path(bucket): Path<String>,
    Query(params): Query<HashMap<String, String>>,
) -> Response {
    let s = state.read();
    let Some(b) = s.buckets.get(&bucket) else {
        return bucket_not_found();
    };

    let prefix = params.get("prefix").cloned().unwrap_or_default();
    let delimiter = params.get("delimiter").cloned();
    let max_keys: usize = params
        .get("max-keys")
        .and_then(|v| v.parse().ok())
        .unwrap_or(1000);
    let start_after = params.get("start-after").cloned().unwrap_or_default();

    let mut keys: Vec<&String> = b
        .objects
        .keys()
        .filter(|k| k.starts_with(&prefix) && **k > start_after)
        .collect();
    keys.sort();

    let mut common_prefixes: Vec<String> = Vec::new();
    let mut contents = String::new();
    let mut count = 0usize;
    let mut last_key: Option<&String> = None;
    let mut is_truncated = false;

    // AWS spec: KeyCount counts both Contents and CommonPrefixes against
    // MaxKeys (code #9). We share a single `count` across both branches.
    for key in keys {
        if count >= max_keys {
            is_truncated = true;
            break;
        }
        if let Some(delim) = &delimiter
            && let Some(idx) = key[prefix.len()..].find(delim.as_str())
        {
            let cp = format!(
                "{}{}",
                &key[..prefix.len()],
                &key[prefix.len()..prefix.len() + idx + delim.len()]
            );
            if !common_prefixes.contains(&cp) {
                common_prefixes.push(cp);
                count += 1;
            }
            continue;
        }
        let obj = &b.objects[key];
        contents.push_str(&format!(
            "<Contents><Key>{}</Key><LastModified>{}</LastModified><ETag>&quot;{}&quot;</ETag><Size>{}</Size><StorageClass>STANDARD</StorageClass></Contents>",
            xml_escape(key),
            obj.last_modified.format("%Y-%m-%dT%H:%M:%S%.3fZ"),
            obj.etag,
            obj.data.len(),
        ));
        last_key = Some(key);
        count += 1;
    }

    let mut cp_xml = String::new();
    for cp in &common_prefixes {
        cp_xml.push_str(&format!(
            "<CommonPrefixes><Prefix>{}</Prefix></CommonPrefixes>",
            xml_escape(cp)
        ));
    }

    let next_marker = if is_truncated {
        last_key
            .map(|k| {
                format!(
                    "<NextContinuationToken>{}</NextContinuationToken>",
                    xml_escape(k)
                )
            })
            .unwrap_or_default()
    } else {
        String::new()
    };

    let xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<ListBucketResult xmlns="http://s3.amazonaws.com/doc/2006-03-01/">
<Name>{name}</Name>
<Prefix>{prefix}</Prefix>
<KeyCount>{count}</KeyCount>
<MaxKeys>{max}</MaxKeys>
<IsTruncated>{truncated}</IsTruncated>
{next}
{contents}
{cp}
</ListBucketResult>"#,
        name = xml_escape(&bucket),
        prefix = xml_escape(&prefix),
        max = max_keys,
        truncated = is_truncated,
        next = next_marker,
        contents = contents,
        cp = cp_xml,
    );
    xml_response(StatusCode::OK, xml)
}

// === Object operations ===

async fn put_object(
    State(state): State<S3State>,
    Path((bucket, key)): Path<(String, String)>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let mut s = state.write();
    let Some(b) = s.buckets.get_mut(&bucket) else {
        return bucket_not_found();
    };

    let etag = etag_for(&body);
    let content_type = headers
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("binary/octet-stream")
        .to_string();
    let metadata = extract_metadata(&headers);

    b.objects.insert(
        key,
        Object {
            data: body,
            etag: etag.clone(),
            content_type,
            last_modified: Utc::now(),
            metadata,
        },
    );

    Response::builder()
        .status(StatusCode::OK)
        .header("ETag", format!("\"{etag}\""))
        .body(Body::empty())
        .unwrap()
}

async fn get_object(
    State(state): State<S3State>,
    Path((bucket, key)): Path<(String, String)>,
) -> Response {
    let obj = {
        let s = state.read();
        let Some(b) = s.buckets.get(&bucket) else {
            return bucket_not_found();
        };
        let Some(obj) = b.objects.get(&key) else {
            return key_not_found();
        };
        // Clone out of the lock scope so we never copy a large body while
        // blocking concurrent writers (code #11).
        obj.clone()
    };
    object_response_builder(&obj)
        .body(Body::from(obj.data.clone()))
        .unwrap()
}

async fn head_object_path(
    State(state): State<S3State>,
    Path((bucket, key)): Path<(String, String)>,
) -> Response {
    let obj = {
        let s = state.read();
        let Some(b) = s.buckets.get(&bucket) else {
            return bucket_not_found();
        };
        let Some(obj) = b.objects.get(&key) else {
            return key_not_found();
        };
        obj.clone()
    };
    object_response_builder(&obj).body(Body::empty()).unwrap()
}

/// Shared `Response::builder()` shape for GET/HEAD object — keeps the two
/// handlers from drifting when a new header is added.
fn object_response_builder(obj: &Object) -> axum::http::response::Builder {
    let mut builder = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, &obj.content_type)
        .header(header::CONTENT_LENGTH, obj.data.len())
        .header(header::LAST_MODIFIED, http_date(obj.last_modified))
        .header("ETag", format!("\"{}\"", obj.etag));
    for (k, v) in &obj.metadata {
        builder = builder.header(format!("x-amz-meta-{k}"), v);
    }
    builder
}

async fn delete_object(
    State(state): State<S3State>,
    Path((bucket, key)): Path<(String, String)>,
) -> Response {
    let mut s = state.write();
    let Some(b) = s.buckets.get_mut(&bucket) else {
        return bucket_not_found();
    };
    b.objects.remove(&key);
    Response::builder()
        .status(StatusCode::NO_CONTENT)
        .body(Body::empty())
        .unwrap()
}

// === Helpers ===

fn http_date(d: DateTime<Utc>) -> String {
    // RFC 7231 IMF-fixdate (the only format AWS SDKs accept on Last-Modified).
    d.format("%a, %d %b %Y %H:%M:%S GMT").to_string()
}

fn etag_for(body: &Bytes) -> String {
    let mut hasher = Md5::new();
    hasher.update(body);
    hex::encode(hasher.finalize())
}

/// Pull `x-amz-meta-*` headers from the request, dropping any whose suffix or
/// value contains characters that would let an attacker inject extra headers
/// when we echo them back on GET/HEAD (CRLF injection — code #3).
fn extract_metadata(headers: &HeaderMap) -> HashMap<String, String> {
    fn safe_suffix(s: &str) -> bool {
        !s.is_empty()
            && s.bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_' || b == b'.')
    }
    fn safe_value(s: &str) -> bool {
        s.bytes().all(|b| b != b'\r' && b != b'\n' && b != 0)
    }
    headers
        .iter()
        .filter_map(|(k, v)| {
            let name = k.as_str().to_ascii_lowercase();
            let suffix = name.strip_prefix("x-amz-meta-")?;
            let value = v.to_str().ok()?;
            if !safe_suffix(suffix) || !safe_value(value) {
                tracing::debug!(key = suffix, "dropping unsafe x-amz-meta-* header");
                return None;
            }
            Some((suffix.to_string(), value.to_string()))
        })
        .collect()
}

fn bucket_not_found() -> Response {
    AwsError::new("NoSuchBucket", "The specified bucket does not exist")
        .status(StatusCode::NOT_FOUND)
        .to_rest_xml_response()
}

fn key_not_found() -> Response {
    AwsError::new("NoSuchKey", "The specified key does not exist")
        .status(StatusCode::NOT_FOUND)
        .to_rest_xml_response()
}

fn xml_response(status: StatusCode, body: String) -> Response {
    Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, "application/xml")
        .body(Body::from(body))
        .unwrap()
}
