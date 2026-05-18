//! CloudWatch (metrics) — supports **both** AWS Query (`monitoring` sdk_id) and
//! Smithy RPC v2 CBOR (`GraniteServiceVersion20100801`). Modern AWS SDKs sent
//! after the CloudWatch protocol migration speak CBOR; older callers still
//! use Query. We expose both so kuroko works with any client version.
//!
//! In-memory metric store: each (namespace, metric_name, dimensions) key
//! accumulates a vector of (timestamp, value) data points. Statistics are
//! computed on demand from the stored points. Alarms are stored verbatim
//! but never evaluated.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use axum::Router;
use bytes::Bytes;
use parking_lot::RwLock;
use serde_json::{Value as JsonValue, json};
use uuid::Uuid;

use crate::aws_error::{AwsError, xml_escape};
use crate::registry::Registry;
use crate::service::{
    CborProtocolService, EMULATED_ACCOUNT_ID, EMULATED_REGION, QueryProtocolService, Service,
    ServiceContext, persistence_error,
};

const SDK_ID: &str = "monitoring";
const NS: &str = "http://monitoring.amazonaws.com/doc/2010-08-01/";
const CBOR_SERVICE: &str = "GraniteServiceVersion20100801";

const ACTIONS: &[&str] = &[
    "PutMetricData",
    "GetMetricStatistics",
    "GetMetricData",
    "ListMetrics",
    "PutMetricAlarm",
    "DescribeAlarms",
    "DeleteAlarms",
];

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
struct State {
    metrics: HashMap<String, MetricSeries>,
    alarms: HashMap<String, Alarm>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct MetricSeries {
    namespace: String,
    metric_name: String,
    dimensions: Vec<(String, String)>,
    unit: String,
    points: Vec<DataPoint>,
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
struct DataPoint {
    timestamp_ms: i64,
    value: f64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct Alarm {
    name: String,
    namespace: Option<String>,
    metric_name: Option<String>,
    statistic: Option<String>,
    threshold: Option<f64>,
    comparison_operator: Option<String>,
    period: Option<i64>,
    evaluation_periods: Option<i64>,
    state: String,
    arn: String,
    created: chrono::DateTime<chrono::Utc>,
}

pub struct CloudWatch {
    state: Arc<RwLock<State>>,
}

impl CloudWatch {
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(State::default())),
        }
    }
}

impl Default for CloudWatch {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Service for CloudWatch {
    fn name(&self) -> &'static str {
        "cloudwatch"
    }

    fn reset(&self) {
        *self.state.write() = State::default();
    }

    fn restore(&self, ctx: &ServiceContext) -> Result<(), AwsError> {
        if let Some(snap) = &ctx.snapshot
            && let Some(data) = snap
                .load::<State>("cloudwatch")
                .map_err(persistence_error)?
        {
            *self.state.write() = data;
        }
        Ok(())
    }

    fn snapshot(&self, ctx: &ServiceContext) -> Result<(), AwsError> {
        if let Some(snap) = &ctx.snapshot {
            let data = self.state.read();
            snap.save("cloudwatch", &*data).map_err(persistence_error)?;
        }
        Ok(())
    }

    fn router(&self, _ctx: ServiceContext) -> Router {
        Router::new()
    }
}

#[async_trait]
impl QueryProtocolService for CloudWatch {
    fn sdk_id(&self) -> &'static str {
        SDK_ID
    }

    fn actions(&self) -> &'static [&'static str] {
        ACTIONS
    }

    async fn dispatch(
        &self,
        _ctx: ServiceContext,
        action: &str,
        params: &HashMap<String, String>,
    ) -> Result<String, AwsError> {
        match action {
            "PutMetricData" => self.put_metric_data(params),
            "GetMetricStatistics" => self.get_metric_statistics(params),
            "ListMetrics" => self.list_metrics(params),
            "PutMetricAlarm" => self.put_metric_alarm(params),
            "DescribeAlarms" => self.describe_alarms(params),
            "DeleteAlarms" => self.delete_alarms(params),
            other => Err(AwsError::unsupported(format!("CloudWatch::{other}"))),
        }
    }
}

/// Input shape that's protocol-independent: pre-parsed by the protocol layer
/// and consumed by the business logic. Query / CBOR populate the same struct.
#[derive(Debug, Default)]
struct PutMetricDataInput {
    namespace: String,
    data: Vec<MetricDatumInput>,
}

