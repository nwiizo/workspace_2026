//! EC2 — AWS Query protocol, sdk_id `ec2`.
//!
//! Implements the most-used IaC provisioning surface: regions, AZs, VPC,
//! Subnet, SecurityGroup (with ingress rules), Instance lifecycle, plus
//! tag association. Instances transition to `running` immediately so test
//! pipelines don't poll for `pending → running`.

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

const SDK_ID: &str = "ec2";
const NS: &str = "http://ec2.amazonaws.com/doc/2016-11-15/";

const ACTIONS: &[&str] = &[
    "DescribeRegions",
    "DescribeAvailabilityZones",
    "CreateVpc",
    "DescribeVpcs",
    "DeleteVpc",
    "CreateSubnet",
    "DescribeSubnets",
    "DeleteSubnet",
    "CreateSecurityGroup",
    "DescribeSecurityGroups",
    "DeleteSecurityGroup",
    "AuthorizeSecurityGroupIngress",
    "RunInstances",
    "TerminateInstances",
    "DescribeInstances",
    "CreateTags",
];

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
struct State {
    vpcs: HashMap<String, Vpc>,
    subnets: HashMap<String, Subnet>,
    security_groups: HashMap<String, SecurityGroup>,
    instances: HashMap<String, Instance>,
    tags: HashMap<String, Vec<(String, String)>>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct Vpc {
    id: String,
    cidr_block: String,
    state: String,
    is_default: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct Subnet {
    id: String,
    vpc_id: String,
    cidr_block: String,
    availability_zone: String,
    state: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct SecurityGroup {
    id: String,
    name: String,
    description: String,
    vpc_id: Option<String>,
    ingress: Vec<IpPermission>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct IpPermission {
    ip_protocol: String,
    from_port: Option<i32>,
    to_port: Option<i32>,
    cidr_ranges: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct Instance {
    id: String,
    image_id: String,
    instance_type: String,
    state: String,
    subnet_id: Option<String>,
    private_ip: String,
    launched: chrono::DateTime<chrono::Utc>,
}

pub struct Ec2 {
    state: Arc<RwLock<State>>,
}

impl Ec2 {
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(State::default())),
        }
    }
}

impl Default for Ec2 {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Service for Ec2 {
    fn name(&self) -> &'static str {
        "ec2"
    }

    fn reset(&self) {
        *self.state.write() = State::default();
    }

    fn restore(&self, ctx: &ServiceContext) -> Result<(), AwsError> {
        if let Some(snap) = &ctx.snapshot
            && let Some(data) = snap.load::<State>("ec2").map_err(persistence_error)?
        {
            *self.state.write() = data;
        }
        Ok(())
    }

    fn snapshot(&self, ctx: &ServiceContext) -> Result<(), AwsError> {
        if let Some(snap) = &ctx.snapshot {
            let data = self.state.read();
            snap.save("ec2", &*data).map_err(persistence_error)?;
        }
        Ok(())
    }

    fn router(&self, _ctx: ServiceContext) -> Router {
        Router::new()
    }
}

#[async_trait]
impl QueryProtocolService for Ec2 {
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
            "DescribeRegions" => Ok(describe_regions_response()),
            "DescribeAvailabilityZones" => Ok(describe_azs_response()),
            "CreateVpc" => self.create_vpc(params),
            "DescribeVpcs" => self.describe_vpcs(params),
            "DeleteVpc" => self.delete_vpc(params),
            "CreateSubnet" => self.create_subnet(params),
            "DescribeSubnets" => self.describe_subnets(params),
            "DeleteSubnet" => self.delete_subnet(params),
            "CreateSecurityGroup" => self.create_security_group(params),
            "DescribeSecurityGroups" => self.describe_security_groups(params),
            "DeleteSecurityGroup" => self.delete_security_group(params),
            "AuthorizeSecurityGroupIngress" => self.authorize_ingress(params),
            "RunInstances" => self.run_instances(params),
            "TerminateInstances" => self.terminate_instances(params),
            "DescribeInstances" => self.describe_instances(params),
            "CreateTags" => self.create_tags(params),
            other => Err(AwsError::unsupported(format!("EC2::{other}"))),
        }
    }
}

