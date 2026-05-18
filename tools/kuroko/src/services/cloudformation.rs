//! CloudFormation — AWS Query protocol, sdk_id `cloudformation`.
//!
//! Stack metadata only — kuroko does not interpret or apply templates. Every
//! CreateStack/UpdateStack transitions the stack to `CREATE_COMPLETE` /
//! `UPDATE_COMPLETE` immediately so callers don't have to poll for status.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use axum::Router;
use parking_lot::RwLock;
use uuid::Uuid;

use crate::aws_error::{AwsError, xml_escape};
use crate::registry::Registry;
use crate::service::{
    EMULATED_ACCOUNT_ID, EMULATED_REGION, QueryProtocolService, Service, ServiceContext,
    persistence_error,
};

const SDK_ID: &str = "cloudformation";
const NS: &str = "http://cloudformation.amazonaws.com/doc/2010-05-15/";

const ACTIONS: &[&str] = &[
    "CreateStack",
    "UpdateStack",
    "DeleteStack",
    "DescribeStacks",
    "ListStacks",
    "DescribeStackEvents",
    "ListStackResources",
    "GetTemplate",
];

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
struct State {
    stacks: HashMap<String, Stack>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct Stack {
    name: String,
    id: String,
    template: String,
    parameters: Vec<(String, String)>,
    tags: Vec<(String, String)>,
    status: String,
    description: Option<String>,
    events: Vec<StackEvent>,
    created: chrono::DateTime<chrono::Utc>,
    updated: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct StackEvent {
    id: String,
    timestamp: chrono::DateTime<chrono::Utc>,
    resource_status: String,
    resource_type: String,
    logical_resource_id: String,
    physical_resource_id: String,
}

pub struct CloudFormation {
    state: Arc<RwLock<State>>,
}

impl CloudFormation {
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(State::default())),
        }
    }
}

impl Default for CloudFormation {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Service for CloudFormation {
    fn name(&self) -> &'static str {
        "cloudformation"
    }

    fn reset(&self) {
        *self.state.write() = State::default();
    }

    fn restore(&self, ctx: &ServiceContext) -> Result<(), AwsError> {
        if let Some(snap) = &ctx.snapshot
            && let Some(data) = snap
                .load::<State>("cloudformation")
                .map_err(persistence_error)?
        {
            *self.state.write() = data;
        }
        Ok(())
    }

    fn snapshot(&self, ctx: &ServiceContext) -> Result<(), AwsError> {
        if let Some(snap) = &ctx.snapshot {
            let data = self.state.read();
            snap.save("cloudformation", &*data)
                .map_err(persistence_error)?;
        }
        Ok(())
    }

    fn router(&self, _ctx: ServiceContext) -> Router {
        Router::new()
    }
}

#[async_trait]
impl QueryProtocolService for CloudFormation {
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
            "CreateStack" => self.create_stack(params),
            "UpdateStack" => self.update_stack(params),
            "DeleteStack" => self.delete_stack(params),
            "DescribeStacks" => self.describe_stacks(params),
            "ListStacks" => self.list_stacks(params),
            "DescribeStackEvents" => self.describe_stack_events(params),
            "ListStackResources" => self.list_stack_resources(params),
            "GetTemplate" => self.get_template(params),
            other => Err(AwsError::unsupported(format!("CloudFormation::{other}"))),
        }
    }
}

impl CloudFormation {
    fn create_stack(&self, params: &HashMap<String, String>) -> Result<String, AwsError> {
        let name = required(params, "StackName")?;
        let template = params
            .get("TemplateBody")
            .cloned()
            .unwrap_or_else(|| "{}".into());
        let parameters = parse_pairs(
            params,
            "Parameters.member",
            "ParameterKey",
            "ParameterValue",
        );
        let tags = parse_pairs(params, "Tags.member", "Key", "Value");
        let description = params.get("Description").cloned();
        let mut s = self.state.write();
        if s.stacks.contains_key(&name) {
            return Err(AwsError::new(
                "AlreadyExistsException",
                format!("stack '{name}' already exists"),
            ));
        }
        let id = stack_id(&name);
        let now = chrono::Utc::now();
        let stack = Stack {
            name: name.clone(),
            id: id.clone(),
            template,
            parameters,
            tags,
            // kuroko marks stacks complete immediately so SDK callers don't
            // have to poll for terminal status during tests.
            status: "CREATE_COMPLETE".into(),
            description,
            events: vec![StackEvent {
                id: Uuid::new_v4().to_string(),
                timestamp: now,
                resource_status: "CREATE_COMPLETE".into(),
                resource_type: "AWS::CloudFormation::Stack".into(),
                logical_resource_id: name.clone(),
                physical_resource_id: id.clone(),
            }],
            created: now,
            updated: None,
        };
        s.stacks.insert(name, stack);
        Ok(wrap(
            "CreateStack",
            &format!("<StackId>{}</StackId>", xml_escape(&id)),
        ))
    }

