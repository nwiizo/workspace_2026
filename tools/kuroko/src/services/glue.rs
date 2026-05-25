//! Glue Data Catalog — AWS JSON 1.1, target prefix `AWSGlue`.
//!
//! Database / Table / Crawler metadata. No actual cataloging or crawling.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use axum::Router;
use bytes::Bytes;
use parking_lot::RwLock;
use serde_json::{Value, json};

use crate::aws_error::AwsError;
use crate::service::{JsonProtocolService, Service, ServiceContext, persistence_error};

const TARGET_PREFIX: &str = "AWSGlue";

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
struct State {
    databases: HashMap<String, Database>,
    crawlers: HashMap<String, Crawler>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct Database {
    name: String,
    description: Option<String>,
    location_uri: Option<String>,
    created: chrono::DateTime<chrono::Utc>,
    tables: HashMap<String, Table>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct Table {
    name: String,
    database_name: String,
    description: Option<String>,
    storage_descriptor: Value,
    table_type: Option<String>,
    parameters: HashMap<String, String>,
    created: chrono::DateTime<chrono::Utc>,
    updated: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct Crawler {
    name: String,
    role: String,
    database_name: Option<String>,
    targets: Value,
    state: String,
    created: chrono::DateTime<chrono::Utc>,
}

pub struct Glue {
    state: Arc<RwLock<State>>,
}

impl Glue {
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(State::default())),
        }
    }
}

impl Default for Glue {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Service for Glue {
    fn name(&self) -> &'static str {
        "glue"
    }
    fn reset(&self) {
        *self.state.write() = State::default();
    }
    fn restore(&self, ctx: &ServiceContext) -> Result<(), AwsError> {
        if let Some(snap) = &ctx.snapshot
            && let Some(data) = snap.load::<State>("glue").map_err(persistence_error)?
        {
            *self.state.write() = data;
        }
        Ok(())
    }
    fn snapshot(&self, ctx: &ServiceContext) -> Result<(), AwsError> {
        if let Some(snap) = &ctx.snapshot {
            let data = self.state.read();
            snap.save("glue", &*data).map_err(persistence_error)?;
        }
        Ok(())
    }
    fn router(&self, _ctx: ServiceContext) -> Router {
        Router::new()
    }
}

#[async_trait]
impl JsonProtocolService for Glue {
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
            "CreateDatabase" => self.create_database(&req),
            "GetDatabase" => self.get_database(&req),
            "GetDatabases" => self.get_databases(),
            "DeleteDatabase" => self.delete_database(&req),
            "CreateTable" => self.create_table(&req),
            "GetTable" => self.get_table(&req),
            "GetTables" => self.get_tables(&req),
            "DeleteTable" => self.delete_table(&req),
            "CreateCrawler" => self.create_crawler(&req),
            "GetCrawler" => self.get_crawler(&req),
            "GetCrawlers" => self.get_crawlers(),
            "DeleteCrawler" => self.delete_crawler(&req),
            "StartCrawler" => self.start_crawler(&req),
            other => Err(AwsError::unsupported(format!("Glue::{other}"))),
        }
    }
}

impl Glue {
    fn create_database(&self, req: &Value) -> Result<Value, AwsError> {
        let input = req
            .get("DatabaseInput")
            .ok_or_else(|| AwsError::new("InvalidInputException", "DatabaseInput required"))?;
        let name = input
            .get("Name")
            .and_then(Value::as_str)
            .ok_or_else(|| AwsError::new("InvalidInputException", "DatabaseInput.Name required"))?
            .to_string();
        let mut s = self.state.write();
        if s.databases.contains_key(&name) {
            return Err(AwsError::new(
                "AlreadyExistsException",
                format!("database '{name}' already exists"),
            ));
        }
        s.databases.insert(
            name.clone(),
            Database {
                name,
                description: input
                    .get("Description")
                    .and_then(Value::as_str)
                    .map(String::from),
                location_uri: input
                    .get("LocationUri")
                    .and_then(Value::as_str)
                    .map(String::from),
                created: chrono::Utc::now(),
                tables: HashMap::new(),
            },
        );
        Ok(json!({}))
    }

    fn get_database(&self, req: &Value) -> Result<Value, AwsError> {
        let name = required(req, "Name")?;
        let s = self.state.read();
        let db = s.databases.get(&name).ok_or_else(|| not_found(&name))?;
        Ok(json!({ "Database": database_json(db) }))
    }

    fn get_databases(&self) -> Result<Value, AwsError> {
        let s = self.state.read();
        let list: Vec<_> = s.databases.values().map(database_json).collect();
        Ok(json!({ "DatabaseList": list }))
    }

    fn delete_database(&self, req: &Value) -> Result<Value, AwsError> {
        let name = required(req, "Name")?;
        self.state
            .write()
            .databases
            .remove(&name)
            .ok_or_else(|| not_found(&name))?;
        Ok(json!({}))
    }

