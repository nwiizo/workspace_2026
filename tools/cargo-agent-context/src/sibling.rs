use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

use crate::error::{Error, Result};

#[derive(Debug, Clone)]
pub struct SiblingReport {
    pub tool: &'static str,
    pub status: SiblingStatus,
    pub grade: Option<String>,
    pub issue_count: Option<usize>,
    pub top_issues: Vec<SiblingIssue>,
    pub blind_spots: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SiblingStatus {
    Loaded,
    Ran,
    NotProvided,
    NotRun,
    RunFailed(String),
    SchemaMismatch(String),
}

#[derive(Debug, Clone)]
pub struct SiblingIssue {
    pub severity: String,
    pub kind: String,
    pub location: String,
    pub message: String,
}

#[derive(Debug, Clone, Copy)]
struct ToolSpec {
    tool: &'static str,
    binary: &'static str,
    json_file: &'static str,
}

const TOOLS: &[ToolSpec] = &[
    ToolSpec {
        tool: "cargo-boundary",
        binary: "cargo-boundary",
        json_file: "boundary.json",
    },
    ToolSpec {
        tool: "cargo-error-map",
        binary: "cargo-error-map",
        json_file: "error-map.json",
    },
    ToolSpec {
        tool: "cargo-async-smell",
        binary: "cargo-async-smell",
        json_file: "async-smell.json",
    },
    ToolSpec {
        tool: "cargo-trait-surface",
        binary: "cargo-trait-surface",
        json_file: "trait-surface.json",
    },
    ToolSpec {
        tool: "cargo-feature-doctor",
        binary: "cargo-feature-doctor",
        json_file: "feature-doctor.json",
    },
    ToolSpec {
        tool: "cargo-test-gap",
        binary: "cargo-test-gap",
        json_file: "test-gap.json",
    },
    ToolSpec {
        tool: "cargo-api-drift",
        binary: "cargo-api-drift",
        json_file: "api-drift.json",
    },
];

pub fn collect_sibling_reports(
    root: &Path,
    from: Option<&Path>,
    run: bool,
) -> Result<Vec<SiblingReport>> {
    if let Some(dir) = from {
        return read_from_dir(dir);
    }
    if run {
        return run_tools(root);
    }
    Ok(TOOLS.iter().map(not_run).collect())
}

fn read_from_dir(dir: &Path) -> Result<Vec<SiblingReport>> {
    let mut reports = Vec::new();
    for spec in TOOLS {
        let path = dir.join(spec.json_file);
        if !path.is_file() {
            reports.push(SiblingReport {
                tool: spec.tool,
                status: SiblingStatus::NotProvided,
                grade: None,
                issue_count: None,
                top_issues: Vec::new(),
                blind_spots: Vec::new(),
            });
            continue;
        }
        let text = fs::read_to_string(&path).map_err(|source| Error::ReadFile {
            path: path.clone(),
            source,
        })?;
        match parse_json(spec.tool, &text) {
            Ok(mut report) => {
                report.status = SiblingStatus::Loaded;
                reports.push(report);
            }
            Err(Error::SiblingSchema { reason, .. }) => reports.push(SiblingReport {
                tool: spec.tool,
                status: SiblingStatus::SchemaMismatch(reason),
                grade: None,
                issue_count: None,
                top_issues: Vec::new(),
                blind_spots: Vec::new(),
            }),
            Err(Error::JsonFile { source, .. }) => reports.push(SiblingReport {
                tool: spec.tool,
                status: SiblingStatus::SchemaMismatch(source.to_string()),
                grade: None,
                issue_count: None,
                top_issues: Vec::new(),
                blind_spots: Vec::new(),
            }),
            Err(err) => return Err(err),
        }
    }
    Ok(reports)
}

fn run_tools(root: &Path) -> Result<Vec<SiblingReport>> {
    let mut reports = Vec::new();
    let mut found = 0usize;
    for spec in TOOLS {
        let Some(binary) = find_binary(root, spec) else {
            reports.push(SiblingReport {
                tool: spec.tool,
                status: SiblingStatus::NotRun,
                grade: None,
                issue_count: None,
                top_issues: Vec::new(),
                blind_spots: Vec::new(),
            });
            continue;
        };
        found += 1;
        let output = Command::new(&binary)
            .arg(root)
            .arg("--json")
            .output()
            .map_err(|source| Error::ReadFile {
                path: binary.clone(),
                source,
            })?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            reports.push(SiblingReport {
                tool: spec.tool,
                status: SiblingStatus::RunFailed(first_line(&stderr)),
                grade: None,
                issue_count: None,
                top_issues: Vec::new(),
                blind_spots: Vec::new(),
            });
            continue;
        }
        let stdout = String::from_utf8_lossy(&output.stdout);
        match parse_json(spec.tool, &stdout) {
            Ok(mut report) => {
                report.status = SiblingStatus::Ran;
                reports.push(report);
            }
            Err(Error::SiblingSchema { reason, .. }) => reports.push(SiblingReport {
                tool: spec.tool,
                status: SiblingStatus::SchemaMismatch(reason),
                grade: None,
                issue_count: None,
                top_issues: Vec::new(),
                blind_spots: Vec::new(),
            }),
            Err(Error::JsonFile { source, .. }) => reports.push(SiblingReport {
                tool: spec.tool,
                status: SiblingStatus::SchemaMismatch(source.to_string()),
                grade: None,
                issue_count: None,
                top_issues: Vec::new(),
                blind_spots: Vec::new(),
            }),
            Err(err) => return Err(err),
        }
    }
    if found == 0 {
        return Ok(vec![SiblingReport {
            tool: "sibling-tools",
            status: SiblingStatus::NotRun,
            grade: None,
            issue_count: None,
            top_issues: Vec::new(),
            blind_spots: vec!["no sibling tools available".to_string()],
        }]);
    }
    Ok(reports)
}

