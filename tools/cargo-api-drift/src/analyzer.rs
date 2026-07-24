use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use cargo_metadata::MetadataCommand;
use design_gate_core::apply_suppressions as apply_core_suppressions;
use serde::Serialize;

use crate::blind_spot::build as build_blind_spots;
use crate::error::{Error, Result};
use crate::issue::{ChangeKind, Classification, Issue, IssueKey};
use crate::parser::{ApiItem, ApiMember, ApiSurface, ItemKind, TraitMethod, parse_path};
use crate::scoring::{Grade, grade, severity};

#[derive(Debug, Clone, Serialize)]
pub struct Analysis {
    pub project: String,
    pub root: PathBuf,
    pub against: String,
    pub files_analyzed: usize,
    pub suppressed_issues: usize,
    pub grade: Grade,
    pub issues: Vec<Issue>,
    pub blind_spots: design_gate_core::BlindSpotManifest,
}

pub fn analyze_path(path: &Path, against: &str) -> Result<Analysis> {
    let current = parse_path(path)?;
    let worktree = design_gate_core::prepare_baseline_worktree(path, against, "cargo-api-drift")?;
    let baseline =
        parse_path(worktree.baseline_path()).map_err(|error| Error::Baseline(error.to_string()))?;
    let mut issues = classify(&baseline, &current);
    sort_issues(&mut issues);
    let suppression = apply_suppressions(issues, &current.root)?;
    let mut issues = suppression.kept;
    sort_issues(&mut issues);
    let blind_spots = build_blind_spots(baseline.parse_failures + current.parse_failures);
    let grade = grade(&issues);
    Ok(Analysis {
        project: project_name(&current.root),
        root: current.root,
        against: against.to_string(),
        files_analyzed: current.files_analyzed,
        suppressed_issues: suppression.suppressed,
        grade,
        issues,
        blind_spots,
    })
}

fn classify(baseline: &ApiSurface, current: &ApiSurface) -> Vec<Issue> {
    let mut issues = Vec::new();
    for (id, old) in &baseline.items {
        let Some(new) = current.items.get(id) else {
            let classification = Classification::Breaking;
            let change_kind = if old.kind == ItemKind::ReExport {
                ChangeKind::ReExportRemoved
            } else {
                ChangeKind::Removed
            };
            push_issue(IssueDraft {
                issues: &mut issues,
                item: old,
                classification,
                change_kind,
                source: id,
                message: message_removed(old),
                remediation: remediation(classification),
            });
            continue;
        };
        compare_item(old, new, &mut issues);
    }
    for (id, new) in &current.items {
        if !baseline.items.contains_key(id) {
            push_issue(IssueDraft {
                issues: &mut issues,
                item: new,
                classification: Classification::Safe,
                change_kind: ChangeKind::Added,
                source: id,
                message: format!(
                    "new public {} `{}` was added",
                    kind_label(new.kind),
                    new.name
                ),
                remediation: "No action required; include in changelog when relevant.".to_string(),
            });
        }
    }
    issues
}

