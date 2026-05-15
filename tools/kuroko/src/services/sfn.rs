//! Step Functions (sfn) — AWS JSON 1.0, target prefix `AWSStepFunctions`.
//!
//! State-machine and execution **metadata only**. kuroko does not interpret
//! the Amazon States Language: a StartExecution call immediately marks the
//! execution as `SUCCEEDED` with the input echoed as the output. This is
//! sufficient for tests that assert SDK call shapes (creation, listing,
//! describing) without driving a real interpreter.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use axum::Router;
use bytes::Bytes;
use parking_lot::RwLock;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::aws_error::AwsError;
use crate::service::{
    EMULATED_ACCOUNT_ID, EMULATED_REGION, JsonProtocolService, Service, ServiceContext,
    persistence_error,
};

const TARGET_PREFIX: &str = "AWSStepFunctions";

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
struct State {
    state_machines: HashMap<String, StateMachine>,
    executions: HashMap<String, Execution>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct StateMachine {
    name: String,
    arn: String,
    role_arn: String,
    definition: String,
    type_: String,
    status: String,
    created: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct Execution {
    arn: String,
    name: String,
    state_machine_arn: String,
    status: String,
    start_date: chrono::DateTime<chrono::Utc>,
    stop_date: Option<chrono::DateTime<chrono::Utc>>,
    input: String,
    output: Option<String>,
}

pub struct Sfn {
    state: Arc<RwLock<State>>,
}

impl Sfn {
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(State::default())),
        }
    }
}

impl Default for Sfn {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Service for Sfn {
    fn name(&self) -> &'static str {
        "sfn"
    }

    fn reset(&self) {
        let mut s = self.state.write();
        s.state_machines.clear();
        s.executions.clear();
    }

    fn restore(&self, ctx: &ServiceContext) -> Result<(), AwsError> {
        if let Some(snap) = &ctx.snapshot
            && let Some(data) = snap.load::<State>("sfn").map_err(persistence_error)?
        {
            *self.state.write() = data;
        }
        Ok(())
    }

    fn snapshot(&self, ctx: &ServiceContext) -> Result<(), AwsError> {
        if let Some(snap) = &ctx.snapshot {
            let data = self.state.read();
            snap.save("sfn", &*data).map_err(persistence_error)?;
        }
        Ok(())
    }

    fn router(&self, _ctx: ServiceContext) -> Router {
        Router::new()
    }
}

#[async_trait]
impl JsonProtocolService for Sfn {
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
            "CreateStateMachine" => self.create_state_machine(&req),
            "ListStateMachines" => self.list_state_machines(&req),
            "DescribeStateMachine" => self.describe_state_machine(&req),
            "UpdateStateMachine" => self.update_state_machine(&req),
            "DeleteStateMachine" => self.delete_state_machine(&req),
            "StartExecution" => self.start_execution(&req),
            "DescribeExecution" => self.describe_execution(&req),
            "ListExecutions" => self.list_executions(&req),
            "StopExecution" => self.stop_execution(&req),
            "GetExecutionHistory" => self.get_execution_history(&req),
            other => Err(AwsError::unsupported(format!("StepFunctions::{other}"))),
        }
    }
}

impl Sfn {
    fn create_state_machine(&self, req: &Value) -> Result<Value, AwsError> {
        let name = req
            .get("name")
            .and_then(Value::as_str)
            .ok_or_else(|| AwsError::new("ValidationException", "name required"))?
            .to_string();
        let definition = req
            .get("definition")
            .and_then(Value::as_str)
            .ok_or_else(|| AwsError::new("ValidationException", "definition required"))?
            .to_string();
        // Reject obviously-malformed definitions early.
        serde_json::from_str::<Value>(&definition).map_err(|e| {
            AwsError::new(
                "InvalidDefinition",
                format!("definition is not valid JSON: {e}"),
            )
        })?;
        let role_arn = req
            .get("roleArn")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let type_ = req
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or("STANDARD")
            .to_string();
        let arn = state_machine_arn(&name);
        let mut s = self.state.write();
        if s.state_machines.contains_key(&arn) {
            return Err(AwsError::new(
                "StateMachineAlreadyExists",
                format!("state machine '{name}' already exists"),
            ));
        }
        let created = chrono::Utc::now();
        s.state_machines.insert(
            arn.clone(),
            StateMachine {
                name,
                arn: arn.clone(),
                role_arn,
                definition,
                type_,
                status: "ACTIVE".to_string(),
                created,
            },
        );
        Ok(json!({
            "stateMachineArn": arn,
            "creationDate": created.timestamp(),
        }))
    }

