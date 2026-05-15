//! EventBridge — AWS JSON 1.1, target prefix `AWSEvents`.
//!
//! Supports event bus / rule / target lifecycle plus PutEvents with target
//! fanout. The supported target protocol today is **SQS**: when a rule
//! matches and the target ARN points to an SQS queue, kuroko enqueues the
//! event into the queue's state. Other target types (Lambda, Step Functions,
//! etc.) accept the wiring but don't dispatch.
//!
//! Event matching is intentionally minimal — the rule's `EventPattern` JSON
//! is parsed as a flat `{"source": ["..."], "detail-type": ["..."]}` map and
//! ANDed against the event. This catches the most-common test scenarios
//! without re-implementing the full EventBridge pattern language.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use axum::Router;
use bytes::Bytes;
use parking_lot::RwLock;
use serde_json::{Value, json};

use crate::aws_error::AwsError;
use crate::registry::Registry;
use crate::service::{
    EMULATED_ACCOUNT_ID, EMULATED_REGION, JsonProtocolService, Service, ServiceContext,
    persistence_error,
};

const TARGET_PREFIX: &str = "AWSEvents";

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
struct State {
    buses: HashMap<String, EventBus>,
    rules: HashMap<String, Rule>,
    /// rule_arn → targets
    targets: HashMap<String, Vec<Target>>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct EventBus {
    name: String,
    arn: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct Rule {
    name: String,
    arn: String,
    event_bus_name: String,
    event_pattern: Option<Value>,
    schedule_expression: Option<String>,
    state: String,
    description: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct Target {
    id: String,
    arn: String,
    input: Option<String>,
}

pub struct EventBridge {
    state: Arc<RwLock<State>>,
    registry: parking_lot::RwLock<Option<std::sync::Weak<Registry>>>,
}

impl EventBridge {
    pub fn new() -> Self {
        let bus = EventBus {
            name: "default".into(),
            arn: bus_arn("default"),
        };
        let mut s = State::default();
        s.buses.insert("default".into(), bus);
        Self {
            state: Arc::new(RwLock::new(s)),
            registry: parking_lot::RwLock::new(None),
        }
    }

    pub fn set_registry(&self, registry: std::sync::Weak<Registry>) {
        *self.registry.write() = Some(registry);
    }
}

impl Default for EventBridge {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Service for EventBridge {
    fn name(&self) -> &'static str {
        "eventbridge"
    }

    fn reset(&self) {
        let mut s = self.state.write();
        *s = State::default();
        s.buses.insert(
            "default".into(),
            EventBus {
                name: "default".into(),
                arn: bus_arn("default"),
            },
        );
    }

    fn restore(&self, ctx: &ServiceContext) -> Result<(), AwsError> {
        if let Some(snap) = &ctx.snapshot
            && let Some(data) = snap
                .load::<State>("eventbridge")
                .map_err(persistence_error)?
        {
            *self.state.write() = data;
        }
        Ok(())
    }

    fn snapshot(&self, ctx: &ServiceContext) -> Result<(), AwsError> {
        if let Some(snap) = &ctx.snapshot {
            let data = self.state.read();
            snap.save("eventbridge", &*data)
                .map_err(persistence_error)?;
        }
        Ok(())
    }

    fn router(&self, _ctx: ServiceContext) -> Router {
        Router::new()
    }
}

#[async_trait]
impl JsonProtocolService for EventBridge {
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
            "CreateEventBus" => self.create_event_bus(&req),
            "ListEventBuses" => self.list_event_buses(&req),
            "DeleteEventBus" => self.delete_event_bus(&req),
            "DescribeEventBus" => self.describe_event_bus(&req),
            "PutRule" => self.put_rule(&req),
            "DescribeRule" => self.describe_rule(&req),
            "ListRules" => self.list_rules(&req),
            "DeleteRule" => self.delete_rule(&req),
            "EnableRule" => self.set_rule_state(&req, "ENABLED"),
            "DisableRule" => self.set_rule_state(&req, "DISABLED"),
            "PutTargets" => self.put_targets(&req),
            "RemoveTargets" => self.remove_targets(&req),
            "ListTargetsByRule" => self.list_targets_by_rule(&req),
            "PutEvents" => self.put_events(&req),
            other => Err(AwsError::unsupported(format!("EventBridge::{other}"))),
        }
    }
}

impl EventBridge {
    fn create_event_bus(&self, req: &Value) -> Result<Value, AwsError> {
        let name = req
            .get("Name")
            .and_then(Value::as_str)
            .ok_or_else(|| AwsError::new("ValidationException", "Name required"))?
            .to_string();
        let mut s = self.state.write();
        if s.buses.contains_key(&name) {
            return Err(AwsError::new(
                "ResourceAlreadyExistsException",
                format!("event bus '{name}' already exists"),
            ));
        }
        let arn = bus_arn(&name);
        s.buses.insert(
            name.clone(),
            EventBus {
                name,
                arn: arn.clone(),
            },
        );
        Ok(json!({ "EventBusArn": arn }))
    }