fn compare_item(old: &ApiItem, new: &ApiItem, issues: &mut Vec<Issue>) {
    if old.kind != new.kind {
        if old.kind == ItemKind::ReExport || new.kind == ItemKind::ReExport {
            return;
        }
        push_issue(IssueDraft {
            issues,
            item: new,
            classification: Classification::Breaking,
            change_kind: ChangeKind::SignatureChanged,
            source: &old.id,
            message: format!(
                "public API kind changed from {} to {} for `{}`",
                kind_label(old.kind),
                kind_label(new.kind),
                old.name
            ),
            remediation: remediation(Classification::Breaking),
        });
        return;
    }

    for derive in old.derives.difference(&new.derives) {
        let source = member_id(&old.id, derive);
        push_issue(IssueDraft {
            issues,
            item: new,
            classification: Classification::Risky,
            change_kind: ChangeKind::DeriveRemoved,
            source: &source,
            message: format!(
                "derive `{derive}` was removed from public type `{}`",
                old.name
            ),
            remediation: remediation(Classification::Risky),
        });
    }
    for derive in new.derives.difference(&old.derives) {
        let source = member_id(&old.id, derive);
        push_issue(IssueDraft {
            issues,
            item: new,
            classification: Classification::Safe,
            change_kind: ChangeKind::Added,
            source: &source,
            message: format!("derive `{derive}` was added to public type `{}`", old.name),
            remediation: remediation(Classification::Safe),
        });
    }
    if old.repr != new.repr {
        push_issue(IssueDraft {
            issues,
            item: new,
            classification: Classification::Risky,
            change_kind: ChangeKind::ReprChanged,
            source: &old.id,
            message: format!("repr changed for public type `{}`", old.name),
            remediation: remediation(Classification::Risky),
        });
    }
    compare_signature_surface(old, new, issues);

    match old.kind {
        ItemKind::Fn
        | ItemKind::TypeAlias
        | ItemKind::Const
        | ItemKind::Static
        | ItemKind::ReExport => {}
        ItemKind::Struct => compare_struct(old, new, issues),
        ItemKind::Enum => compare_enum(old, new, issues),
        ItemKind::Trait => compare_trait(old, new, issues),
    }
}

fn compare_struct(old: &ApiItem, new: &ApiItem, issues: &mut Vec<Issue>) {
    let old_map = member_map(&old.fields);
    let new_map = member_map(&new.fields);
    let old_fields = old_map.keys().cloned().collect::<BTreeSet<_>>();
    let new_fields = new_map.keys().cloned().collect::<BTreeSet<_>>();
    for field in old_fields.difference(&new_fields) {
        let source = member_id(&old.id, field);
        push_issue(IssueDraft {
            issues,
            item: new,
            classification: Classification::Breaking,
            change_kind: ChangeKind::FieldRemoved,
            source: &source,
            message: format!("public field `{field}` was removed from `{}`", old.name),
            remediation: remediation(Classification::Breaking),
        });
    }
    for field in new_fields.difference(&old_fields) {
        let classification = if exhaustive_struct(old) {
            Classification::Breaking
        } else {
            Classification::Risky
        };
        let source = member_id(&old.id, field);
        push_issue(IssueDraft {
            issues,
            item: new,
            classification,
            change_kind: ChangeKind::FieldAdded,
            source: &source,
            message: format!("public field `{field}` was added to `{}`", old.name),
            remediation: remediation(classification),
        });
    }
    for field in old_fields.intersection(&new_fields) {
        if old_map.get(field) != new_map.get(field) {
            let source = member_id(&old.id, field);
            push_issue(IssueDraft {
                issues,
                item: new,
                classification: Classification::Breaking,
                change_kind: ChangeKind::FieldTypeChanged,
                source: &source,
                message: format!("public field `{field}` changed type in `{}`", old.name),
                remediation: remediation(Classification::Breaking),
            });
        }
    }
    if member_order(&old.fields) != member_order(&new.fields) && old_fields == new_fields {
        push_issue(IssueDraft {
            issues,
            item: new,
            classification: Classification::Risky,
            change_kind: ChangeKind::OrderChanged,
            source: &old.id,
            message: format!("public field order changed for `{}`", old.name),
            remediation: remediation(Classification::Risky),
        });
    }
}

fn compare_signature_surface(old: &ApiItem, new: &ApiItem, issues: &mut Vec<Issue>) {
    if old.cfg_attrs != new.cfg_attrs {
        push_issue(IssueDraft {
            issues,
            item: new,
            classification: Classification::Risky,
            change_kind: ChangeKind::CfgChanged,
            source: &old.id,
            message: format!("cfg attributes changed for public `{}`", old.name),
            remediation: remediation(Classification::Risky),
        });
    }
    compare_bounds(&old.id, &old.name, &old.bounds, &new.bounds, new, issues);
    if old.signature != new.signature {
        push_issue(IssueDraft {
            issues,
            item: new,
            classification: Classification::Breaking,
            change_kind: ChangeKind::SignatureChanged,
            source: &old.id,
            message: format!(
                "public {} signature changed for `{}`",
                kind_label(old.kind),
                old.name
            ),
            remediation: remediation(Classification::Breaking),
        });
    }
}

