//! ELBv2 (Application/Network Load Balancer) — AWS Query protocol.
//!
//! sdk_id `elasticloadbalancing`. Covers LB / target group / listener
//! lifecycle plus target registration and health reporting. Health is
//! reported as `healthy` for every registered target (kuroko does no actual
//! probing).

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

const SDK_ID: &str = "elasticloadbalancing";
const NS: &str = "http://elasticloadbalancing.amazonaws.com/doc/2015-12-01/";

const ACTIONS: &[&str] = &[
    "CreateLoadBalancer",
    "DescribeLoadBalancers",
    "DeleteLoadBalancer",
    "CreateTargetGroup",
    "DescribeTargetGroups",
    "DeleteTargetGroup",
    "RegisterTargets",
    "DeregisterTargets",
    "DescribeTargetHealth",
    "CreateListener",
    "DescribeListeners",
    "DeleteListener",
];

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
struct State {
    load_balancers: HashMap<String, LoadBalancer>,
    target_groups: HashMap<String, TargetGroup>,
    listeners: HashMap<String, Listener>,
    /// target_group_arn → list of (id, port)
    targets: HashMap<String, Vec<Target>>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct LoadBalancer {
    name: String,
    arn: String,
    dns_name: String,
    scheme: String,
    type_: String,
    state: String,
    created: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct TargetGroup {
    name: String,
    arn: String,
    protocol: String,
    port: i32,
    vpc_id: Option<String>,
    target_type: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct Listener {
    arn: String,
    lb_arn: String,
    protocol: String,
    port: i32,
    default_target_group: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct Target {
    id: String,
    port: i32,
}

pub struct Elbv2 {
    state: Arc<RwLock<State>>,
}

impl Elbv2 {
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(State::default())),
        }
    }
}

impl Default for Elbv2 {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Service for Elbv2 {
    fn name(&self) -> &'static str {
        "elbv2"
    }

    fn reset(&self) {
        *self.state.write() = State::default();
    }

    fn restore(&self, ctx: &ServiceContext) -> Result<(), AwsError> {
        if let Some(snap) = &ctx.snapshot
            && let Some(data) = snap.load::<State>("elbv2").map_err(persistence_error)?
        {
            *self.state.write() = data;
        }
        Ok(())
    }

    fn snapshot(&self, ctx: &ServiceContext) -> Result<(), AwsError> {
        if let Some(snap) = &ctx.snapshot {
            let data = self.state.read();
            snap.save("elbv2", &*data).map_err(persistence_error)?;
        }
        Ok(())
    }

    fn router(&self, _ctx: ServiceContext) -> Router {
        Router::new()
    }
}

#[async_trait]
impl QueryProtocolService for Elbv2 {
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
            "CreateLoadBalancer" => self.create_load_balancer(params),
            "DescribeLoadBalancers" => self.describe_load_balancers(params),
            "DeleteLoadBalancer" => self.delete_load_balancer(params),
            "CreateTargetGroup" => self.create_target_group(params),
            "DescribeTargetGroups" => self.describe_target_groups(params),
            "DeleteTargetGroup" => self.delete_target_group(params),
            "RegisterTargets" => self.register_targets(params),
            "DeregisterTargets" => self.deregister_targets(params),
            "DescribeTargetHealth" => self.describe_target_health(params),
            "CreateListener" => self.create_listener(params),
            "DescribeListeners" => self.describe_listeners(params),
            "DeleteListener" => self.delete_listener(params),
            other => Err(AwsError::unsupported(format!("ELBv2::{other}"))),
        }
    }
}

