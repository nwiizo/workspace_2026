//! Cognito Identity Provider — AWS JSON 1.1, target prefix
//! `AWSCognitoIdentityProviderService`.
//!
//! User pool and user metadata only. No password hashing or token issuance —
//! kuroko stores the requested status verbatim so tests can verify the
//! lifecycle without driving a real identity store.

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

const TARGET_PREFIX: &str = "AWSCognitoIdentityProviderService";

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
struct State {
    pools: HashMap<String, UserPool>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct UserPool {
    id: String,
    name: String,
    arn: String,
    created: chrono::DateTime<chrono::Utc>,
    clients: HashMap<String, UserPoolClient>,
    users: HashMap<String, User>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct UserPoolClient {
    id: String,
    name: String,
    secret: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct User {
    username: String,
    attributes: HashMap<String, String>,
    status: String,
    enabled: bool,
    created: chrono::DateTime<chrono::Utc>,
    last_modified: chrono::DateTime<chrono::Utc>,
}

pub struct Cognito {
    state: Arc<RwLock<State>>,
}

impl Cognito {
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(State::default())),
        }
    }
}

impl Default for Cognito {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Service for Cognito {
    fn name(&self) -> &'static str {
        "cognito"
    }

    fn reset(&self) {
        self.state.write().pools.clear();
    }

    fn restore(&self, ctx: &ServiceContext) -> Result<(), AwsError> {
        if let Some(snap) = &ctx.snapshot
            && let Some(data) = snap.load::<State>("cognito").map_err(persistence_error)?
        {
            *self.state.write() = data;
        }
        Ok(())
    }

    fn snapshot(&self, ctx: &ServiceContext) -> Result<(), AwsError> {
        if let Some(snap) = &ctx.snapshot {
            let data = self.state.read();
            snap.save("cognito", &*data).map_err(persistence_error)?;
        }
        Ok(())
    }

    fn router(&self, _ctx: ServiceContext) -> Router {
        Router::new()
    }
}

#[async_trait]
impl JsonProtocolService for Cognito {
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
            "CreateUserPool" => self.create_user_pool(&req),
            "DescribeUserPool" => self.describe_user_pool(&req),
            "ListUserPools" => self.list_user_pools(&req),
            "DeleteUserPool" => self.delete_user_pool(&req),
            "CreateUserPoolClient" => self.create_user_pool_client(&req),
            "DescribeUserPoolClient" => self.describe_user_pool_client(&req),
            "ListUserPoolClients" => self.list_user_pool_clients(&req),
            "AdminCreateUser" => self.admin_create_user(&req),
            "AdminGetUser" => self.admin_get_user(&req),
            "ListUsers" => self.list_users(&req),
            "AdminDeleteUser" => self.admin_delete_user(&req),
            "AdminSetUserPassword" => self.admin_set_user_password(&req),
            other => Err(AwsError::unsupported(format!("Cognito::{other}"))),
        }
    }
}

impl Cognito {
    fn create_user_pool(&self, req: &Value) -> Result<Value, AwsError> {
        let name = req
            .get("PoolName")
            .and_then(Value::as_str)
            .ok_or_else(|| AwsError::new("InvalidParameterException", "PoolName required"))?
            .to_string();
        let id = format!("{EMULATED_REGION}_{}", short_id().to_uppercase());
        let arn =
            format!("arn:aws:cognito-idp:{EMULATED_REGION}:{EMULATED_ACCOUNT_ID}:userpool/{id}");
        let pool = UserPool {
            id: id.clone(),
            name,
            arn,
            created: chrono::Utc::now(),
            clients: HashMap::new(),
            users: HashMap::new(),
        };
        let resp = pool_json(&pool);
        self.state.write().pools.insert(id, pool);
        Ok(json!({ "UserPool": resp }))
    }

    fn describe_user_pool(&self, req: &Value) -> Result<Value, AwsError> {
        let id = pool_id(req)?;
        let s = self.state.read();
        let pool = s.pools.get(&id).ok_or_else(|| not_found_pool(&id))?;
        Ok(json!({ "UserPool": pool_json(pool) }))
    }

    fn list_user_pools(&self, _req: &Value) -> Result<Value, AwsError> {
        let s = self.state.read();
        let pools: Vec<_> = s
            .pools
            .values()
            .map(|p| {
                json!({
                    "Id": p.id,
                    "Name": p.name,
                    "CreationDate": p.created.timestamp(),
                    "LastModifiedDate": p.created.timestamp(),
                })
            })
            .collect();
        Ok(json!({ "UserPools": pools }))
    }

