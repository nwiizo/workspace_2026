use ra_ap_syntax::ast::{self, AstNode, HasArgList};
use ra_ap_syntax::{SyntaxKind, SyntaxNode};

use crate::issue::{IssueType, RiskAxis};

use super::{
    CONDITION_HIGH, CONDITION_MEDIUM, FileContext, Finding, compact_path, excluded_test_context,
};

pub(crate) fn detect(root: &SyntaxNode, ctx: &FileContext<'_>, findings: &mut Vec<Finding>) {
    if ctx.config.is_allowed(IssueType::DefaultSwallow) {
        return;
    }
    for method in root.descendants().filter_map(ast::MethodCallExpr::cast) {
        if excluded_test_context(method.syntax()) {
            continue;
        }
        let Some(name) = method.name_ref().map(|name| name.text().to_string()) else {
            continue;
        };
        if has_args(&method) {
            continue;
        }
        match name.as_str() {
            "ok" => {
                if method
                    .syntax()
                    .parent()
                    .is_some_and(|parent| parent.kind() == SyntaxKind::EXPR_STMT)
                {
                    push(
                        ctx,
                        findings,
                        &method,
                        CONDITION_MEDIUM,
                        "ok",
                        "`.ok()` in statement position silently discards the error".to_string(),
                        "Handle or log the error, or document why it can be ignored.",
                    );
                }
            }
            "unwrap_or_default" => {
                let receiver = method
                    .receiver()
                    .map(|receiver| compact_path(receiver.syntax().text().to_string()))
                    .unwrap_or_default();
                if ctx
                    .config
                    .fallible_markers
                    .iter()
                    .any(|marker| receiver.contains(marker.as_str()))
                {
                    push(
                        ctx,
                        findings,
                        &method,
                        CONDITION_HIGH,
                        "unwrap_or_default",
                        "`unwrap_or_default()` fabricates a default value when the fallible chain fails"
                            .to_string(),
                        "Propagate the error with `?`, or make the fallback explicit with unwrap_or_else and a logged reason.",
                    );
                }
            }
            _ => {}
        }
    }
}

fn has_args(method: &ast::MethodCallExpr) -> bool {
    method
        .arg_list()
        .is_some_and(|args| args.args().next().is_some())
}

fn push(
    ctx: &FileContext<'_>,
    findings: &mut Vec<Finding>,
    method: &ast::MethodCallExpr,
    condition: u8,
    target: &str,
    message: String,
    remediation: &str,
) {
    findings.push(Finding {
        issue_type: IssueType::DefaultSwallow,
        risk: RiskAxis::DataLoss,
        condition,
        file: ctx.file.to_path_buf(),
        rel_path: ctx.rel_path.to_string(),
        line: ctx.line(method.syntax().text_range()),
        function: ctx.identity_for(method.syntax()),
        target: target.to_string(),
        message,
        remediation: remediation.to_string(),
    });
}

#[cfg(test)]
mod tests {
    use super::super::parse_fixture;
    use crate::issue::IssueType;

    #[test]
    fn statement_ok_and_parse_default_are_reported() {
        let findings = parse_fixture(
            r#"
            fn run(raw: &str) -> u16 {
                std::fs::remove_file("state.json").ok();
                raw.parse::<u16>().unwrap_or_default()
            }
            "#,
        );
        let swallows: Vec<_> = findings
            .iter()
            .filter(|finding| finding.issue_type == IssueType::DefaultSwallow)
            .collect();
        assert_eq!(swallows.len(), 2);
    }

    #[test]
    fn plain_option_unwrap_or_default_is_not_reported() {
        let findings = parse_fixture(
            r#"
            fn pick(values: Option<Vec<u8>>) -> Vec<u8> {
                values.unwrap_or_default()
            }
            "#,
        );
        assert!(
            !findings
                .iter()
                .any(|finding| finding.issue_type == IssueType::DefaultSwallow)
        );
    }
}
