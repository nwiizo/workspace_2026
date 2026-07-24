use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use cargo_metadata::MetadataCommand;
use design_gate_core::{
    RustFileWalkerOptions, apply_suppressions as apply_core_suppressions, rust_files,
};
use ra_ap_syntax::Edition;
use rayon::prelude::*;
use serde::Serialize;

use crate::blind_spot::{BlindSpotManifest, build as build_blind_spots};
use crate::config::Config;
use crate::error::{Error, Result};
use crate::graph::{ErrorGraph, FunctionIndex};
use crate::issue::{Issue, IssueKey, IssueType, Layer};
use crate::parser::{FunctionInfo, ParsedFile, parse_file};
use crate::scoring::{Grade, grade, severity};

#[derive(Debug, Clone, Serialize)]
pub struct Analysis {
    pub project: String,
    pub root: PathBuf,
    pub files_analyzed: usize,
    pub suppressed_issues: usize,
    pub grade: Grade,
    pub issues: Vec<Issue>,
    pub graph: ErrorGraph,
    pub blind_spots: BlindSpotManifest,
}

#[derive(Debug, Clone, Copy)]
struct CargoContext {
    edition: Edition,
    metadata_failed: bool,
    edition_fallback_2024: bool,
}

pub fn analyze_path(path: &Path, config: &Config) -> Result<Analysis> {
    let root = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let cargo = cargo_context(&root);
    let project = project_name(&root);
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
        .map(|file| {
            let is_boundary = config.is_boundary_path(file);
            let is_lib_reachable = is_library_reachable(file);
            parse_file(&root, file, cargo.edition, is_lib_reachable, is_boundary)
        })
        .collect();

    let mut parsed = Vec::new();
    for result in parsed_results {
        parsed.push(result?);
    }

    let parse_failures: usize = parsed.iter().map(|file| file.parse_errors).sum();
    let functions: Vec<FunctionInfo> = parsed
        .iter()
        .flat_map(|file| file.functions.iter().cloned())
        .collect();
    let mut index = FunctionIndex::new(&functions);
    let fan_in = index.fan_in();
    let mut issues = Vec::new();

    detect_public_signature_issues(&parsed, &fan_in, config, &mut issues);
    detect_enum_bloat(&parsed, &fan_in, config, &mut issues);
    detect_boundary_panic(&parsed, &fan_in, config, &mut issues);
    detect_missing_context(&functions, &mut index, &fan_in, config, &mut issues);

    let suppression = apply_suppressions(issues)?;
    let mut issues = suppression.kept;
    for issue in &mut issues {
        if let Some((rel_path, _)) = issue.key.source.split_once(':') {
            issue.file = PathBuf::from(rel_path);
        }
    }
    issues.sort_by(|a, b| {
        b.severity
            .cmp(&a.severity)
            .then_with(|| a.file.cmp(&b.file))
            .then_with(|| a.line.cmp(&b.line))
            .then_with(|| a.key.issue_type.id().cmp(b.key.issue_type.id()))
    });
    issues.dedup_by(|a, b| a.key == b.key);
    let grade = grade(&issues);
    let graph = ErrorGraph::from_index(&mut index);
    let blind_spots = build_blind_spots(
        parse_failures,
        cargo.metadata_failed,
        cargo.edition_fallback_2024,
        index.used_bare_name_fallback(),
    );
    Ok(Analysis {
        project,
        root,
        files_analyzed: parsed.len(),
        suppressed_issues: suppression.suppressed,
        grade,
        issues,
        graph,
        blind_spots,
    })
}

fn project_name(root: &Path) -> String {
    let manifest = if root.is_file() {
        root.parent().map(|parent| parent.join("Cargo.toml"))
    } else {
        Some(root.join("Cargo.toml"))
    };
    if let Some(manifest) = manifest {
        if manifest.is_file() {
            let mut command = MetadataCommand::new();
            command.manifest_path(manifest);
            if let Ok(metadata) = command.no_deps().exec() {
                if let Some(package) = metadata.root_package() {
                    return package.name.to_string();
                }
            }
        }
    }
    root.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("project")
        .to_string()
}

