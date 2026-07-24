use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use cargo_metadata::MetadataCommand;
use design_gate_core::{
    RustFileWalkerOptions, SuppressionResult, apply_suppressions as apply_core_suppressions,
    rust_files,
};
use ra_ap_syntax::Edition;
use rayon::prelude::*;
use serde::Serialize;

use crate::blind_spot::{BlindSpotManifest, build as build_blind_spots};
use crate::config::Config;
use crate::error::{Error, Result};
use crate::issue::{Issue, IssueKey, IssueType, Layer};
use crate::parser::{ImplInfo, ParsedFile, TraitInfo, parse_file};
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
    pub traits: Vec<TraitDetail>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TraitDetail {
    pub name: String,
    pub file: PathBuf,
    pub line: usize,
    pub public: bool,
    pub method_count: usize,
    pub associated_type_count: usize,
    pub methods: Vec<String>,
    pub impls: Vec<String>,
    pub dyn_uses: Vec<String>,
    pub object_safe: bool,
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
        .map(|file| parse_file(&root, file, cargo.edition))
        .collect();
    let mut parsed = Vec::new();
    for result in parsed_results {
        parsed.push(result?);
    }

    let parse_failures = parsed.iter().map(|file| file.parse_errors).sum();
    let traits = parsed
        .iter()
        .flat_map(|file| file.traits.iter().cloned())
        .collect::<Vec<_>>();
    let impls = parsed
        .iter()
        .flat_map(|file| file.impls.iter().cloned())
        .collect::<Vec<_>>();
    let dyn_uses = parsed
        .iter()
        .flat_map(|file| file.dyn_uses.iter().cloned())
        .collect::<Vec<_>>();
    let mut fan_in = trait_fan_in(&traits, &impls, &dyn_uses);
    let mut issues = Vec::new();

    detect_oversized_traits(&traits, &fan_in, config, &mut issues);
    detect_single_impl_traits(&traits, &impls, &fan_in, config, &mut issues);
    detect_object_safety_risks(&traits, &dyn_uses, &fan_in, &mut issues);
    detect_broad_blanket_impls(&impls, &fan_in, &mut issues);
    detect_unmockable_boundaries(&parsed, &mut fan_in, &mut issues);

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
    let trait_details = trait_details(&traits, &impls, &dyn_uses);
    let blind_spots = build_blind_spots(
        parse_failures,
        cargo.metadata_failed,
        cargo.edition_fallback_2024,
    );
    Ok(Analysis {
        project,
        root,
        files_analyzed: parsed.len(),
        suppressed_issues: suppression.suppressed,
        grade: grade(&issues),
        issues,
        blind_spots,
        traits: trait_details,
    })
}

fn detect_oversized_traits(
    traits: &[TraitInfo],
    fan_in: &HashMap<String, usize>,
    config: &Config,
    issues: &mut Vec<Issue>,
) {
    for tr in traits {
        let method_over = tr.methods.len().saturating_sub(config.method_threshold);
        let type_over = tr
            .associated_type_count
            .saturating_sub(config.associated_type_threshold);
        if method_over == 0 && type_over == 0 {
            continue;
        }
        let fan = *fan_in.get(&tr.name).unwrap_or(&0);
        let magnitude = method_over + type_over;
        push_issue(
            issues,
            IssueDraft {
                issue_type: IssueType::OversizedTrait,
                public_reach: tr.public,
                fan_in: fan,
                magnitude,
                file: tr.file.clone(),
                line: tr.line,
                source: tr.source.clone(),
                target: format!("{}-methods-{}-assoc-types", tr.methods.len(), tr.associated_type_count),
                message: format!(
                    "trait `{}` has {} methods and {} associated types (thresholds: {}/{})",
                    tr.name, tr.methods.len(), tr.associated_type_count, config.method_threshold, config.associated_type_threshold
                ),
                remediation: "Split independent responsibilities or move infrequently used operations behind smaller extension traits.".to_string(),
            },
        );
    }
}

