//! DynamoDB — AWS JSON 1.0 protocol via `X-Amz-Target: DynamoDB_20120810.<Action>`.
//!
//! Items are stored as `serde_json::Map` (preserving the AWS-typed-value
//! envelope: `{"S":"..."}`, `{"N":"123"}`, ...) keyed by a composite of
//! partition key (and sort key if defined). This avoids re-implementing the
//! AWS DynamoDB type system while staying interoperable with the AWS SDK.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use axum::Router;
use bytes::Bytes;
use parking_lot::RwLock;
use serde_json::{Map, Value, json};

use crate::aws_error::AwsError;
use crate::service::{JsonProtocolService, Service, ServiceContext};

const TARGET_PREFIX: &str = "DynamoDB_20120810";

#[derive(Debug, Default)]
struct State {
    tables: HashMap<String, Table>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct Table {
    name: String,
    hash_key: String,
    range_key: Option<String>,
    items: HashMap<String, Map<String, Value>>,
    created: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
struct PersistedState {
    tables: HashMap<String, Table>,
}

pub struct DynamoDb {
    state: Arc<RwLock<State>>,
}

impl DynamoDb {
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(State::default())),
        }
    }
}

impl Default for DynamoDb {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Service for DynamoDb {
    fn name(&self) -> &'static str {
        "dynamodb"
    }

    fn reset(&self) {
        self.state.write().tables.clear();
    }

    fn restore(&self, ctx: &ServiceContext) -> Result<(), AwsError> {
        if let Some(snap) = &ctx.snapshot
            && let Some(data) = snap
                .load::<PersistedState>("dynamodb")
                .map_err(crate::service::persistence_error)?
        {
            self.state.write().tables = data.tables;
        }
        Ok(())
    }

    fn snapshot(&self, ctx: &ServiceContext) -> Result<(), AwsError> {
        if let Some(snap) = &ctx.snapshot {
            let data = PersistedState {
                tables: self.state.read().tables.clone(),
            };
            snap.save("dynamodb", &data)
                .map_err(crate::service::persistence_error)?;
        }
        Ok(())
    }

    fn router(&self, _ctx: ServiceContext) -> Router {
        Router::new()
    }
}

#[async_trait]
impl JsonProtocolService for DynamoDb {
    fn target_prefix(&self) -> &'static str {
        TARGET_PREFIX
    }

    async fn dispatch(
        &self,
        _ctx: ServiceContext,
        action: &str,
        body: Bytes,
    ) -> Result<Value, AwsError> {
        let req: Value = if body.is_empty() {
            json!({})
        } else {
            serde_json::from_slice(&body)
                .map_err(|e| AwsError::new("InvalidRequest", e.to_string()))?
        };

        match action {
            "CreateTable" => self.create_table(&req),
            "DeleteTable" => self.delete_table(&req),
            "DescribeTable" => self.describe_table(&req),
            "ListTables" => self.list_tables(&req),
            "PutItem" => self.put_item(&req),
            "GetItem" => self.get_item(&req),
            "DeleteItem" => self.delete_item(&req),
            "UpdateItem" => self.update_item(&req),
            "Scan" => self.scan(&req),
            "Query" => self.query(&req),
            "BatchGetItem" => self.batch_get_item(&req),
            "BatchWriteItem" => self.batch_write_item(&req),
            other => Err(AwsError::unsupported(format!("DynamoDB::{other}"))),
        }
    }
}