fn compare_trait_method_surface(
    owner_old: &ApiItem,
    owner_new: &ApiItem,
    method: &str,
    old: &TraitMethod,
    new: &TraitMethod,
    issues: &mut Vec<Issue>,
) {
    let source = member_id(&owner_old.id, method);
    if old.cfg_attrs != new.cfg_attrs {
        push_issue(IssueDraft {
            issues,
            item: owner_new,
            classification: Classification::Risky,
            change_kind: ChangeKind::CfgChanged,
            source: &source,
            message: format!(
                "cfg attributes changed for trait method `{method}` in `{}`",
                owner_old.name
            ),
            remediation: remediation(Classification::Risky),
        });
    }
    compare_bounds(&source, method, &old.bounds, &new.bounds, owner_new, issues);
    if old.signature != new.signature {
        push_issue(IssueDraft {
            issues,
            item: owner_new,
            classification: Classification::Breaking,
            change_kind: ChangeKind::SignatureChanged,
            source: &source,
            message: format!(
                "trait method `{method}` signature changed in `{}`",
                owner_old.name
            ),
            remediation: remediation(Classification::Breaking),
        });
    }
}

fn compare_bounds(
    source_prefix: &str,
    item_name: &str,
    old: &BTreeMap<String, BTreeSet<String>>,
    new: &BTreeMap<String, BTreeSet<String>>,
    item: &ApiItem,
    issues: &mut Vec<Issue>,
) {
    let params = old
        .keys()
        .chain(new.keys())
        .cloned()
        .collect::<BTreeSet<_>>();
    for param in params {
        let old_bounds = old.get(&param).cloned().unwrap_or_default();
        let new_bounds = new.get(&param).cloned().unwrap_or_default();
        for bound in new_bounds.difference(&old_bounds) {
            let source = member_id(source_prefix, &format!("{param}:{bound}"));
            push_issue(IssueDraft {
                issues,
                item,
                classification: Classification::Breaking,
                change_kind: ChangeKind::BoundAdded,
                source: &source,
                message: format!("bound `{bound}` was added to `{param}` on `{item_name}`"),
                remediation: remediation(Classification::Breaking),
            });
        }
        for bound in old_bounds.difference(&new_bounds) {
            let source = member_id(source_prefix, &format!("{param}:{bound}"));
            push_issue(IssueDraft {
                issues,
                item,
                classification: Classification::Risky,
                change_kind: ChangeKind::BoundRemoved,
                source: &source,
                message: format!("bound `{bound}` was relaxed from `{param}` on `{item_name}`"),
                remediation: remediation(Classification::Risky),
            });
        }
    }
}

fn member_map(members: &[ApiMember]) -> BTreeMap<String, String> {
    members
        .iter()
        .map(|member| (member.name.clone(), member.signature.clone()))
        .collect()
}

fn member_order(members: &[ApiMember]) -> Vec<String> {
    members.iter().map(|member| member.name.clone()).collect()
}

fn member_id(owner: &str, member: &str) -> String {
    format!("{owner}::{member}")
}

fn exhaustive_struct(item: &ApiItem) -> bool {
    !item.non_exhaustive && item.all_fields_public
}