#[derive(Debug, Default)]
struct MetricDatumInput {
    metric_name: String,
    value: f64,
    unit: String,
    timestamp_ms: i64,
    dimensions: Vec<(String, String)>,
}

#[derive(Debug, Default)]
struct GetMetricStatisticsInput {
    namespace: String,
    metric_name: String,
    dimensions: Vec<(String, String)>,
    start_ms: i64,
    end_ms: i64,
}

#[derive(Debug, Default)]
struct ListMetricsInput {
    namespace_filter: Option<String>,
    metric_filter: Option<String>,
}

#[derive(Debug)]
struct DatapointResult {
    ts_rfc3339: String,
    ts_secs: f64,
    sum: f64,
    average: f64,
    minimum: f64,
    maximum: f64,
    sample_count: f64,
}

#[derive(Debug, Default)]
struct PutMetricAlarmInput {
    name: String,
    namespace: Option<String>,
    metric_name: Option<String>,
    statistic: Option<String>,
    threshold: Option<f64>,
    comparison_operator: Option<String>,
    period: Option<i64>,
    evaluation_periods: Option<i64>,
}

impl CloudWatch {
    fn put_metric_data(&self, params: &HashMap<String, String>) -> Result<String, AwsError> {
        let namespace = required(params, "Namespace")?;
        let mut data = Vec::new();
        let mut i = 1;
        loop {
            let metric_name_key = format!("MetricData.member.{i}.MetricName");
            let Some(metric_name) = params.get(&metric_name_key).cloned() else {
                break;
            };
            let value: f64 = params
                .get(&format!("MetricData.member.{i}.Value"))
                .and_then(|v| v.parse().ok())
                .unwrap_or(0.0);
            let unit = params
                .get(&format!("MetricData.member.{i}.Unit"))
                .cloned()
                .unwrap_or_else(|| "None".into());
            let timestamp_ms = params
                .get(&format!("MetricData.member.{i}.Timestamp"))
                .and_then(|t| parse_timestamp(t))
                .unwrap_or_else(|| chrono::Utc::now().timestamp_millis());
            let dimensions =
                parse_dimensions_query(params, &format!("MetricData.member.{i}.Dimensions"));
            data.push(MetricDatumInput {
                metric_name,
                value,
                unit,
                timestamp_ms,
                dimensions,
            });
            i += 1;
        }
        self.do_put_metric_data(PutMetricDataInput { namespace, data });
        Ok(empty("PutMetricData"))
    }

    fn do_put_metric_data(&self, input: PutMetricDataInput) {
        let mut s = self.state.write();
        for d in input.data {
            let key = metric_key(&input.namespace, &d.metric_name, &d.dimensions);
            let entry = s.metrics.entry(key).or_insert_with(|| MetricSeries {
                namespace: input.namespace.clone(),
                metric_name: d.metric_name.clone(),
                dimensions: d.dimensions.clone(),
                unit: d.unit.clone(),
                points: Vec::new(),
            });
            entry.points.push(DataPoint {
                timestamp_ms: d.timestamp_ms,
                value: d.value,
            });
        }
    }

    fn get_metric_statistics(&self, params: &HashMap<String, String>) -> Result<String, AwsError> {
        let input = GetMetricStatisticsInput {
            namespace: required(params, "Namespace")?,
            metric_name: required(params, "MetricName")?,
            dimensions: parse_dimensions_query(params, "Dimensions"),
            start_ms: params
                .get("StartTime")
                .and_then(|t| parse_timestamp(t))
                .unwrap_or(i64::MIN),
            end_ms: params
                .get("EndTime")
                .and_then(|t| parse_timestamp(t))
                .unwrap_or(i64::MAX),
        };
        let (datapoint, label, unit) = self.compute_statistics(&input);
        let mut datapoints = String::new();
        if let Some(d) = datapoint {
            datapoints.push_str(&format!(
                "<member><Timestamp>{ts}</Timestamp><Sum>{sum}</Sum><Average>{avg}</Average><Minimum>{min}</Minimum><Maximum>{max}</Maximum><SampleCount>{count}</SampleCount><Unit>{unit}</Unit></member>",
                ts = d.ts_rfc3339,
                sum = d.sum,
                avg = d.average,
                min = d.minimum,
                max = d.maximum,
                count = d.sample_count,
                unit = xml_escape(&unit),
            ));
        }
        let body = format!(
            "<Label>{}</Label><Datapoints>{datapoints}</Datapoints>",
            xml_escape(&label)
        );
        Ok(wrap("GetMetricStatistics", &body))
    }