impl DynamoDb {
    fn create_table(&self, req: &Value) -> Result<Value, AwsError> {
        let name = req
            .get("TableName")
            .and_then(Value::as_str)
            .ok_or_else(|| AwsError::new("ValidationException", "TableName required"))?
            .to_string();
        let key_schema = req
            .get("KeySchema")
            .and_then(Value::as_array)
            .ok_or_else(|| AwsError::new("ValidationException", "KeySchema required"))?;

        let mut hash_key = None;
        let mut range_key = None;
        for k in key_schema {
            let name = k
                .get("AttributeName")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            let kind = k.get("KeyType").and_then(Value::as_str).unwrap_or("");
            match kind {
                "HASH" => hash_key = Some(name),
                "RANGE" => range_key = Some(name),
                _ => {}
            }
        }
        let hash_key = hash_key
            .ok_or_else(|| AwsError::new("ValidationException", "missing HASH key in KeySchema"))?;

        let mut s = self.state.write();
        if s.tables.contains_key(&name) {
            return Err(AwsError::new(
                "ResourceInUseException",
                format!("table {name} already exists"),
            ));
        }
        let table = Table {
            name: name.clone(),
            hash_key,
            range_key,
            items: HashMap::new(),
            created: chrono::Utc::now(),
        };
        let desc = describe_table_json(&table, 0);
        s.tables.insert(name, table);
        Ok(json!({ "TableDescription": desc }))
    }

    fn delete_table(&self, req: &Value) -> Result<Value, AwsError> {
        let name = name_of(req)?;
        let mut s = self.state.write();
        match s.tables.remove(&name) {
            Some(t) => Ok(json!({ "TableDescription": describe_table_json(&t, t.items.len()) })),
            None => Err(not_found(&name)),
        }
    }

    fn describe_table(&self, req: &Value) -> Result<Value, AwsError> {
        let name = name_of(req)?;
        let s = self.state.read();
        match s.tables.get(&name) {
            Some(t) => Ok(json!({ "Table": describe_table_json(t, t.items.len()) })),
            None => Err(not_found(&name)),
        }
    }

    fn list_tables(&self, _req: &Value) -> Result<Value, AwsError> {
        let s = self.state.read();
        let mut names: Vec<_> = s.tables.keys().cloned().collect();
        names.sort();
        Ok(json!({ "TableNames": names }))
    }

    fn put_item(&self, req: &Value) -> Result<Value, AwsError> {
        let name = name_of(req)?;
        let item = req
            .get("Item")
            .and_then(Value::as_object)
            .ok_or_else(|| AwsError::new("ValidationException", "Item required"))?
            .clone();
        let mut s = self.state.write();
        let t = s.tables.get_mut(&name).ok_or_else(|| not_found(&name))?;
        let key = composite_key(t, &item)?;
        t.items.insert(key, item);
        Ok(json!({}))
    }

    fn get_item(&self, req: &Value) -> Result<Value, AwsError> {
        let name = name_of(req)?;
        let key_map = req
            .get("Key")
            .and_then(Value::as_object)
            .ok_or_else(|| AwsError::new("ValidationException", "Key required"))?
            .clone();
        let s = self.state.read();
        let t = s.tables.get(&name).ok_or_else(|| not_found(&name))?;
        let key = composite_key(t, &key_map)?;
        match t.items.get(&key) {
            Some(item) => Ok(json!({ "Item": item })),
            None => Ok(json!({})),
        }
    }

    fn delete_item(&self, req: &Value) -> Result<Value, AwsError> {
        let name = name_of(req)?;
        let key_map = req
            .get("Key")
            .and_then(Value::as_object)
            .ok_or_else(|| AwsError::new("ValidationException", "Key required"))?
            .clone();
        let mut s = self.state.write();
        let t = s.tables.get_mut(&name).ok_or_else(|| not_found(&name))?;
        let key = composite_key(t, &key_map)?;
        t.items.remove(&key);
        Ok(json!({}))
    }

