//! IAM — AWS Query protocol with XML responses, sdk_id `iam`.
//!
//! Covers user / role / policy / access key lifecycle for the common CI
//! provisioning paths. Policy *evaluation* is out of scope — IAM in kuroko
//! is a metadata store, not an enforcement engine.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use axum::Router;
use parking_lot::RwLock;
use uuid::Uuid;

use crate::aws_error::{AwsError, xml_escape};
use crate::registry::Registry;
use crate::service::{
    EMULATED_ACCOUNT_ID, QueryProtocolService, Service, ServiceContext, persistence_error,
};

const SDK_ID: &str = "iam";
const NS: &str = "https://iam.amazonaws.com/doc/2010-05-08/";

const ACTIONS: &[&str] = &[
    "CreateUser",
    "GetUser",
    "ListUsers",
    "DeleteUser",
    "CreateRole",
    "GetRole",
    "ListRoles",
    "DeleteRole",
    "AttachRolePolicy",
    "DetachRolePolicy",
    "ListAttachedRolePolicies",
    "CreatePolicy",
    "GetPolicy",
    "ListPolicies",
    "DeletePolicy",
    "CreateAccessKey",
    "ListAccessKeys",
    "DeleteAccessKey",
];

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
struct State {
    users: HashMap<String, User>,
    roles: HashMap<String, Role>,
    policies: HashMap<String, Policy>,
    role_policies: HashMap<String, Vec<String>>,
    access_keys: HashMap<String, AccessKey>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct User {
    name: String,
    path: String,
    arn: String,
    id: String,
    created: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct Role {
    name: String,
    path: String,
    arn: String,
    id: String,
    assume_role_policy: String,
    description: Option<String>,
    created: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct Policy {
    name: String,
    arn: String,
    id: String,
    document: String,
    description: Option<String>,
    created: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct AccessKey {
    user_name: String,
    access_key_id: String,
    secret: String,
    status: String,
    created: chrono::DateTime<chrono::Utc>,
}

pub struct Iam {
    state: Arc<RwLock<State>>,
}

impl Iam {
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(State::default())),
        }
    }
}

impl Default for Iam {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Service for Iam {
    fn name(&self) -> &'static str {
        "iam"
    }

    fn reset(&self) {
        *self.state.write() = State::default();
    }

    fn restore(&self, ctx: &ServiceContext) -> Result<(), AwsError> {
        if let Some(snap) = &ctx.snapshot
            && let Some(data) = snap.load::<State>("iam").map_err(persistence_error)?
        {
            *self.state.write() = data;
        }
        Ok(())
    }

    fn snapshot(&self, ctx: &ServiceContext) -> Result<(), AwsError> {
        if let Some(snap) = &ctx.snapshot {
            let data = self.state.read();
            snap.save("iam", &*data).map_err(persistence_error)?;
        }
        Ok(())
    }

    fn router(&self, _ctx: ServiceContext) -> Router {
        Router::new()
    }
}

#[async_trait]
impl QueryProtocolService for Iam {
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
            "CreateUser" => self.create_user(params),
            "GetUser" => self.get_user(params),
            "ListUsers" => self.list_users(),
            "DeleteUser" => self.delete_user(params),
            "CreateRole" => self.create_role(params),
            "GetRole" => self.get_role(params),
            "ListRoles" => self.list_roles(),
            "DeleteRole" => self.delete_role(params),
            "AttachRolePolicy" => self.attach_role_policy(params),
            "DetachRolePolicy" => self.detach_role_policy(params),
            "ListAttachedRolePolicies" => self.list_attached_role_policies(params),
            "CreatePolicy" => self.create_policy(params),
            "GetPolicy" => self.get_policy(params),
            "ListPolicies" => self.list_policies(),
            "DeletePolicy" => self.delete_policy(params),
            "CreateAccessKey" => self.create_access_key(params),
            "ListAccessKeys" => self.list_access_keys(params),
            "DeleteAccessKey" => self.delete_access_key(params),
            other => Err(AwsError::unsupported(format!("IAM::{other}"))),
        }
    }
}

