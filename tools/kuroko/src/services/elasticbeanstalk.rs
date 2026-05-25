//! Elastic Beanstalk — AWS Query protocol, sdk_id `Elastic Beanstalk`.

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

const SDK_ID: &str = "Elastic Beanstalk";
const NS: &str = "http://elasticbeanstalk.amazonaws.com/docs/2010-12-01/";
const ACTIONS: &[&str] = &[
    "CreateApplication",
    "DescribeApplications",
    "UpdateApplication",
    "DeleteApplication",
    "CreateEnvironment",
    "DescribeEnvironments",
    "TerminateEnvironment",
];

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
struct State {
    applications: HashMap<String, Application>,
    environments: HashMap<String, Environment>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct Application {
    name: String,
    arn: String,
    description: String,
    versions: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct Environment {
    id: String,
    name: String,
    arn: String,
    application_name: String,
    status: String,
    cname: String,
}

pub struct ElasticBeanstalk {
    state: Arc<RwLock<State>>,
}
impl ElasticBeanstalk {
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(State::default())),
        }
    }
}
impl Default for ElasticBeanstalk {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Service for ElasticBeanstalk {
    fn name(&self) -> &'static str {
        "elasticbeanstalk"
    }
    fn reset(&self) {
        *self.state.write() = State::default();
    }
    fn restore(&self, ctx: &ServiceContext) -> Result<(), AwsError> {
        if let Some(snap) = &ctx.snapshot
            && let Some(data) = snap
                .load::<State>("elasticbeanstalk")
                .map_err(persistence_error)?
        {
            *self.state.write() = data;
        }
        Ok(())
    }
    fn snapshot(&self, ctx: &ServiceContext) -> Result<(), AwsError> {
        if let Some(snap) = &ctx.snapshot {
            snap.save("elasticbeanstalk", &*self.state.read())
                .map_err(persistence_error)?;
        }
        Ok(())
    }
    fn router(&self, _ctx: ServiceContext) -> Router {
        Router::new()
    }
}