    fn update_item(&self, req: &Value) -> Result<Value, AwsError> {
        let name = name_of(req)?;
        let key_map = req
            .get("Key")
            .and_then(Value::as_object)
            .ok_or_else(|| AwsError::new("ValidationException", "Key required"))?
            .clone();
        let mut s = self.state.write();
        let t = s.tables.get_mut(&name).ok_or_else(|| not_found(&name))?;
        let key = composite_key(t, &key_map)?;
        let entry = t.items.entry(key).or_insert_with(|| key_map.clone());

        if let Some(values) = req.get("AttributeUpdates").and_then(Value::as_object) {
            for (attr, op) in values {
                let action = op.get("Action").and_then(Value::as_str).unwrap_or("PUT");
                match action {
                    "PUT" => {
                        if let Some(v) = op.get("Value") {
                            entry.insert(attr.clone(), v.clone());
                        }
                    }
                    "DELETE" => {
                        entry.remove(attr);
                    }
                    _ => {}
                }
            }
        }

        Ok(json!({ "Attributes": entry }))
    }

    fn scan(&self, req: &Value) -> Result<Value, AwsError> {
        let name = name_of(req)?;
        let limit = req.get("Limit").and_then(Value::as_u64).unwrap_or(u64::MAX);
        let s = self.state.read();
        let t = s.tables.get(&name).ok_or_else(|| not_found(&name))?;
        let items: Vec<_> = t.items.values().take(limit as usize).cloned().collect();
        let count = items.len();
        Ok(json!({
            "Items": items,
            "Count": count,
            "ScannedCount": count,
        }))
    }

    /// AWS DynamoDB Query semantics, abridged:
    ///
    /// - `KeyConditionExpression` must reference the hash key with `=`, and
    ///   optionally the sort key with `=`, `<`, `<=`, `>`, `>=`, `BETWEEN`, or
    ///   `begins_with()`.
    /// - `ExpressionAttributeValues` carries `:placeholder` → typed value.
    /// - `ExpressionAttributeNames` substitutes `#alias` → real attribute name.
    /// - `ScanIndexForward=false` reverses sort order.
    /// - `Limit` truncates results.
    ///
    /// We implement the operator subset commonly used by SDK callers and fall
    /// back to "scan and post-filter" so unrecognized expressions don't 500.
    fn query(&self, req: &Value) -> Result<Value, AwsError> {
        let name = name_of(req)?;
        let s = self.state.read();
        let t = s.tables.get(&name).ok_or_else(|| not_found(&name))?;

        let names = req
            .get("ExpressionAttributeNames")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        let values = req
            .get("ExpressionAttributeValues")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();

        let key_cond = req
            .get("KeyConditionExpression")
            .and_then(Value::as_str)
            .unwrap_or("");
        let conditions = parse_key_condition(key_cond, &names);

        let mut items: Vec<Map<String, Value>> = t
            .items
            .values()
            .filter(|item| conditions.iter().all(|c| c.matches(item, &values)))
            .cloned()
            .collect();

        // Sort by sort key when present, then optionally reverse.
        if let Some(rk) = &t.range_key {
            items.sort_by(|a, b| sort_key_cmp(a.get(rk), b.get(rk)));
        }
        if req.get("ScanIndexForward").and_then(Value::as_bool) == Some(false) {
            items.reverse();
        }

        let scanned = items.len();
        if let Some(limit) = req.get("Limit").and_then(Value::as_u64) {
            items.truncate(limit as usize);
        }
        let count = items.len();
        Ok(json!({
            "Items": items,
            "Count": count,
            "ScannedCount": scanned,
        }))
    }

    fn batch_get_item(&self, req: &Value) -> Result<Value, AwsError> {
        let request_items = req
            .get("RequestItems")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        let mut responses = Map::new();
        let s = self.state.read();
        for (table_name, spec) in request_items {
            let Some(t) = s.tables.get(&table_name) else {
                continue;
            };
            let mut out = Vec::new();
            if let Some(keys) = spec.get("Keys").and_then(Value::as_array) {
                for k in keys {
                    if let Some(map) = k.as_object() {
                        let key = composite_key(t, map)?;
                        if let Some(item) = t.items.get(&key) {
                            out.push(Value::Object(item.clone()));
                        }
                    }
                }
            }
            responses.insert(table_name, Value::Array(out));
        }
        Ok(json!({ "Responses": responses, "UnprocessedKeys": {} }))
    }