impl Iam {
    fn create_user(&self, params: &HashMap<String, String>) -> Result<String, AwsError> {
        let name = required(params, "UserName")?;
        let path = params.get("Path").cloned().unwrap_or_else(|| "/".into());
        let mut s = self.state.write();
        if s.users.contains_key(&name) {
            return Err(already_exists("user", &name));
        }
        let user = User {
            id: format!("AIDA{}", short_id()),
            arn: format!("arn:aws:iam::{EMULATED_ACCOUNT_ID}:user{path}{name}"),
            name: name.clone(),
            path,
            created: chrono::Utc::now(),
        };
        let body = format!("<User>{}</User>", user_inner(&user));
        s.users.insert(name, user);
        Ok(wrap("CreateUser", &body))
    }

    fn get_user(&self, params: &HashMap<String, String>) -> Result<String, AwsError> {
        let name = params
            .get("UserName")
            .cloned()
            .unwrap_or_else(|| "kuroko".to_string());
        let s = self.state.read();
        let user = s.users.get(&name).ok_or_else(|| no_such_entity(&name))?;
        Ok(wrap(
            "GetUser",
            &format!("<User>{}</User>", user_inner(user)),
        ))
    }

    fn list_users(&self) -> Result<String, AwsError> {
        let s = self.state.read();
        let mut members = String::new();
        for u in s.users.values() {
            members.push_str(&format!("<member>{}</member>", user_inner(u)));
        }
        Ok(wrap(
            "ListUsers",
            &format!("<Users>{members}</Users><IsTruncated>false</IsTruncated>"),
        ))
    }

    fn delete_user(&self, params: &HashMap<String, String>) -> Result<String, AwsError> {
        let name = required(params, "UserName")?;
        let mut s = self.state.write();
        s.users.remove(&name).ok_or_else(|| no_such_entity(&name))?;
        s.access_keys.retain(|_, ak| ak.user_name != name);
        Ok(empty("DeleteUser"))
    }

    fn create_role(&self, params: &HashMap<String, String>) -> Result<String, AwsError> {
        let name = required(params, "RoleName")?;
        let path = params.get("Path").cloned().unwrap_or_else(|| "/".into());
        let assume_role_policy = required(params, "AssumeRolePolicyDocument")?;
        let description = params.get("Description").cloned();
        let mut s = self.state.write();
        if s.roles.contains_key(&name) {
            return Err(already_exists("role", &name));
        }
        let role = Role {
            id: format!("AROA{}", short_id()),
            arn: format!("arn:aws:iam::{EMULATED_ACCOUNT_ID}:role{path}{name}"),
            name: name.clone(),
            path,
            assume_role_policy,
            description,
            created: chrono::Utc::now(),
        };
        let body = format!("<Role>{}</Role>", role_inner(&role));
        s.roles.insert(name, role);
        Ok(wrap("CreateRole", &body))
    }

    fn get_role(&self, params: &HashMap<String, String>) -> Result<String, AwsError> {
        let name = required(params, "RoleName")?;
        let s = self.state.read();
        let role = s.roles.get(&name).ok_or_else(|| no_such_entity(&name))?;
        Ok(wrap(
            "GetRole",
            &format!("<Role>{}</Role>", role_inner(role)),
        ))
    }

    fn list_roles(&self) -> Result<String, AwsError> {
        let s = self.state.read();
        let mut members = String::new();
        for r in s.roles.values() {
            members.push_str(&format!("<member>{}</member>", role_inner(r)));
        }
        Ok(wrap(
            "ListRoles",
            &format!("<Roles>{members}</Roles><IsTruncated>false</IsTruncated>"),
        ))
    }

    fn delete_role(&self, params: &HashMap<String, String>) -> Result<String, AwsError> {
        let name = required(params, "RoleName")?;
        let mut s = self.state.write();
        s.roles.remove(&name).ok_or_else(|| no_such_entity(&name))?;
        s.role_policies.remove(&name);
        Ok(empty("DeleteRole"))
    }

    fn attach_role_policy(&self, params: &HashMap<String, String>) -> Result<String, AwsError> {
        let role = required(params, "RoleName")?;
        let policy_arn = required(params, "PolicyArn")?;
        let mut s = self.state.write();
        if !s.roles.contains_key(&role) {
            return Err(no_such_entity(&role));
        }
        s.role_policies.entry(role).or_default().push(policy_arn);
        Ok(empty("AttachRolePolicy"))
    }

