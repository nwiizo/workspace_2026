use std::collections::{BTreeMap, BTreeSet};

use crate::config::BoundaryConfig;
use crate::git::Volatility;
use crate::model::{Issue, IssueKey, IssueType, Location, Severity};
use crate::parser::{ParsedFile, PathRef};
use crate::scoring;

#[derive(Debug)]
pub struct FileContext<'a> {
    pub parsed: &'a ParsedFile,
    pub module: &'a str,
    pub layer: Option<LayerContext>,
    pub volatility: Volatility,
}

#[derive(Debug, Clone)]
pub struct LayerContext {
    pub name: String,
    pub rank: usize,
}

#[derive(Debug, Clone)]
struct Candidate {
    issue_type: IssueType,
    source: String,
    target: String,
    depth: usize,
    source_layer: Option<String>,
    target_layer: Option<String>,
    location: Location,
    message: String,
    message_ja: String,
    suggestion: String,
    suggestion_ja: String,
    volatility: Volatility,
}

#[derive(Debug)]
struct IssueAccumulator {
    depth: usize,
    source_layer: Option<String>,
    target_layer: Option<String>,
    locations: Vec<Location>,
    message: String,
    message_ja: String,
    suggestion: String,
    suggestion_ja: String,
    max_volatility: Volatility,
}

