//! Serde shapes for `eval.yaml` and `tasks/*.yaml`.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum GraderType {
    Text,
    Code,
    SelfReport,
    Llm,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Grader {
    pub name: String,
    #[serde(rename = "type")]
    pub kind: GraderType,
    #[serde(default)]
    pub config: BTreeMap<String, serde_yaml::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EvalRunOptions {
    #[serde(default)]
    pub trials_per_task: Option<u32>,
    #[serde(default)]
    pub timeout_seconds: Option<u64>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub parallel: Option<bool>,
    #[serde(default)]
    pub workers: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalConfig {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    pub skill: String,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default, rename = "config")]
    pub options: Option<EvalRunOptions>,
    #[serde(default)]
    pub graders: Vec<Grader>,
    pub tasks: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TaskInputs {
    pub prompt: String,
    #[serde(default)]
    pub context: Option<serde_yaml::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TaskExpected {
    #[serde(default)]
    pub output_contains: Vec<String>,
    /// When false, the executor is not asked to append a Self-report block.
    /// Default is true.
    #[serde(default)]
    pub require_self_report: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    pub inputs: TaskInputs,
    #[serde(default)]
    pub expected: Option<TaskExpected>,
    #[serde(default)]
    pub graders: Vec<Grader>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProjectConfig {
    #[serde(default)]
    pub paths: ProjectPaths,
    #[serde(default)]
    pub defaults: ProjectDefaults,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProjectPaths {
    #[serde(default = "default_skills")]
    pub skills: String,
    #[serde(default = "default_evals")]
    pub evals: String,
    #[serde(default = "default_results")]
    pub results: String,
}

fn default_skills() -> String {
    ".".into()
}
fn default_evals() -> String {
    "evals/".into()
}
fn default_results() -> String {
    "results/".into()
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProjectDefaults {
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub timeout: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraderResult {
    pub name: String,
    pub pass: bool,
    pub score: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    pub duration_ms: u128,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PhaseStatus {
    Ok,
    Stuck,
    Skipped,
    Missing,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhaseEntry {
    pub phase: String,
    pub status: PhaseStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnclearPoint {
    pub issue: String,
    pub cause: String,
    pub rule: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SelfReport {
    #[serde(default)]
    pub phase_trace: Vec<PhaseEntry>,
    #[serde(default)]
    pub unclear_points: Vec<UnclearPoint>,
    #[serde(default)]
    pub discretionary_fill_ins: Vec<String>,
    #[serde(default)]
    pub retries: u32,
    pub raw: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskTrial {
    pub trial: u32,
    pub output: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub self_report: Option<SelfReport>,
    pub graders: Vec<GraderResult>,
    pub pass_rate: f64,
    pub duration_ms: u128,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    pub task_id: String,
    pub task_name: String,
    pub trials: Vec<TaskTrial>,
    pub pass_rate: f64,
    pub duration_ms: u128,
}