    fn detach_role_policy(&self, params: &HashMap<String, String>) -> Result<String, AwsError> {
        let role = required(params, "RoleName")?;
        let policy_arn = required(params, "PolicyArn")?;
        let mut s = self.state.write();
        if let Some(list) = s.role_policies.get_mut(&role) {
            list.retain(|p| p != &policy_arn);
        }
        Ok(empty("DetachRolePolicy"))
    }

    fn list_attached_role_policies(
        &self,
        params: &HashMap<String, String>,
    ) -> Result<String, AwsError> {
        let role = required(params, "RoleName")?;
        let s = self.state.read();
        let attached = s.role_policies.get(&role).cloned().unwrap_or_default();
        let mut members = String::new();
        for arn in attached {
            let name = arn.rsplit('/').next().unwrap_or(&arn);
            members.push_str(&format!(
                "<member><PolicyArn>{}</PolicyArn><PolicyName>{}</PolicyName></member>",
                xml_escape(&arn),
                xml_escape(name),
            ));
        }
        Ok(wrap(
            "ListAttachedRolePolicies",
            &format!(
                "<AttachedPolicies>{members}</AttachedPolicies><IsTruncated>false</IsTruncated>"
            ),
        ))
    }

    fn create_policy(&self, params: &HashMap<String, String>) -> Result<String, AwsError> {
        let name = required(params, "PolicyName")?;
        let document = required(params, "PolicyDocument")?;
        let description = params.get("Description").cloned();
        let arn = format!("arn:aws:iam::{EMULATED_ACCOUNT_ID}:policy/{name}");
        let mut s = self.state.write();
        if s.policies.contains_key(&name) {
            return Err(already_exists("policy", &name));
        }
        let policy = Policy {
            id: format!("ANPA{}", short_id()),
            arn,
            name: name.clone(),
            document,
            description,
            created: chrono::Utc::now(),
        };
        let body = format!("<Policy>{}</Policy>", policy_inner(&policy));
        s.policies.insert(name, policy);
        Ok(wrap("CreatePolicy", &body))
    }

    fn get_policy(&self, params: &HashMap<String, String>) -> Result<String, AwsError> {
        let arn = required(params, "PolicyArn")?;
        let s = self.state.read();
        let policy = s
            .policies
            .values()
            .find(|p| p.arn == arn)
            .ok_or_else(|| no_such_entity(&arn))?;
        Ok(wrap(
            "GetPolicy",
            &format!("<Policy>{}</Policy>", policy_inner(policy)),
        ))
    }

    fn list_policies(&self) -> Result<String, AwsError> {
        let s = self.state.read();
        let mut members = String::new();
        for p in s.policies.values() {
            members.push_str(&format!("<member>{}</member>", policy_inner(p)));
        }
        Ok(wrap(
            "ListPolicies",
            &format!("<Policies>{members}</Policies><IsTruncated>false</IsTruncated>"),
        ))
    }

    fn delete_policy(&self, params: &HashMap<String, String>) -> Result<String, AwsError> {
        let arn = required(params, "PolicyArn")?;
        let mut s = self.state.write();
        let name = s
            .policies
            .iter()
            .find(|(_, p)| p.arn == arn)
            .map(|(n, _)| n.clone())
            .ok_or_else(|| no_such_entity(&arn))?;
        s.policies.remove(&name);
        Ok(empty("DeletePolicy"))
    }

    fn create_access_key(&self, params: &HashMap<String, String>) -> Result<String, AwsError> {
        let user = required(params, "UserName")?;
        let mut s = self.state.write();
        if !s.users.contains_key(&user) {
            return Err(no_such_entity(&user));
        }
        let ak = AccessKey {
            user_name: user.clone(),
            access_key_id: format!("AKIA{}", short_id().to_uppercase()),
            secret: random_secret(),
            status: "Active".into(),
            created: chrono::Utc::now(),
        };
        let body = format!(
            "<AccessKey><UserName>{user}</UserName><AccessKeyId>{ak_id}</AccessKeyId><Status>Active</Status><SecretAccessKey>{secret}</SecretAccessKey><CreateDate>{ts}</CreateDate></AccessKey>",
            user = xml_escape(&user),
            ak_id = xml_escape(&ak.access_key_id),
            secret = xml_escape(&ak.secret),
            ts = ak.created.to_rfc3339(),
        );
        s.access_keys.insert(ak.access_key_id.clone(), ak);
        Ok(wrap("CreateAccessKey", &body))
    }