fn cargo_context(root: &Path) -> CargoContext {
    let manifest = if root.is_file() {
        root.parent().map(|parent| parent.join("Cargo.toml"))
    } else {
        Some(root.join("Cargo.toml"))
    };
    let Some(manifest) = manifest.filter(|path| path.is_file()) else {
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
        Err(_) => CargoContext {
            edition: read_manifest_edition(&manifest).unwrap_or(Edition::Edition2024),
            metadata_failed: true,
            edition_fallback_2024: read_manifest_edition(&manifest).is_none(),
        },
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

fn is_library_reachable(path: &Path) -> bool {
    let normalized = path.to_string_lossy().replace('\\', "/");
    normalized.ends_with("/src/lib.rs")
        || (normalized.contains("/src/")
            && !normalized.ends_with("/src/main.rs")
            && !normalized.contains("/src/bin/"))
}

fn detect_public_signature_issues(
    parsed: &[ParsedFile],
    fan_in: &HashMap<String, usize>,
    config: &Config,
    issues: &mut Vec<Issue>,
) {
    for file in parsed {
        for sig in &file.public_signatures {
            let normalized = sig.text.split_whitespace().collect::<String>();
            let fan = *fan_in.get(&sig.source).unwrap_or(&0);
            if normalized.contains("anyhow::Result") || normalized.contains("anyhow::Error") {
                push_issue(
                    issues,
                    IssueDraft {
                        issue_type: IssueType::AnyhowLeak,
                        public_reach: true,
                        boundary: false,
                        fan_in: fan,
                        file: sig.file.clone(),
                        line: sig.line,
                        source: sig.source.clone(),
                        target: "signature".to_string(),
                        message: "anyhow type appears in a library public API".to_string(),
                        remediation: "Introduce a crate-specific error enum and convert anyhow at the binary boundary.".to_string(),
                    },
                    config,
                );
            }
            if normalized.contains("Box<dynstd::error::Error")
                || normalized.contains("Box<dynError")
            {
                push_issue(
                    issues,
                    IssueDraft {
                        issue_type: IssueType::DynErrorExposure,
                        public_reach: true,
                        boundary: false,
                        fan_in: fan,
                        file: sig.file.clone(),
                        line: sig.line,
                        source: sig.source.clone(),
                        target: "signature".to_string(),
                        message: "Box<dyn Error> appears in a library public API".to_string(),
                        remediation: "Return a concrete error enum so callers can match and preserve semver intent.".to_string(),
                    },
                    config,
                );
            }
        }
    }
}

fn detect_enum_bloat(
    parsed: &[ParsedFile],
    fan_in: &HashMap<String, usize>,
    config: &Config,
    issues: &mut Vec<Issue>,
) {
    for file in parsed {
        for enm in &file.enums {
            if !enm.derives_thiserror || enm.variant_count <= config.enum_variant_threshold {
                continue;
            }
            let fan = *fan_in.get(&enm.source).unwrap_or(&0);
            push_issue(
                issues,
                IssueDraft {
                    issue_type: IssueType::ErrorEnumBloat,
                    public_reach: file.is_lib_reachable,
                    boundary: false,
                    fan_in: fan,
                    file: enm.file.clone(),
                    line: enm.line,
                    source: enm.source.clone(),
                    target: "enum".to_string(),
                    message: format!(
                        "thiserror enum `{}` has {} variants (threshold {})",
                        enm.name, enm.variant_count, config.enum_variant_threshold
                    ),
                    remediation: "Split by domain boundary or wrap lower-level errors behind smaller variants.".to_string(),
                },
                config,
            );
        }
    }
}

fn detect_boundary_panic(
    parsed: &[ParsedFile],
    fan_in: &HashMap<String, usize>,
    config: &Config,
    issues: &mut Vec<Issue>,
) {
    for file in parsed {
        for panic in &file.panic_sites {
            if panic.is_boundary {
                continue;
            }
            let fan = *fan_in.get(&panic.function).unwrap_or(&0);
            push_issue(
                issues,
                IssueDraft {
                    issue_type: IssueType::BoundaryPanic,
                    public_reach: file.is_lib_reachable,
                    boundary: false,
                    fan_in: fan,
                    file: panic.file.clone(),
                    line: panic.line,
                    source: panic.function.clone(),
                    target: panic.kind.clone(),
                    message: format!("{} remains outside boundary code", panic.kind),
                    remediation: "Return Result, use explicit fallback handling, or move invariant checks to the binary/test boundary.".to_string(),
                },
                config,
            );
        }
    }
}

fn detect_missing_context(
    functions: &[FunctionInfo],
    index: &mut FunctionIndex<'_>,
    fan_in: &HashMap<String, usize>,
    config: &Config,
    issues: &mut Vec<Issue>,
) {
    let mut memo = HashMap::new();
    let mut seen = HashSet::new();
    for (idx, function) in functions.iter().enumerate() {
        if !function.has_question || function.has_context {
            continue;
        }
        let path = longest_uncontextualized_chain(idx, functions, index, &mut memo);
        if path.len() < config.context_depth_threshold {
            continue;
        }
        let Some(target_idx) = path.last().copied() else {
            continue;
        };
        let target = &functions[target_idx];
        let key = (function.id.clone(), target.id.clone());
        if !seen.insert(key) {
            continue;
        }
        let fan = *fan_in.get(&function.id).unwrap_or(&0);
        push_issue(
            issues,
            IssueDraft {
                issue_type: IssueType::MissingContext,
                public_reach: function.is_public,
                boundary: function.is_boundary,
                fan_in: fan,
                file: function.file.clone(),
                line: function.line,
                source: function.id.clone(),
                target: target.id.clone(),
                message: format!("`?` propagates through {} functions without context", path.len()),
                remediation: "Add `.context(...)` or `.with_context(...)` at the boundary where domain meaning is known.".to_string(),
            },
            config,
        );
    }
}

fn longest_uncontextualized_chain(
    idx: usize,
    functions: &[FunctionInfo],
    index: &mut FunctionIndex<'_>,
    memo: &mut HashMap<usize, Vec<usize>>,
) -> Vec<usize> {
    const MAX_DEPTH: usize = 64;
    if let Some(cached) = memo.get(&idx) {
        return cached.clone();
    }
    let result = longest_chain_inner(idx, functions, index, memo, &mut HashSet::new(), MAX_DEPTH);
    memo.insert(idx, result.clone());
    result
}

fn longest_chain_inner(
    idx: usize,
    functions: &[FunctionInfo],
    index: &mut FunctionIndex<'_>,
    memo: &mut HashMap<usize, Vec<usize>>,
    visiting: &mut HashSet<usize>,
    remaining_depth: usize,
) -> Vec<usize> {
    if remaining_depth == 0 || !visiting.insert(idx) {
        return vec![idx];
    }
    let mut current_best = vec![idx];
    for target_idx in index.resolve_indices(idx) {
        let target = &functions[target_idx];
        if !target.has_question || target.has_context {
            continue;
        }
        let suffix = if let Some(cached) = memo.get(&target_idx) {
            cached.clone()
        } else {
            longest_chain_inner(
                target_idx,
                functions,
                index,
                memo,
                visiting,
                remaining_depth - 1,
            )
        };
        let mut candidate = vec![idx];
        candidate.extend(suffix);
        if candidate.len() > current_best.len() {
            current_best = candidate;
        }
    }
    visiting.remove(&idx);
    current_best
}

struct IssueDraft {
    issue_type: IssueType,
    public_reach: bool,
    boundary: bool,
    fan_in: usize,
    file: PathBuf,
    line: usize,
    source: String,
    target: String,
    message: String,
    remediation: String,
}

fn push_issue(issues: &mut Vec<Issue>, draft: IssueDraft, config: &Config) {
    if config.is_allowed(draft.issue_type) {
        return;
    }
    let layer = if draft.public_reach {
        Layer::PublicApi
    } else if draft.boundary {
        Layer::Boundary
    } else {
        Layer::Internal
    };
    issues.push(Issue {
        key: IssueKey {
            issue_type: draft.issue_type,
            source: draft.source,
            target: draft.target,
        },
        severity: severity(
            draft.issue_type,
            draft.public_reach,
            draft.boundary,
            draft.fan_in,
        ),
        file: draft.file,
        line: draft.line,
        layer,
        message: draft.message,
        remediation: draft.remediation,
        fan_in: draft.fan_in,
    });
}

struct SuppressionResult {
    kept: Vec<Issue>,
    suppressed: usize,
}

fn apply_suppressions(issues: Vec<Issue>) -> Result<SuppressionResult> {
    let result = apply_core_suppressions(
        issues,
        |issue| issue.file.as_path(),
        |issue| issue.line,
        |issue| issue.key.issue_type.id(),
        "error-map",
        |marker, issue| marker == issue,
    )?;
    Ok(SuppressionResult {
        kept: result.kept,
        suppressed: result.suppressed,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn async_fn_item_suppression_uses_cst_item_start() {
        let source = r#"
// error-map-allow: boundary-panic
pub async fn allowed() {
    let _ = Some(1).unwrap();
}
"#;
        assert!(design_gate_core::is_suppressed(
            source,
            4,
            IssueType::BoundaryPanic.id(),
            "error-map",
            &|marker, issue| marker == issue
        ));
    }
}