impl Ec2 {
    fn create_vpc(&self, params: &HashMap<String, String>) -> Result<String, AwsError> {
        let cidr = required(params, "CidrBlock")?;
        let id = format!("vpc-{}", short_id());
        let vpc = Vpc {
            id: id.clone(),
            cidr_block: cidr,
            state: "available".into(),
            is_default: false,
        };
        let body = format!("<vpc>{}</vpc>", vpc_xml(&vpc));
        self.state.write().vpcs.insert(id, vpc);
        Ok(wrap("CreateVpc", &body))
    }

    fn describe_vpcs(&self, params: &HashMap<String, String>) -> Result<String, AwsError> {
        let filter_ids = collect_indexed(params, "VpcId");
        let s = self.state.read();
        let mut items = String::new();
        for v in s.vpcs.values() {
            if !filter_ids.is_empty() && !filter_ids.contains(&v.id) {
                continue;
            }
            items.push_str(&format!("<item>{}</item>", vpc_xml(v)));
        }
        Ok(wrap("DescribeVpcs", &format!("<vpcSet>{items}</vpcSet>")))
    }

    fn delete_vpc(&self, params: &HashMap<String, String>) -> Result<String, AwsError> {
        let id = required(params, "VpcId")?;
        self.state.write().vpcs.remove(&id);
        Ok(success_return("DeleteVpc"))
    }

    fn create_subnet(&self, params: &HashMap<String, String>) -> Result<String, AwsError> {
        let vpc_id = required(params, "VpcId")?;
        let cidr = required(params, "CidrBlock")?;
        let az = params
            .get("AvailabilityZone")
            .cloned()
            .unwrap_or_else(|| format!("{EMULATED_REGION}a"));
        let s_state = self.state.read();
        if !s_state.vpcs.contains_key(&vpc_id) {
            return Err(AwsError::new(
                "InvalidVpcID.NotFound",
                format!("vpc '{vpc_id}' not found"),
            ));
        }
        drop(s_state);
        let id = format!("subnet-{}", short_id());
        let subnet = Subnet {
            id: id.clone(),
            vpc_id,
            cidr_block: cidr,
            availability_zone: az,
            state: "available".into(),
        };
        let body = format!("<subnet>{}</subnet>", subnet_xml(&subnet));
        self.state.write().subnets.insert(id, subnet);
        Ok(wrap("CreateSubnet", &body))
    }

    fn describe_subnets(&self, params: &HashMap<String, String>) -> Result<String, AwsError> {
        let filter_ids = collect_indexed(params, "SubnetId");
        let s = self.state.read();
        let mut items = String::new();
        for v in s.subnets.values() {
            if !filter_ids.is_empty() && !filter_ids.contains(&v.id) {
                continue;
            }
            items.push_str(&format!("<item>{}</item>", subnet_xml(v)));
        }
        Ok(wrap(
            "DescribeSubnets",
            &format!("<subnetSet>{items}</subnetSet>"),
        ))
    }

    fn delete_subnet(&self, params: &HashMap<String, String>) -> Result<String, AwsError> {
        let id = required(params, "SubnetId")?;
        self.state.write().subnets.remove(&id);
        Ok(success_return("DeleteSubnet"))
    }

    fn create_security_group(&self, params: &HashMap<String, String>) -> Result<String, AwsError> {
        let name = required(params, "GroupName")?;
        let description = required(params, "GroupDescription")?;
        let vpc_id = params.get("VpcId").cloned();
        let id = format!("sg-{}", short_id());
        let sg = SecurityGroup {
            id: id.clone(),
            name,
            description,
            vpc_id,
            ingress: Vec::new(),
        };
        let body = format!("<groupId>{}</groupId>", xml_escape(&id));
        self.state.write().security_groups.insert(id, sg);
        Ok(wrap("CreateSecurityGroup", &body))
    }

    fn describe_security_groups(
        &self,
        params: &HashMap<String, String>,
    ) -> Result<String, AwsError> {
        let filter_ids = collect_indexed(params, "GroupId");
        let s = self.state.read();
        let mut items = String::new();
        for sg in s.security_groups.values() {
            if !filter_ids.is_empty() && !filter_ids.contains(&sg.id) {
                continue;
            }
            items.push_str(&format!("<item>{}</item>", sg_xml(sg)));
        }
        Ok(wrap(
            "DescribeSecurityGroups",
            &format!("<securityGroupInfo>{items}</securityGroupInfo>"),
        ))
    }