impl Elbv2 {
    fn create_load_balancer(&self, params: &HashMap<String, String>) -> Result<String, AwsError> {
        let name = required(params, "Name")?;
        let scheme = params
            .get("Scheme")
            .cloned()
            .unwrap_or_else(|| "internet-facing".into());
        let type_ = params
            .get("Type")
            .cloned()
            .unwrap_or_else(|| "application".into());
        let mut s = self.state.write();
        if s.load_balancers.contains_key(&name) {
            return Err(AwsError::new(
                "DuplicateLoadBalancerNameException",
                format!("load balancer '{name}' already exists"),
            ));
        }
        let lb_id = short_hash();
        let arn = format!(
            "arn:aws:elasticloadbalancing:{EMULATED_REGION}:{EMULATED_ACCOUNT_ID}:loadbalancer/{type_}/{name}/{lb_id}"
        );
        let lb = LoadBalancer {
            name: name.clone(),
            arn,
            dns_name: format!("{name}-{lb_id}.{EMULATED_REGION}.elb.amazonaws.com"),
            scheme,
            type_,
            state: "active".into(),
            created: chrono::Utc::now(),
        };
        let body = format!("<LoadBalancers>{}</LoadBalancers>", lb_xml(&lb));
        s.load_balancers.insert(name, lb);
        Ok(wrap("CreateLoadBalancer", &body))
    }

    fn describe_load_balancers(
        &self,
        params: &HashMap<String, String>,
    ) -> Result<String, AwsError> {
        let filter_names = collect_indexed(params, "Names.member");
        let s = self.state.read();
        let mut members = String::new();
        for lb in s.load_balancers.values() {
            if !filter_names.is_empty() && !filter_names.contains(&lb.name) {
                continue;
            }
            members.push_str(&lb_xml(lb));
        }
        Ok(wrap(
            "DescribeLoadBalancers",
            &format!("<LoadBalancers>{members}</LoadBalancers>"),
        ))
    }

    fn delete_load_balancer(&self, params: &HashMap<String, String>) -> Result<String, AwsError> {
        let arn = required(params, "LoadBalancerArn")?;
        let mut s = self.state.write();
        let name = s
            .load_balancers
            .iter()
            .find(|(_, lb)| lb.arn == arn)
            .map(|(n, _)| n.clone())
            .ok_or_else(|| no_such_load_balancer(&arn))?;
        s.load_balancers.remove(&name);
        s.listeners.retain(|_, l| l.lb_arn != arn);
        Ok(empty("DeleteLoadBalancer"))
    }

    fn create_target_group(&self, params: &HashMap<String, String>) -> Result<String, AwsError> {
        let name = required(params, "Name")?;
        let protocol = params
            .get("Protocol")
            .cloned()
            .unwrap_or_else(|| "HTTP".into());
        let port: i32 = params
            .get("Port")
            .and_then(|p| p.parse().ok())
            .unwrap_or(80);
        let target_type = params
            .get("TargetType")
            .cloned()
            .unwrap_or_else(|| "instance".into());
        let vpc_id = params.get("VpcId").cloned();
        let mut s = self.state.write();
        if s.target_groups.contains_key(&name) {
            return Err(AwsError::new(
                "DuplicateTargetGroupNameException",
                format!("target group '{name}' already exists"),
            ));
        }
        let tg_id = short_hash();
        let arn = format!(
            "arn:aws:elasticloadbalancing:{EMULATED_REGION}:{EMULATED_ACCOUNT_ID}:targetgroup/{name}/{tg_id}"
        );
        let tg = TargetGroup {
            name: name.clone(),
            arn,
            protocol,
            port,
            vpc_id,
            target_type,
        };
        let body = format!("<TargetGroups>{}</TargetGroups>", tg_xml(&tg));
        s.target_groups.insert(name, tg);
        Ok(wrap("CreateTargetGroup", &body))
    }

    fn describe_target_groups(&self, params: &HashMap<String, String>) -> Result<String, AwsError> {
        let filter_names = collect_indexed(params, "Names.member");
        let filter_arns = collect_indexed(params, "TargetGroupArns.member");
        let s = self.state.read();
        let mut members = String::new();
        for tg in s.target_groups.values() {
            if !filter_names.is_empty() && !filter_names.contains(&tg.name) {
                continue;
            }
            if !filter_arns.is_empty() && !filter_arns.contains(&tg.arn) {
                continue;
            }
            members.push_str(&tg_xml(tg));
        }
        Ok(wrap(
            "DescribeTargetGroups",
            &format!("<TargetGroups>{members}</TargetGroups>"),
        ))
    }