    fn batch_write_item(&self, req: &Value) -> Result<Value, AwsError> {
        let request_items = req
            .get("RequestItems")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        let mut s = self.state.write();
        for (table_name, ops) in request_items {
            let Some(t) = s.tables.get_mut(&table_name) else {
                continue;
            };
            if let Some(arr) = ops.as_array() {
                for op in arr {
                    if let Some(put) = op
                        .get("PutRequest")
                        .and_then(|p| p.get("Item"))
                        .and_then(Value::as_object)
                    {
                        let key = composite_key(t, put)?;
                        t.items.insert(key, put.clone());
                    } else if let Some(del) = op
                        .get("DeleteRequest")
                        .and_then(|d| d.get("Key"))
                        .and_then(Value::as_object)
                    {
                        let key = composite_key(t, del)?;
                        t.items.remove(&key);
                    }
                }
            }
        }
        Ok(json!({ "UnprocessedItems": {} }))
    }
}

/// One condition inside a KeyConditionExpression, e.g. `pk = :v` or
/// `begins_with(sk, :prefix)`.
#[derive(Debug, Clone)]
struct KeyCondition {
    attr: String,
    op: KeyOp,
    value_ref: String,
    /// For `BETWEEN`, the second value placeholder.
    value_ref2: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum KeyOp {
    Eq,
    Lt,
    Le,
    Gt,
    Ge,
    BeginsWith,
    Between,
}

impl KeyCondition {
    fn matches(&self, item: &Map<String, Value>, values: &Map<String, Value>) -> bool {
        let Some(item_v) = item.get(&self.attr) else {
            return false;
        };
        let Some(want) = values.get(&self.value_ref) else {
            return false;
        };
        match self.op {
            KeyOp::Eq => item_v == want,
            KeyOp::Lt => compare_attr(item_v, want).is_some_and(|o| o.is_lt()),
            KeyOp::Le => compare_attr(item_v, want).is_some_and(|o| o.is_le()),
            KeyOp::Gt => compare_attr(item_v, want).is_some_and(|o| o.is_gt()),
            KeyOp::Ge => compare_attr(item_v, want).is_some_and(|o| o.is_ge()),
            KeyOp::BeginsWith => begins_with(item_v, want),
            KeyOp::Between => {
                let Some(low) = values.get(&self.value_ref) else {
                    return false;
                };
                let Some(high) = self.value_ref2.as_ref().and_then(|r| values.get(r)) else {
                    return false;
                };
                compare_attr(item_v, low).is_some_and(|o| o.is_ge())
                    && compare_attr(item_v, high).is_some_and(|o| o.is_le())
            }
        }
    }
}

/// Parse `pk = :v AND sk OP :w` style expressions. Robust enough for the
/// SDK-emitted shapes; unknown syntax produces zero conditions, which the
/// query falls back to scanning.
///
/// `ExpressionAttributeNames` keys are the alias tokens *with* the leading
/// `#` (per the AWS docs and SDK emission). We look them up using the alias
/// string as-is — earlier code stripped the `#` and then re-added it, which
/// produced a `##alias` key that never matched.
fn parse_key_condition(expr: &str, names: &Map<String, Value>) -> Vec<KeyCondition> {
    if expr.trim().is_empty() {
        return Vec::new();
    }
    let resolve = |s: &str| -> String {
        if s.starts_with('#')
            && let Some(real) = names.get(s).and_then(Value::as_str)
        {
            return real.to_string();
        }
        s.to_string()
    };

    // BETWEEN spans " AND " between its two operands, which trips the outer
    // term-split. Recombine `<attr> BETWEEN :a` and `:b` back into a single
    // condition before doing per-operator parsing.
    let terms = recombine_between_terms(expr);

    let mut out = Vec::new();
    for term in terms {
        let term = term.trim();
        if let Some(rest) = term
            .strip_prefix("begins_with(")
            .and_then(|s| s.strip_suffix(')'))
        {
            let mut parts = rest.splitn(2, ',').map(str::trim);
            if let (Some(a), Some(v)) = (parts.next(), parts.next()) {
                out.push(KeyCondition {
                    attr: resolve(a),
                    op: KeyOp::BeginsWith,
                    value_ref: v.to_string(),
                    value_ref2: None,
                });
            }
            continue;
        }
        if let Some((attr, rest)) = term.split_once(" BETWEEN ")
            && let Some((low, high)) = rest.split_once(" AND ")
        {
            out.push(KeyCondition {
                attr: resolve(attr.trim()),
                op: KeyOp::Between,
                value_ref: low.trim().to_string(),
                value_ref2: Some(high.trim().to_string()),
            });
            continue;
        }
        let (op_str, op) = if term.contains(" <= ") {
            (" <= ", KeyOp::Le)
        } else if term.contains(" >= ") {
            (" >= ", KeyOp::Ge)
        } else if term.contains(" < ") {
            (" < ", KeyOp::Lt)
        } else if term.contains(" > ") {
            (" > ", KeyOp::Gt)
        } else if term.contains(" = ") {
            (" = ", KeyOp::Eq)
        } else {
            continue;
        };
        if let Some((a, v)) = term.split_once(op_str) {
            out.push(KeyCondition {
                attr: resolve(a.trim()),
                op,
                value_ref: v.trim().to_string(),
                value_ref2: None,
            });
        }
    }
    out
}

/// Split on top-level " AND " while keeping `BETWEEN :a AND :b` intact.
fn recombine_between_terms(expr: &str) -> Vec<String> {
    let raw: Vec<&str> = expr.split(" AND ").collect();
    let mut out: Vec<String> = Vec::with_capacity(raw.len());
    let mut i = 0;
    while i < raw.len() {
        let cur = raw[i];
        if cur.contains(" BETWEEN ") && i + 1 < raw.len() {
            out.push(format!("{cur} AND {}", raw[i + 1]));
            i += 2;
        } else {
            out.push(cur.to_string());
            i += 1;
        }
    }
    out
}

#[cfg(test)]
mod parse_tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_eq() {
        let conds = parse_key_condition("pk = :p", &Map::new());
        assert_eq!(conds.len(), 1);
        assert_eq!(conds[0].attr, "pk");
        assert_eq!(conds[0].op, KeyOp::Eq);
    }