    fn list_state_machines(&self, _req: &Value) -> Result<Value, AwsError> {
        let s = self.state.read();
        let machines: Vec<_> = s
            .state_machines
            .values()
            .map(|sm| {
                json!({
                    "stateMachineArn": sm.arn,
                    "name": sm.name,
                    "type": sm.type_,
                    "creationDate": sm.created.timestamp(),
                })
            })
            .collect();
        Ok(json!({ "stateMachines": machines }))
    }

    fn describe_state_machine(&self, req: &Value) -> Result<Value, AwsError> {
        let arn = req
            .get("stateMachineArn")
            .and_then(Value::as_str)
            .ok_or_else(|| AwsError::new("ValidationException", "stateMachineArn required"))?
            .to_string();
        let s = self.state.read();
        let sm = s
            .state_machines
            .get(&arn)
            .ok_or_else(|| not_found_machine(&arn))?;
        Ok(json!({
            "stateMachineArn": sm.arn,
            "name": sm.name,
            "status": sm.status,
            "definition": sm.definition,
            "roleArn": sm.role_arn,
            "type": sm.type_,
            "creationDate": sm.created.timestamp(),
        }))
    }

    fn update_state_machine(&self, req: &Value) -> Result<Value, AwsError> {
        let arn = req
            .get("stateMachineArn")
            .and_then(Value::as_str)
            .ok_or_else(|| AwsError::new("ValidationException", "stateMachineArn required"))?
            .to_string();
        let mut s = self.state.write();
        let sm = s
            .state_machines
            .get_mut(&arn)
            .ok_or_else(|| not_found_machine(&arn))?;
        if let Some(d) = req.get("definition").and_then(Value::as_str) {
            serde_json::from_str::<Value>(d).map_err(|e| {
                AwsError::new(
                    "InvalidDefinition",
                    format!("definition is not valid JSON: {e}"),
                )
            })?;
            sm.definition = d.to_string();
        }
        if let Some(r) = req.get("roleArn").and_then(Value::as_str) {
            sm.role_arn = r.to_string();
        }
        Ok(json!({ "updateDate": chrono::Utc::now().timestamp() }))
    }

    fn delete_state_machine(&self, req: &Value) -> Result<Value, AwsError> {
        let arn = req
            .get("stateMachineArn")
            .and_then(Value::as_str)
            .ok_or_else(|| AwsError::new("ValidationException", "stateMachineArn required"))?;
        self.state.write().state_machines.remove(arn);
        Ok(json!({}))
    }

    fn start_execution(&self, req: &Value) -> Result<Value, AwsError> {
        let machine_arn = req
            .get("stateMachineArn")
            .and_then(Value::as_str)
            .ok_or_else(|| AwsError::new("ValidationException", "stateMachineArn required"))?
            .to_string();
        let name = req
            .get("name")
            .and_then(Value::as_str)
            .map(String::from)
            .unwrap_or_else(|| Uuid::new_v4().to_string());
        let input = req
            .get("input")
            .and_then(Value::as_str)
            .unwrap_or("{}")
            .to_string();
        let start_date = chrono::Utc::now();
        let arn = execution_arn(&machine_arn, &name);

        let mut s = self.state.write();
        s.state_machines
            .get(&machine_arn)
            .ok_or_else(|| not_found_machine(&machine_arn))?;
        if s.executions.contains_key(&arn) {
            return Err(AwsError::new(
                "ExecutionAlreadyExists",
                format!("execution '{name}' already exists"),
            ));
        }
        s.executions.insert(
            arn.clone(),
            Execution {
                arn: arn.clone(),
                name,
                state_machine_arn: machine_arn,
                // kuroko cannot interpret ASL, so we surface the simplest
                // useful state to a caller that just wants "the execution
                // succeeded": immediate SUCCEEDED with input echoed.
                status: "SUCCEEDED".to_string(),
                start_date,
                stop_date: Some(start_date),
                input: input.clone(),
                output: Some(input),
            },
        );
        Ok(json!({
            "executionArn": arn,
            "startDate": start_date.timestamp(),
        }))
    }