    fn update_stack(&self, params: &HashMap<String, String>) -> Result<String, AwsError> {
        let name = required(params, "StackName")?;
        let mut s = self.state.write();
        let stack = s
            .stacks
            .get_mut(&name)
            .ok_or_else(|| no_such_stack(&name))?;
        if let Some(t) = params.get("TemplateBody") {
            stack.template = t.clone();
        }
        let new_params = parse_pairs(
            params,
            "Parameters.member",
            "ParameterKey",
            "ParameterValue",
        );
        if !new_params.is_empty() {
            stack.parameters = new_params;
        }
        stack.status = "UPDATE_COMPLETE".into();
        let now = chrono::Utc::now();
        stack.updated = Some(now);
        stack.events.push(StackEvent {
            id: Uuid::new_v4().to_string(),
            timestamp: now,
            resource_status: "UPDATE_COMPLETE".into(),
            resource_type: "AWS::CloudFormation::Stack".into(),
            logical_resource_id: stack.name.clone(),
            physical_resource_id: stack.id.clone(),
        });
        Ok(wrap(
            "UpdateStack",
            &format!("<StackId>{}</StackId>", xml_escape(&stack.id)),
        ))
    }

    fn delete_stack(&self, params: &HashMap<String, String>) -> Result<String, AwsError> {
        let name = required(params, "StackName")?;
        let mut s = self.state.write();
        s.stacks.remove(&name);
        Ok(empty("DeleteStack"))
    }

    fn describe_stacks(&self, params: &HashMap<String, String>) -> Result<String, AwsError> {
        let s = self.state.read();
        let name_filter = params.get("StackName").cloned();
        let mut members = String::new();
        let mut empty_result = true;
        for stack in s.stacks.values() {
            if let Some(n) = &name_filter
                && &stack.name != n
                && &stack.id != n
            {
                continue;
            }
            members.push_str(&stack_xml(stack));
            empty_result = false;
        }
        if let Some(n) = &name_filter
            && empty_result
        {
            return Err(no_such_stack(n));
        }
        Ok(wrap(
            "DescribeStacks",
            &format!("<Stacks>{members}</Stacks>"),
        ))
    }

    fn list_stacks(&self, params: &HashMap<String, String>) -> Result<String, AwsError> {
        let status_filter = collect_indexed(params, "StackStatusFilter.member");
        let s = self.state.read();
        let mut members = String::new();
        for stack in s.stacks.values() {
            if !status_filter.is_empty() && !status_filter.contains(&stack.status) {
                continue;
            }
            members.push_str(&format!(
                "<member><StackId>{id}</StackId><StackName>{name}</StackName><StackStatus>{status}</StackStatus><CreationTime>{created}</CreationTime></member>",
                id = xml_escape(&stack.id),
                name = xml_escape(&stack.name),
                status = stack.status,
                created = stack.created.to_rfc3339(),
            ));
        }
        Ok(wrap(
            "ListStacks",
            &format!("<StackSummaries>{members}</StackSummaries>"),
        ))
    }

    fn describe_stack_events(&self, params: &HashMap<String, String>) -> Result<String, AwsError> {
        let name = required(params, "StackName")?;
        let s = self.state.read();
        let stack = s.stacks.get(&name).ok_or_else(|| no_such_stack(&name))?;
        let mut members = String::new();
        for ev in stack.events.iter().rev() {
            members.push_str(&format!(
                "<member><EventId>{id}</EventId><StackId>{sid}</StackId><StackName>{sname}</StackName><Timestamp>{ts}</Timestamp><ResourceStatus>{rs}</ResourceStatus><ResourceType>{rt}</ResourceType><LogicalResourceId>{lr}</LogicalResourceId><PhysicalResourceId>{pr}</PhysicalResourceId></member>",
                id = xml_escape(&ev.id),
                sid = xml_escape(&stack.id),
                sname = xml_escape(&stack.name),
                ts = ev.timestamp.to_rfc3339(),
                rs = ev.resource_status,
                rt = xml_escape(&ev.resource_type),
                lr = xml_escape(&ev.logical_resource_id),
                pr = xml_escape(&ev.physical_resource_id),
            ));
        }
        Ok(wrap(
            "DescribeStackEvents",
            &format!("<StackEvents>{members}</StackEvents>"),
        ))
    }