    #[test]
    fn parses_eq_and_begins_with() {
        let conds = parse_key_condition("pk = :p AND begins_with(sk, :prefix)", &Map::new());
        assert_eq!(conds.len(), 2);
        assert_eq!(conds[1].op, KeyOp::BeginsWith);
    }

    #[test]
    fn parses_between_with_high_bound() {
        let conds = parse_key_condition("pk = :p AND sk BETWEEN :lo AND :hi", &Map::new());
        assert_eq!(conds.len(), 2);
        assert_eq!(conds[1].op, KeyOp::Between);
        assert_eq!(conds[1].value_ref, ":lo");
        assert_eq!(conds[1].value_ref2.as_deref(), Some(":hi"));
    }

    #[test]
    fn resolves_attribute_name_alias() {
        let mut names = Map::new();
        names.insert("#k".into(), json!("partition_key"));
        let conds = parse_key_condition("#k = :p", &names);
        assert_eq!(conds[0].attr, "partition_key");
    }
}

/// DynamoDB attribute comparison. Returns None if the two values are different
/// AWS scalar types (which means "not comparable" under DynamoDB semantics).
///
/// `N` values are compared as signed decimal numbers without converting to
/// `f64` — DynamoDB allows up to 38 digits of precision, which `f64`
/// (15–17 significant digits) silently loses.
fn compare_attr(a: &Value, b: &Value) -> Option<std::cmp::Ordering> {
    let (a_obj, b_obj) = (a.as_object()?, b.as_object()?);
    let (a_kind, a_val) = a_obj.iter().next()?;
    let (b_kind, b_val) = b_obj.iter().next()?;
    if a_kind != b_kind {
        return None;
    }
    match a_kind.as_str() {
        "N" => compare_decimal(a_val.as_str()?, b_val.as_str()?),
        "S" => Some(a_val.as_str()?.cmp(b_val.as_str()?)),
        _ => None,
    }
}

