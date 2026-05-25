//! AWS Organizations — JSON 1.1, target prefix `AWSOrganizationsV20161128`.

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
    EMULATED_ACCOUNT_ID, JsonProtocolService, Service, ServiceContext, persistence_error,
};

const TARGET_PREFIX: &str = "AWSOrganizationsV20161128";

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
struct State {
    organization: Option<Organization>,
    accounts: HashMap<String, Account>,
    ous: HashMap<String, OrganizationalUnit>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct Organization {
    id: String,
    arn: String,
    feature_set: String,
    master_account_id: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct Account {
    id: String,
    arn: String,
    email: String,
    name: String,
    status: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct OrganizationalUnit {
    id: String,
    arn: String,
    name: String,
}

pub struct Organizations {
    state: Arc<RwLock<State>>,
}

impl Organizations {
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(State::default())),
        }
    }
}
impl Default for Organizations {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Service for Organizations {
    fn name(&self) -> &'static str {
        "organizations"
    }
    fn reset(&self) {
        *self.state.write() = State::default();
    }
    fn restore(&self, ctx: &ServiceContext) -> Result<(), AwsError> {
        if let Some(snap) = &ctx.snapshot
            && let Some(data) = snap
                .load::<State>("organizations")
                .map_err(persistence_error)?
        {
            *self.state.write() = data;
        }
        Ok(())
    }
    fn snapshot(&self, ctx: &ServiceContext) -> Result<(), AwsError> {
        if let Some(snap) = &ctx.snapshot {
            let data = self.state.read();
            snap.save("organizations", &*data)
                .map_err(persistence_error)?;
        }
        Ok(())
    }
    fn router(&self, _ctx: ServiceContext) -> Router {
        Router::new()
    }
}

#[async_trait]
impl JsonProtocolService for Organizations {
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
                .map_err(|e| AwsError::new("InvalidInputException", e.to_string()))?
        };
        match action {
            "CreateOrganization" => self.create_organization(&req),
            "DescribeOrganization" => self.describe_organization(),
            "DeleteOrganization" => {
                self.state.write().organization = None;
                Ok(json!({}))
            }
            "CreateAccount" => self.create_account(&req),
            "DescribeAccount" => self.describe_account(&req),
            "ListAccounts" => self.list_accounts(),
            "CloseAccount" => self.close_account(&req),
            "CreateOrganizationalUnit" => self.create_ou(&req),
            "ListOrganizationalUnitsForParent" => self.list_ous(),
            other => Err(AwsError::unsupported(format!("Organizations::{other}"))),
        }
    }
}

impl Organizations {
    fn create_organization(&self, req: &Value) -> Result<Value, AwsError> {
        let feature_set = req
            .get("FeatureSet")
            .and_then(Value::as_str)
            .unwrap_or("ALL")
            .to_string();
        let id = format!(
            "o-{}",
            Uuid::new_v4().simple().to_string()[..10].to_lowercase()
        );
        let arn = format!("arn:aws:organizations::{EMULATED_ACCOUNT_ID}:organization/{id}");
        let org = Organization {
            id: id.clone(),
            arn: arn.clone(),
            feature_set,
            master_account_id: EMULATED_ACCOUNT_ID.to_string(),
        };
        let mut s = self.state.write();
        if s.organization.is_some() {
            return Err(AwsError::new(
                "AlreadyInOrganizationException",
                "an organization already exists",
            ));
        }
        s.organization = Some(org.clone());
        Ok(json!({
            "Organization": {
                "Id": org.id,
                "Arn": org.arn,
                "FeatureSet": org.feature_set,
                "MasterAccountId": org.master_account_id,
                "MasterAccountArn": format!("arn:aws:organizations::{EMULATED_ACCOUNT_ID}:account/{id}/{EMULATED_ACCOUNT_ID}"),
                "MasterAccountEmail": "root@kuroko.test",
            }
        }))
    }

    fn describe_organization(&self) -> Result<Value, AwsError> {
        let s = self.state.read();
        let org = s.organization.as_ref().ok_or_else(|| {
            AwsError::new(
                "AWSOrganizationsNotInUseException",
                "organization not created",
            )
        })?;
        Ok(json!({
            "Organization": {
                "Id": org.id,
                "Arn": org.arn,
                "FeatureSet": org.feature_set,
                "MasterAccountId": org.master_account_id,
                "MasterAccountArn": format!("arn:aws:organizations::{EMULATED_ACCOUNT_ID}:account/{id}/{acct}", id = org.id, acct = EMULATED_ACCOUNT_ID),
                "MasterAccountEmail": "root@kuroko.test",
            }
        }))
    }