fn detect_single_impl_traits(
    traits: &[TraitInfo],
    impls: &[ImplInfo],
    fan_in: &HashMap<String, usize>,
    config: &Config,
    issues: &mut Vec<Issue>,
) {
    for tr in traits {
        if config.is_intentional_trait(&tr.name) {
            continue;
        }
        let concrete_impls = impls
            .iter()
            .filter(|imp| !imp.in_test && imp.trait_name.as_deref() == Some(tr.name.as_str()))
            .count();
        if concrete_impls > 1 {
            continue;
        }
        let (target, message) = if concrete_impls == 0 {
            (
                "no-production-impl".to_string(),
                format!("trait `{}` has no non-test implementations", tr.name),
            )
        } else {
            (
                "one-production-impl".to_string(),
                format!(
                    "trait `{}` has exactly one non-test implementation",
                    tr.name
                ),
            )
        };
        let fan = *fan_in.get(&tr.name).unwrap_or(&0);
        push_issue(
            issues,
            IssueDraft {
                issue_type: IssueType::SingleImplAbstraction,
                public_reach: tr.public,
                fan_in: fan,
                magnitude: 1,
                file: tr.file.clone(),
                line: tr.line,
                source: tr.source.clone(),
                target,
                message,
                remediation: "Inline the abstraction, add a second real implementation, or declare intent in trait-surface.toml.".to_string(),
            },
        );
    }
}

fn detect_object_safety_risks(
    traits: &[TraitInfo],
    dyn_uses: &[crate::parser::DynUse],
    fan_in: &HashMap<String, usize>,
    issues: &mut Vec<Issue>,
) {
    for tr in traits {
        let dyn_count = dyn_uses
            .iter()
            .filter(|use_site| use_site.trait_name == tr.name)
            .count();
        if dyn_count == 0 {
            continue;
        }
        let mut risky = tr
            .methods
            .iter()
            .filter(|method| {
                (method.is_async && !tr.has_async_trait_attr)
                    || ((method.has_generic_params
                        || method.returns_self
                        || method.takes_self_type)
                        && !method.has_where_self_sized)
            })
            .map(|method| {
                if method.is_async && !tr.has_async_trait_attr {
                    format!("{}(async)", method.name)
                } else {
                    method.name.clone()
                }
            })
            .collect::<Vec<_>>();
        risky.sort();
        risky.dedup();
        if risky.is_empty() {
            continue;
        }
        let fan = *fan_in.get(&tr.name).unwrap_or(&0);
        let async_only = !tr.has_async_trait_attr
            && tr.methods.iter().any(|method| method.is_async)
            && tr.methods.iter().all(|method| {
                !method.has_generic_params && !method.returns_self && !method.takes_self_type
            });
        let scored_fan = if async_only { fan.min(4) } else { fan };
        push_issue(
            issues,
            IssueDraft {
                issue_type: IssueType::ObjectSafetyRisk,
                public_reach: tr.public
                    || dyn_uses
                        .iter()
                        .any(|use_site| use_site.trait_name == tr.name && use_site.public_context),
                fan_in: scored_fan,
                magnitude: risky.len(),
                file: tr.file.clone(),
                line: tr.line,
                source: tr.source.clone(),
                target: risky.join(","),
                message: format!(
                    "`dyn {}` is used but methods may break object safety: {}",
                    tr.name,
                    risky.join(", ")
                ),
                remediation: "Add `where Self: Sized` to non-object-safe methods or split object-safe and generic construction APIs.".to_string(),
            },
        );
    }
}

fn detect_broad_blanket_impls(
    impls: &[ImplInfo],
    fan_in: &HashMap<String, usize>,
    issues: &mut Vec<Issue>,
) {
    for imp in impls {
        if imp.in_test {
            continue;
        }
        let Some(bound) = &imp.broad_blanket else {
            continue;
        };
        let Some(trait_name) = &imp.trait_name else {
            continue;
        };
        let fan = *fan_in.get(trait_name).unwrap_or(&0);
        push_issue(
            issues,
            IssueDraft {
                issue_type: IssueType::BroadBlanketImpl,
                public_reach: true,
                fan_in: fan,
                magnitude: 2,
                file: imp.file.clone(),
                line: imp.line,
                source: format!("{}:{}", imp.rel_path, trait_name),
                target: format!("impl<{}> for {}", bound, imp.target),
                message: format!(
                    "blanket impl for `{trait_name}` is constrained only by broad bound `{bound}`"
                ),
                remediation: "Constrain the impl with a domain marker trait or implement the trait for concrete owned types.".to_string(),
            },
        );
    }
}

