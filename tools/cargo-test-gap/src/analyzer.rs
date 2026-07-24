use std::fs;
use std::path::{Path, PathBuf};

use cargo_metadata::MetadataCommand;
use design_gate_core::{
    RustFileWalkerOptions, apply_suppressions as apply_core_suppressions, relative_path, rust_files,
};
use ra_ap_syntax::Edition;
use rayon::prelude::*;
use serde::Serialize;

use crate::blind_spot::{BlindSpotManifest, build as build_blind_spots};
use crate::churn::ChurnMap;
use crate::coverage::CoverageMap;
use crate::error::{Error, Result};
use crate::issue::{Issue, IssueKey, IssueType};
use crate::parser::{FunctionInfo, parse_file};
use crate::scoring::{Grade, grade, risk, severity_for_risk};

#[derive(Debug, Clone, Default)]
pub struct AnalyzeOptions {
    pub llvm_cov: Option<PathBuf>,
}

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

#[derive(Debug, Clone, Copy)]
struct CargoContext {
    edition: Edition,
    metadata_failed: bool,
    edition_fallback_2024: bool,
}

pub fn analyze_path(path: &Path, options: &AnalyzeOptions) -> Result<Analysis> {
    let root = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let cargo = cargo_context(&root);
    let project = project_name(&root);
    let files = analysis_files(&root)?;
    if files.is_empty() {
        return Err(Error::NoRustFiles(root));
    }
    let parsed = files
        .par_iter()
        .map(|file| parse_file(&root, file, cargo.edition))
        .collect::<Result<Vec<_>>>()?;
    let files_analyzed = parsed.len();
    let parse_failures = parsed.iter().map(|file| file.parse_errors).sum();
    let functions = parsed
        .into_iter()
        .flat_map(|file| file.functions)
        .collect::<Vec<_>>();
    let coverage_approximated = options.llvm_cov.is_none();
    let coverage = if let Some(path) = &options.llvm_cov {
        CoverageMap::from_llvm_cov(&root, path)?
    } else {
        CoverageMap::from_reachability(&functions)
    };
    let churn = ChurnMap::collect(churn_root(&root), &functions);

    let mut git_churn_failed = false;
    let mut llvm_cov_matches = 0usize;
    let mut issues = Vec::new();
    for function in functions.iter().filter(|function| !function.is_test) {
        let churn_result = churn.churn_for(function);
        git_churn_failed |= churn_result.failed;
        let coverage_match = coverage.coverage_match_for(function);
        if options.llvm_cov.is_some() && coverage_match.is_some() {
            llvm_cov_matches += 1;
        }
        let coverage = coverage_match.unwrap_or(0.0);
        let exposure = exposure(function);
        let risk = risk(churn_result.score, function.complexity, exposure, coverage);
        issues.push(issue_for(
            &root,
            function,
            churn_result.score,
            exposure,
            coverage,
            risk,
        ));
    }

    let suppression = apply_suppressions(issues)?;
    let mut issues = suppression.kept;
    for issue in &mut issues {
        issue.file = relative_path(&root, &issue.file);
    }
    issues.sort_by(|a, b| {
        b.risk
            .total_cmp(&a.risk)
            .then_with(|| b.severity.cmp(&a.severity))
            .then_with(|| a.file.cmp(&b.file))
            .then_with(|| a.line.cmp(&b.line))
            .then_with(|| a.key.source.cmp(&b.key.source))
    });
    issues.dedup_by(|a, b| a.key == b.key);
    let grade = grade(&issues);
    let llvm_cov_unmatched = options.llvm_cov.is_some()
        && !issues.is_empty()
        && (coverage.is_empty() || llvm_cov_matches == 0);
    if llvm_cov_unmatched {
        eprintln!(
            "warning: llvm-cov JSON did not match any production function; coverage falls back to 0% for all candidates"
        );
    }
    let blind_spots = build_blind_spots(
        parse_failures,
        cargo.metadata_failed,
        cargo.edition_fallback_2024,
        git_churn_failed,
        coverage_approximated,
        llvm_cov_unmatched,
    );
    Ok(Analysis {
        project,
        root,
        files_analyzed,
        suppressed_issues: suppression.suppressed,
        grade,
        issues,
        blind_spots,
    })
}

