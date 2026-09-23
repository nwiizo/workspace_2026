use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use cargo_metadata::MetadataCommand;
use design_gate_core::{
    RustFileWalkerOptions, apply_suppressions as apply_core_suppressions, relative_path_string,
    rust_files,
};
use ra_ap_syntax::Edition;
use rayon::prelude::*;
use serde::Serialize;

use crate::blind_spot::{BlindSpotManifest, build as build_blind_spots};
use crate::config::{Config, parse_issue_type};
use crate::error::{Error, Result};
use crate::issue::{Issue, IssueKey, IssueType, RiskAxis};
use crate::orphan::{self, OrphanScan};
use crate::parser::{CONDITION_HIGH, ParsedFile, parse_file_from_disk};
use crate::scoring::{Grade, grade, severity};

#[derive(Debug, Clone, Serialize)]
pub struct Analysis {
    pub project: String,
    pub root: PathBuf,
    pub files_analyzed: usize,
    pub suppressed_issues: usize,
    pub grade: Grade,
    pub issues: Vec<Issue>,
    pub blind_spots: BlindSpotManifest,
}

#[derive(Debug, Clone)]
struct CargoContext {
    project: String,
    edition: Edition,
    metadata_failed: bool,
    edition_fallback_2024: bool,
}

#[derive(Debug, Clone)]
struct VolatilityContext {
    counts: HashMap<String, usize>,
    unavailable: bool,
    shallow: bool,
}

#[derive(Debug, Clone)]
struct SuppressionCandidate {
    issue: Issue,
    abs_file: PathBuf,
}

pub fn analyze_path(path: &Path, config: &Config) -> Result<Analysis> {
    let root = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let cargo = cargo_context(&root);
    let files = rust_files(
        &root,
        RustFileWalkerOptions {
            prefer_src: true,
            on_no_files: None,
        },
    )?;
    if files.is_empty() {
        return Err(Error::NoRustFiles(root));
    }

    let parsed_results: Vec<Result<ParsedFile>> = files
        .par_iter()
        .map(|file| parse_file_from_disk(&root, file, cargo.edition, config))
        .collect();

    let mut parsed = Vec::new();
    for result in parsed_results {
        parsed.push(result?);
    }

    let orphan_scan = if config.is_allowed(IssueType::OrphanFile) {
        OrphanScan::default()
    } else {
        orphan::scan(&root, &files, cargo.edition)
    };
    let orphan_rel: HashSet<String> = orphan_scan
        .orphans
        .iter()
        .map(|path| relative_path_string(&root, path))
        .collect();

    let volatility = git_volatility(&root);
    let parse_failures: usize = parsed.iter().map(|file| file.parse_errors).sum();
    let mut issues = Vec::new();
    for file in &parsed {
        for finding in &file.findings {
            // An unreachable file is never compiled; report the orphan
            // itself instead of its contents.
            if orphan_rel.contains(&finding.rel_path) {
                continue;
            }
            let count = volatility
                .counts
                .get(&finding.rel_path)
                .copied()
                .unwrap_or(0);
            let score = match count {
                0 => 0,
                1..=2 => 1,
                _ => 2,
            };
            let volatility_score = (!volatility.unavailable).then_some(score);
            issues.push(SuppressionCandidate {
                abs_file: finding.file.clone(),
                issue: Issue {
                    key: IssueKey {
                        issue_type: finding.issue_type,
                        source: finding.function.clone(),
                        target: finding.target.clone(),
                    },
                    severity: severity(finding.risk, finding.condition, volatility_score),
                    file: PathBuf::from(&finding.rel_path),
                    rel_path: finding.rel_path.clone(),
                    line: finding.line,
                    risk: finding.risk,
                    message: finding.message.clone(),
                    remediation: finding.remediation.clone(),
                    volatility: count,
                },
            });
        }
    }

    for path in &orphan_scan.orphans {
        let rel = relative_path_string(&root, path);
        let count = volatility.counts.get(&rel).copied().unwrap_or(0);
        let score = match count {
            0 => 0,
            1..=2 => 1,
            _ => 2,
        };
        let volatility_score = (!volatility.unavailable).then_some(score);
        issues.push(SuppressionCandidate {
            abs_file: path.clone(),
            issue: Issue {
                key: IssueKey {
                    issue_type: IssueType::OrphanFile,
                    source: rel.clone(),
                    target: "unreachable".to_string(),
                },
                severity: severity(RiskAxis::Signal, CONDITION_HIGH, volatility_score),
                file: PathBuf::from(&rel),
                rel_path: rel.clone(),
                line: 1,
                risk: RiskAxis::Signal,
                message: format!("`{rel}` is not reachable from the crate's module tree"),
                remediation:
                    "Delete the leftover file or declare it with `mod ...;`; unreachable files are never compiled and mislead readers and agents."
                        .to_string(),
                volatility: count,
            },
        });
    }

    let suppression = apply_suppressions(issues)?;
    let mut issues: Vec<Issue> = suppression
        .kept
        .into_iter()
        .map(|candidate| candidate.issue)
        .collect();
    issues.sort_by(|a, b| {
        b.severity
            .cmp(&a.severity)
            .then_with(|| a.file.cmp(&b.file))
            .then_with(|| a.line.cmp(&b.line))
            .then_with(|| a.key.issue_type.id().cmp(b.key.issue_type.id()))
            .then_with(|| a.key.target.cmp(&b.key.target))
    });
    issues.dedup_by(|left, right| left.key == right.key);
    let grade = grade(&issues);
    let blind_spots = build_blind_spots(
        parse_failures,
        cargo.metadata_failed,
        cargo.edition_fallback_2024,
        volatility.unavailable,
        volatility.shallow,
        orphan_scan.skipped,
        orphan_scan.unreliable,
    );

    Ok(Analysis {
        project: cargo.project,
        root,
        files_analyzed: parsed.len(),
        suppressed_issues: suppression.suppressed,
        grade,
        issues,
        blind_spots,
    })
}