    fn list_stack_resources(&self, params: &HashMap<String, String>) -> Result<String, AwsError> {
        let name = required(params, "StackName")?;
        let s = self.state.read();
        s.stacks.get(&name).ok_or_else(|| no_such_stack(&name))?;
        // kuroko doesn't parse the template into individual resources — the
        // stack itself is the only known resource.
        Ok(wrap(
            "ListStackResources",
            "<StackResourceSummaries></StackResourceSummaries>",
        ))
    }

    fn get_template(&self, params: &HashMap<String, String>) -> Result<String, AwsError> {
        let name = required(params, "StackName")?;
        let s = self.state.read();
        let stack = s.stacks.get(&name).ok_or_else(|| no_such_stack(&name))?;
        Ok(wrap(
            "GetTemplate",
            &format!(
                "<TemplateBody>{}</TemplateBody>",
                xml_escape(&stack.template)
            ),
        ))
    }
}

fn stack_id(name: &str) -> String {
    format!(
        "arn:aws:cloudformation:{EMULATED_REGION}:{EMULATED_ACCOUNT_ID}:stack/{name}/{id}",
        id = Uuid::new_v4()
    )
}

fn stack_xml(stack: &Stack) -> String {
    let mut params_xml = String::new();
    for (k, v) in &stack.parameters {
        params_xml.push_str(&format!(
            "<member><ParameterKey>{}</ParameterKey><ParameterValue>{}</ParameterValue></member>",
            xml_escape(k),
            xml_escape(v)
        ));
    }
    let mut tags_xml = String::new();
    for (k, v) in &stack.tags {
        tags_xml.push_str(&format!(
            "<member><Key>{}</Key><Value>{}</Value></member>",
            xml_escape(k),
            xml_escape(v)
        ));
    }
    let description = stack
        .description
        .as_deref()
        .map(|d| format!("<Description>{}</Description>", xml_escape(d)))
        .unwrap_or_default();
    let updated = stack
        .updated
        .map(|u| format!("<LastUpdatedTime>{}</LastUpdatedTime>", u.to_rfc3339()))
        .unwrap_or_default();
    format!(
        "<member><StackId>{id}</StackId><StackName>{name}</StackName>{description}<Parameters>{params_xml}</Parameters><Tags>{tags_xml}</Tags><StackStatus>{status}</StackStatus><CreationTime>{created}</CreationTime>{updated}</member>",
        id = xml_escape(&stack.id),
        name = xml_escape(&stack.name),
        status = stack.status,
        created = stack.created.to_rfc3339(),
    )
}

fn parse_pairs(
    params: &HashMap<String, String>,
    prefix: &str,
    key_field: &str,
    value_field: &str,
) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut i = 1;
    loop {
        let key = params.get(&format!("{prefix}.{i}.{key_field}"));
        let val = params.get(&format!("{prefix}.{i}.{value_field}"));
        match (key, val) {
            (Some(k), Some(v)) => out.push((k.clone(), v.clone())),
            _ => break,
        }
        i += 1;
    }
    out
}

fn collect_indexed(params: &HashMap<String, String>, prefix: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut i = 1;
    while let Some(v) = params.get(&format!("{prefix}.{i}")) {
        out.push(v.clone());
        i += 1;
    }
    out
}

fn required(p: &HashMap<String, String>, key: &str) -> Result<String, AwsError> {
    p.get(key)
        .cloned()
        .ok_or_else(|| AwsError::new("ValidationError", format!("{key} required")))
}

fn no_such_stack(name: &str) -> AwsError {
    AwsError::new(
        "ValidationError",
        format!("Stack with id {name} does not exist"),
    )
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
    registry.register_query(Arc::new(CloudFormation::new()));
}