    fn list_event_buses(&self, _req: &Value) -> Result<Value, AwsError> {
        let s = self.state.read();
        let buses: Vec<_> = s
            .buses
            .values()
            .map(|b| json!({ "Name": b.name, "Arn": b.arn }))
            .collect();
        Ok(json!({ "EventBuses": buses }))
    }

    fn delete_event_bus(&self, req: &Value) -> Result<Value, AwsError> {
        let name = req
            .get("Name")
            .and_then(Value::as_str)
            .ok_or_else(|| AwsError::new("ValidationException", "Name required"))?;
        if name == "default" {
            return Err(AwsError::new(
                "ValidationException",
                "cannot delete the default event bus",
            ));
        }
        self.state.write().buses.remove(name);
        Ok(json!({}))
    }

    fn describe_event_bus(&self, req: &Value) -> Result<Value, AwsError> {
        let name = req.get("Name").and_then(Value::as_str).unwrap_or("default");
        let s = self.state.read();
        let bus = s.buses.get(name).ok_or_else(|| not_found_bus(name))?;
        Ok(json!({ "Name": bus.name, "Arn": bus.arn, "Policy": "" }))
    }

    fn put_rule(&self, req: &Value) -> Result<Value, AwsError> {
        let name = req
            .get("Name")
            .and_then(Value::as_str)
            .ok_or_else(|| AwsError::new("ValidationException", "Name required"))?
            .to_string();
        let bus_name = req
            .get("EventBusName")
            .and_then(Value::as_str)
            .unwrap_or("default")
            .to_string();
        let event_pattern = req
            .get("EventPattern")
            .and_then(Value::as_str)
            .map(|s| serde_json::from_str::<Value>(s).unwrap_or(Value::Null));
        let schedule_expression = req
            .get("ScheduleExpression")
            .and_then(Value::as_str)
            .map(String::from);
        let state = req
            .get("State")
            .and_then(Value::as_str)
            .unwrap_or("ENABLED")
            .to_string();
        let description = req
            .get("Description")
            .and_then(Value::as_str)
            .map(String::from);
        let arn = rule_arn(&bus_name, &name);
        let rule = Rule {
            name: name.clone(),
            arn: arn.clone(),
            event_bus_name: bus_name,
            event_pattern,
            schedule_expression,
            state,
            description,
        };
        self.state
            .write()
            .rules
            .insert(rule_key(&rule.event_bus_name, &name), rule);
        Ok(json!({ "RuleArn": arn }))
    }

    fn describe_rule(&self, req: &Value) -> Result<Value, AwsError> {
        let (bus_name, name) = parse_rule_ref(req)?;
        let key = rule_key(&bus_name, &name);
        let s = self.state.read();
        let r = s.rules.get(&key).ok_or_else(|| not_found_rule(&name))?;
        Ok(rule_json(r))
    }

    fn list_rules(&self, req: &Value) -> Result<Value, AwsError> {
        let bus_name = req
            .get("EventBusName")
            .and_then(Value::as_str)
            .unwrap_or("default");
        let s = self.state.read();
        let rules: Vec<_> = s
            .rules
            .values()
            .filter(|r| r.event_bus_name == bus_name)
            .map(rule_json)
            .collect();
        Ok(json!({ "Rules": rules }))
    }

    fn delete_rule(&self, req: &Value) -> Result<Value, AwsError> {
        let (bus_name, name) = parse_rule_ref(req)?;
        let key = rule_key(&bus_name, &name);
        let mut s = self.state.write();
        if let Some(rule) = s.rules.remove(&key) {
            s.targets.remove(&rule.arn);
        }
        Ok(json!({}))
    }

    fn set_rule_state(&self, req: &Value, state: &str) -> Result<Value, AwsError> {
        let (bus_name, name) = parse_rule_ref(req)?;
        let key = rule_key(&bus_name, &name);
        let mut s = self.state.write();
        let r = s.rules.get_mut(&key).ok_or_else(|| not_found_rule(&name))?;
        r.state = state.to_string();
        Ok(json!({}))
    }