fn apply_suppressions(
    issues: Vec<SuppressionCandidate>,
) -> Result<design_gate_core::SuppressionResult<SuppressionCandidate>> {
    Ok(apply_core_suppressions(
        issues,
        |candidate| candidate.abs_file.as_path(),
        |candidate| candidate.issue.line,
        |candidate| candidate.issue.key.issue_type.id(),
        "anti-slop",
        |entry, issue_type| parse_issue_type(entry).is_some_and(|parsed| parsed.id() == issue_type),
    )?)
}

fn cargo_context(root: &Path) -> CargoContext {
    let manifest = if root.is_file() {
        root.parent().map(|parent| parent.join("Cargo.toml"))
    } else {
        Some(root.join("Cargo.toml"))
    };
    let Some(manifest) = manifest.filter(|path| path.is_file()) else {
        return CargoContext {
            project: fallback_project_name(root),
            edition: Edition::Edition2024,
            metadata_failed: true,
            edition_fallback_2024: true,
        };
    };
    let mut command = MetadataCommand::new();
    command.manifest_path(&manifest);
    match command.no_deps().exec() {
        Ok(metadata) => {
            let package = metadata.root_package();
            let edition =
                package.and_then(|package| parse_edition(package.edition.to_string().as_str()));
            let project = package
                .map(|package| package.name.to_string())
                .unwrap_or_else(|| fallback_project_name(root));
            CargoContext {
                project,
                edition: edition.unwrap_or(Edition::Edition2024),
                metadata_failed: false,
                edition_fallback_2024: edition.is_none(),
            }
        }
        Err(_) => {
            let edition = read_manifest_edition(&manifest);
            CargoContext {
                project: fallback_project_name(root),
                edition: edition.unwrap_or(Edition::Edition2024),
                metadata_failed: true,
                edition_fallback_2024: edition.is_none(),
            }
        }
    }
}

fn fallback_project_name(root: &Path) -> String {
    root.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("project")
        .to_string()
}

fn parse_edition(value: &str) -> Option<Edition> {
    match value {
        "2015" => Some(Edition::Edition2015),
        "2018" => Some(Edition::Edition2018),
        "2021" => Some(Edition::Edition2021),
        "2024" => Some(Edition::Edition2024),
        _ => None,
    }
}

fn read_manifest_edition(manifest: &Path) -> Option<Edition> {
    let source = fs::read_to_string(manifest).ok()?;
    let value: toml::Value = toml::from_str(&source).ok()?;
    value
        .get("package")?
        .get("edition")?
        .as_str()
        .and_then(parse_edition)
}

fn git_volatility(root: &Path) -> VolatilityContext {
    // A shallow clone (CI checkouts default to depth 1) lists every file in
    // its single commit, which would inflate every finding by one volatility
    // notch; disable the axis instead of reporting uniform noise.
    if is_shallow_repository(root) {
        return VolatilityContext {
            counts: HashMap::new(),
            unavailable: true,
            shallow: true,
        };
    }
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["log", "--since=180 days", "--name-only", "--pretty=format:"])
        .output();
    let Ok(output) = output else {
        return VolatilityContext {
            counts: HashMap::new(),
            unavailable: true,
            shallow: false,
        };
    };
    if !output.status.success() {
        return VolatilityContext {
            counts: HashMap::new(),
            unavailable: true,
            shallow: false,
        };
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let mut counts = HashMap::new();
    for line in text.lines().map(str::trim).filter(|line| !line.is_empty()) {
        *counts.entry(line.replace('\\', "/")).or_insert(0) += 1;
    }
    VolatilityContext {
        counts,
        unavailable: false,
        shallow: false,
    }
}

fn is_shallow_repository(root: &Path) -> bool {
    Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["rev-parse", "--is-shallow-repository"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .is_some_and(|output| String::from_utf8_lossy(&output.stdout).trim() == "true")
}