    fn create_account(&self, req: &Value) -> Result<Value, AwsError> {
        let email = req
            .get("Email")
            .and_then(Value::as_str)
            .ok_or_else(|| AwsError::new("InvalidInputException", "Email required"))?
            .to_string();
        let name = req
            .get("AccountName")
            .and_then(Value::as_str)
            .ok_or_else(|| AwsError::new("InvalidInputException", "AccountName required"))?
            .to_string();
        let id = format!(
            "{:012}",
            chrono::Utc::now().timestamp_millis() % 1_000_000_000_000
        );
        let request_id = Uuid::new_v4().to_string();
        let arn = format!("arn:aws:organizations::{EMULATED_ACCOUNT_ID}:account/o-kuroko/{id}");
        let account = Account {
            id: id.clone(),
            arn,
            email,
            name,
            status: "ACTIVE".into(),
        };
        self.state.write().accounts.insert(id.clone(), account);
        Ok(json!({
            "CreateAccountStatus": {
                "Id": request_id,
                "AccountId": id,
                "State": "SUCCEEDED",
                "AccountName": "kuroko-account",
            }
        }))
    }

    fn describe_account(&self, req: &Value) -> Result<Value, AwsError> {
        let id = req
            .get("AccountId")
            .and_then(Value::as_str)
            .ok_or_else(|| AwsError::new("InvalidInputException", "AccountId required"))?;
        let s = self.state.read();
        let account = s.accounts.get(id).ok_or_else(|| {
            AwsError::new(
                "AccountNotFoundException",
                format!("account '{id}' not found"),
            )
        })?;
        Ok(json!({
            "Account": {
                "Id": account.id,
                "Arn": account.arn,
                "Email": account.email,
                "Name": account.name,
                "Status": account.status,
            }
        }))
    }

    fn list_accounts(&self) -> Result<Value, AwsError> {
        let s = self.state.read();
        let accounts: Vec<_> = s
            .accounts
            .values()
            .map(|a| {
                json!({
                    "Id": a.id,
                    "Arn": a.arn,
                    "Email": a.email,
                    "Name": a.name,
                    "Status": a.status,
                })
            })
            .collect();
        Ok(json!({ "Accounts": accounts }))
    }

    fn close_account(&self, req: &Value) -> Result<Value, AwsError> {
        let id = req
            .get("AccountId")
            .and_then(Value::as_str)
            .ok_or_else(|| AwsError::new("InvalidInputException", "AccountId required"))?;
        let mut s = self.state.write();
        if let Some(a) = s.accounts.get_mut(id) {
            a.status = "SUSPENDED".into();
        }
        Ok(json!({}))
    }

    fn create_ou(&self, req: &Value) -> Result<Value, AwsError> {
        let name = req
            .get("Name")
            .and_then(Value::as_str)
            .ok_or_else(|| AwsError::new("InvalidInputException", "Name required"))?
            .to_string();
        let id = format!(
            "ou-{}",
            Uuid::new_v4().simple().to_string()[..14].to_lowercase()
        );
        let arn = format!("arn:aws:organizations::{EMULATED_ACCOUNT_ID}:ou/o-kuroko/{id}");
        let ou = OrganizationalUnit {
            id: id.clone(),
            arn: arn.clone(),
            name: name.clone(),
        };
        self.state.write().ous.insert(id.clone(), ou);
        Ok(json!({
            "OrganizationalUnit": {
                "Id": id,
                "Arn": arn,
                "Name": name,
            }
        }))
    }

    fn list_ous(&self) -> Result<Value, AwsError> {
        let s = self.state.read();
        let ous: Vec<_> = s
            .ous
            .values()
            .map(|o| {
                json!({
                    "Id": o.id,
                    "Arn": o.arn,
                    "Name": o.name,
                })
            })
            .collect();
        Ok(json!({ "OrganizationalUnits": ous }))
    }
}

pub fn register(registry: &Arc<crate::registry::Registry>) {
    registry.register_json(Arc::new(Organizations::new()));
}