fn parse_json(tool: &'static str, text: &str) -> Result<SiblingReport> {
    let value: Value = serde_json::from_str(text).map_err(|source| Error::JsonFile {
        path: PathBuf::from(tool),
        source,
    })?;
    let grade = value
        .get("grade")
        .and_then(Value::as_str)
        .map(ToString::to_string)
        .ok_or_else(|| Error::SiblingSchema {
            tool: tool.to_string(),
            reason: "missing string field `grade`".to_string(),
        })?;
    let issues = value
        .get("issues")
        .and_then(Value::as_array)
        .ok_or_else(|| Error::SiblingSchema {
            tool: tool.to_string(),
            reason: "missing array field `issues`".to_string(),
        })?;
    let mut top_issues = issues.iter().map(issue_from_value).collect::<Vec<_>>();
    top_issues.sort_by(|a, b| {
        severity_rank(&b.severity)
            .cmp(&severity_rank(&a.severity))
            .then_with(|| a.kind.cmp(&b.kind))
            .then_with(|| a.location.cmp(&b.location))
            .then_with(|| a.message.cmp(&b.message))
    });
    top_issues.truncate(5);
    Ok(SiblingReport {
        tool,
        status: SiblingStatus::Loaded,
        grade: Some(grade),
        issue_count: Some(issues.len()),
        top_issues,
        blind_spots: blind_spots(&value),
    })
}