    /// Returns (datapoint, label, unit). `None` for datapoint means no values
    /// fell inside the requested window.
    fn compute_statistics(
        &self,
        input: &GetMetricStatisticsInput,
    ) -> (Option<DatapointResult>, String, String) {
        let key = metric_key(&input.namespace, &input.metric_name, &input.dimensions);
        let s = self.state.read();
        let Some(series) = s.metrics.get(&key) else {
            return (None, input.metric_name.clone(), "None".to_string());
        };
        let in_window: Vec<&DataPoint> = series
            .points
            .iter()
            .filter(|p| p.timestamp_ms >= input.start_ms && p.timestamp_ms <= input.end_ms)
            .collect();
        if in_window.is_empty() {
            return (None, input.metric_name.clone(), series.unit.clone());
        }
        let sum: f64 = in_window.iter().map(|p| p.value).sum();
        let count = in_window.len() as f64;
        let avg = sum / count;
        let min = in_window
            .iter()
            .map(|p| p.value)
            .fold(f64::INFINITY, f64::min);
        let max = in_window
            .iter()
            .map(|p| p.value)
            .fold(f64::NEG_INFINITY, f64::max);
        let ts = chrono::DateTime::<chrono::Utc>::from_timestamp_millis(in_window[0].timestamp_ms)
            .unwrap_or_else(chrono::Utc::now);
        (
            Some(DatapointResult {
                ts_rfc3339: ts.to_rfc3339(),
                ts_secs: ts.timestamp() as f64,
                sum,
                average: avg,
                minimum: min,
                maximum: max,
                sample_count: count,
            }),
            input.metric_name.clone(),
            series.unit.clone(),
        )
    }

    fn list_metrics(&self, params: &HashMap<String, String>) -> Result<String, AwsError> {
        let filter = ListMetricsInput {
            namespace_filter: params.get("Namespace").cloned(),
            metric_filter: params.get("MetricName").cloned(),
        };
        let series_snapshot = self.snapshot_series(&filter);
        let mut members = String::new();
        for series in &series_snapshot {
            let mut dim_xml = String::new();
            for (n, v) in &series.dimensions {
                dim_xml.push_str(&format!(
                    "<member><Name>{}</Name><Value>{}</Value></member>",
                    xml_escape(n),
                    xml_escape(v)
                ));
            }
            members.push_str(&format!(
                "<member><Namespace>{ns}</Namespace><MetricName>{name}</MetricName><Dimensions>{dim_xml}</Dimensions></member>",
                ns = xml_escape(&series.namespace),
                name = xml_escape(&series.metric_name),
            ));
        }
        Ok(wrap(
            "ListMetrics",
            &format!("<Metrics>{members}</Metrics>"),
        ))
    }

    fn snapshot_series(&self, filter: &ListMetricsInput) -> Vec<MetricSeries> {
        let s = self.state.read();
        s.metrics
            .values()
            .filter(|series| {
                filter
                    .namespace_filter
                    .as_deref()
                    .is_none_or(|n| n == series.namespace)
                    && filter
                        .metric_filter
                        .as_deref()
                        .is_none_or(|m| m == series.metric_name)
            })
            .cloned()
            .collect()
    }

    fn put_metric_alarm(&self, params: &HashMap<String, String>) -> Result<String, AwsError> {
        let input = PutMetricAlarmInput {
            name: required(params, "AlarmName")?,
            namespace: params.get("Namespace").cloned(),
            metric_name: params.get("MetricName").cloned(),
            statistic: params.get("Statistic").cloned(),
            threshold: params.get("Threshold").and_then(|v| v.parse().ok()),
            comparison_operator: params.get("ComparisonOperator").cloned(),
            period: params.get("Period").and_then(|v| v.parse().ok()),
            evaluation_periods: params.get("EvaluationPeriods").and_then(|v| v.parse().ok()),
        };
        self.do_put_metric_alarm(input);
        Ok(empty("PutMetricAlarm"))
    }