    fn list_access_keys(&self, params: &HashMap<String, String>) -> Result<String, AwsError> {
        let user = params
            .get("UserName")
            .cloned()
            .unwrap_or_else(|| "kuroko".to_string());
        let s = self.state.read();
        let mut members = String::new();
        for ak in s.access_keys.values().filter(|ak| ak.user_name == user) {
            members.push_str(&format!(
                "<member><UserName>{}</UserName><AccessKeyId>{}</AccessKeyId><Status>{}</Status><CreateDate>{}</CreateDate></member>",
                xml_escape(&ak.user_name),
                xml_escape(&ak.access_key_id),
                xml_escape(&ak.status),
                ak.created.to_rfc3339(),
            ));
        }
        Ok(wrap(
            "ListAccessKeys",
            &format!(
                "<AccessKeyMetadata>{members}</AccessKeyMetadata><IsTruncated>false</IsTruncated>"
            ),
        ))
    }

    fn delete_access_key(&self, params: &HashMap<String, String>) -> Result<String, AwsError> {
        let id = required(params, "AccessKeyId")?;
        self.state.write().access_keys.remove(&id);
        Ok(empty("DeleteAccessKey"))
    }
}

fn required(p: &HashMap<String, String>, key: &str) -> Result<String, AwsError> {
    p.get(key)
        .cloned()
        .ok_or_else(|| AwsError::new("ValidationError", format!("{key} required")))
}

fn no_such_entity(name: &str) -> AwsError {
    AwsError::new("NoSuchEntity", format!("entity '{name}' does not exist"))
}

fn already_exists(kind: &str, name: &str) -> AwsError {
    AwsError::new(
        "EntityAlreadyExists",
        format!("{kind} '{name}' already exists"),
    )
}

fn short_id() -> String {
    Uuid::new_v4().simple().to_string()[..16].to_string()
}

fn random_secret() -> String {
    use rand::Rng;
    (0..40)
        .map(|_| {
            let c = rand::thread_rng().gen_range(0u8..36);
            if c < 10 {
                (b'0' + c) as char
            } else {
                (b'a' + (c - 10)) as char
            }
        })
        .collect()
}

fn user_inner(u: &User) -> String {
    format!(
        "<Path>{path}</Path><UserName>{name}</UserName><UserId>{id}</UserId><Arn>{arn}</Arn><CreateDate>{ts}</CreateDate>",
        path = xml_escape(&u.path),
        name = xml_escape(&u.name),
        id = u.id,
        arn = xml_escape(&u.arn),
        ts = u.created.to_rfc3339(),
    )
}

fn role_inner(r: &Role) -> String {
    let mut body = format!(
        "<Path>{path}</Path><RoleName>{name}</RoleName><RoleId>{id}</RoleId><Arn>{arn}</Arn><CreateDate>{ts}</CreateDate><AssumeRolePolicyDocument>{policy}</AssumeRolePolicyDocument>",
        path = xml_escape(&r.path),
        name = xml_escape(&r.name),
        id = r.id,
        arn = xml_escape(&r.arn),
        ts = r.created.to_rfc3339(),
        // AWS URL-encodes the AssumeRolePolicyDocument in the response. The
        // SDK percent-decodes it on the way back, so we encode it here.
        policy = urlencoding::encode(&r.assume_role_policy),
    );
    if let Some(d) = &r.description {
        body.push_str(&format!("<Description>{}</Description>", xml_escape(d)));
    }
    body
}

fn policy_inner(p: &Policy) -> String {
    let mut body = format!(
        "<PolicyName>{name}</PolicyName><PolicyId>{id}</PolicyId><Arn>{arn}</Arn><Path>/</Path><DefaultVersionId>v1</DefaultVersionId><AttachmentCount>0</AttachmentCount><IsAttachable>true</IsAttachable><CreateDate>{ts}</CreateDate><UpdateDate>{ts}</UpdateDate>",
        name = xml_escape(&p.name),
        id = p.id,
        arn = xml_escape(&p.arn),
        ts = p.created.to_rfc3339(),
    );
    if let Some(d) = &p.description {
        body.push_str(&format!("<Description>{}</Description>", xml_escape(d)));
    }
    body
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
    registry.register_query(Arc::new(Iam::new()));
}