/// Compare two strings as decimal numbers (sign + integer + optional fraction)
/// without floating-point conversion. Preserves full DynamoDB N-type precision.
fn compare_decimal(a: &str, b: &str) -> Option<std::cmp::Ordering> {
    use std::cmp::Ordering;
    let (a_neg, a_body) = split_sign(a.trim())?;
    let (b_neg, b_body) = split_sign(b.trim())?;
    if a_neg != b_neg {
        return Some(if a_neg {
            Ordering::Less
        } else {
            Ordering::Greater
        });
    }
    let mag = compare_magnitude(a_body, b_body)?;
    Some(if a_neg { mag.reverse() } else { mag })
}

fn split_sign(s: &str) -> Option<(bool, &str)> {
    if let Some(rest) = s.strip_prefix('-') {
        Some((true, rest))
    } else {
        Some((false, s.strip_prefix('+').unwrap_or(s)))
    }
}

fn compare_magnitude(a: &str, b: &str) -> Option<std::cmp::Ordering> {
    use std::cmp::Ordering;
    let (a_int, a_frac) = a.split_once('.').unwrap_or((a, ""));
    let (b_int, b_frac) = b.split_once('.').unwrap_or((b, ""));
    if !a_int.bytes().all(|c| c.is_ascii_digit())
        || !b_int.bytes().all(|c| c.is_ascii_digit())
        || !a_frac.bytes().all(|c| c.is_ascii_digit())
        || !b_frac.bytes().all(|c| c.is_ascii_digit())
    {
        return None;
    }
    let a_int = a_int.trim_start_matches('0');
    let b_int = b_int.trim_start_matches('0');
    let int_cmp = a_int.len().cmp(&b_int.len()).then_with(|| a_int.cmp(b_int));
    if int_cmp != Ordering::Equal {
        return Some(int_cmp);
    }
    // Equal integer part — compare fractions left-aligned, padding the shorter
    // with zeros so "1.10" and "1.1" compare equal.
    let max = a_frac.len().max(b_frac.len());
    let a_pad: String = a_frac
        .chars()
        .chain(std::iter::repeat('0'))
        .take(max)
        .collect();
    let b_pad: String = b_frac
        .chars()
        .chain(std::iter::repeat('0'))
        .take(max)
        .collect();
    Some(a_pad.cmp(&b_pad))
}

#[cfg(test)]
mod decimal_tests {
    use super::*;
    use std::cmp::Ordering;

    #[test]
    fn preserves_precision_beyond_f64() {
        // 19-digit integers — exceeds f64 precision; naive parse loses bits.
        let a = compare_decimal("12345678901234567890", "12345678901234567891");
        assert_eq!(a, Some(Ordering::Less));
    }

    #[test]
    fn handles_negative_numbers() {
        assert_eq!(compare_decimal("-5", "-3"), Some(Ordering::Less));
        assert_eq!(compare_decimal("-1", "1"), Some(Ordering::Less));
    }

    #[test]
    fn handles_fraction_padding() {
        assert_eq!(compare_decimal("1.10", "1.1"), Some(Ordering::Equal));
        assert_eq!(compare_decimal("1.2", "1.10"), Some(Ordering::Greater));
    }

    #[test]
    fn rejects_garbage() {
        assert_eq!(compare_decimal("nope", "1"), None);
    }
}

