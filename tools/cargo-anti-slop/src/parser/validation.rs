use ra_ap_syntax::SyntaxNode;
use ra_ap_syntax::ast::{self, AstNode, HasName};

use crate::issue::{IssueType, RiskAxis};

use super::erasure::function_visibility_condition;
use super::{FileContext, Finding, compact_path, excluded_test_context, in_trait_impl};

pub(crate) fn detect(root: &SyntaxNode, ctx: &FileContext<'_>, findings: &mut Vec<Finding>) {
    if ctx.config.is_allowed(IssueType::BoolValidation) {
        return;
    }
    for func in root.descendants().filter_map(ast::Fn::cast) {
        if excluded_test_context(func.syntax()) || in_trait_impl(func.syntax()) {
            continue;
        }
        let Some(name) = func.name().map(|name| name.text().to_string()) else {
            continue;
        };
        if !is_validation_name(&name) || !returns_bare_bool(&func) || !has_input_param(&func) {
            continue;
        }
        let identity = ctx
            .function_names
            .get(&func.syntax().text_range())
            .cloned()
            .unwrap_or_else(|| format!("{}:{name}", ctx.rel_path));
        findings.push(Finding {
            issue_type: IssueType::BoolValidation,
            risk: RiskAxis::Evidence,
            condition: function_visibility_condition(&func),
            file: ctx.file.to_path_buf(),
            rel_path: ctx.rel_path.to_string(),
            line: ctx.line(func.syntax().text_range()),
            function: identity,
            target: name.clone(),
            message: format!(
                "`{name}` validates and returns `bool`, discarding the proof of validation"
            ),
            remediation:
                "Parse, don't validate: return a typed value (e.g. `Email::new(&str) -> Result<Email, E>`) so the check leaves evidence in the type."
                    .to_string(),
        });
    }
}

// anti-slop-allow: bool-validation -- name-classifier predicate, not domain validation
fn is_validation_name(name: &str) -> bool {
    name.contains("valid") || name.starts_with("verify")
}

fn returns_bare_bool(func: &ast::Fn) -> bool {
    func.ret_type()
        .and_then(|ret| ret.ty())
        .map(|ty| compact_path(ty.syntax().text().to_string()) == "bool")
        .unwrap_or(false)
}

// Single-character/byte classifiers (`is_valid_cap_letter(b: u8)`) are
// predicates over an alphabet, not validation of raw input data.
fn has_input_param(func: &ast::Fn) -> bool {
    func.param_list().is_some_and(|params| {
        params.params().any(|param| {
            param
                .ty()
                .map(|ty| {
                    let ty = compact_path(ty.syntax().text().to_string());
                    !matches!(ty.as_str(), "u8" | "char" | "&u8" | "&char" | "bool")
                })
                .unwrap_or(false)
        })
    })
}

#[cfg(test)]
mod tests {
    use super::super::parse_fixture;
    use crate::issue::IssueType;

    #[test]
    fn bool_returning_validator_is_reported() {
        let findings = parse_fixture(
            r#"
            pub fn validate_email(raw: &str) -> bool {
                raw.contains('@')
            }
            "#,
        );
        assert!(
            findings
                .iter()
                .any(|finding| finding.issue_type == IssueType::BoolValidation)
        );
    }

    #[test]
    fn state_query_without_input_is_not_reported() {
        let findings = parse_fixture(
            r#"
            pub struct Session {
                expires_at: u64,
            }
            impl Session {
                pub fn is_valid(&self) -> bool {
                    self.expires_at > 0
                }
            }
            "#,
        );
        assert!(
            !findings
                .iter()
                .any(|finding| finding.issue_type == IssueType::BoolValidation)
        );
    }

    #[test]
    fn byte_classifier_predicate_is_not_reported() {
        let findings = parse_fixture(
            r#"
            pub fn is_valid_cap_letter(b: u8) -> bool {
                b.is_ascii_alphanumeric() || b == b'_'
            }
            "#,
        );
        assert!(
            !findings
                .iter()
                .any(|finding| finding.issue_type == IssueType::BoolValidation)
        );
    }

    #[test]
    fn smart_constructor_is_not_reported() {
        let findings = parse_fixture(
            r#"
            pub struct Email(String);
            impl Email {
                pub fn validate(raw: &str) -> Result<Email, String> {
                    if raw.contains('@') {
                        Ok(Email(raw.to_string()))
                    } else {
                        Err("invalid".to_string())
                    }
                }
            }
            "#,
        );
        assert!(
            !findings
                .iter()
                .any(|finding| finding.issue_type == IssueType::BoolValidation)
        );
    }
}