    fn delete_security_group(&self, params: &HashMap<String, String>) -> Result<String, AwsError> {
        if let Some(id) = params.get("GroupId") {
            self.state.write().security_groups.remove(id);
        } else if let Some(name) = params.get("GroupName") {
            let mut s = self.state.write();
            let target = s
                .security_groups
                .iter()
                .find(|(_, sg)| sg.name == *name)
                .map(|(k, _)| k.clone());
            if let Some(k) = target {
                s.security_groups.remove(&k);
            }
        }
        Ok(success_return("DeleteSecurityGroup"))
    }

    fn authorize_ingress(&self, params: &HashMap<String, String>) -> Result<String, AwsError> {
        let group_id = required(params, "GroupId")?;
        let mut s = self.state.write();
        let sg = s.security_groups.get_mut(&group_id).ok_or_else(|| {
            AwsError::new(
                "InvalidGroup.NotFound",
                format!("security group '{group_id}' not found"),
            )
        })?;
        // Two input shapes: legacy flat (CidrIp, IpProtocol, FromPort, ToPort)
        // and modern IpPermissions.N.IpProtocol etc. Accept both.
        if let Some(proto) = params.get("IpProtocol") {
            let perm = IpPermission {
                ip_protocol: proto.clone(),
                from_port: params.get("FromPort").and_then(|p| p.parse().ok()),
                to_port: params.get("ToPort").and_then(|p| p.parse().ok()),
                cidr_ranges: params
                    .get("CidrIp")
                    .cloned()
                    .map(|c| vec![c])
                    .unwrap_or_default(),
            };
            sg.ingress.push(perm);
        }
        let mut i = 1;
        loop {
            let proto_key = format!("IpPermissions.{i}.IpProtocol");
            let Some(proto) = params.get(&proto_key).cloned() else {
                break;
            };
            let from_port = params
                .get(&format!("IpPermissions.{i}.FromPort"))
                .and_then(|p| p.parse().ok());
            let to_port = params
                .get(&format!("IpPermissions.{i}.ToPort"))
                .and_then(|p| p.parse().ok());
            let mut cidrs = Vec::new();
            let mut j = 1;
            loop {
                let cidr_key = format!("IpPermissions.{i}.IpRanges.{j}.CidrIp");
                let Some(c) = params.get(&cidr_key).cloned() else {
                    break;
                };
                cidrs.push(c);
                j += 1;
            }
            sg.ingress.push(IpPermission {
                ip_protocol: proto,
                from_port,
                to_port,
                cidr_ranges: cidrs,
            });
            i += 1;
        }
        Ok(success_return("AuthorizeSecurityGroupIngress"))
    }

    fn run_instances(&self, params: &HashMap<String, String>) -> Result<String, AwsError> {
        let image_id = required(params, "ImageId")?;
        let instance_type = params
            .get("InstanceType")
            .cloned()
            .unwrap_or_else(|| "t3.micro".into());
        let min: usize = params
            .get("MinCount")
            .and_then(|v| v.parse().ok())
            .unwrap_or(1);
        let max: usize = params
            .get("MaxCount")
            .and_then(|v| v.parse().ok())
            .unwrap_or(min);
        let count = max.max(min);
        let subnet_id = params.get("SubnetId").cloned();

        let mut s = self.state.write();
        let mut items = String::new();
        for _ in 0..count {
            let id = format!("i-{}", short_id());
            let inst = Instance {
                id: id.clone(),
                image_id: image_id.clone(),
                instance_type: instance_type.clone(),
                state: "running".into(),
                subnet_id: subnet_id.clone(),
                private_ip: format!("10.0.0.{}", (rand::random::<u8>().max(2))),
                launched: chrono::Utc::now(),
            };
            items.push_str(&format!("<item>{}</item>", instance_xml(&inst)));
            s.instances.insert(id, inst);
        }
        let body = format!(
            "<reservationId>r-{rid}</reservationId><ownerId>{EMULATED_ACCOUNT_ID}</ownerId><instancesSet>{items}</instancesSet>",
            rid = short_id(),
        );
        Ok(wrap("RunInstances", &body))
    }