    fn put_targets(&self, req: &Value) -> Result<Value, AwsError> {
        let (bus_name, name) = parse_rule_ref(req)?;
        let key = rule_key(&bus_name, &name);
        let raw_targets = req
            .get("Targets")
            .and_then(Value::as_array)
            .ok_or_else(|| AwsError::new("ValidationException", "Targets required"))?;
        let mut s = self.state.write();
        let r = s.rules.get(&key).ok_or_else(|| not_found_rule(&name))?;
        let arn = r.arn.clone();
        let mut targets = s.targets.remove(&arn).unwrap_or_default();
        for t in raw_targets {
            let id = t
                .get("Id")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            let target_arn = t
                .get("Arn")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            let input = t.get("Input").and_then(Value::as_str).map(String::from);
            targets.retain(|x| x.id != id);
            targets.push(Target {
                id,
                arn: target_arn,
                input,
            });
        }
        s.targets.insert(arn, targets);
        Ok(json!({ "FailedEntryCount": 0, "FailedEntries": [] }))
    }

    fn remove_targets(&self, req: &Value) -> Result<Value, AwsError> {
        let (bus_name, name) = parse_rule_ref(req)?;
        let key = rule_key(&bus_name, &name);
        let ids: Vec<String> = req
            .get("Ids")
            .and_then(Value::as_array)
            .map(|a| {
                a.iter()
                    .filter_map(Value::as_str)
                    .map(String::from)
                    .collect()
            })
            .unwrap_or_default();
        let mut s = self.state.write();
        let r = s.rules.get(&key).ok_or_else(|| not_found_rule(&name))?;
        let arn = r.arn.clone();
        if let Some(targets) = s.targets.get_mut(&arn) {
            targets.retain(|t| !ids.contains(&t.id));
        }
        Ok(json!({ "FailedEntryCount": 0, "FailedEntries": [] }))
    }

    fn list_targets_by_rule(&self, req: &Value) -> Result<Value, AwsError> {
        let (bus_name, name) = parse_rule_ref(req)?;
        let key = rule_key(&bus_name, &name);
        let s = self.state.read();
        let r = s.rules.get(&key).ok_or_else(|| not_found_rule(&name))?;
        let targets: Vec<_> = s
            .targets
            .get(&r.arn)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .map(|t| {
                let mut v = json!({ "Id": t.id, "Arn": t.arn });
                if let Some(input) = t.input {
                    v["Input"] = json!(input);
                }
                v
            })
            .collect();
        Ok(json!({ "Targets": targets }))
    }