    fn do_put_metric_alarm(&self, input: PutMetricAlarmInput) {
        let alarm = Alarm {
            arn: alarm_arn(&input.name),
            name: input.name.clone(),
            namespace: input.namespace,
            metric_name: input.metric_name,
            statistic: input.statistic,
            threshold: input.threshold,
            comparison_operator: input.comparison_operator,
            period: input.period,
            evaluation_periods: input.evaluation_periods,
            state: "INSUFFICIENT_DATA".into(),
            created: chrono::Utc::now(),
        };
        self.state.write().alarms.insert(input.name, alarm);
    }

    fn describe_alarms(&self, params: &HashMap<String, String>) -> Result<String, AwsError> {
        let names_filter = collect_indexed(params, "AlarmNames.member");
        let s = self.state.read();
        let mut metric_alarms = String::new();
        for alarm in s.alarms.values() {
            if !names_filter.is_empty() && !names_filter.contains(&alarm.name) {
                continue;
            }
            metric_alarms.push_str(&alarm_xml(alarm));
        }
        Ok(wrap(
            "DescribeAlarms",
            &format!("<MetricAlarms>{metric_alarms}</MetricAlarms>"),
        ))
    }

    fn delete_alarms(&self, params: &HashMap<String, String>) -> Result<String, AwsError> {
        let names = collect_indexed(params, "AlarmNames.member");
        let mut s = self.state.write();
        for n in names {
            s.alarms.remove(&n);
        }
        Ok(empty("DeleteAlarms"))
    }
}

#[async_trait]
impl CborProtocolService for CloudWatch {
    fn smithy_service(&self) -> &'static str {
        CBOR_SERVICE
    }

    async fn dispatch(
        &self,
        _ctx: ServiceContext,
        operation: &str,
        body: Bytes,
    ) -> Result<Bytes, AwsError> {
        // CBOR may carry tagged values and `Tag(n)` enums that serde_json::Value
        // cannot represent directly. Decode to `ciborium::value::Value` first
        // and convert into JsonValue manually so we never trip the
        // "invalid type: enum" deserializer error.
        let req: JsonValue = if body.is_empty() {
            json!({})
        } else {
            let cbor: ciborium::value::Value = ciborium::de::from_reader(body.as_ref())
                .map_err(|e| AwsError::new("InvalidRequest", e.to_string()))?;
            cbor_to_json(cbor)
        };
        // GetMetricStatistics returns datapoints whose Timestamp must be a
        // CBOR tag(1) value, not a plain float — that's what
        // `expected tag` from the SDK means. We therefore build that response
        // as a `ciborium::value::Value` directly; the other operations are
        // tag-free so JSON-based construction is fine.
        let bytes = match operation {
            "PutMetricData" => {
                self.cbor_put_metric_data(&req)?;
                encode_json(&json!({}))?
            }
            "GetMetricStatistics" => {
                let v = self.cbor_get_metric_statistics_value(&req)?;
                encode_cbor_value(&v)?
            }
            "ListMetrics" => encode_json(&self.cbor_list_metrics(&req)?)?,
            "PutMetricAlarm" => {
                self.cbor_put_metric_alarm(&req)?;
                encode_json(&json!({}))?
            }
            "DescribeAlarms" => encode_json(&self.cbor_describe_alarms(&req)?)?,
            "DeleteAlarms" => {
                self.cbor_delete_alarms(&req)?;
                encode_json(&json!({}))?
            }
            other => return Err(AwsError::unsupported(format!("CloudWatch::{other}"))),
        };
        Ok(bytes)
    }
}

impl CloudWatch {
    fn cbor_put_metric_data(&self, req: &JsonValue) -> Result<(), AwsError> {
        let namespace = req
            .get("Namespace")
            .and_then(JsonValue::as_str)
            .ok_or_else(|| AwsError::new("ValidationException", "Namespace required"))?
            .to_string();
        let data: Vec<MetricDatumInput> = req
            .get("MetricData")
            .and_then(JsonValue::as_array)
            .map(|arr| arr.iter().map(parse_datum_cbor).collect())
            .unwrap_or_default();
        self.do_put_metric_data(PutMetricDataInput { namespace, data });
        Ok(())
    }