    fn terminate_instances(&self, params: &HashMap<String, String>) -> Result<String, AwsError> {
        let ids = collect_indexed(params, "InstanceId");
        let mut s = self.state.write();
        let mut items = String::new();
        for id in ids {
            if let Some(inst) = s.instances.get_mut(&id) {
                let previous = inst.state.clone();
                inst.state = "terminated".into();
                items.push_str(&format!(
                    "<item><instanceId>{id}</instanceId><currentState><name>terminated</name><code>48</code></currentState><previousState><name>{prev}</name><code>16</code></previousState></item>",
                    id = xml_escape(&id),
                    prev = xml_escape(&previous),
                ));
            }
        }
        Ok(wrap(
            "TerminateInstances",
            &format!("<instancesSet>{items}</instancesSet>"),
        ))
    }

    fn describe_instances(&self, params: &HashMap<String, String>) -> Result<String, AwsError> {
        let filter_ids = collect_indexed(params, "InstanceId");
        let s = self.state.read();
        let mut instances_xml = String::new();
        for inst in s.instances.values() {
            if !filter_ids.is_empty() && !filter_ids.contains(&inst.id) {
                continue;
            }
            instances_xml.push_str(&format!("<item>{}</item>", instance_xml(inst)));
        }
        // One reservation wrapping every visible instance is good enough.
        let reservations = if instances_xml.is_empty() {
            String::new()
        } else {
            format!(
                "<item><reservationId>r-{rid}</reservationId><ownerId>{EMULATED_ACCOUNT_ID}</ownerId><instancesSet>{instances_xml}</instancesSet></item>",
                rid = short_id(),
            )
        };
        Ok(wrap(
            "DescribeInstances",
            &format!("<reservationSet>{reservations}</reservationSet>"),
        ))
    }

    fn create_tags(&self, params: &HashMap<String, String>) -> Result<String, AwsError> {
        let resource_ids = collect_indexed(params, "ResourceId");
        let mut tags: Vec<(String, String)> = Vec::new();
        let mut i = 1;
        loop {
            let key = params.get(&format!("Tag.{i}.Key"));
            let value = params.get(&format!("Tag.{i}.Value"));
            match (key, value) {
                (Some(k), Some(v)) => tags.push((k.clone(), v.clone())),
                _ => break,
            }
            i += 1;
        }
        let mut s = self.state.write();
        for id in resource_ids {
            s.tags.entry(id).or_default().extend(tags.clone());
        }
        Ok(success_return("CreateTags"))
    }
}

fn describe_regions_response() -> String {
    let regions = [
        "us-east-1",
        "us-east-2",
        "us-west-1",
        "us-west-2",
        "eu-west-1",
        "eu-central-1",
        "ap-northeast-1",
        "ap-southeast-1",
        "ap-southeast-2",
    ];
    let items: String = regions
        .iter()
        .map(|r| {
            format!(
                "<item><regionName>{}</regionName><regionEndpoint>ec2.{}.amazonaws.com</regionEndpoint><optInStatus>opt-in-not-required</optInStatus></item>",
                r, r
            )
        })
        .collect();
    wrap(
        "DescribeRegions",
        &format!("<regionInfo>{items}</regionInfo>"),
    )
}

fn describe_azs_response() -> String {
    let zones = [
        format!("{EMULATED_REGION}a"),
        format!("{EMULATED_REGION}b"),
        format!("{EMULATED_REGION}c"),
    ];
    let items: String = zones
        .iter()
        .map(|z| {
            format!(
                "<item><zoneName>{z}</zoneName><state>available</state><regionName>{EMULATED_REGION}</regionName><zoneId>use1-az1</zoneId></item>"
            )
        })
        .collect();
    wrap(
        "DescribeAvailabilityZones",
        &format!("<availabilityZoneInfo>{items}</availabilityZoneInfo>"),
    )
}

fn vpc_xml(v: &Vpc) -> String {
    format!(
        "<vpcId>{id}</vpcId><state>{state}</state><cidrBlock>{cidr}</cidrBlock><isDefault>{is_default}</isDefault>",
        id = xml_escape(&v.id),
        state = xml_escape(&v.state),
        cidr = xml_escape(&v.cidr_block),
        is_default = v.is_default,
    )
}