fn issue_from_value(value: &Value) -> SiblingIssue {
    let severity = string_field(value, &["severity"]).unwrap_or_else(|| "unknown".to_string());
    let kind = value
        .get("key")
        .and_then(|key| {
            string_field(
                key,
                &[
                    "issue_type",
                    "issueType",
                    "kind",
                    "change_kind",
                    "classification",
                ],
            )
        })
        .or_else(|| string_field(value, &["kind", "issue_type", "change_kind"]))
        .unwrap_or_else(|| "issue".to_string());
    let message = string_field(value, &["message", "summary", "description"])
        .unwrap_or_else(|| compact_value(value));
    let location = value
        .get("key")
        .and_then(|key| string_field(key, &["source", "target"]))
        .or_else(|| location_from_locations(value.get("locations")))
        .or_else(|| string_field(value, &["file", "path", "source"]))
        .unwrap_or_else(|| "-".to_string());
    SiblingIssue {
        severity,
        kind,
        location,
        message: truncate(&message, 120),
    }
}

fn blind_spots(value: &Value) -> Vec<String> {
    let Some(manifest) = value.get("blind_spots") else {
        return Vec::new();
    };
    let mut spots = Vec::new();
    if let Some(items) = manifest.get("items").and_then(Value::as_array) {
        for item in items {
            if let Some(text) = string_field(item, &["description", "id"]) {
                spots.push(text);
            }
        }
    }
    if let Some(items) = manifest.get("blind_spots").and_then(Value::as_array) {
        for item in items {
            if let Some(text) = string_field(item, &["description", "id"]) {
                spots.push(text);
            }
        }
    }
    if let Some(notes) = manifest.get("notes").and_then(Value::as_array) {
        for note in notes {
            if let Some(text) = note.as_str() {
                spots.push(text.to_string());
            }
        }
    }
    spots.sort();
    spots.dedup();
    spots.truncate(6);
    spots
}

fn find_binary(root: &Path, spec: &ToolSpec) -> Option<PathBuf> {
    // Sibling checkouts are rebuilt in place; a PATH install can be stale.
    find_nearby(root, spec.binary).or_else(|| find_in_path(spec.binary))
}

fn find_in_path(binary: &str) -> Option<PathBuf> {
    let path = env::var_os("PATH")?;
    env::split_paths(&path)
        .map(|dir| dir.join(binary))
        .find(|candidate| candidate.is_file())
}

fn find_nearby(root: &Path, binary: &str) -> Option<PathBuf> {
    let tools_root = root.parent()?;
    let tool_dir = tools_root.join(binary);
    [
        tool_dir.join("target/release").join(binary),
        tool_dir.join("target/debug").join(binary),
    ]
    .into_iter()
    .find(|candidate| candidate.is_file())
}

fn not_run(spec: &ToolSpec) -> SiblingReport {
    SiblingReport {
        tool: spec.tool,
        status: SiblingStatus::NotRun,
        grade: None,
        issue_count: None,
        top_issues: Vec::new(),
        blind_spots: Vec::new(),
    }
}

fn string_field(value: &Value, fields: &[&str]) -> Option<String> {
    fields
        .iter()
        .find_map(|field| value.get(*field).and_then(Value::as_str))
        .map(ToString::to_string)
}

fn location_from_locations(value: Option<&Value>) -> Option<String> {
    let first = value.and_then(Value::as_array)?.first()?;
    let file = string_field(first, &["file", "path"])?;
    let line = first.get("line").and_then(Value::as_u64).unwrap_or(0);
    if line == 0 {
        Some(file)
    } else {
        Some(format!("{file}:{line}"))
    }
}

fn compact_value(value: &Value) -> String {
    truncate(
        &value
            .to_string()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" "),
        120,
    )
}

fn truncate(value: &str, max: usize) -> String {
    if value.chars().count() <= max {
        return value.to_string();
    }
    value
        .chars()
        .take(max.saturating_sub(3))
        .collect::<String>()
        + "..."
}

fn first_line(value: &str) -> String {
    value
        .lines()
        .next()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(|line| truncate(line, 120))
        .unwrap_or_else(|| "command exited with non-zero status".to_string())
}

fn severity_rank(value: &str) -> usize {
    match value.to_ascii_lowercase().as_str() {
        "critical" => 4,
        "high" => 3,
        "medium" => 2,
        "low" => 1,
        _ => 0,
    }
}