fn detect_unmockable_boundaries(
    parsed: &[ParsedFile],
    fan_in: &mut HashMap<String, usize>,
    issues: &mut Vec<Issue>,
) {
    for dep in parsed.iter().flat_map(|file| file.io_dependencies.iter()) {
        *fan_in.entry(dep.item.clone()).or_insert(0) += 1;
        push_issue(
            issues,
            IssueDraft {
                issue_type: IssueType::UnmockableBoundary,
                public_reach: true,
                fan_in: *fan_in.get(&dep.item).unwrap_or(&1),
                magnitude: 2,
                file: dep.file.clone(),
                line: dep.line,
                source: format!("{}:{}", dep.rel_path, dep.item),
                target: dep.concrete_type.clone(),
                message: format!(
                    "public API `{}` exposes concrete I/O type `{}` without a trait boundary",
                    dep.item, dep.concrete_type
                ),
                remediation: "Accept a small domain trait, generic capability, or adapter type so tests can substitute the boundary.".to_string(),
            },
        );
    }
}

fn trait_fan_in(
    traits: &[TraitInfo],
    impls: &[ImplInfo],
    dyn_uses: &[crate::parser::DynUse],
) -> HashMap<String, usize> {
    let mut counts = traits
        .iter()
        .map(|tr| (tr.name.clone(), 0usize))
        .collect::<HashMap<_, _>>();
    for imp in impls {
        if imp.in_test {
            continue;
        }
        if let Some(name) = &imp.trait_name {
            *counts.entry(name.clone()).or_insert(0) += 1;
        }
    }
    for use_site in dyn_uses {
        *counts.entry(use_site.trait_name.clone()).or_insert(0) += 1;
    }
    counts
}

fn trait_details(
    traits: &[TraitInfo],
    impls: &[ImplInfo],
    dyn_uses: &[crate::parser::DynUse],
) -> Vec<TraitDetail> {
    let mut details = traits
        .iter()
        .map(|tr| {
            let implementations = impls
                .iter()
                .filter(|imp| !imp.in_test && imp.trait_name.as_deref() == Some(tr.name.as_str()))
                .map(|imp| format!("{}:{} for {}", imp.rel_path, imp.line, imp.target))
                .collect::<Vec<_>>();
            let uses = dyn_uses
                .iter()
                .filter(|use_site| use_site.trait_name == tr.name)
                .map(|use_site| format!("{}:{}", use_site.rel_path, use_site.line))
                .collect::<Vec<_>>();
            TraitDetail {
                name: tr.name.clone(),
                file: PathBuf::from(&tr.rel_path),
                line: tr.line,
                public: tr.public,
                method_count: tr.methods.len(),
                associated_type_count: tr.associated_type_count,
                methods: tr
                    .methods
                    .iter()
                    .map(|method| format!("{}:{}", method.name, method.line))
                    .collect(),
                object_safe: tr.methods.iter().all(|method| {
                    (!method.is_async || tr.has_async_trait_attr)
                        && (!(method.has_generic_params
                            || method.returns_self
                            || method.takes_self_type)
                            || method.has_where_self_sized)
                }),
                impls: implementations,
                dyn_uses: uses,
            }
        })
        .collect::<Vec<_>>();
    details.sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.file.cmp(&b.file)));
    details
}

struct IssueDraft {
    issue_type: IssueType,
    public_reach: bool,
    fan_in: usize,
    magnitude: usize,
    file: PathBuf,
    line: usize,
    source: String,
    target: String,
    message: String,
    remediation: String,
}

fn push_issue(issues: &mut Vec<Issue>, draft: IssueDraft) {
    let layer = if draft.public_reach {
        Layer::PublicApi
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
            draft.fan_in,
            draft.magnitude,
        ),
        file: draft.file,
        line: draft.line,
        layer,
        message: draft.message,
        remediation: draft.remediation,
        fan_in: draft.fan_in,
    });
}

fn apply_suppressions(issues: Vec<Issue>) -> Result<SuppressionResult<Issue>> {
    apply_core_suppressions(
        issues,
        |issue| issue.file.as_path(),
        |issue| issue.line,
        |issue| issue.key.issue_type.id(),
        "trait-surface",
        |marker, issue| marker == issue,
    )
    .map_err(Error::from)
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