fn subnet_xml(s: &Subnet) -> String {
    format!(
        "<subnetId>{id}</subnetId><vpcId>{vpc}</vpcId><cidrBlock>{cidr}</cidrBlock><availabilityZone>{az}</availabilityZone><state>{state}</state>",
        id = xml_escape(&s.id),
        vpc = xml_escape(&s.vpc_id),
        cidr = xml_escape(&s.cidr_block),
        az = xml_escape(&s.availability_zone),
        state = xml_escape(&s.state),
    )
}

fn sg_xml(sg: &SecurityGroup) -> String {
    let mut perms = String::new();
    for p in &sg.ingress {
        let mut ranges = String::new();
        for c in &p.cidr_ranges {
            ranges.push_str(&format!("<item><cidrIp>{}</cidrIp></item>", xml_escape(c)));
        }
        perms.push_str(&format!(
            "<item><ipProtocol>{proto}</ipProtocol><fromPort>{from}</fromPort><toPort>{to}</toPort><ipRanges>{ranges}</ipRanges></item>",
            proto = xml_escape(&p.ip_protocol),
            from = p.from_port.unwrap_or_default(),
            to = p.to_port.unwrap_or_default(),
        ));
    }
    let vpc = sg
        .vpc_id
        .as_deref()
        .map(|v| format!("<vpcId>{}</vpcId>", xml_escape(v)))
        .unwrap_or_default();
    format!(
        "<groupId>{id}</groupId><groupName>{name}</groupName><groupDescription>{desc}</groupDescription>{vpc}<ipPermissions>{perms}</ipPermissions><ownerId>{EMULATED_ACCOUNT_ID}</ownerId>",
        id = xml_escape(&sg.id),
        name = xml_escape(&sg.name),
        desc = xml_escape(&sg.description),
    )
}

fn instance_xml(i: &Instance) -> String {
    let subnet = i
        .subnet_id
        .as_deref()
        .map(|s| format!("<subnetId>{}</subnetId>", xml_escape(s)))
        .unwrap_or_default();
    let state_code = match i.state.as_str() {
        "pending" => 0,
        "running" => 16,
        "shutting-down" => 32,
        "terminated" => 48,
        "stopping" => 64,
        "stopped" => 80,
        _ => 16,
    };
    format!(
        "<instanceId>{id}</instanceId><imageId>{img}</imageId><instanceType>{itype}</instanceType><instanceState><name>{state}</name><code>{code}</code></instanceState>{subnet}<privateIpAddress>{ip}</privateIpAddress><launchTime>{ts}</launchTime>",
        id = xml_escape(&i.id),
        img = xml_escape(&i.image_id),
        itype = xml_escape(&i.instance_type),
        state = xml_escape(&i.state),
        code = state_code,
        ip = xml_escape(&i.private_ip),
        ts = i.launched.to_rfc3339(),
    )
}

fn collect_indexed(params: &HashMap<String, String>, prefix: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut i = 1;
    loop {
        // AWS Query uses two indexing conventions for EC2: the older
        // `<Prefix>.<N>` and the newer `<Prefix>.member.<N>` (introduced
        // in some SDKs). Try both.
        let v = params
            .get(&format!("{prefix}.{i}"))
            .or_else(|| params.get(&format!("{prefix}.member.{i}")));
        let Some(v) = v else { break };
        out.push(v.clone());
        i += 1;
    }
    out
}

fn required(p: &HashMap<String, String>, key: &str) -> Result<String, AwsError> {
    p.get(key)
        .cloned()
        .ok_or_else(|| AwsError::new("MissingParameter", format!("{key} required")))
}

fn short_id() -> String {
    Uuid::new_v4().simple().to_string()[..10].to_string()
}

fn wrap(action: &str, body: &str) -> String {
    let rid = Uuid::new_v4();
    format!(
        "<{action}Response xmlns=\"{NS}\"><requestId>{rid}</requestId>{body}</{action}Response>"
    )
}

fn success_return(action: &str) -> String {
    let rid = Uuid::new_v4();
    format!(
        "<{action}Response xmlns=\"{NS}\"><requestId>{rid}</requestId><return>true</return></{action}Response>"
    )
}

pub fn register(registry: &Arc<Registry>) {
    registry.register_query(Arc::new(Ec2::new()));
}