    fn delete_user_pool(&self, req: &Value) -> Result<Value, AwsError> {
        let id = pool_id(req)?;
        self.state
            .write()
            .pools
            .remove(&id)
            .ok_or_else(|| not_found_pool(&id))?;
        Ok(json!({}))
    }

    fn create_user_pool_client(&self, req: &Value) -> Result<Value, AwsError> {
        let pool_id = pool_id(req)?;
        let name = req
            .get("ClientName")
            .and_then(Value::as_str)
            .ok_or_else(|| AwsError::new("InvalidParameterException", "ClientName required"))?
            .to_string();
        let generate_secret = req
            .get("GenerateSecret")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let mut s = self.state.write();
        let pool = s
            .pools
            .get_mut(&pool_id)
            .ok_or_else(|| not_found_pool(&pool_id))?;
        let client_id = short_id();
        let client = UserPoolClient {
            id: client_id.clone(),
            name,
            secret: generate_secret.then(|| Uuid::new_v4().simple().to_string()),
        };
        let resp = client_json(&pool_id, &client);
        pool.clients.insert(client_id, client);
        Ok(json!({ "UserPoolClient": resp }))
    }

    fn describe_user_pool_client(&self, req: &Value) -> Result<Value, AwsError> {
        let pool_id = pool_id(req)?;
        let client_id = required(req, "ClientId")?;
        let s = self.state.read();
        let pool = s
            .pools
            .get(&pool_id)
            .ok_or_else(|| not_found_pool(&pool_id))?;
        let client = pool
            .clients
            .get(&client_id)
            .ok_or_else(|| not_found_client(&client_id))?;
        Ok(json!({ "UserPoolClient": client_json(&pool_id, client) }))
    }

    fn list_user_pool_clients(&self, req: &Value) -> Result<Value, AwsError> {
        let pool_id = pool_id(req)?;
        let s = self.state.read();
        let pool = s
            .pools
            .get(&pool_id)
            .ok_or_else(|| not_found_pool(&pool_id))?;
        let clients: Vec<_> = pool
            .clients
            .values()
            .map(|c| {
                json!({
                    "ClientId": c.id,
                    "ClientName": c.name,
                    "UserPoolId": pool_id,
                })
            })
            .collect();
        Ok(json!({ "UserPoolClients": clients }))
    }

    fn admin_create_user(&self, req: &Value) -> Result<Value, AwsError> {
        let pool_id = pool_id(req)?;
        let username = required(req, "Username")?;
        let attributes = parse_attributes(req.get("UserAttributes"));
        let mut s = self.state.write();
        let pool = s
            .pools
            .get_mut(&pool_id)
            .ok_or_else(|| not_found_pool(&pool_id))?;
        if pool.users.contains_key(&username) {
            return Err(AwsError::new(
                "UsernameExistsException",
                format!("user '{username}' already exists"),
            ));
        }
        let now = chrono::Utc::now();
        let user = User {
            username: username.clone(),
            attributes,
            status: "FORCE_CHANGE_PASSWORD".into(),
            enabled: true,
            created: now,
            last_modified: now,
        };
        let resp = user_json(&user);
        pool.users.insert(username, user);
        Ok(json!({ "User": resp }))
    }

    fn admin_get_user(&self, req: &Value) -> Result<Value, AwsError> {
        let pool_id = pool_id(req)?;
        let username = required(req, "Username")?;
        let s = self.state.read();
        let pool = s
            .pools
            .get(&pool_id)
            .ok_or_else(|| not_found_pool(&pool_id))?;
        let user = pool
            .users
            .get(&username)
            .ok_or_else(|| not_found_user(&username))?;
        let attrs: Vec<_> = user
            .attributes
            .iter()
            .map(|(k, v)| json!({ "Name": k, "Value": v }))
            .collect();
        // AdminGetUser response wraps attributes under `UserAttributes` (not
        // `Attributes`, which is what ListUsers uses for the same field). The
        // SDK relies on the distinction so we mirror it exactly.
        Ok(json!({
            "Username": user.username,
            "UserAttributes": attrs,
            "UserStatus": user.status,
            "Enabled": user.enabled,
            "UserCreateDate": user.created.timestamp(),
            "UserLastModifiedDate": user.last_modified.timestamp(),
        }))
    }

