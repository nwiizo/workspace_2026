use serde::Serialize;

/// State of a component during simulation (LTS state space).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ComponentState {
    Healthy,
    Degraded,
    Failed,
}

/// How a failure propagated (LTS action).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FailureMode {
    /// Initial component that was explicitly failed.
    Origin,
    /// Direct dependency failure (sync + critical/high + no fallback).
    DirectDependency,
    /// Cascade propagation through transitive dependencies.
    CascadePropagation,
    /// Partial failure (async, has fallback, or low criticality).
    Degraded,
}

/// One step in the cascade failure path.
#[derive(Debug, Clone, Serialize)]
pub struct CascadeStep {
    pub component_id: String,
    pub component_name: String,
    pub depth: usize,
    pub state: ComponentState,
    pub failure_mode: FailureMode,
    pub propagation_probability: f64,
}

/// Blast radius of a cascade failure.
#[derive(Debug, Clone, Serialize)]
pub struct BlastRadius {
    pub directly_affected: Vec<String>,
    pub transitively_affected: Vec<String>,
    pub total_affected: usize,
    pub total_components: usize,
    pub impact_percentage: f64,
}

/// Full result of a cascade failure simulation.
#[derive(Debug, Clone, Serialize)]
pub struct CascadeResult {
    pub origin_component: String,
    pub cascade_path: Vec<CascadeStep>,
    pub blast_radius: BlastRadius,
    pub estimated_recovery_seconds: f64,
    pub severity: Severity,
}

/// Severity classification based on impact percentage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Minimal,  // < 10%
    Moderate, // 10-30%
    Major,    // 30-60%
    Critical, // > 60%
}

impl Severity {
    pub fn from_impact(pct: f64) -> Self {
        if pct >= 60.0 {
            Self::Critical
        } else if pct >= 30.0 {
            Self::Major
        } else if pct >= 10.0 {
            Self::Moderate
        } else {
            Self::Minimal
        }
    }
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Minimal => write!(f, "MINIMAL"),
            Self::Moderate => write!(f, "MODERATE"),
            Self::Major => write!(f, "MAJOR"),
            Self::Critical => write!(f, "CRITICAL"),
        }
    }
}

/// SPOF entry.
#[derive(Debug, Clone, Serialize)]
pub struct SpofEntry {
    pub component_id: String,
    pub component_name: String,
    pub criticality_score: f64,
    pub components_at_risk: Vec<String>,
    pub is_articulation_point: bool,
    pub redundancy: u32,
    pub recommendation: String,
}

/// Bridge (critical edge).
#[derive(Debug, Clone, Serialize)]
pub struct BridgeEntry {
    pub from: String,
    pub to: String,
    pub criticality_score: f64,
}

/// Full SPOF analysis result.
#[derive(Debug, Clone, Serialize)]
pub struct SpofResult {
    pub single_points_of_failure: Vec<SpofEntry>,
    pub bridges: Vec<BridgeEntry>,
    pub resilience_score: f64,
}
