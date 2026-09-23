use ra_ap_syntax::SyntaxNode;
use ra_ap_syntax::ast::{self, AstNode, HasAttrs, HasGenericArgs, HasName, HasVisibility};

use crate::issue::{IssueType, RiskAxis};

use super::{
    CONDITION_LOW, CONDITION_MEDIUM, FileContext, Finding, compact_path, excluded_test_context,
    in_trait_impl, visibility_condition,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ErasureKind {
    Any,
    Value,
    Dictionary,
}

#[derive(Debug, Clone)]
pub(crate) struct Erasure {
    pub kind: ErasureKind,
    pub display: String,
}

const WRAPPER_TYPES: &[&str] = &["Box", "Rc", "Arc", "Option", "Vec", "Cow"];

pub(crate) fn classify_type(ty: &ast::Type, ctx: &FileContext<'_>) -> Option<Erasure> {
    match ty {
        ast::Type::RefType(inner) => classify_type(&inner.ty()?, ctx),
        ast::Type::ParenType(inner) => classify_type(&inner.ty()?, ctx),
        ast::Type::DynTraitType(dyn_ty) => {
            let bounds = dyn_ty.type_bound_list()?;
            for bound in bounds.bounds() {
                let text = compact_path(bound.syntax().text().to_string());
                let resolved = ctx.aliases.resolve_path(&text);
                if is_any_path(&resolved) {
                    return Some(Erasure {
                        kind: ErasureKind::Any,
                        display: "dyn Any".to_string(),
                    });
                }
            }
            None
        }
        ast::Type::PathType(path_ty) => {
            let path = path_ty.path()?;
            let segment = path.segment()?;
            let name = segment.name_ref()?.text().to_string();
            let base = path_base_text(&path, &name);
            let resolved = ctx.aliases.resolve_path(&base);
            if is_any_path(&resolved) {
                return Some(Erasure {
                    kind: ErasureKind::Any,
                    display: resolved,
                });
            }
            if ctx.config.erased_types.contains(&resolved) {
                return Some(Erasure {
                    kind: ErasureKind::Value,
                    display: resolved,
                });
            }
            let args = type_args(&segment);
            if ctx.config.map_types.contains(&name) {
                let value = args.get(1)?;
                let inner = classify_type(value, ctx)?;
                return Some(Erasure {
                    kind: ErasureKind::Dictionary,
                    display: format!("{name}<_, {}>", inner.display),
                });
            }
            if WRAPPER_TYPES.contains(&name.as_str()) {
                let inner = args.first()?;
                return classify_type(inner, ctx);
            }
            None
        }
        _ => None,
    }
}

fn is_any_path(resolved: &str) -> bool {
    matches!(resolved, "std::any::Any" | "core::any::Any" | "Any")
}

fn path_base_text(path: &ast::Path, name: &str) -> String {
    match path.qualifier() {
        Some(qualifier) => format!(
            "{}::{name}",
            compact_path(qualifier.syntax().text().to_string())
        ),
        None => name.to_string(),
    }
}

fn type_args(segment: &ast::PathSegment) -> Vec<ast::Type> {
    segment
        .generic_arg_list()
        .map(|list| {
            list.generic_args()
                .filter_map(|arg| match arg {
                    ast::GenericArg::TypeArg(type_arg) => type_arg.ty(),
                    _ => None,
                })
                .collect()
        })
        .unwrap_or_default()
}

pub(crate) fn detect(root: &SyntaxNode, ctx: &FileContext<'_>, findings: &mut Vec<Finding>) {
    detect_functions(root, ctx, findings);
    detect_aliases(root, ctx, findings);
    detect_fields(root, ctx, findings);
}

fn detect_functions(root: &SyntaxNode, ctx: &FileContext<'_>, findings: &mut Vec<Finding>) {
    for func in root.descendants().filter_map(ast::Fn::cast) {
        if excluded_test_context(func.syntax()) || in_trait_impl(func.syntax()) {
            continue;
        }
        let identity = ctx
            .function_names
            .get(&func.syntax().text_range())
            .cloned()
            .unwrap_or_else(|| format!("{}:<fn>", ctx.rel_path));
        let condition = function_visibility_condition(&func);
        if let Some(params) = func.param_list() {
            for param in params.params() {
                let Some(ty) = param.ty() else {
                    continue;
                };
                let Some(erasure) = classify_type(&ty, ctx) else {
                    continue;
                };
                let name = param
                    .pat()
                    .map(|pat| compact_path(pat.syntax().text().to_string()))
                    .unwrap_or_else(|| "_".to_string());
                push_signature_finding(
                    ctx,
                    findings,
                    &erasure,
                    &identity,
                    condition,
                    ty.syntax(),
                    format!("param:{name}:{}", erasure.display),
                    format!(
                        "parameter `{name}` erases its contract to `{}`",
                        erasure.display
                    ),
                    "Parse at the boundary into a typed struct or enum and pass the evidence, not the raw value.",
                );
            }
        }
        if let Some(ret) = func.ret_type()
            && let Some(ty) = ret.ty()
            && let Some(erasure) = classify_type(&ty, ctx)
        {
            push_signature_finding(
                ctx,
                findings,
                &erasure,
                &identity,
                condition,
                ty.syntax(),
                format!("return:{}", erasure.display),
                format!("function returns erased type `{}`", erasure.display),
                "Return a typed struct or enum; keep Value/Any behind the boundary that produced it.",
            );
        }
    }
}

fn detect_aliases(root: &SyntaxNode, ctx: &FileContext<'_>, findings: &mut Vec<Finding>) {
    if ctx.config.is_allowed(IssueType::ErasedAlias) {
        return;
    }
    for alias in root.descendants().filter_map(ast::TypeAlias::cast) {
        if excluded_test_context(alias.syntax()) {
            continue;
        }
        let Some(ty) = alias.ty() else {
            continue;
        };
        let Some(erasure) = classify_type(&ty, ctx) else {
            continue;
        };
        let name = alias
            .name()
            .map(|name| name.text().to_string())
            .unwrap_or_else(|| "<alias>".to_string());
        let condition = visibility_condition(alias.visibility()).max(CONDITION_MEDIUM);
        findings.push(Finding {
            issue_type: IssueType::ErasedAlias,
            risk: RiskAxis::Evidence,
            condition,
            file: ctx.file.to_path_buf(),
            rel_path: ctx.rel_path.to_string(),
            line: ctx.line(alias.syntax().text_range()),
            function: ctx.item_identity(&name),
            target: erasure.display.clone(),
            message: format!("type alias `{name}` conceals erased type `{}`", erasure.display),
            remediation:
                "Replace the alias with a real domain type; an alias does not restore lost type evidence."
                    .to_string(),
        });
    }
}

fn detect_fields(root: &SyntaxNode, ctx: &FileContext<'_>, findings: &mut Vec<Finding>) {
    for record in root.descendants().filter_map(ast::RecordField::cast) {
        if excluded_test_context(record.syntax()) {
            continue;
        }
        let Some(ty) = record.ty() else {
            continue;
        };
        let Some(erasure) = classify_type(&ty, ctx) else {
            continue;
        };
        let field = record
            .name()
            .map(|name| name.text().to_string())
            .unwrap_or_else(|| "<field>".to_string());
        let owner = record
            .syntax()
            .ancestors()
            .find_map(ast::Struct::cast)
            .and_then(|item| item.name())
            .map(|name| name.text().to_string())
            .unwrap_or_else(|| "<struct>".to_string());
        // `#[serde(flatten)]` on a catch-all map is a declared
        // forward-compatibility boundary, not accidental erasure.
        let condition = if has_serde_flatten(&record) {
            CONDITION_LOW
        } else {
            visibility_condition(record.visibility())
        };
        push_signature_finding(
            ctx,
            findings,
            &erasure,
            &ctx.item_identity(&format!("{owner}::{field}")),
            condition,
            record.syntax(),
            format!("field:{field}:{}", erasure.display),
            format!(
                "struct field `{field}` stores erased type `{}`",
                erasure.display
            ),
            "Model the field as a typed struct or enum; deserialize once at the edge.",
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn push_signature_finding(
    ctx: &FileContext<'_>,
    findings: &mut Vec<Finding>,
    erasure: &Erasure,
    identity: &str,
    condition: u8,
    node: &SyntaxNode,
    target: String,
    message: String,
    remediation: &str,
) {
    let issue_type = match erasure.kind {
        ErasureKind::Dictionary => IssueType::ErasedDictionary,
        ErasureKind::Any | ErasureKind::Value => IssueType::ErasedSignature,
    };
    if ctx.config.is_allowed(issue_type) {
        return;
    }
    let remediation = match issue_type {
        IssueType::ErasedDictionary => {
            "Give dictionary values a concrete contract type, or model the shape as a struct."
        }
        _ => remediation,
    };
    findings.push(Finding {
        issue_type,
        risk: RiskAxis::Evidence,
        condition,
        file: ctx.file.to_path_buf(),
        rel_path: ctx.rel_path.to_string(),
        line: ctx.line(node.text_range()),
        function: identity.to_string(),
        target,
        message,
        remediation: remediation.to_string(),
    });
}

fn has_serde_flatten(field: &ast::RecordField) -> bool {
    field.attrs().any(|attr| {
        let text = compact_path(attr.syntax().text().to_string());
        text.starts_with("#[serde(") && text.contains("flatten")
    })
}

pub(crate) fn function_visibility_condition(func: &ast::Fn) -> u8 {
    let own = visibility_condition(func.visibility());
    let trait_vis = func
        .syntax()
        .ancestors()
        .skip(1)
        .find_map(ast::Trait::cast)
        .map(|tr| visibility_condition(tr.visibility()));
    trait_vis.unwrap_or(own)
}

#[cfg(test)]
mod tests {
    use super::super::parse_fixture;
    use crate::issue::IssueType;

    #[test]
    fn serde_flatten_catch_all_scores_low() {
        let findings = parse_fixture(
            r#"
            use std::collections::BTreeMap;
            use serde::Serialize;
            use serde_json::Value;

            #[derive(Serialize)]
            pub struct Stats {
                pub total: usize,
                #[serde(flatten)]
                pub measurements: BTreeMap<String, Value>,
            }

            #[derive(Serialize)]
            pub struct Bag {
                pub extra: BTreeMap<String, Value>,
            }
            "#,
        );
        let conditions: Vec<u8> = findings
            .iter()
            .filter(|finding| finding.issue_type == IssueType::ErasedDictionary)
            .map(|finding| finding.condition)
            .collect();
        assert_eq!(conditions.len(), 2, "{findings:?}");
        assert!(conditions.contains(&super::CONDITION_LOW));
        assert!(conditions.iter().any(|c| *c > super::CONDITION_LOW));
    }
}