    fn create_table(&self, req: &Value) -> Result<Value, AwsError> {
        let database_name = required(req, "DatabaseName")?;
        let input = req
            .get("TableInput")
            .ok_or_else(|| AwsError::new("InvalidInputException", "TableInput required"))?;
        let name = input
            .get("Name")
            .and_then(Value::as_str)
            .ok_or_else(|| AwsError::new("InvalidInputException", "TableInput.Name required"))?
            .to_string();
        let mut s = self.state.write();
        let db = s
            .databases
            .get_mut(&database_name)
            .ok_or_else(|| not_found(&database_name))?;
        if db.tables.contains_key(&name) {
            return Err(AwsError::new(
                "AlreadyExistsException",
                format!("table '{name}' already exists"),
            ));
        }
        let parameters: HashMap<String, String> = input
            .get("Parameters")
            .and_then(Value::as_object)
            .map(|m| {
                m.iter()
                    .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                    .collect()
            })
            .unwrap_or_default();
        let now = chrono::Utc::now();
        db.tables.insert(
            name.clone(),
            Table {
                name,
                database_name: database_name.clone(),
                description: input
                    .get("Description")
                    .and_then(Value::as_str)
                    .map(String::from),
                storage_descriptor: input
                    .get("StorageDescriptor")
                    .cloned()
                    .unwrap_or(Value::Null),
                table_type: input
                    .get("TableType")
                    .and_then(Value::as_str)
                    .map(String::from),
                parameters,
                created: now,
                updated: now,
            },
        );
        Ok(json!({}))
    }

    fn get_table(&self, req: &Value) -> Result<Value, AwsError> {
        let database_name = required(req, "DatabaseName")?;
        let name = required(req, "Name")?;
        let s = self.state.read();
        let db = s
            .databases
            .get(&database_name)
            .ok_or_else(|| not_found(&database_name))?;
        let table = db.tables.get(&name).ok_or_else(|| not_found(&name))?;
        Ok(json!({ "Table": table_json(table) }))
    }

    fn get_tables(&self, req: &Value) -> Result<Value, AwsError> {
        let database_name = required(req, "DatabaseName")?;
        let s = self.state.read();
        let db = s
            .databases
            .get(&database_name)
            .ok_or_else(|| not_found(&database_name))?;
        let tables: Vec<_> = db.tables.values().map(table_json).collect();
        Ok(json!({ "TableList": tables }))
    }

    fn delete_table(&self, req: &Value) -> Result<Value, AwsError> {
        let database_name = required(req, "DatabaseName")?;
        let name = required(req, "Name")?;
        let mut s = self.state.write();
        let db = s
            .databases
            .get_mut(&database_name)
            .ok_or_else(|| not_found(&database_name))?;
        db.tables.remove(&name).ok_or_else(|| not_found(&name))?;
        Ok(json!({}))
    }

    fn create_crawler(&self, req: &Value) -> Result<Value, AwsError> {
        let name = required(req, "Name")?;
        let role = required(req, "Role")?;
        let mut s = self.state.write();
        if s.crawlers.contains_key(&name) {
            return Err(AwsError::new(
                "AlreadyExistsException",
                format!("crawler '{name}' already exists"),
            ));
        }
        s.crawlers.insert(
            name.clone(),
            Crawler {
                name,
                role,
                database_name: req
                    .get("DatabaseName")
                    .and_then(Value::as_str)
                    .map(String::from),
                targets: req.get("Targets").cloned().unwrap_or(Value::Null),
                state: "READY".into(),
                created: chrono::Utc::now(),
            },
        );
        Ok(json!({}))
    }

    fn get_crawler(&self, req: &Value) -> Result<Value, AwsError> {
        let name = required(req, "Name")?;
        let s = self.state.read();
        let c = s.crawlers.get(&name).ok_or_else(|| not_found(&name))?;
        Ok(json!({ "Crawler": crawler_json(c) }))
    }

    fn get_crawlers(&self) -> Result<Value, AwsError> {
        let s = self.state.read();
        let crawlers: Vec<_> = s.crawlers.values().map(crawler_json).collect();
        Ok(json!({ "Crawlers": crawlers }))
    }

    fn delete_crawler(&self, req: &Value) -> Result<Value, AwsError> {
        let name = required(req, "Name")?;
        self.state
            .write()
            .crawlers
            .remove(&name)
            .ok_or_else(|| not_found(&name))?;
        Ok(json!({}))
    }

    fn start_crawler(&self, req: &Value) -> Result<Value, AwsError> {
        let name = required(req, "Name")?;
        let mut s = self.state.write();
        let c = s.crawlers.get_mut(&name).ok_or_else(|| not_found(&name))?;
        c.state = "READY".into();
        Ok(json!({}))
    }
}

fn required(req: &Value, key: &str) -> Result<String, AwsError> {
    req.get(key)
        .and_then(Value::as_str)
        .map(String::from)
        .ok_or_else(|| AwsError::new("InvalidInputException", format!("{key} required")))
}

fn not_found(name: &str) -> AwsError {
    AwsError::new(
        "EntityNotFoundException",
        format!("entity '{name}' not found"),
    )
}

fn database_json(db: &Database) -> Value {
    json!({
        "Name": db.name,
        "Description": db.description,
        "LocationUri": db.location_uri,
        "CreateTime": db.created.timestamp(),
        "Parameters": {},
    })
}

fn table_json(t: &Table) -> Value {
    json!({
        "Name": t.name,
        "DatabaseName": t.database_name,
        "Description": t.description,
        "StorageDescriptor": t.storage_descriptor,
        "TableType": t.table_type,
        "Parameters": t.parameters,
        "CreateTime": t.created.timestamp(),
        "UpdateTime": t.updated.timestamp(),
    })
}

fn crawler_json(c: &Crawler) -> Value {
    json!({
        "Name": c.name,
        "Role": c.role,
        "DatabaseName": c.database_name,
        "Targets": c.targets,
        "State": c.state,
        "CreationTime": c.created.timestamp(),
    })
}

pub fn register(registry: &Arc<crate::registry::Registry>) {
    registry.register_json(Arc::new(Glue::new()));
}
