use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComponentType {
    Service,
    Database,
    Cache,
    Queue,
    LoadBalancer,
    Gateway,
    Storage,
    AiAgent,
    External,
}

impl fmt::Display for ComponentType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Service => write!(f, "service"),
            Self::Database => write!(f, "database"),
            Self::Cache => write!(f, "cache"),
            Self::Queue => write!(f, "queue"),
            Self::LoadBalancer => write!(f, "load_balancer"),
            Self::Gateway => write!(f, "gateway"),
            Self::Storage => write!(f, "storage"),
            Self::AiAgent => write!(f, "ai_agent"),
            Self::External => write!(f, "external"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DependencyType {
    Sync,
    Async,
    Batch,
    EventDriven,
}

impl fmt::Display for DependencyType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Sync => write!(f, "sync"),
            Self::Async => write!(f, "async"),
            Self::Batch => write!(f, "batch"),
            Self::EventDriven => write!(f, "event_driven"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Criticality {
    Low,
    Medium,
    High,
    Critical,
}

impl Criticality {
    pub fn weight(&self) -> f64 {
        match self {
            Self::Low => 0.25,
            Self::Medium => 0.5,
            Self::High => 0.75,
            Self::Critical => 1.0,
        }
    }
}

impl fmt::Display for Criticality {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Low => write!(f, "low"),
            Self::Medium => write!(f, "medium"),
            Self::High => write!(f, "high"),
            Self::Critical => write!(f, "critical"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Component {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub component_type: ComponentType,
    #[serde(default = "default_redundancy")]
    pub redundancy: u32,
    #[serde(default)]
    pub failure_probability: f64,
    #[serde(default)]
    pub recovery_time_seconds: f64,
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

fn default_redundancy() -> u32 {
    1
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dependency {
    pub from: String,
    pub to: String,
    #[serde(rename = "type")]
    pub dependency_type: DependencyType,
    pub criticality: Criticality,
    #[serde(default)]
    pub has_fallback: bool,
    #[serde(default)]
    pub has_retry: bool,
    #[serde(default)]
    pub timeout_seconds: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Topology {
    pub name: String,
    #[serde(default)]
    pub version: Option<String>,
    pub components: Vec<Component>,
    pub dependencies: Vec<Dependency>,
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}