    fn list_users(&self, req: &Value) -> Result<Value, AwsError> {
        let pool_id = pool_id(req)?;
        let s = self.state.read();
        let pool = s
            .pools
            .get(&pool_id)
            .ok_or_else(|| not_found_pool(&pool_id))?;
        let users: Vec<_> = pool.users.values().map(user_json).collect();
        Ok(json!({ "Users": users }))
    }

    fn admin_delete_user(&self, req: &Value) -> Result<Value, AwsError> {
        let pool_id = pool_id(req)?;
        let username = required(req, "Username")?;
        let mut s = self.state.write();
        let pool = s
            .pools
            .get_mut(&pool_id)
            .ok_or_else(|| not_found_pool(&pool_id))?;
        pool.users
            .remove(&username)
            .ok_or_else(|| not_found_user(&username))?;
        Ok(json!({}))
    }

    fn admin_set_user_password(&self, req: &Value) -> Result<Value, AwsError> {
        let pool_id = pool_id(req)?;
        let username = required(req, "Username")?;
        let permanent = req
            .get("Permanent")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let mut s = self.state.write();
        let pool = s
            .pools
            .get_mut(&pool_id)
            .ok_or_else(|| not_found_pool(&pool_id))?;
        let user = pool
            .users
            .get_mut(&username)
            .ok_or_else(|| not_found_user(&username))?;
        user.last_modified = chrono::Utc::now();
        user.status = if permanent {
            "CONFIRMED".into()
        } else {
            "FORCE_CHANGE_PASSWORD".into()
        };
        Ok(json!({}))
    }
}

fn pool_id(req: &Value) -> Result<String, AwsError> {
    req.get("UserPoolId")
        .and_then(Value::as_str)
        .map(String::from)
        .ok_or_else(|| AwsError::new("InvalidParameterException", "UserPoolId required"))
}

fn required(req: &Value, key: &str) -> Result<String, AwsError> {
    req.get(key)
        .and_then(Value::as_str)
        .map(String::from)
        .ok_or_else(|| AwsError::new("InvalidParameterException", format!("{key} required")))
}

fn parse_attributes(value: Option<&Value>) -> HashMap<String, String> {
    value
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(|item| {
                    let name = item.get("Name").and_then(Value::as_str)?;
                    let val = item.get("Value").and_then(Value::as_str)?;
                    Some((name.to_string(), val.to_string()))
                })
                .collect()
        })
        .unwrap_or_default()
}

fn pool_json(p: &UserPool) -> Value {
    json!({
        "Id": p.id,
        "Name": p.name,
        "Arn": p.arn,
        "CreationDate": p.created.timestamp(),
        "LastModifiedDate": p.created.timestamp(),
    })
}

fn client_json(pool_id: &str, c: &UserPoolClient) -> Value {
    let mut v = json!({
        "ClientId": c.id,
        "ClientName": c.name,
        "UserPoolId": pool_id,
    });
    if let Some(s) = &c.secret {
        v["ClientSecret"] = Value::String(s.clone());
    }
    v
}

fn user_json(u: &User) -> Value {
    let attrs: Vec<_> = u
        .attributes
        .iter()
        .map(|(k, v)| json!({ "Name": k, "Value": v }))
        .collect();
    json!({
        "Username": u.username,
        "Attributes": attrs,
        "UserStatus": u.status,
        "Enabled": u.enabled,
        "UserCreateDate": u.created.timestamp(),
        "UserLastModifiedDate": u.last_modified.timestamp(),
    })
}

fn not_found_pool(id: &str) -> AwsError {
    AwsError::new(
        "ResourceNotFoundException",
        format!("user pool '{id}' not found"),
    )
}

fn not_found_client(id: &str) -> AwsError {
    AwsError::new(
        "ResourceNotFoundException",
        format!("client '{id}' not found"),
    )
}

fn not_found_user(name: &str) -> AwsError {
    AwsError::new("UserNotFoundException", format!("user '{name}' not found"))
}

fn short_id() -> String {
    Uuid::new_v4().simple().to_string()[..9].to_string()
}

pub fn register(registry: &Arc<crate::registry::Registry>) {
    registry.register_json(Arc::new(Cognito::new()));
}