    /// Returns the response as a `ciborium::value::Value` so the timestamp
    /// field can be wire-encoded with CBOR tag(1) — the SDK rejects plain
    /// numeric timestamps.
    fn cbor_get_metric_statistics_value(
        &self,
        req: &JsonValue,
    ) -> Result<ciborium::value::Value, AwsError> {
        use ciborium::value::Value as CV;
        let input = GetMetricStatisticsInput {
            namespace: cbor_str(req, "Namespace")?,
            metric_name: cbor_str(req, "MetricName")?,
            dimensions: parse_dimensions_cbor(req.get("Dimensions")),
            start_ms: cbor_timestamp(req.get("StartTime")).unwrap_or(i64::MIN),
            end_ms: cbor_timestamp(req.get("EndTime")).unwrap_or(i64::MAX),
        };
        let (datapoint, label, unit) = self.compute_statistics(&input);
        let mut datapoints_cbor: Vec<CV> = Vec::new();
        if let Some(d) = datapoint {
            datapoints_cbor.push(CV::Map(vec![
                (
                    CV::Text("Timestamp".into()),
                    CV::Tag(1, Box::new(CV::Float(d.ts_secs))),
                ),
                (CV::Text("Sum".into()), CV::Float(d.sum)),
                (CV::Text("Average".into()), CV::Float(d.average)),
                (CV::Text("Minimum".into()), CV::Float(d.minimum)),
                (CV::Text("Maximum".into()), CV::Float(d.maximum)),
                (CV::Text("SampleCount".into()), CV::Float(d.sample_count)),
                (CV::Text("Unit".into()), CV::Text(unit)),
            ]));
        }
        Ok(CV::Map(vec![
            (CV::Text("Label".into()), CV::Text(label)),
            (CV::Text("Datapoints".into()), CV::Array(datapoints_cbor)),
        ]))
    }

    fn cbor_list_metrics(&self, req: &JsonValue) -> Result<JsonValue, AwsError> {
        let filter = ListMetricsInput {
            namespace_filter: req
                .get("Namespace")
                .and_then(JsonValue::as_str)
                .map(String::from),
            metric_filter: req
                .get("MetricName")
                .and_then(JsonValue::as_str)
                .map(String::from),
        };
        let series = self.snapshot_series(&filter);
        let metrics: Vec<JsonValue> = series
            .iter()
            .map(|s| {
                json!({
                    "Namespace": s.namespace,
                    "MetricName": s.metric_name,
                    "Dimensions": s
                        .dimensions
                        .iter()
                        .map(|(n, v)| json!({"Name": n, "Value": v}))
                        .collect::<Vec<_>>(),
                })
            })
            .collect();
        Ok(json!({ "Metrics": metrics }))
    }

    fn cbor_put_metric_alarm(&self, req: &JsonValue) -> Result<(), AwsError> {
        let input = PutMetricAlarmInput {
            name: cbor_str(req, "AlarmName")?,
            namespace: req
                .get("Namespace")
                .and_then(JsonValue::as_str)
                .map(String::from),
            metric_name: req
                .get("MetricName")
                .and_then(JsonValue::as_str)
                .map(String::from),
            statistic: req
                .get("Statistic")
                .and_then(JsonValue::as_str)
                .map(String::from),
            threshold: req.get("Threshold").and_then(JsonValue::as_f64),
            comparison_operator: req
                .get("ComparisonOperator")
                .and_then(JsonValue::as_str)
                .map(String::from),
            period: req.get("Period").and_then(JsonValue::as_i64),
            evaluation_periods: req.get("EvaluationPeriods").and_then(JsonValue::as_i64),
        };
        self.do_put_metric_alarm(input);
        Ok(())
    }

    fn cbor_describe_alarms(&self, req: &JsonValue) -> Result<JsonValue, AwsError> {
        let names_filter: Vec<String> = req
            .get("AlarmNames")
            .and_then(JsonValue::as_array)
            .map(|a| {
                a.iter()
                    .filter_map(JsonValue::as_str)
                    .map(String::from)
                    .collect()
            })
            .unwrap_or_default();
        let s = self.state.read();
        let metric_alarms: Vec<JsonValue> = s
            .alarms
            .values()
            .filter(|a| names_filter.is_empty() || names_filter.contains(&a.name))
            .map(|a| {
                json!({
                    "AlarmName": a.name,
                    "AlarmArn": a.arn,
                    "StateValue": a.state,
                    "Namespace": a.namespace,
                    "MetricName": a.metric_name,
                    "Statistic": a.statistic,
                    "Threshold": a.threshold,
                    "ComparisonOperator": a.comparison_operator,
                    "Period": a.period,
                    "EvaluationPeriods": a.evaluation_periods,
                })
            })
            .collect();
        Ok(json!({ "MetricAlarms": metric_alarms }))
    }

