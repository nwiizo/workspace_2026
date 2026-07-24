use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use cargo_metadata::{DependencyKind, Metadata, MetadataCommand, Package};
use design_gate_core::{
    RustFileWalkerOptions, apply_suppressions as apply_core_suppressions, rust_files,
};
use ra_ap_syntax::Edition;
use rayon::prelude::*;
use serde::Serialize;
use toml::Value;

use crate::blind_spot::{BlindSpotManifest, build as build_blind_spots};
use crate::config::{Config, parse_issue_type};
use crate::error::{Error, Result};
use crate::issue::{Issue, IssueKey, IssueType, Surface};
use crate::parser::{CfgExpr, ParsedFile, PublicItem, parse_file, relative_path};
use crate::scoring::{Grade, grade, severity};

#[derive(Debug, Clone, Serialize)]
pub struct Analysis {
    pub project: String,
    pub root: PathBuf,
    pub manifest: PathBuf,
    pub files_analyzed: usize,
    pub suppressed_issues: usize,
    pub grade: Grade,
    pub feature_count: usize,
    pub combination_estimate: String,
    pub issues: Vec<Issue>,
    pub matrix: Vec<FeatureMatrixRow>,
    pub hack_suggestions: Vec<HackSuggestion>,
    pub blind_spots: BlindSpotManifest,
}