    fn put_events(&self, req: &Value) -> Result<Value, AwsError> {
        let entries = req
            .get("Entries")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();

        // Snapshot rules/targets for the buses we care about, then drop lock.
        let routes: Vec<(Rule, Vec<Target>)> = {
            let s = self.state.read();
            s.rules
                .values()
                .filter(|r| r.state == "ENABLED")
                .map(|r| {
                    (
                        r.clone(),
                        s.targets.get(&r.arn).cloned().unwrap_or_default(),
                    )
                })
                .collect()
        };

        let mut event_ids = Vec::new();
        let registry = self.registry.read().clone().and_then(|w| w.upgrade());
        for entry in &entries {
            let event_id = uuid::Uuid::new_v4().to_string();
            event_ids.push(event_id.clone());
            for (rule, targets) in &routes {
                if rule.event_bus_name
                    != entry
                        .get("EventBusName")
                        .and_then(Value::as_str)
                        .unwrap_or("default")
                {
                    continue;
                }
                if !matches_pattern(&rule.event_pattern, entry) {
                    continue;
                }
                if let Some(reg) = &registry {
                    for t in targets {
                        deliver(reg, &t.arn, entry, &t.input, &event_id);
                    }
                }
            }
        }
        Ok(json!({
            "FailedEntryCount": 0,
            "Entries": event_ids
                .into_iter()
                .map(|id| json!({ "EventId": id }))
                .collect::<Vec<_>>(),
        }))
    }
}

fn parse_rule_ref(req: &Value) -> Result<(String, String), AwsError> {
    let name = req
        .get("Name")
        .and_then(Value::as_str)
        .or_else(|| req.get("Rule").and_then(Value::as_str))
        .ok_or_else(|| AwsError::new("ValidationException", "Rule name required"))?
        .to_string();
    let bus = req
        .get("EventBusName")
        .and_then(Value::as_str)
        .unwrap_or("default")
        .to_string();
    Ok((bus, name))
}

fn rule_key(bus: &str, name: &str) -> String {
    format!("{bus}::{name}")
}

fn bus_arn(name: &str) -> String {
    format!("arn:aws:events:{EMULATED_REGION}:{EMULATED_ACCOUNT_ID}:event-bus/{name}")
}

fn rule_arn(bus: &str, name: &str) -> String {
    if bus == "default" {
        format!("arn:aws:events:{EMULATED_REGION}:{EMULATED_ACCOUNT_ID}:rule/{name}")
    } else {
        format!("arn:aws:events:{EMULATED_REGION}:{EMULATED_ACCOUNT_ID}:rule/{bus}/{name}")
    }
}

fn rule_json(r: &Rule) -> Value {
    let mut v = json!({
        "Name": r.name,
        "Arn": r.arn,
        "EventBusName": r.event_bus_name,
        "State": r.state,
        "Description": r.description,
    });
    if let Some(p) = &r.event_pattern {
        v["EventPattern"] = json!(serde_json::to_string(p).unwrap_or_default());
    }
    if let Some(s) = &r.schedule_expression {
        v["ScheduleExpression"] = json!(s);
    }
    v
}

fn not_found_bus(name: &str) -> AwsError {
    AwsError::new(
        "ResourceNotFoundException",
        format!("event bus '{name}' not found"),
    )
}

fn not_found_rule(name: &str) -> AwsError {
    AwsError::new(
        "ResourceNotFoundException",
        format!("rule '{name}' not found"),
    )
}

/// Pattern matcher.
///
/// AWS event patterns use lower-case / hyphenated keys (`source`,
/// `detail-type`) but the wire-level event entries the SDK sends use
/// CamelCase (`Source`, `DetailType`). We translate the pattern key to the
/// corresponding entry field name when looking up the actual value.
///
/// Each pattern value is an array of accepted strings — the entry's value
/// must equal at least one. Unknown pattern keys are ignored.
fn matches_pattern(pattern: &Option<Value>, entry: &Value) -> bool {
    let Some(Value::Object(map)) = pattern else {
        return pattern.is_none();
    };
    for (k, accept_list) in map {
        let Some(arr) = accept_list.as_array() else {
            continue;
        };
        let accepts: Vec<&str> = arr.iter().filter_map(|v| v.as_str()).collect();
        if accepts.is_empty() {
            continue;
        }
        let entry_key = match k.as_str() {
            "source" => "Source",
            "detail-type" => "DetailType",
            "account" => "Account",
            "region" => "Region",
            "resources" => "Resources",
            other => other,
        };
        let Some(Value::String(actual)) = entry.get(entry_key) else {
            return false;
        };
        if !accepts.contains(&actual.as_str()) {
            return false;
        }
    }
    true
}

fn deliver(
    registry: &Arc<Registry>,
    target_arn: &str,
    entry: &Value,
    input_override: &Option<String>,
    event_id: &str,
) {
    let body = input_override.clone().unwrap_or_else(|| {
        json!({
            "id": event_id,
            "version": "0",
            "source": entry.get("Source").cloned().unwrap_or(Value::Null),
            "detail-type": entry.get("DetailType").cloned().unwrap_or(Value::Null),
            "detail": entry.get("Detail").and_then(|d| d.as_str())
                .map(|s| serde_json::from_str::<Value>(s).unwrap_or(Value::Null))
                .unwrap_or(Value::Null),
            "time": chrono::Utc::now().to_rfc3339(),
            "resources": entry.get("Resources").cloned().unwrap_or_else(|| json!([])),
        })
        .to_string()
    });

    if target_arn.starts_with("arn:aws:sqs:") {
        let queue_name = target_arn.rsplit(':').next().unwrap_or("");
        if let Some(svc) = registry.get("sqs")
            && let Some(sqs) = svc
                .as_any()
                .and_then(|a| a.downcast_ref::<crate::services::sqs::Sqs>())
        {
            sqs.push_external(queue_name, &body);
            return;
        }
    }
    tracing::debug!(
        target = target_arn,
        "EventBridge: target protocol not wired"
    );
}

pub fn register(registry: &Arc<Registry>) {
    let eb = Arc::new(EventBridge::new());
    eb.set_registry(Arc::downgrade(registry));
    registry.register_json(eb);
}