    fn cbor_delete_alarms(&self, req: &JsonValue) -> Result<(), AwsError> {
        let names: Vec<String> = req
            .get("AlarmNames")
            .and_then(JsonValue::as_array)
            .map(|a| {
                a.iter()
                    .filter_map(JsonValue::as_str)
                    .map(String::from)
                    .collect()
            })
            .unwrap_or_default();
        let mut s = self.state.write();
        for n in names {
            s.alarms.remove(&n);
        }
        Ok(())
    }
}

fn encode_json(value: &JsonValue) -> Result<Bytes, AwsError> {
    let mut buf = Vec::new();
    ciborium::ser::into_writer(value, &mut buf).map_err(|e| AwsError::internal(e.to_string()))?;
    Ok(Bytes::from(buf))
}

fn encode_cbor_value(value: &ciborium::value::Value) -> Result<Bytes, AwsError> {
    let mut buf = Vec::new();
    ciborium::ser::into_writer(value, &mut buf).map_err(|e| AwsError::internal(e.to_string()))?;
    Ok(Bytes::from(buf))
}

/// Best-effort `ciborium::value::Value` → `serde_json::Value` conversion.
/// CBOR `Bytes` becomes a base64 string (so callers can round-trip if they
/// care); tags are unwrapped (we only need the payload); CBOR `Map` with
/// non-text keys is dropped.
fn cbor_to_json(v: ciborium::value::Value) -> JsonValue {
    use ciborium::value::Value as CV;
    match v {
        CV::Null => JsonValue::Null,
        CV::Bool(b) => JsonValue::Bool(b),
        CV::Integer(i) => {
            let as_i128: i128 = i.into();
            if let Ok(i64v) = i64::try_from(as_i128) {
                JsonValue::Number(i64v.into())
            } else {
                // Fall back to a JSON string if the integer doesn't fit.
                JsonValue::String(as_i128.to_string())
            }
        }
        CV::Float(f) => serde_json::Number::from_f64(f)
            .map(JsonValue::Number)
            .unwrap_or(JsonValue::Null),
        CV::Text(s) => JsonValue::String(s),
        CV::Bytes(b) => {
            use base64::Engine;
            JsonValue::String(base64::engine::general_purpose::STANDARD.encode(b))
        }
        CV::Array(arr) => JsonValue::Array(arr.into_iter().map(cbor_to_json).collect()),
        CV::Map(entries) => {
            let mut obj = serde_json::Map::new();
            for (k, val) in entries {
                if let CV::Text(key) = k {
                    obj.insert(key, cbor_to_json(val));
                }
            }
            JsonValue::Object(obj)
        }
        // Tagged values: unwrap and keep the inner. CBOR tag 1 is "epoch
        // timestamp", and our timestamp parser handles both seconds (number)
        // and string forms — either way the inner value is what we need.
        CV::Tag(_tag, inner) => cbor_to_json(*inner),
        _ => JsonValue::Null,
    }
}

fn cbor_str(req: &JsonValue, key: &str) -> Result<String, AwsError> {
    req.get(key)
        .and_then(JsonValue::as_str)
        .map(String::from)
        .ok_or_else(|| AwsError::new("ValidationException", format!("{key} required")))
}

fn parse_datum_cbor(d: &JsonValue) -> MetricDatumInput {
    MetricDatumInput {
        metric_name: d
            .get("MetricName")
            .and_then(JsonValue::as_str)
            .unwrap_or("")
            .to_string(),
        value: d.get("Value").and_then(JsonValue::as_f64).unwrap_or(0.0),
        unit: d
            .get("Unit")
            .and_then(JsonValue::as_str)
            .unwrap_or("None")
            .to_string(),
        timestamp_ms: cbor_timestamp(d.get("Timestamp"))
            .unwrap_or_else(|| chrono::Utc::now().timestamp_millis()),
        dimensions: parse_dimensions_cbor(d.get("Dimensions")),
    }
}

fn parse_dimensions_cbor(value: Option<&JsonValue>) -> Vec<(String, String)> {
    value
        .and_then(JsonValue::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(|d| {
                    let n = d.get("Name").and_then(JsonValue::as_str)?;
                    let v = d.get("Value").and_then(JsonValue::as_str)?;
                    Some((n.to_string(), v.to_string()))
                })
                .collect()
        })
        .unwrap_or_default()
}