#[async_trait]
impl QueryProtocolService for ElasticBeanstalk {
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
            "CreateApplication" => {
                let name = required(params, "ApplicationName")?;
                let desc = params.get("Description").cloned().unwrap_or_default();
                let mut s = self.state.write();
                if s.applications.contains_key(&name) {
                    return Err(AwsError::new(
                        "TooManyApplicationsException",
                        format!("application '{name}' exists"),
                    ));
                }
                let app = Application {
                    arn: app_arn(&name),
                    name: name.clone(),
                    description: desc,
                    versions: Vec::new(),
                };
                let body = format!("<Application>{}</Application>", application_xml(&app));
                s.applications.insert(name, app);
                Ok(wrap("CreateApplication", &body))
            }
            "DescribeApplications" => {
                let filter = collect_members(params, "ApplicationNames.member.");
                let s = self.state.read();
                let mut members = String::new();
                for app in s.applications.values() {
                    if !filter.is_empty() && !filter.contains(&app.name) {
                        continue;
                    }
                    members.push_str(&format!("<member>{}</member>", application_xml(app)));
                }
                Ok(wrap(
                    "DescribeApplications",
                    &format!("<Applications>{members}</Applications>"),
                ))
            }
            "UpdateApplication" => {
                let name = required(params, "ApplicationName")?;
                let desc = params.get("Description").cloned();
                let mut s = self.state.write();
                let app = s.applications.get_mut(&name).ok_or_else(|| {
                    AwsError::new(
                        "OperationInProgressException",
                        format!("application '{name}' not found"),
                    )
                })?;
                if let Some(d) = desc {
                    app.description = d;
                }
                Ok(wrap(
                    "UpdateApplication",
                    &format!("<Application>{}</Application>", application_xml(app)),
                ))
            }
            "DeleteApplication" => {
                let name = required(params, "ApplicationName")?;
                let mut s = self.state.write();
                if s.applications.remove(&name).is_none() {
                    return Err(AwsError::new(
                        "OperationInProgressException",
                        format!("application '{name}' not found"),
                    ));
                }
                Ok(empty("DeleteApplication"))
            }
            "CreateEnvironment" => {
                let app_name = required(params, "ApplicationName")?;
                let env_name = required(params, "EnvironmentName")?;
                if !self.state.read().applications.contains_key(&app_name) {
                    return Err(AwsError::new(
                        "OperationInProgressException",
                        format!("application '{app_name}' not found"),
                    ));
                }
                let id = format!("e-{}", &Uuid::new_v4().simple().to_string()[..10]);
                let env = Environment {
                    id: id.clone(),
                    arn: env_arn(&env_name),
                    application_name: app_name,
                    cname: format!("{env_name}.kuroko.elasticbeanstalk.com"),
                    name: env_name,
                    status: "Ready".into(),
                };
                let body = environment_xml(&env);
                self.state.write().environments.insert(id, env);
                Ok(wrap("CreateEnvironment", &body))
            }
            "DescribeEnvironments" => {
                let app_filter = params.get("ApplicationName").cloned();
                let s = self.state.read();
                let mut members = String::new();
                for env in s.environments.values() {
                    if let Some(a) = &app_filter
                        && &env.application_name != a
                    {
                        continue;
                    }
                    members.push_str(&format!("<member>{}</member>", environment_xml(env)));
                }
                Ok(wrap(
                    "DescribeEnvironments",
                    &format!("<Environments>{members}</Environments>"),
                ))
            }
            "TerminateEnvironment" => {
                let id = params.get("EnvironmentId").cloned();
                let name = params.get("EnvironmentName").cloned();
                let mut s = self.state.write();
                let key = s
                    .environments
                    .iter()
                    .find(|(_, e)| id.as_deref() == Some(&e.id) || name.as_deref() == Some(&e.name))
                    .map(|(k, _)| k.clone());
                let key = key.ok_or_else(|| {
                    AwsError::new("OperationInProgressException", "environment not found")
                })?;
                let mut env = s.environments.remove(&key).unwrap();
                env.status = "Terminated".into();
                Ok(wrap("TerminateEnvironment", &environment_xml(&env)))
            }
            other => Err(AwsError::unsupported(format!("ElasticBeanstalk::{other}"))),
        }
    }
}

fn collect_members(params: &HashMap<String, String>, prefix: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut i = 1;
    loop {
        let key = format!("{prefix}{i}");
        let Some(v) = params.get(&key) else { break };
        out.push(v.clone());
        i += 1;
    }
    out
}

fn application_xml(a: &Application) -> String {
    format!(
        "<ApplicationName>{name}</ApplicationName><ApplicationArn>{arn}</ApplicationArn><Description>{desc}</Description>",
        name = xml_escape(&a.name),
        arn = xml_escape(&a.arn),
        desc = xml_escape(&a.description),
    )
}

fn environment_xml(e: &Environment) -> String {
    format!(
        "<EnvironmentId>{id}</EnvironmentId><EnvironmentName>{name}</EnvironmentName><EnvironmentArn>{arn}</EnvironmentArn><ApplicationName>{app}</ApplicationName><Status>{status}</Status><CNAME>{cname}</CNAME>",
        id = xml_escape(&e.id),
        name = xml_escape(&e.name),
        arn = xml_escape(&e.arn),
        app = xml_escape(&e.application_name),
        status = xml_escape(&e.status),
        cname = xml_escape(&e.cname),
    )
}

fn app_arn(name: &str) -> String {
    format!("arn:aws:elasticbeanstalk:{EMULATED_REGION}:{EMULATED_ACCOUNT_ID}:application/{name}")
}

fn env_arn(name: &str) -> String {
    format!("arn:aws:elasticbeanstalk:{EMULATED_REGION}:{EMULATED_ACCOUNT_ID}:environment/{name}")
}

fn required(p: &HashMap<String, String>, key: &str) -> Result<String, AwsError> {
    p.get(key)
        .cloned()
        .ok_or_else(|| AwsError::new("MissingParameter", format!("{key} required")))
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
    registry.register_query(Arc::new(ElasticBeanstalk::new()));
}