fn issue_for(
    root: &Path,
    function: &FunctionInfo,
    churn: f64,
    exposure: f64,
    coverage: f64,
    risk: f64,
) -> Issue {
    let severity = severity_for_risk(risk);
    let target = factor_summary(function, coverage);
    let rel_file = relative_path(root, &function.file);
    Issue {
        key: IssueKey {
            issue_type: IssueType::TestGap,
            source: function.id.clone(),
            target,
        },
        severity,
        file: function.file.clone(),
        line: function.line,
        function: function.qualified_name.clone(),
        risk,
        churn,
        complexity: function.complexity,
        exposure,
        coverage,
        message: format!(
            "`{}` ranks with risk {:.2} (churn {:.1}, complexity {}, exposure {:.1}, coverage {:.1}%).",
            function.qualified_name, risk, churn, function.complexity, exposure, coverage
        ),
        remediation: format!(
            "Start with focused tests around `{}` in {} before lower-risk code.",
            function.qualified_name,
            rel_file.display()
        ),
    }
}

fn factor_summary(function: &FunctionInfo, _coverage: f64) -> String {
    let mut factors = Vec::new();
    factors.push(if function.is_public {
        "public-api".to_string()
    } else {
        "internal".to_string()
    });
    if function.returns_result {
        factors.push("result-return".to_string());
    }
    factors.sort();
    factors.join(";")
}

fn exposure(function: &FunctionInfo) -> f64 {
    let mut score = 1.0;
    if function.is_public {
        score += 3.0;
    }
    if function.returns_result {
        score += 2.0;
    }
    score
}

fn apply_suppressions(issues: Vec<Issue>) -> Result<design_gate_core::SuppressionResult<Issue>> {
    Ok(apply_core_suppressions(
        issues,
        |issue| issue.file.as_path(),
        |issue| issue.line,
        |issue| issue.key.issue_type.id(),
        "test-gap",
        |marker, issue| marker == issue || marker == "all",
    )?)
}

fn analysis_files(root: &Path) -> Result<Vec<PathBuf>> {
    let options = RustFileWalkerOptions {
        prefer_src: false,
        on_no_files: None,
    };
    if root.is_file() {
        return Ok(rust_files(root, options)?);
    }
    let mut files = Vec::new();
    for dir in ["src", "tests", "benches", "examples"] {
        let scan_root = root.join(dir);
        if scan_root.is_dir() {
            files.extend(rust_files(&scan_root, options)?);
        }
    }
    if files.is_empty() {
        files = rust_files(root, options)?;
    }
    files.sort();
    files.dedup();
    Ok(files)
}

fn churn_root(root: &Path) -> &Path {
    if root.is_file() {
        root.parent().unwrap_or(root)
    } else {
        root
    }
}

fn project_name(root: &Path) -> String {
    let manifest = manifest_path(root);
    if let Some(manifest) = manifest.filter(|path| path.is_file()) {
        let mut command = MetadataCommand::new();
        command.manifest_path(manifest);
        if let Ok(metadata) = command.no_deps().exec()
            && let Some(package) = metadata.root_package()
        {
            return package.name.to_string();
        }
    }
    root.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("project")
        .to_string()
}

fn cargo_context(root: &Path) -> CargoContext {
    let Some(manifest) = manifest_path(root).filter(|path| path.is_file()) else {
        return CargoContext {
            edition: Edition::Edition2024,
            metadata_failed: true,
            edition_fallback_2024: true,
        };
    };
    let mut command = MetadataCommand::new();
    command.manifest_path(&manifest);
    match command.no_deps().exec() {
        Ok(metadata) => {
            let edition = metadata
                .root_package()
                .and_then(|package| parse_edition(package.edition.to_string().as_str()));
            CargoContext {
                edition: edition.unwrap_or(Edition::Edition2024),
                metadata_failed: false,
                edition_fallback_2024: edition.is_none(),
            }
        }
        Err(_) => {
            let edition = read_manifest_edition(&manifest);
            CargoContext {
                edition: edition.unwrap_or(Edition::Edition2024),
                metadata_failed: true,
                edition_fallback_2024: edition.is_none(),
            }
        }
    }
}

fn manifest_path(root: &Path) -> Option<PathBuf> {
    if root.is_file() {
        root.parent().map(|parent| parent.join("Cargo.toml"))
    } else {
        Some(root.join("Cargo.toml"))
    }
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