/// CBOR encodes timestamps either as a float seconds-since-epoch, an int,
/// or as a tagged value `tag(1) <number>`. ciborium normalizes the tag form
/// into a number, so we just need to handle int and float.
fn cbor_timestamp(value: Option<&JsonValue>) -> Option<i64> {
    let v = value?;
    if let Some(i) = v.as_i64() {
        return Some(i * 1000);
    }
    if let Some(f) = v.as_f64() {
        return Some((f * 1000.0) as i64);
    }
    if let Some(s) = v.as_str() {
        return parse_timestamp(s);
    }
    None
}

fn metric_key(namespace: &str, metric: &str, dims: &[(String, String)]) -> String {
    let mut sorted = dims.to_vec();
    sorted.sort_by(|a, b| a.0.cmp(&b.0));
    let dim_str: String = sorted
        .iter()
        .map(|(k, v)| format!("{k}={v}"))
        .collect::<Vec<_>>()
        .join(",");
    format!("{namespace}|{metric}|{dim_str}")
}

fn parse_dimensions_query(params: &HashMap<String, String>, prefix: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut i = 1;
    loop {
        let name_key = format!("{prefix}.member.{i}.Name");
        let value_key = format!("{prefix}.member.{i}.Value");
        let (Some(name), Some(value)) = (params.get(&name_key), params.get(&value_key)) else {
            break;
        };
        out.push((name.clone(), value.clone()));
        i += 1;
    }
    out
}

fn collect_indexed(params: &HashMap<String, String>, prefix: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut i = 1;
    loop {
        let key = format!("{prefix}.{i}");
        let Some(v) = params.get(&key) else {
            break;
        };
        out.push(v.clone());
        i += 1;
    }
    out
}

fn parse_timestamp(s: &str) -> Option<i64> {
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(s) {
        return Some(dt.timestamp_millis());
    }
    s.parse::<f64>().ok().map(|secs| (secs * 1000.0) as i64)
}

fn alarm_arn(name: &str) -> String {
    format!("arn:aws:cloudwatch:{EMULATED_REGION}:{EMULATED_ACCOUNT_ID}:alarm:{name}")
}

fn alarm_xml(a: &Alarm) -> String {
    format!(
        "<member><AlarmName>{name}</AlarmName><AlarmArn>{arn}</AlarmArn><StateValue>{state}</StateValue><Namespace>{ns}</Namespace><MetricName>{metric}</MetricName><Statistic>{stat}</Statistic><Threshold>{thr}</Threshold><ComparisonOperator>{op}</ComparisonOperator><Period>{period}</Period><EvaluationPeriods>{eval}</EvaluationPeriods></member>",
        name = xml_escape(&a.name),
        arn = xml_escape(&a.arn),
        state = a.state,
        ns = xml_escape(a.namespace.as_deref().unwrap_or("")),
        metric = xml_escape(a.metric_name.as_deref().unwrap_or("")),
        stat = xml_escape(a.statistic.as_deref().unwrap_or("")),
        thr = a.threshold.unwrap_or_default(),
        op = xml_escape(a.comparison_operator.as_deref().unwrap_or("")),
        period = a.period.unwrap_or_default(),
        eval = a.evaluation_periods.unwrap_or_default(),
    )
}

fn required(p: &HashMap<String, String>, key: &str) -> Result<String, AwsError> {
    p.get(key)
        .cloned()
        .ok_or_else(|| AwsError::new("ValidationError", format!("{key} required")))
}

fn wrap(action: &str, body: &str) -> String {
    let rid = Uuid::new_v4();
    format!(
        "<{action}Response xmlns=\"{NS}\"><{action}Result>{body}</{action}Result><ResponseMetadata><RequestId>{rid}</RequestId></ResponseMetadata></{action}Response>"
    )
}

fn empty(action: &str) -> String {
    let rid = Uuid::new_v4();
    format!(
        "<{action}Response xmlns=\"{NS}\"><ResponseMetadata><RequestId>{rid}</RequestId></ResponseMetadata></{action}Response>"
    )
}

pub fn register(registry: &Arc<Registry>) {
    let cw = Arc::new(CloudWatch::new());
    registry.register_query(cw.clone());
    registry.register_cbor(cw);
}