    fn describe_execution(&self, req: &Value) -> Result<Value, AwsError> {
        let arn = req
            .get("executionArn")
            .and_then(Value::as_str)
            .ok_or_else(|| AwsError::new("ValidationException", "executionArn required"))?;
        let s = self.state.read();
        let e = s
            .executions
            .get(arn)
            .ok_or_else(|| not_found_execution(arn))?;
        let mut v = json!({
            "executionArn": e.arn,
            "stateMachineArn": e.state_machine_arn,
            "name": e.name,
            "status": e.status,
            "startDate": e.start_date.timestamp(),
            "input": e.input,
        });
        if let Some(stop) = e.stop_date {
            v["stopDate"] = json!(stop.timestamp());
        }
        if let Some(o) = &e.output {
            v["output"] = json!(o);
        }
        Ok(v)
    }

    fn list_executions(&self, req: &Value) -> Result<Value, AwsError> {
        let machine = req
            .get("stateMachineArn")
            .and_then(Value::as_str)
            .ok_or_else(|| AwsError::new("ValidationException", "stateMachineArn required"))?
            .to_string();
        let status_filter = req.get("statusFilter").and_then(Value::as_str);
        let s = self.state.read();
        let execs: Vec<_> = s
            .executions
            .values()
            .filter(|e| e.state_machine_arn == machine)
            .filter(|e| status_filter.is_none_or(|f| f == e.status))
            .map(|e| {
                json!({
                    "executionArn": e.arn,
                    "stateMachineArn": e.state_machine_arn,
                    "name": e.name,
                    "status": e.status,
                    "startDate": e.start_date.timestamp(),
                    "stopDate": e.stop_date.map(|d| d.timestamp()),
                })
            })
            .collect();
        Ok(json!({ "executions": execs }))
    }

    fn stop_execution(&self, req: &Value) -> Result<Value, AwsError> {
        let arn = req
            .get("executionArn")
            .and_then(Value::as_str)
            .ok_or_else(|| AwsError::new("ValidationException", "executionArn required"))?
            .to_string();
        let mut s = self.state.write();
        let e = s
            .executions
            .get_mut(&arn)
            .ok_or_else(|| not_found_execution(&arn))?;
        if e.status == "RUNNING" {
            e.status = "ABORTED".to_string();
            e.stop_date = Some(chrono::Utc::now());
        }
        Ok(json!({ "stopDate": chrono::Utc::now().timestamp() }))
    }

    fn get_execution_history(&self, req: &Value) -> Result<Value, AwsError> {
        let arn = req
            .get("executionArn")
            .and_then(Value::as_str)
            .ok_or_else(|| AwsError::new("ValidationException", "executionArn required"))?;
        let s = self.state.read();
        let e = s
            .executions
            .get(arn)
            .ok_or_else(|| not_found_execution(arn))?;
        // Synthesize a two-event history: start + succeed. Real history is
        // the per-state transition log; kuroko has none to emit.
        Ok(json!({
            "events": [
                {
                    "timestamp": e.start_date.timestamp(),
                    "type": "ExecutionStarted",
                    "id": 1,
                    "executionStartedEventDetails": {
                        "input": e.input,
                        "roleArn": "",
                    }
                },
                {
                    "timestamp": e.stop_date.unwrap_or(e.start_date).timestamp(),
                    "type": "ExecutionSucceeded",
                    "id": 2,
                    "previousEventId": 1,
                    "executionSucceededEventDetails": {
                        "output": e.output.clone().unwrap_or_default(),
                    }
                }
            ]
        }))
    }
}

fn state_machine_arn(name: &str) -> String {
    format!("arn:aws:states:{EMULATED_REGION}:{EMULATED_ACCOUNT_ID}:stateMachine:{name}")
}

fn execution_arn(machine_arn: &str, exec_name: &str) -> String {
    let machine_name = machine_arn.rsplit(':').next().unwrap_or("unknown");
    format!(
        "arn:aws:states:{EMULATED_REGION}:{EMULATED_ACCOUNT_ID}:execution:{machine_name}:{exec_name}"
    )
}

fn not_found_machine(arn: &str) -> AwsError {
    AwsError::new(
        "StateMachineDoesNotExist",
        format!("state machine '{arn}' does not exist"),
    )
}

fn not_found_execution(arn: &str) -> AwsError {
    AwsError::new(
        "ExecutionDoesNotExist",
        format!("execution '{arn}' does not exist"),
    )
}

pub fn register(registry: &Arc<crate::registry::Registry>) {
    registry.register_json(Arc::new(Sfn::new()));
}