    fn delete_target_group(&self, params: &HashMap<String, String>) -> Result<String, AwsError> {
        let arn = required(params, "TargetGroupArn")?;
        let mut s = self.state.write();
        let name = s
            .target_groups
            .iter()
            .find(|(_, tg)| tg.arn == arn)
            .map(|(n, _)| n.clone())
            .ok_or_else(|| no_such_target_group(&arn))?;
        s.target_groups.remove(&name);
        s.targets.remove(&arn);
        Ok(empty("DeleteTargetGroup"))
    }

    fn register_targets(&self, params: &HashMap<String, String>) -> Result<String, AwsError> {
        let arn = required(params, "TargetGroupArn")?;
        let targets = parse_targets(params);
        let mut s = self.state.write();
        if !s.target_groups.values().any(|tg| tg.arn == arn) {
            return Err(no_such_target_group(&arn));
        }
        let list = s.targets.entry(arn).or_default();
        for t in targets {
            if !list.iter().any(|x| x.id == t.id && x.port == t.port) {
                list.push(t);
            }
        }
        Ok(empty("RegisterTargets"))
    }

    fn deregister_targets(&self, params: &HashMap<String, String>) -> Result<String, AwsError> {
        let arn = required(params, "TargetGroupArn")?;
        let to_remove = parse_targets(params);
        let mut s = self.state.write();
        if let Some(list) = s.targets.get_mut(&arn) {
            list.retain(|t| !to_remove.iter().any(|r| r.id == t.id && r.port == t.port));
        }
        Ok(empty("DeregisterTargets"))
    }

    fn describe_target_health(&self, params: &HashMap<String, String>) -> Result<String, AwsError> {
        let arn = required(params, "TargetGroupArn")?;
        let s = self.state.read();
        let list = s.targets.get(&arn).cloned().unwrap_or_default();
        let mut members = String::new();
        for t in list {
            members.push_str(&format!(
                "<member><Target><Id>{id}</Id><Port>{port}</Port></Target><TargetHealth><State>healthy</State></TargetHealth></member>",
                id = xml_escape(&t.id),
                port = t.port,
            ));
        }
        Ok(wrap(
            "DescribeTargetHealth",
            &format!("<TargetHealthDescriptions>{members}</TargetHealthDescriptions>"),
        ))
    }

    fn create_listener(&self, params: &HashMap<String, String>) -> Result<String, AwsError> {
        let lb_arn = required(params, "LoadBalancerArn")?;
        let protocol = required(params, "Protocol")?;
        let port: i32 = required(params, "Port")?
            .parse()
            .map_err(|_| AwsError::new("ValidationError", "Port must be an integer"))?;
        let default_tg = params
            .get("DefaultActions.member.1.TargetGroupArn")
            .cloned();
        let mut s = self.state.write();
        if !s.load_balancers.values().any(|lb| lb.arn == lb_arn) {
            return Err(no_such_load_balancer(&lb_arn));
        }
        let listener_id = short_hash();
        let arn = format!("{lb_arn}/listener/app/{listener_id}");
        let listener = Listener {
            arn: arn.clone(),
            lb_arn,
            protocol,
            port,
            default_target_group: default_tg,
        };
        let body = format!("<Listeners>{}</Listeners>", listener_xml(&listener));
        s.listeners.insert(arn, listener);
        Ok(wrap("CreateListener", &body))
    }

    fn describe_listeners(&self, params: &HashMap<String, String>) -> Result<String, AwsError> {
        let lb_arn = params.get("LoadBalancerArn").cloned();
        let s = self.state.read();
        let mut members = String::new();
        for l in s.listeners.values() {
            if let Some(want) = &lb_arn
                && &l.lb_arn != want
            {
                continue;
            }
            members.push_str(&listener_xml(l));
        }
        Ok(wrap(
            "DescribeListeners",
            &format!("<Listeners>{members}</Listeners>"),
        ))
    }