fn compare_enum(old: &ApiItem, new: &ApiItem, issues: &mut Vec<Issue>) {
    let old_map = member_map(&old.variants);
    let new_map = member_map(&new.variants);
    let old_variants = old_map.keys().cloned().collect::<BTreeSet<_>>();
    let new_variants = new_map.keys().cloned().collect::<BTreeSet<_>>();
    for variant in old_variants.difference(&new_variants) {
        let source = member_id(&old.id, variant);
        push_issue(IssueDraft {
            issues,
            item: new,
            classification: Classification::Breaking,
            change_kind: ChangeKind::VariantRemoved,
            source: &source,
            message: format!(
                "public enum variant `{variant}` was removed from `{}`",
                old.name
            ),
            remediation: remediation(Classification::Breaking),
        });
    }
    for variant in new_variants.difference(&old_variants) {
        let classification = if old.non_exhaustive || old.is_error_enum {
            Classification::Risky
        } else {
            Classification::Breaking
        };
        let source = member_id(&old.id, variant);
        push_issue(IssueDraft {
            issues,
            item: new,
            classification,
            change_kind: ChangeKind::VariantAdded,
            source: &source,
            message: format!(
                "public enum variant `{variant}` was added to `{}`",
                old.name
            ),
            remediation: remediation(classification),
        });
    }
    for variant in old_variants.intersection(&new_variants) {
        if old_map.get(variant) != new_map.get(variant) {
            let source = member_id(&old.id, variant);
            push_issue(IssueDraft {
                issues,
                item: new,
                classification: Classification::Breaking,
                change_kind: ChangeKind::SignatureChanged,
                source: &source,
                message: format!(
                    "public enum variant `{variant}` changed shape in `{}`",
                    old.name
                ),
                remediation: remediation(Classification::Breaking),
            });
        }
    }
    if member_order(&old.variants) != member_order(&new.variants) && old_variants == new_variants {
        push_issue(IssueDraft {
            issues,
            item: new,
            classification: Classification::Risky,
            change_kind: ChangeKind::OrderChanged,
            source: &old.id,
            message: format!("public enum variant order changed for `{}`", old.name),
            remediation: remediation(Classification::Risky),
        });
    }
}

fn compare_trait(old: &ApiItem, new: &ApiItem, issues: &mut Vec<Issue>) {
    let old_methods = old.trait_methods.keys().cloned().collect::<BTreeSet<_>>();
    let new_methods = new.trait_methods.keys().cloned().collect::<BTreeSet<_>>();
    for method in old_methods.difference(&new_methods) {
        let source = member_id(&old.id, method);
        push_issue(IssueDraft {
            issues,
            item: new,
            classification: Classification::Breaking,
            change_kind: ChangeKind::Removed,
            source: &source,
            message: format!("trait method `{method}` was removed from `{}`", old.name),
            remediation: remediation(Classification::Breaking),
        });
    }
    for method in new_methods.difference(&old_methods) {
        let Some(info) = new.trait_methods.get(method) else {
            continue;
        };
        let classification = if info.has_default {
            Classification::Risky
        } else {
            Classification::Breaking
        };
        let source = member_id(&old.id, method);
        push_issue(IssueDraft {
            issues,
            item: new,
            classification,
            change_kind: ChangeKind::TraitMethodAdded,
            source: &source,
            message: format!("trait method `{method}` was added to `{}`", old.name),
            remediation: remediation(classification),
        });
    }
    for method in old_methods.intersection(&new_methods) {
        let (Some(old_info), Some(new_info)) =
            (old.trait_methods.get(method), new.trait_methods.get(method))
        else {
            continue;
        };
        let source = member_id(&old.id, method);
        compare_trait_method_surface(old, new, method, old_info, new_info, issues);
        if old_info.has_default && !new_info.has_default {
            push_issue(IssueDraft {
                issues,
                item: new,
                classification: Classification::Breaking,
                change_kind: ChangeKind::TraitMethodDefaultRemoved,
                source: &source,
                message: format!(
                    "trait method `{method}` lost its default body in `{}`",
                    old.name
                ),
                remediation: remediation(Classification::Breaking),
            });
        } else if !old_info.has_default && new_info.has_default {
            push_issue(IssueDraft {
                issues,
                item: new,
                classification: Classification::Risky,
                change_kind: ChangeKind::TraitMethodAdded,
                source: &source,
                message: format!(
                    "trait method `{method}` gained a default body in `{}`",
                    old.name
                ),
                remediation: remediation(Classification::Risky),
            });
        }
    }
}

struct IssueDraft<'a> {
    issues: &'a mut Vec<Issue>,
    item: &'a ApiItem,
    classification: Classification,
    change_kind: ChangeKind,
    source: &'a str,
    message: String,
    remediation: String,
}

fn push_issue(draft: IssueDraft<'_>) {
    draft.issues.push(Issue {
        key: IssueKey {
            classification: draft.classification,
            source: draft.source.to_string(),
            target: draft.change_kind.id().to_string(),
        },
        change_kind: draft.change_kind,
        severity: severity(draft.classification, draft.change_kind),
        file: draft.item.file.clone(),
        line: draft.item.line,
        message: draft.message,
        remediation: draft.remediation,
    });
}