fn begins_with(item: &Value, want: &Value) -> bool {
    let (Some(i), Some(w)) = (
        item.get("S").and_then(Value::as_str),
        want.get("S").and_then(Value::as_str),
    ) else {
        return false;
    };
    i.starts_with(w)
}

fn sort_key_cmp(a: Option<&Value>, b: Option<&Value>) -> std::cmp::Ordering {
    match (a, b) {
        (Some(a), Some(b)) => compare_attr(a, b).unwrap_or(std::cmp::Ordering::Equal),
        (Some(_), None) => std::cmp::Ordering::Greater,
        (None, Some(_)) => std::cmp::Ordering::Less,
        (None, None) => std::cmp::Ordering::Equal,
    }
}

fn name_of(req: &Value) -> Result<String, AwsError> {
    req.get("TableName")
        .and_then(Value::as_str)
        .map(String::from)
        .ok_or_else(|| AwsError::new("ValidationException", "TableName required"))
}

fn not_found(name: &str) -> AwsError {
    AwsError::new(
        "ResourceNotFoundException",
        format!("table {name} does not exist"),
    )
}

fn composite_key(t: &Table, item: &Map<String, Value>) -> Result<String, AwsError> {
    let h = item.get(&t.hash_key).ok_or_else(|| {
        AwsError::new(
            "ValidationException",
            format!("missing hash key {}", t.hash_key),
        )
    })?;
    let hs = canonical_value(h);
    if let Some(rk) = &t.range_key {
        let r = item.get(rk).ok_or_else(|| {
            AwsError::new("ValidationException", format!("missing range key {rk}"))
        })?;
        Ok(format!("{hs}|{}", canonical_value(r)))
    } else {
        Ok(hs)
    }
}

fn canonical_value(v: &Value) -> String {
    // The wire format uses {"S": "x"} / {"N": "1"} / {"B": "..."}. Pick the
    // first scalar field for a stable string key.
    if let Some(obj) = v.as_object() {
        for (k, val) in obj {
            if let Some(s) = val.as_str() {
                return format!("{k}:{s}");
            }
        }
    }
    v.to_string()
}

fn describe_table_json(t: &Table, item_count: usize) -> Value {
    let mut key_schema = vec![json!({"AttributeName": t.hash_key, "KeyType": "HASH"})];
    if let Some(r) = &t.range_key {
        key_schema.push(json!({"AttributeName": r, "KeyType": "RANGE"}));
    }
    json!({
        "TableName": t.name,
        "TableStatus": "ACTIVE",
        "KeySchema": key_schema,
        "ItemCount": item_count,
        "CreationDateTime": t.created.timestamp(),
        "TableArn": format!(
            "arn:aws:dynamodb:{}:{}:table/{}",
            crate::service::EMULATED_REGION,
            crate::service::EMULATED_ACCOUNT_ID,
            t.name
        ),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn create_put_get_roundtrip() {
        let svc = DynamoDb::new();
        let ctx = ServiceContext::new(None);

        let create = r#"{
            "TableName":"t1",
            "KeySchema":[{"AttributeName":"pk","KeyType":"HASH"}],
            "AttributeDefinitions":[{"AttributeName":"pk","AttributeType":"S"}]
        }"#;
        svc.dispatch(ctx.clone(), "CreateTable", Bytes::from(create))
            .await
            .unwrap();

        let put = r#"{"TableName":"t1","Item":{"pk":{"S":"a"},"v":{"S":"hello"}}}"#;
        svc.dispatch(ctx.clone(), "PutItem", Bytes::from(put))
            .await
            .unwrap();

        let get = r#"{"TableName":"t1","Key":{"pk":{"S":"a"}}}"#;
        let resp = svc
            .dispatch(ctx, "GetItem", Bytes::from(get))
            .await
            .unwrap();
        assert_eq!(resp["Item"]["v"]["S"], "hello");
    }
}