pub fn run_lints(files: &[FileContext<'_>], config: &BoundaryConfig) -> Vec<Issue> {
    let mut candidates = Vec::new();
    candidates.extend(layer_violations(files, config));
    candidates.extend(internal_crossings(files));
    candidates.extend(forbidden_imports(files, config));
    candidates.extend(pub_leaks(files));
    aggregate(candidates)
}

fn layer_violations(files: &[FileContext<'_>], config: &BoundaryConfig) -> Vec<Candidate> {
    let mut out = Vec::new();
    for file in files {
        let Some(source_layer) = &file.layer else {
            continue;
        };
        for reference in references(file) {
            if reference.allows(IssueType::LayerViolation) {
                continue;
            }
            let target_path = reference.path.as_str();
            let Some(target_layer) = config.layer_for_path_string(target_path) else {
                continue;
            };
            if config.is_allowed(
                &source_layer.name,
                &target_layer.name,
                source_layer.rank,
                target_layer.rank,
            ) {
                continue;
            }
            let depth = target_layer.rank.saturating_sub(source_layer.rank).max(1);
            out.push(Candidate {
                issue_type: IssueType::LayerViolation,
                source: file.module.to_string(),
                target: target_path.to_string(),
                depth,
                source_layer: Some(source_layer.name.clone()),
                target_layer: Some(target_layer.name),
                location: reference.location.clone(),
                message: format!(
                    "{module} depends on forbidden layer path {target_path}",
                    module = file.module
                ),
                message_ja: format!(
                    "{} が禁止された層のパス {} に依存しています",
                    file.module, target_path
                ),
                suggestion:
                    "invert the dependency through an application/domain port or move the reference outward"
                        .to_string(),
                suggestion_ja:
                    "application/domain 側の port 経由に依存を反転するか、参照元を外側の層へ移してください"
                        .to_string(),
                volatility: file.volatility,
            });
        }
    }
    out
}

fn internal_crossings(files: &[FileContext<'_>]) -> Vec<Candidate> {
    let mut out = Vec::new();
    for file in files {
        for reference in references(file) {
            if reference.allows(IssueType::InternalCrossing) {
                continue;
            }
            let target_path = reference.path.as_str();
            let Some(prefix) = internal_prefix(target_path) else {
                continue;
            };
            if module_starts_with(file.module, &prefix) {
                continue;
            }
            out.push(Candidate {
                issue_type: IssueType::InternalCrossing,
                source: file.module.to_string(),
                target: target_path.to_string(),
                depth: 1,
                source_layer: file.layer.as_ref().map(|layer| layer.name.clone()),
                target_layer: None,
                location: reference.location.clone(),
                message: format!("{} crosses into internal module {}", file.module, target_path),
                message_ja: format!(
                    "{} が internal module {} を境界外から参照しています",
                    file.module, target_path
                ),
                suggestion:
                    "use a public facade at the owning module boundary or move the caller inside the boundary"
                        .to_string(),
                suggestion_ja:
                    "所有 module の公開 facade を使うか、呼び出し側をその境界内へ移してください"
                        .to_string(),
                volatility: file.volatility,
            });
        }
    }
    out
}

fn forbidden_imports(files: &[FileContext<'_>], config: &BoundaryConfig) -> Vec<Candidate> {
    let mut out = Vec::new();
    for file in files {
        let Some(layer) = &file.layer else {
            continue;
        };
        for reference in references(file) {
            if reference.allows(IssueType::ForbiddenImport) {
                continue;
            }
            let Some(crate_name) = external_crate_name(&reference.path, reference.is_use) else {
                continue;
            };
            for rule in &config.forbidden_imports {
                if rule.layer != layer.name || !rule.crates.iter().any(|name| name == crate_name) {
                    continue;
                }
                out.push(Candidate {
                    issue_type: IssueType::ForbiddenImport,
                    source: file.module.to_string(),
                    target: crate_name.to_string(),
                    depth: 2,
                    source_layer: Some(layer.name.clone()),
                    target_layer: None,
                    location: reference.location.clone(),
                    message: format!("{} references forbidden crate {}", file.module, crate_name),
                    message_ja: format!(
                        "{} が禁止された crate {} を参照しています",
                        file.module, crate_name
                    ),
                    suggestion:
                        "depend on a boundary-owned trait or adapter instead of importing the crate directly"
                            .to_string(),
                    suggestion_ja:
                        "crate を直接参照せず、境界側が所有する trait または adapter に依存してください"
                            .to_string(),
                    volatility: file.volatility,
                });
            }
        }
    }
    out
}

fn pub_leaks(files: &[FileContext<'_>]) -> Vec<Candidate> {
    let referenced_names = referenced_public_names(files);
    let mut out = Vec::new();
    for file in files {
        for item in &file.parsed.pub_items {
            if item.kind == "mod" {
                continue;
            }
            if item.allows(IssueType::PubLeak) {
                continue;
            }
            let key = format!("{}::{}", file.module, item.name);
            if referenced_names.contains(&key) || referenced_names.contains(&item.name) {
                continue;
            }
            out.push(Candidate {
                issue_type: IssueType::PubLeak,
                source: file.module.to_string(),
                target: item.name.clone(),
                depth: 1,
                source_layer: file.layer.as_ref().map(|layer| layer.name.clone()),
                target_layer: None,
                location: item.location.clone(),
                message: format!(
                    "pub {} `{}` is not referenced by the analyzed source",
                    item.kind, item.name
                ),
                message_ja: format!(
                    "pub {} `{}` は解析対象ソース内で参照されていません",
                    item.kind, item.name
                ),
                suggestion:
                    "make it `pub(crate)` or private unless it is intentionally exported outside the analyzed crate"
                        .to_string(),
                suggestion_ja:
                    "crate 外へ意図的に公開する API でなければ `pub(crate)` または private にしてください"
                        .to_string(),
                volatility: Volatility::Low,
            });
        }
    }
    out
}

fn referenced_public_names(files: &[FileContext<'_>]) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    for file in files {
        for reference in references(file) {
            let cleaned = reference.path.trim_start_matches("crate::");
            for part in cleaned.split("::").filter(|part| !part.is_empty()) {
                names.insert(part.to_string());
            }
            names.insert(cleaned.to_string());
        }
    }
    names
}

fn references<'a>(file: &'a FileContext<'a>) -> impl Iterator<Item = &'a PathRef> {
    file.parsed.path_refs.iter()
}

fn aggregate(candidates: Vec<Candidate>) -> Vec<Issue> {
    let mut grouped: BTreeMap<IssueKey, IssueAccumulator> = BTreeMap::new();
    for candidate in candidates {
        let target = normalize_issue_target(&candidate.source, &candidate.target);
        let key = IssueKey {
            issue_type: candidate.issue_type,
            source: candidate.source,
            target,
        };
        grouped
            .entry(key)
            .and_modify(|entry| {
                entry.depth = entry.depth.max(candidate.depth);
                entry.locations.push(candidate.location.clone());
                entry.max_volatility = entry.max_volatility.max(candidate.volatility);
            })
            .or_insert_with(|| IssueAccumulator {
                depth: candidate.depth,
                source_layer: candidate.source_layer,
                target_layer: candidate.target_layer,
                locations: vec![candidate.location],
                message: candidate.message,
                message_ja: candidate.message_ja,
                suggestion: candidate.suggestion,
                suggestion_ja: candidate.suggestion_ja,
                max_volatility: candidate.volatility,
            });
    }
    let mut issues: Vec<Issue> = grouped
        .into_iter()
        .map(|(key, entry)| {
            let occurrences = entry.locations.len();
            let mut score = scoring::issue_score(entry.depth, occurrences, entry.max_volatility);
            let mut severity = scoring::severity(score);
            if key.issue_type == IssueType::PubLeak {
                score = score.min(1.0);
                severity = severity.min(Severity::Low);
            }
            Issue {
                key,
                severity,
                score,
                depth: entry.depth,
                occurrences,
                source_layer: entry.source_layer,
                target_layer: entry.target_layer,
                locations: entry.locations,
                message: entry.message,
                message_ja: entry.message_ja,
                suggestion: entry.suggestion,
                suggestion_ja: entry.suggestion_ja,
            }
        })
        .collect();
    issues.sort_by(|left, right| {
        right
            .severity
            .cmp(&left.severity)
            .then_with(|| right.score.total_cmp(&left.score))
            .then_with(|| left.key.cmp(&right.key))
    });
    issues
}

fn internal_prefix(path: &str) -> Option<Vec<&str>> {
    let parts: Vec<&str> = path
        .split("::")
        .filter(|part| !part.is_empty() && *part != "crate" && *part != "self")
        .collect();
    for (index, part) in parts.iter().enumerate() {
        if matches!(*part, "internal" | "_internal" | "private") && index > 0 {
            return Some(parts[..index].to_vec());
        }
    }
    None
}

fn module_starts_with(module: &str, prefix: &[&str]) -> bool {
    let module_parts: Vec<&str> = module
        .split("::")
        .filter(|part| !part.is_empty() && *part != "crate")
        .collect();
    module_parts.starts_with(prefix)
}

fn external_crate_name(path: &str, is_use: bool) -> Option<&str> {
    if !is_use && !path.contains("::") {
        return None;
    }
    let first = path.split("::").next()?;
    if matches!(first, "crate" | "self" | "super") {
        None
    } else {
        Some(first)
    }
}

fn normalize_issue_target(source: &str, target: &str) -> String {
    if target.starts_with("crate::") {
        return target.to_string();
    }
    if let Some(rest) = target.strip_prefix("self::") {
        let source = source.trim_start_matches("crate::");
        return format!("crate::{source}::{rest}");
    }
    if let Some(rest) = target.strip_prefix("super::") {
        let source = source.trim_start_matches("crate::");
        let parent = source
            .rsplit_once("::")
            .map_or("crate", |(parent, _)| parent);
        if parent == "crate" {
            return format!("crate::{rest}");
        }
        return format!("crate::{parent}::{rest}");
    }
    target.to_string()
}