    fn delete_listener(&self, params: &HashMap<String, String>) -> Result<String, AwsError> {
        let arn = required(params, "ListenerArn")?;
        self.state.write().listeners.remove(&arn);
        Ok(empty("DeleteListener"))
    }
}

fn parse_targets(params: &HashMap<String, String>) -> Vec<Target> {
    let mut out = Vec::new();
    let mut i = 1;
    loop {
        let id_key = format!("Targets.member.{i}.Id");
        let port_key = format!("Targets.member.{i}.Port");
        let Some(id) = params.get(&id_key).cloned() else {
            break;
        };
        let port = params
            .get(&port_key)
            .and_then(|p| p.parse().ok())
            .unwrap_or(80);
        out.push(Target { id, port });
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

fn required(p: &HashMap<String, String>, key: &str) -> Result<String, AwsError> {
    p.get(key)
        .cloned()
        .ok_or_else(|| AwsError::new("ValidationError", format!("{key} required")))
}

fn no_such_load_balancer(arn: &str) -> AwsError {
    AwsError::new(
        "LoadBalancerNotFoundException",
        format!("load balancer '{arn}' not found"),
    )
}

fn no_such_target_group(arn: &str) -> AwsError {
    AwsError::new(
        "TargetGroupNotFoundException",
        format!("target group '{arn}' not found"),
    )
}

fn short_hash() -> String {
    Uuid::new_v4().simple().to_string()[..16].to_string()
}

fn lb_xml(lb: &LoadBalancer) -> String {
    format!(
        "<member><LoadBalancerArn>{arn}</LoadBalancerArn><DNSName>{dns}</DNSName><LoadBalancerName>{name}</LoadBalancerName><Scheme>{scheme}</Scheme><Type>{type_}</Type><State><Code>{state}</Code></State><CreatedTime>{ts}</CreatedTime></member>",
        arn = xml_escape(&lb.arn),
        dns = xml_escape(&lb.dns_name),
        name = xml_escape(&lb.name),
        scheme = xml_escape(&lb.scheme),
        type_ = xml_escape(&lb.type_),
        state = xml_escape(&lb.state),
        ts = lb.created.to_rfc3339(),
    )
}

fn tg_xml(tg: &TargetGroup) -> String {
    let vpc = tg
        .vpc_id
        .as_deref()
        .map(|v| format!("<VpcId>{}</VpcId>", xml_escape(v)))
        .unwrap_or_default();
    format!(
        "<member><TargetGroupArn>{arn}</TargetGroupArn><TargetGroupName>{name}</TargetGroupName><Protocol>{proto}</Protocol><Port>{port}</Port>{vpc}<TargetType>{tt}</TargetType></member>",
        arn = xml_escape(&tg.arn),
        name = xml_escape(&tg.name),
        proto = xml_escape(&tg.protocol),
        port = tg.port,
        tt = xml_escape(&tg.target_type),
    )
}

fn listener_xml(l: &Listener) -> String {
    let action = l
        .default_target_group
        .as_deref()
        .map(|arn| {
            format!(
                "<DefaultActions><member><Type>forward</Type><TargetGroupArn>{}</TargetGroupArn></member></DefaultActions>",
                xml_escape(arn)
            )
        })
        .unwrap_or_default();
    format!(
        "<member><ListenerArn>{arn}</ListenerArn><LoadBalancerArn>{lb}</LoadBalancerArn><Protocol>{proto}</Protocol><Port>{port}</Port>{action}</member>",
        arn = xml_escape(&l.arn),
        lb = xml_escape(&l.lb_arn),
        proto = xml_escape(&l.protocol),
        port = l.port,
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
    registry.register_query(Arc::new(Elbv2::new()));
}
