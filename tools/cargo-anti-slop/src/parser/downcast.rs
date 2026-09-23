use std::collections::HashMap;

use ra_ap_syntax::SyntaxNode;
use ra_ap_syntax::ast::{self, AstNode, HasGenericArgs};

use crate::issue::{IssueType, RiskAxis};

use super::{
    CONDITION_HIGH, CONDITION_LOW, CONDITION_MEDIUM, FileContext, Finding, compact_path,
    excluded_test_context,
};

struct Candidate {
    line: usize,
    identity: String,
    target: String,
    error_ish: bool,
}

pub(crate) fn detect(root: &SyntaxNode, ctx: &FileContext<'_>, findings: &mut Vec<Finding>) {
    if ctx.config.is_allowed(IssueType::RuntimeDowncast) {
        return;
    }
    let mut candidates = Vec::new();
    for method in root.descendants().filter_map(ast::MethodCallExpr::cast) {
        if excluded_test_context(method.syntax()) {
            continue;
        }
        let Some(name) = method.name_ref().map(|name| name.text().to_string()) else {
            continue;
        };
        let has_turbofish = method.generic_arg_list().is_some();
        let matched = matches!(name.as_str(), "downcast_ref" | "downcast_mut" | "downcast")
            || (name == "is" && has_turbofish);
        if !matched {
            continue;
        }
        let receiver = method
            .receiver()
            .map(|receiver| compact_path(receiver.syntax().text().to_string()))
            .unwrap_or_default();
        let type_arg = method
            .generic_arg_list()
            .map(|args| compact_path(args.syntax().text().to_string()));
        candidates.push(Candidate {
            line: ctx.line(method.syntax().text_range()),
            identity: ctx.identity_for(method.syntax()),
            target: match type_arg {
                Some(args) => format!("{name}{args}"),
                None => name,
            },
            error_ish: is_error_ish(&receiver),
        });
    }
    for call in root.descendants().filter_map(ast::CallExpr::cast) {
        if excluded_test_context(call.syntax()) {
            continue;
        }
        let Some(expr) = call.expr() else {
            continue;
        };
        let path = ctx
            .aliases
            .resolve_path(&compact_path(expr.syntax().text().to_string()));
        let base = path.split("::<").next().unwrap_or(&path);
        if !matches!(
            base,
            "TypeId::of" | "std::any::TypeId::of" | "core::any::TypeId::of"
        ) {
            continue;
        }
        candidates.push(Candidate {
            line: ctx.line(call.syntax().text_range()),
            identity: ctx.identity_for(call.syntax()),
            target: "TypeId::of".to_string(),
            error_ish: false,
        });
    }

    let mut per_function: HashMap<String, usize> = HashMap::new();
    for candidate in &candidates {
        *per_function.entry(candidate.identity.clone()).or_insert(0) += 1;
    }
    for candidate in candidates {
        let condition = if candidate.error_ish {
            CONDITION_LOW
        } else if per_function.get(&candidate.identity).copied().unwrap_or(0) >= 2 {
            CONDITION_HIGH
        } else {
            CONDITION_MEDIUM
        };
        findings.push(Finding {
            issue_type: IssueType::RuntimeDowncast,
            risk: RiskAxis::Evidence,
            condition,
            file: ctx.file.to_path_buf(),
            rel_path: ctx.rel_path.to_string(),
            line: candidate.line,
            function: candidate.identity,
            target: candidate.target.clone(),
            message: format!(
                "runtime type inspection `{}` re-asserts evidence the type system already had",
                candidate.target
            ),
            remediation:
                "Model the variants as an enum or move behavior into the trait; parse at the boundary instead of probing types at runtime."
                    .to_string(),
        });
    }
}

fn is_error_ish(receiver: &str) -> bool {
    let lowered = receiver.to_ascii_lowercase();
    lowered.contains("err") || lowered.contains("cause") || lowered.contains("source")
}

#[cfg(test)]
mod tests {
    use super::super::parse_fixture;
    use crate::issue::IssueType;

    #[test]
    fn multiple_downcasts_in_one_function_score_high() {
        let findings = parse_fixture(
            r#"
            use std::any::Any;
            fn dispatch(value: &dyn Any) {
                if let Some(a) = value.downcast_ref::<u8>() {
                    let _ = a;
                } else if let Some(b) = value.downcast_ref::<u16>() {
                    let _ = b;
                }
            }
            "#,
        );
        let downcasts: Vec<_> = findings
            .iter()
            .filter(|finding| finding.issue_type == IssueType::RuntimeDowncast)
            .collect();
        assert_eq!(downcasts.len(), 2);
        assert!(downcasts.iter().all(|finding| finding.condition == 3));
    }

    #[test]
    fn error_boundary_downcast_scores_low() {
        let findings = parse_fixture(
            r#"
            fn classify(error: &anyhow::Error) -> bool {
                error.downcast_ref::<std::io::Error>().is_some()
            }
            "#,
        );
        let downcasts: Vec<_> = findings
            .iter()
            .filter(|finding| finding.issue_type == IssueType::RuntimeDowncast)
            .collect();
        assert_eq!(downcasts.len(), 1);
        assert_eq!(downcasts[0].condition, 1);
    }
}