#[derive(Debug, Clone, Serialize)]
pub struct FeatureMatrixRow {
    pub feature: String,
    pub default: bool,
    pub cfg_refs: usize,
    pub issue_count: usize,
    pub status: FeatureStatus,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum FeatureStatus {
    Risk,
    Covered,
    ManifestOnly,
}

impl FeatureStatus {
    pub(crate) const fn label(&self) -> &'static str {
        match self {
            Self::Risk => "risk",
            Self::Covered => "covered",
            Self::ManifestOnly => "manifest-only",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct HackSuggestion {
    pub reason: String,
    pub reason_ja: String,
    pub command: String,
    pub excluded_features: Vec<String>,
}

#[derive(Debug)]
struct CargoContext {
    project: String,
    manifest: PathBuf,
    metadata: Option<Metadata>,
    features: BTreeMap<String, Vec<String>>,
    default_features: BTreeSet<String>,
    optional_deps: Vec<OptionalDep>,
    edition: Edition,
    metadata_failed: bool,
}

#[derive(Debug)]
struct OptionalDep {
    manifest_name: String,
    crate_names: BTreeSet<String>,
    gating_features: BTreeSet<String>,
}

#[derive(Debug, Clone)]
struct IssueDraft {
    issue_type: IssueType,
    file: PathBuf,
    line: usize,
    surface: Surface,
    source: String,
    target: String,
    features: Vec<String>,
    message: String,
    remediation: String,
    affected_combinations: u128,
    public_api: bool,
    usage: usize,
}

pub fn analyze_path(path: &Path, config: &Config) -> Result<Analysis> {
    let root = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let cargo = cargo_context(&root)?;
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
        .map(|file| parse_file(&root, file, cargo.edition))
        .collect();
    let mut parsed = Vec::new();
    for result in parsed_results {
        parsed.push(result?);
    }

    let mut issues = Vec::new();
    detect_default_leaks(&root, &cargo, config, &mut issues);
    detect_exclusive_undeclared(&cargo, &parsed, config, &mut issues);
    detect_untested_cfg_paths(&cargo, &parsed, config, &mut issues);
    detect_optional_dep_exposure(&cargo, &parsed, config, &mut issues);
    detect_non_additive_features(&cargo, &parsed, config, &mut issues);

    let suppression = apply_suppressions(issues)?;
    let mut issues = suppression.kept;
    for issue in &mut issues {
        issue.file = PathBuf::from(relative_path(&root, &issue.file));
    }
    issues.sort_by(|a, b| {
        b.severity
            .cmp(&a.severity)
            .then_with(|| a.file.cmp(&b.file))
            .then_with(|| a.line.cmp(&b.line))
            .then_with(|| a.key.issue_type.id().cmp(b.key.issue_type.id()))
            .then_with(|| a.key.source.cmp(&b.key.source))
            .then_with(|| a.key.target.cmp(&b.key.target))
    });
    issues.dedup_by(|left, right| left.key == right.key);
    let parse_failures = parsed.iter().map(|file| file.parse_errors).sum();
    let feature_count = cargo.features.len();
    let matrix = feature_matrix(&cargo, &parsed, &issues);
    let hack_suggestions = hack_suggestions(&issues);
    let blind_spots = build_blind_spots(parse_failures, cargo.metadata_failed, feature_count);
    Ok(Analysis {
        project: cargo.project,
        root,
        manifest: cargo.manifest,
        files_analyzed: parsed.len(),
        suppressed_issues: suppression.suppressed,
        grade: grade(&issues),
        feature_count,
        combination_estimate: combination_estimate(feature_count),
        issues,
        matrix,
        hack_suggestions,
        blind_spots,
    })
}

fn cargo_context(root: &Path) -> Result<CargoContext> {
    let manifest = find_manifest(root).ok_or_else(|| Error::NoManifest(root.to_path_buf()))?;
    let manifest_source = fs::read_to_string(&manifest).map_err(|source| Error::ReadFile {
        path: manifest.clone(),
        source,
    })?;
    let manifest_value: Value =
        toml::from_str(&manifest_source).map_err(|source| Error::ManifestToml {
            path: manifest.clone(),
            source,
        })?;
    let (metadata, metadata_failed) = metadata(&manifest);
    let root_package = metadata.as_ref().and_then(Metadata::root_package);
    let project = root_package
        .map(|package| package.name.to_string())
        .or_else(|| manifest_package_name(&manifest_value))
        .unwrap_or_else(|| "project".to_string());
    let edition = root_package
        .and_then(|package| parse_edition(package.edition.to_string().as_str()))
        .or_else(|| manifest_edition(&manifest_value))
        .unwrap_or(Edition::Edition2024);
    let features = root_package
        .map(|package| {
            package
                .features
                .iter()
                .map(|(name, members)| (name.to_string(), members.clone()))
                .collect::<BTreeMap<_, _>>()
        })
        .unwrap_or_else(|| manifest_features(&manifest_value));
    let default_features = expand_default_features(&features);
    let optional_deps = root_package
        .map(|package| optional_deps_from_package(package, &features))
        .unwrap_or_else(|| optional_deps_from_manifest(&manifest_value));
    Ok(CargoContext {
        project,
        manifest,
        metadata,
        features,
        default_features,
        optional_deps,
        edition,
        metadata_failed,
    })
}

fn metadata(manifest: &Path) -> (Option<Metadata>, bool) {
    let mut command = MetadataCommand::new();
    command.manifest_path(manifest);
    if let Ok(metadata) = command.exec() {
        return (Some(metadata), false);
    }
    let mut fallback = MetadataCommand::new();
    fallback.manifest_path(manifest);
    (fallback.no_deps().exec().ok(), true)
}

fn find_manifest(path: &Path) -> Option<PathBuf> {
    let start = if path.is_file() { path.parent()? } else { path };
    for dir in start.ancestors() {
        let candidate = dir.join("Cargo.toml");
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
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

fn manifest_package_name(value: &Value) -> Option<String> {
    value
        .get("package")?
        .get("name")?
        .as_str()
        .map(str::to_string)
}

fn manifest_edition(value: &Value) -> Option<Edition> {
    value
        .get("package")?
        .get("edition")?
        .as_str()
        .and_then(parse_edition)
}

fn manifest_features(value: &Value) -> BTreeMap<String, Vec<String>> {
    let mut features = BTreeMap::new();
    let Some(table) = value.get("features").and_then(Value::as_table) else {
        return features;
    };
    for (name, members) in table {
        let values = members
            .as_array()
            .map(|items| {
                items
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::to_string)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        features.insert(name.to_string(), values);
    }
    features
}

fn expand_default_features(features: &BTreeMap<String, Vec<String>>) -> BTreeSet<String> {
    expand_feature_members(features.get("default").into_iter().flatten(), features)
}

fn expand_feature_members<'a>(
    members: impl IntoIterator<Item = &'a String>,
    features: &BTreeMap<String, Vec<String>>,
) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let mut stack = members.into_iter().cloned().collect::<Vec<_>>();
    while let Some(feature) = stack.pop() {
        let name = feature_name(&feature);
        if !out.insert(name.clone()) {
            continue;
        }
        if let Some(children) = features.get(&name) {
            stack.extend(children.iter().cloned());
        }
    }
    out
}

fn feature_name(value: &str) -> String {
    let before_slash = value.split('/').next().unwrap_or(value);
    let without_dep = before_slash.strip_prefix("dep:").unwrap_or(before_slash);
    // Cargo weak dependency syntax `dep?/feature` names the dependency before `?`.
    without_dep
        .strip_suffix('?')
        .unwrap_or(without_dep)
        .to_string()
}

fn optional_deps_from_package(
    package: &Package,
    features: &BTreeMap<String, Vec<String>>,
) -> Vec<OptionalDep> {
    let feature_to_optional_deps = feature_to_optional_deps(features);
    package
        .dependencies
        .iter()
        .filter(|dep| dep.optional && dep.kind == DependencyKind::Normal)
        .map(|dep| {
            let manifest_name = dep.rename.clone().unwrap_or_else(|| dep.name.clone());
            OptionalDep {
                manifest_name: manifest_name.clone(),
                crate_names: crate_name_aliases(&manifest_name, &dep.name),
                gating_features: gating_features_for_dep(&manifest_name, &feature_to_optional_deps),
            }
        })
        .collect()
}

fn optional_deps_from_manifest(value: &Value) -> Vec<OptionalDep> {
    dependency_tables(value)
        .into_iter()
        .flat_map(|table| {
            table.iter().filter_map(|(name, value)| {
                let dep_table = value.as_table()?;
                if !dep_table
                    .get("optional")
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
                {
                    return None;
                }
                let package = dep_table
                    .get("package")
                    .and_then(Value::as_str)
                    .unwrap_or(name);
                Some(OptionalDep {
                    manifest_name: name.to_string(),
                    crate_names: crate_name_aliases(name, package),
                    gating_features: BTreeSet::from([name.to_string()]),
                })
            })
        })
        .collect()
}

fn feature_to_optional_deps(
    features: &BTreeMap<String, Vec<String>>,
) -> BTreeMap<String, BTreeSet<String>> {
    features
        .keys()
        .map(|feature| {
            let expanded = expand_feature_members(std::slice::from_ref(feature), features);
            (feature.clone(), expanded)
        })
        .collect()
}

fn gating_features_for_dep(
    dep_name: &str,
    feature_to_optional_deps: &BTreeMap<String, BTreeSet<String>>,
) -> BTreeSet<String> {
    feature_to_optional_deps
        .iter()
        .filter(|(_, deps)| deps.contains(dep_name))
        .map(|(feature, _)| feature.clone())
        .collect()
}

fn dependency_tables(value: &Value) -> Vec<&toml::map::Map<String, Value>> {
    let mut tables = Vec::new();
    if let Some(table) = value.get("dependencies").and_then(Value::as_table) {
        tables.push(table);
    }
    if let Some(targets) = value.get("target").and_then(Value::as_table) {
        for target in targets.values() {
            if let Some(table) = target.get("dependencies").and_then(Value::as_table) {
                tables.push(table);
            }
        }
    }
    tables
}

fn crate_name_aliases(local: &str, package: &str) -> BTreeSet<String> {
    [local, package]
        .into_iter()
        .flat_map(|name| [name.to_string(), name.replace('-', "_")])
        .collect()
}

fn detect_default_leaks(
    root: &Path,
    cargo: &CargoContext,
    config: &Config,
    issues: &mut Vec<Issue>,
) {
    let Some(metadata) = &cargo.metadata else {
        return;
    };
    let Some(root_package) = metadata.root_package() else {
        return;
    };
    let resolved = resolved_root_dependencies(metadata);
    for dep in root_package
        .dependencies
        .iter()
        .filter(|dep| dep.kind == DependencyKind::Normal)
    {
        if !dep.uses_default_features {
            continue;
        }
        let local_name = dep.rename.clone().unwrap_or_else(|| dep.name.clone());
        let dep_package = resolve_dependency_package(metadata, &resolved, &local_name, &dep.name);
        let Some(dep_package) = dep_package else {
            continue;
        };
        let default_entries = expand_feature_members(
            dep_package.features.get("default").into_iter().flatten(),
            &dep_package.features,
        );
        if default_entries.is_empty() {
            continue;
        }
        let optional_names = dep_package
            .dependencies
            .iter()
            .filter(|dep| dep.optional)
            .map(|dep| dep.rename.clone().unwrap_or_else(|| dep.name.clone()))
            .collect::<HashSet<_>>();
        let risky_entries = default_entries
            .iter()
            .filter(|entry| default_entry_is_risky(entry, &optional_names))
            .cloned()
            .collect::<Vec<_>>();
        if risky_entries.is_empty() {
            continue;
        }
        push_issue(
            issues,
            IssueDraft {
                issue_type: IssueType::DefaultLeak,
                file: cargo.manifest.clone(),
                line: 1,
                surface: Surface::Manifest,
                source: relative_path(root, &cargo.manifest),
                target: local_name.clone(),
                features: risky_entries.clone(),
                message: format!(
                    "`{local_name}` enables dependency defaults that include {}",
                    risky_entries.join(", ")
                ),
                remediation: format!(
                    "Set `{local_name}.default-features = false` and opt into the required features explicitly."
                ),
                affected_combinations: 1,
                public_api: false,
                usage: risky_entries.len(),
            },
            config,
        );
    }
}

fn resolved_root_dependencies(metadata: &Metadata) -> HashMap<String, cargo_metadata::PackageId> {
    let mut out = HashMap::new();
    let Some(resolve) = &metadata.resolve else {
        return out;
    };
    let Some(root_id) = &resolve.root else {
        return out;
    };
    let Some(root_node) = resolve.nodes.iter().find(|node| &node.id == root_id) else {
        return out;
    };
    for dep in &root_node.deps {
        out.insert(dep.name.to_string(), dep.pkg.clone());
    }
    out
}

fn resolve_dependency_package<'a>(
    metadata: &'a Metadata,
    resolved: &HashMap<String, cargo_metadata::PackageId>,
    local_name: &str,
    package_name: &str,
) -> Option<&'a Package> {
    resolved
        .get(local_name)
        .or_else(|| resolved.get(package_name))
        .and_then(|package_id| {
            metadata
                .packages
                .iter()
                .find(|package| &package.id == package_id)
        })
        .or_else(|| {
            metadata
                .packages
                .iter()
                .find(|package| package.name == package_name)
        })
}

fn default_entry_is_risky(entry: &str, optional_names: &HashSet<String>) -> bool {
    let name = feature_name(entry);
    let optional = optional_names.contains(&name);
    broad_default_keyword(&name) || (optional && risky_keyword(&name))
}

fn broad_default_keyword(name: &str) -> bool {
    matches!(
        name.replace('_', "-").as_str(),
        "full"
            | "heavy"
            | "blocking"
            | "default-tls"
            | "native-tls"
            | "openssl"
            | "rustls"
            | "tls"
            | "vendored"
    )
}

fn risky_keyword(name: &str) -> bool {
    let normalized = name.replace('_', "-");
    if normalized == "tokio" || normalized == "tracing" {
        return false;
    }
    normalized
        .split('-')
        .chain(std::iter::once(normalized.as_str()))
        .any(|part| {
            // Fallback risk vocabulary for defaults that commonly pull runtimes,
            // TLS stacks, compression codecs, blocking clients, or vendored C deps.
            matches!(
                part,
                "full"
                    | "heavy"
                    | "tokio"
                    | "async-std"
                    | "native-tls"
                    | "openssl"
                    | "rustls"
                    | "tls"
                    | "default-tls"
                    | "blocking"
                    | "cookies"
                    | "gzip"
                    | "brotli"
                    | "deflate"
                    | "zstd"
                    | "http2"
                    | "vendored"
            )
        })
}

fn detect_exclusive_undeclared(
    cargo: &CargoContext,
    parsed: &[ParsedFile],
    config: &Config,
    issues: &mut Vec<Issue>,
) {
    let guards = parsed
        .iter()
        .flat_map(|file| file.compile_error_guards.iter())
        .cloned()
        .collect::<Vec<_>>();
    for (left, right) in exclusive_pairs(cargo.features.keys()) {
        if guards
            .iter()
            .any(|guard| guard.contains(&left) && guard.contains(&right))
        {
            continue;
        }
        let affected = combinations_with_required(cargo.features.len(), 2);
        push_issue(
            issues,
            IssueDraft {
                issue_type: IssueType::ExclusiveUndeclared,
                file: cargo.manifest.clone(),
                line: 1,
                surface: Surface::FeatureGraph,
                source: "Cargo.toml".to_string(),
                target: format!("{left}+{right}"),
                features: vec![left.clone(), right.clone()],
                message: format!(
                    "`{left}` and `{right}` look mutually exclusive but have no compile_error! guard."
                ),
                remediation: format!(
                    "Add `#[cfg(all(feature = \"{left}\", feature = \"{right}\"))] compile_error!(...)` near the feature-gated implementation."
                ),
                affected_combinations: affected,
                public_api: false,
                usage: 2,
            },
            config,
        );
    }
}

fn exclusive_pairs<'a>(features: impl Iterator<Item = &'a String>) -> Vec<(String, String)> {
    let features = features.cloned().collect::<Vec<_>>();
    let mut pairs = Vec::new();
    for (idx, left) in features.iter().enumerate() {
        for right in features.iter().skip(idx + 1) {
            if looks_exclusive(left, right) {
                pairs.push((left.clone(), right.clone()));
            }
        }
    }
    pairs
}

fn looks_exclusive(left: &str, right: &str) -> bool {
    let Some((left_prefix, left_suffix)) = split_feature_name(left) else {
        return false;
    };
    let Some((right_prefix, right_suffix)) = split_feature_name(right) else {
        return false;
    };
    (left_prefix == right_prefix && name_parts_conflict(left_suffix, right_suffix))
        || (left_suffix == right_suffix && name_parts_conflict(left_prefix, right_prefix))
}

fn split_feature_name(name: &str) -> Option<(&str, &str)> {
    name.rsplit_once('-').or_else(|| name.rsplit_once('_'))
}

fn name_parts_conflict(left: &str, right: &str) -> bool {
    let normalized_left = left.replace('_', "-");
    let normalized_right = right.replace('_', "-");
    let groups: &[&[&str]] = &[
        &["tokio", "async-std", "smol"],
        &["native", "rustls", "openssl", "native-tls"],
        &["sync", "async"],
        &["blocking", "async"],
    ];
    groups.iter().any(|group| {
        group.contains(&normalized_left.as_str()) && group.contains(&normalized_right.as_str())
    })
}

fn detect_untested_cfg_paths(
    cargo: &CargoContext,
    parsed: &[ParsedFile],
    config: &Config,
    issues: &mut Vec<Issue>,
) {
    let all_features = cargo.features.keys().cloned().collect::<BTreeSet<_>>();
    for site in parsed.iter().flat_map(|file| file.cfg_sites.iter()) {
        let mut all_referenced_features = BTreeSet::new();
        site.expr.features(&mut all_referenced_features);
        if all_referenced_features.is_empty() {
            continue;
        }
        let default_on = site.expr.evaluate(&cargo.default_features);
        let all_on = site.expr.evaluate(&all_features);
        if default_on != Some(false) || all_on != Some(false) {
            continue;
        }
        let mut features = BTreeSet::new();
        site.expr.required_features(&mut features);
        let mut excluded = BTreeSet::new();
        site.expr.not_features(&mut excluded);
        let affected = estimate_matching_combinations(&site.expr, &all_features);
        let limitation = if exclusive_pairs(cargo.features.keys())
            .iter()
            .any(|(left, right)| features.contains(left) && features.contains(right))
        {
            " This may overlap a known two-point limitation for mutually exclusive feature pairs."
        } else {
            ""
        };
        let exclusion = if excluded.is_empty() {
            String::new()
        } else {
            format!(
                " Keep {} disabled.",
                excluded.into_iter().collect::<Vec<_>>().join(", ")
            )
        };
        push_issue(
            issues,
            IssueDraft {
                issue_type: IssueType::UntestedCfgPath,
                file: site.file.clone(),
                line: site.line,
                surface: Surface::CfgPath,
                source: site.source.clone(),
                target: site.expr.display(),
                features: features.into_iter().collect(),
                message: format!(
                    "This cfg branch is false under both default features and all features.{limitation}"
                ),
                remediation: format!(
                    "Add an explicit cargo-hack or CI check for this feature combination, or simplify the cfg expression.{exclusion}"
                ),
                affected_combinations: affected,
                public_api: site.public_api,
                usage: 1,
            },
            config,
        );
    }
}

fn detect_optional_dep_exposure(
    cargo: &CargoContext,
    parsed: &[ParsedFile],
    config: &Config,
    issues: &mut Vec<Issue>,
) {
    for dep in &cargo.optional_deps {
        let public_items = parsed
            .iter()
            .flat_map(|file| file.public_items.iter())
            .filter(|item| signature_mentions_dep(item, dep))
            .collect::<Vec<_>>();
        let usage = public_items.len();
        for item in public_items {
            if item_has_feature_gate(item, dep) {
                continue;
            }
            let suggested_feature = dep
                .gating_features
                .iter()
                .next()
                .cloned()
                .unwrap_or_else(|| dep.manifest_name.clone());
            push_issue(
                issues,
                IssueDraft {
                    issue_type: IssueType::OptionalDepExposure,
                    file: item.file.clone(),
                    line: item.line,
                    surface: Surface::PublicApi,
                    source: item.source.clone(),
                    target: dep.manifest_name.clone(),
                    features: vec![suggested_feature.clone()],
                    message: format!(
                        "Optional dependency `{}` appears in a public API without a matching feature gate.",
                        dep.manifest_name
                    ),
                    remediation: format!(
                        "Gate this item with `#[cfg(feature = \"{}\")]`, hide the dependency behind an owned type, or make the dependency non-optional.",
                        suggested_feature
                    ),
                    affected_combinations: combinations_without_feature(cargo.features.len()),
                    public_api: true,
                    usage,
                },
                config,
            );
        }
    }
}

fn signature_mentions_dep(item: &PublicItem, dep: &OptionalDep) -> bool {
    dep.crate_names
        .iter()
        .any(|crate_name| item.type_paths.contains(crate_name))
}

fn item_has_feature_gate(item: &PublicItem, dep: &OptionalDep) -> bool {
    if dep.gating_features.is_empty() || item.cfg_exprs.is_empty() {
        return false;
    }

    let mut variables = BTreeSet::new();
    for expr in &item.cfg_exprs {
        expr.features(&mut variables);
    }
    variables.retain(|feature| !dep.gating_features.contains(feature));
    if variables.len() > 20 {
        return false;
    }
    let variables = variables.into_iter().collect::<Vec<_>>();
    let combinations = 1usize << variables.len();

    for mask in 0..combinations {
        let enabled = variables
            .iter()
            .enumerate()
            .filter(|(index, _)| mask & (1 << index) != 0)
            .map(|(_, feature)| feature.clone())
            .collect::<BTreeSet<_>>();
        if item
            .cfg_exprs
            .iter()
            .all(|expr| cfg_can_be_true(expr, &enabled))
        {
            return false;
        }
    }
    true
}

fn cfg_can_be_true(expr: &CfgExpr, enabled: &BTreeSet<String>) -> bool {
    match expr {
        CfgExpr::Feature(feature) => enabled.contains(feature),
        CfgExpr::All(items) => items.iter().all(|item| cfg_can_be_true(item, enabled)),
        CfgExpr::Any(items) => items.iter().any(|item| cfg_can_be_true(item, enabled)),
        CfgExpr::Not(item) => cfg_can_be_false(item, enabled),
        CfgExpr::Other => true,
    }
}

fn cfg_can_be_false(expr: &CfgExpr, enabled: &BTreeSet<String>) -> bool {
    match expr {
        CfgExpr::Feature(feature) => !enabled.contains(feature),
        CfgExpr::All(items) => items.iter().any(|item| cfg_can_be_false(item, enabled)),
        CfgExpr::Any(items) => items.iter().all(|item| cfg_can_be_false(item, enabled)),
        CfgExpr::Not(item) => cfg_can_be_true(item, enabled),
        CfgExpr::Other => true,
    }
}

fn detect_non_additive_features(
    cargo: &CargoContext,
    parsed: &[ParsedFile],
    config: &Config,
    issues: &mut Vec<Issue>,
) {
    for item in parsed.iter().flat_map(|file| file.public_items.iter()) {
        let mut not_features = BTreeSet::new();
        for expr in &item.cfg_exprs {
            expr.not_features(&mut not_features);
        }
        for feature in not_features {
            push_issue(
                issues,
                IssueDraft {
                    issue_type: IssueType::NonAdditiveFeature,
                    file: item.file.clone(),
                    line: item.line,
                    surface: Surface::PublicApi,
                    source: item.source.clone(),
                    target: feature.clone(),
                    features: vec![feature.clone()],
                    message: format!(
                        "Public API `{}` is removed when feature `{feature}` is enabled.",
                        item.source
                    ),
                    remediation: "Prefer additive features. Keep the public item stable and move behavior differences behind private implementation choices.".to_string(),
                    affected_combinations: combinations_with_required(cargo.features.len(), 1),
                    public_api: true,
                    usage: 1,
                },
                config,
            );
        }
    }
}

fn push_issue(issues: &mut Vec<Issue>, draft: IssueDraft, config: &Config) {
    if config.is_allowed(draft.issue_type) {
        return;
    }
    let severity = severity(draft.affected_combinations, draft.public_api, draft.usage);
    issues.push(Issue {
        key: IssueKey {
            issue_type: draft.issue_type,
            source: draft.source,
            target: draft.target,
        },
        severity,
        file: draft.file,
        line: draft.line,
        surface: draft.surface,
        features: draft.features,
        message: draft.message,
        remediation: draft.remediation,
        affected_combinations: draft.affected_combinations,
        public_api: draft.public_api,
        usage: draft.usage,
    });
}

fn apply_suppressions(issues: Vec<Issue>) -> Result<design_gate_core::SuppressionResult<Issue>> {
    let mut rust_issues = Vec::new();
    let mut kept_manifest = Vec::new();
    for issue in issues {
        if issue.file.extension().and_then(|ext| ext.to_str()) == Some("rs") {
            rust_issues.push(issue);
        } else {
            kept_manifest.push(issue);
        }
    }
    let result = apply_core_suppressions(
        rust_issues,
        |issue| issue.file.as_path(),
        |issue| issue.line,
        |issue| issue.key.issue_type.id(),
        "feature-doctor",
        |marker, issue| {
            parse_issue_type(marker)
                .map(|issue_type| issue_type.id() == issue)
                .unwrap_or(false)
        },
    )?;
    kept_manifest.extend(result.kept);
    Ok(design_gate_core::SuppressionResult {
        kept: kept_manifest,
        suppressed: result.suppressed,
    })
}

fn feature_matrix(
    cargo: &CargoContext,
    parsed: &[ParsedFile],
    issues: &[Issue],
) -> Vec<FeatureMatrixRow> {
    let mut cfg_refs: BTreeMap<String, usize> = BTreeMap::new();
    for site in parsed.iter().flat_map(|file| file.cfg_sites.iter()) {
        let mut features = BTreeSet::new();
        site.expr.features(&mut features);
        for feature in features {
            *cfg_refs.entry(feature).or_insert(0) += 1;
        }
    }
    cargo
        .features
        .keys()
        .filter(|feature| feature.as_str() != "default")
        .map(|feature| {
            let issue_count = issues
                .iter()
                .filter(|issue| issue.features.iter().any(|item| item == feature))
                .count();
            let refs = cfg_refs.get(feature).copied().unwrap_or(0);
            let status = if issue_count > 0 {
                FeatureStatus::Risk
            } else if refs > 0 {
                FeatureStatus::Covered
            } else {
                FeatureStatus::ManifestOnly
            };
            FeatureMatrixRow {
                feature: feature.clone(),
                default: cargo.default_features.contains(feature),
                cfg_refs: refs,
                issue_count,
                status,
            }
        })
        .collect()
}

fn hack_suggestions(issues: &[Issue]) -> Vec<HackSuggestion> {
    let mut suggestions = Vec::new();
    let mut seen = BTreeSet::new();
    for issue in issues {
        match issue.issue_type() {
            IssueType::ExclusiveUndeclared => {
                if issue.features.len() < 2 {
                    continue;
                }
                let command = format!(
                    "cargo hack check --no-default-features --features \"{}\"",
                    issue.features.join(" ")
                );
                if seen.insert(command.clone()) {
                    suggestions.push(HackSuggestion {
                        reason: format!("Reproduce {}", issue.key.target),
                        reason_ja: format!("{} を再現", issue.key.target),
                        command,
                        excluded_features: Vec::new(),
                    });
                }
            }
            IssueType::UntestedCfgPath => {
                if issue.features.is_empty() {
                    let command = "cargo hack check --feature-powerset --no-dev-deps".to_string();
                    if seen.insert(command.clone()) {
                        suggestions.push(HackSuggestion {
                            reason: format!(
                                "Explore {} because it cannot be represented as one additive --features command",
                                issue.key.target
                            ),
                            reason_ja: format!(
                                "{} は単一の加法的 --features で表せないため feature powerset で探索",
                                issue.key.target
                            ),
                            command,
                            excluded_features: Vec::new(),
                        });
                    }
                    continue;
                }
                let excluded_features = excluded_features_from_target(&issue.key.target);
                let command = format!(
                    "cargo hack check --no-default-features --features \"{}\"",
                    issue.features.join(" ")
                );
                if seen.insert(command.clone()) {
                    suggestions.push(HackSuggestion {
                        reason: format!("Exercise {}", issue.key.target),
                        reason_ja: format!("{} を検査", issue.key.target),
                        command,
                        excluded_features,
                    });
                }
            }
            IssueType::DefaultLeak => {
                let command = "cargo hack check --feature-powerset --no-dev-deps".to_string();
                if seen.insert(command.clone()) {
                    suggestions.push(HackSuggestion {
                        reason: "Explore default-feature-sensitive dependency combinations"
                            .to_string(),
                        reason_ja: "default feature に影響される依存組合せを探索".to_string(),
                        command,
                        excluded_features: Vec::new(),
                    });
                }
            }
            IssueType::OptionalDepExposure | IssueType::NonAdditiveFeature => {}
        }
    }
    suggestions
}

fn excluded_features_from_target(target: &str) -> Vec<String> {
    let marker = "not(feature=";
    let mut rest = target;
    let mut out = Vec::new();
    while let Some(idx) = rest.find(marker) {
        let value_start = idx + marker.len();
        let Some(after_start) = rest.get(value_start..) else {
            break;
        };
        let end = after_start.find([')', ',']).unwrap_or(after_start.len());
        if let Some(feature) = after_start.get(..end) {
            out.push(feature.trim().to_string());
        }
        rest = after_start.get(end..).unwrap_or_default();
    }
    out.sort();
    out.dedup();
    out
}

fn estimate_matching_combinations(expr: &CfgExpr, all_features: &BTreeSet<String>) -> u128 {
    let features = all_features.iter().cloned().collect::<Vec<_>>();
    if features.len() <= 16 {
        let total = 1u128 << features.len();
        let mut count = 0u128;
        for mask in 0..total {
            let enabled = features
                .iter()
                .enumerate()
                .filter_map(|(idx, feature)| {
                    if (mask & (1u128 << idx)) != 0 {
                        Some(feature.clone())
                    } else {
                        None
                    }
                })
                .collect::<BTreeSet<_>>();
            if expr.evaluate(&enabled) == Some(true) {
                count += 1;
            }
        }
        return count;
    }
    combinations_with_required(features.len(), 1)
}

fn combinations_with_required(feature_count: usize, required: usize) -> u128 {
    if feature_count <= required {
        return 1;
    }
    checked_power_two(feature_count - required)
}

fn combinations_without_feature(feature_count: usize) -> u128 {
    combinations_with_required(feature_count, 1)
}

fn checked_power_two(exp: usize) -> u128 {
    if exp >= 128 { u128::MAX } else { 1u128 << exp }
}

fn combination_estimate(feature_count: usize) -> String {
    if feature_count >= 128 {
        format!("2^{feature_count}+")
    } else {
        format!("2^{feature_count}={}", 1u128 << feature_count)
    }
}