fn apply_suppressions(
    mut issues: Vec<Issue>,
    root: &Path,
) -> Result<design_gate_core::SuppressionResult<Issue>> {
    for issue in &mut issues {
        issue.file = root.join(&issue.file);
    }
    let (readable, unreadable): (Vec<_>, Vec<_>) =
        issues.into_iter().partition(|issue| issue.file.is_file());
    let mut suppression = apply_core_suppressions(
        readable,
        |issue| issue.file.as_path(),
        |issue| issue.line,
        |issue| issue.key.classification.id(),
        "api-drift",
        |marker, issue_type| marker == issue_type,
    )?;
    suppression.kept.extend(unreadable);
    for issue in &mut suppression.kept {
        if let Ok(relative) = issue.file.strip_prefix(root) {
            issue.file = relative.to_path_buf();
        }
    }
    Ok(suppression)
}

fn sort_issues(issues: &mut Vec<Issue>) {
    issues.sort_by(|a, b| {
        b.severity
            .cmp(&a.severity)
            .then_with(|| a.file.cmp(&b.file))
            .then_with(|| a.line.cmp(&b.line))
            .then_with(|| a.key.classification.id().cmp(b.key.classification.id()))
            .then_with(|| a.key.source.cmp(&b.key.source))
            .then_with(|| a.key.target.cmp(&b.key.target))
    });
    issues.dedup_by(|a, b| a.key == b.key);
}

fn message_removed(item: &ApiItem) -> String {
    format!(
        "public {} `{}` was removed",
        kind_label(item.kind),
        item.name
    )
}

fn remediation(classification: Classification) -> String {
    match classification {
        Classification::Breaking => {
            "Restore compatibility, provide a deprecated shim, or release a semver-major version."
                .to_string()
        }
        Classification::Risky => {
            "Call out the behavior or source-compatibility risk in release notes and downstream migration notes."
                .to_string()
        }
        Classification::Safe => "No action required.".to_string(),
    }
}

fn kind_label(kind: ItemKind) -> &'static str {
    match kind {
        ItemKind::Fn => "fn",
        ItemKind::Struct => "struct",
        ItemKind::Enum => "enum",
        ItemKind::Trait => "trait",
        ItemKind::TypeAlias => "type alias",
        ItemKind::Const => "const",
        ItemKind::Static => "static",
        ItemKind::ReExport => "re-export",
    }
}

fn project_name(root: &Path) -> String {
    let start = if root.is_file() {
        root.parent().unwrap_or(root)
    } else {
        root
    };
    for ancestor in start.ancestors() {
        let manifest = ancestor.join("Cargo.toml");
        if !manifest.is_file() {
            continue;
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::{ApiItem, ItemKind};
    use std::collections::{BTreeMap, BTreeSet};

    fn item(id: &str, kind: ItemKind) -> ApiItem {
        ApiItem {
            id: id.to_string(),
            kind,
            name: id.rsplit(':').next().unwrap_or(id).to_string(),
            file: PathBuf::from("src/lib.rs"),
            line: 1,
            signature: id.to_string(),
            cfg_attrs: BTreeSet::new(),
            bounds: BTreeMap::new(),
            derives: BTreeSet::new(),
            repr: None,
            non_exhaustive: false,
            all_fields_public: true,
            is_error_enum: false,
            fields: Vec::new(),
            variants: Vec::new(),
            trait_methods: BTreeMap::new(),
        }
    }

    #[test]
    fn re_export_removal_is_breaking() {
        let mut old = ApiSurface::default();
        old.items.insert(
            "src/lib.rs:Thing".to_string(),
            item("src/lib.rs:Thing", ItemKind::ReExport),
        );
        let new = ApiSurface::default();
        let issues = classify(&old, &new);
        assert_eq!(issues[0].key.classification, Classification::Breaking);
        assert_eq!(issues[0].change_kind, ChangeKind::ReExportRemoved);
    }
}
