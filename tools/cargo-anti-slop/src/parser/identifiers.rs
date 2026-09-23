use std::collections::BTreeMap;

use ra_ap_syntax::SyntaxNode;
use ra_ap_syntax::ast::{self, AstNode};

use crate::issue::{IssueType, RiskAxis};

use super::erasure::function_visibility_condition;
use super::{FileContext, Finding, compact_path, excluded_test_context, in_trait_impl, is_ident};

const RAW_ID_TYPES: &[&str] = &[
    "u8", "u16", "u32", "u64", "u128", "usize", "i32", "i64", "String", "&str", "&String",
];

pub(crate) fn detect(root: &SyntaxNode, ctx: &FileContext<'_>, findings: &mut Vec<Finding>) {
    if ctx.config.is_allowed(IssueType::RawIdParams) {
        return;
    }
    for func in root.descendants().filter_map(ast::Fn::cast) {
        if excluded_test_context(func.syntax()) || in_trait_impl(func.syntax()) {
            continue;
        }
        let Some(params) = func.param_list() else {
            continue;
        };
        let mut by_type: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for param in params.params() {
            let Some(name) = param
                .pat()
                .map(|pat| compact_path(pat.syntax().text().to_string()))
                .filter(|name| is_ident(name) && is_id_name(name))
            else {
                continue;
            };
            let Some(ty) = param
                .ty()
                .map(|ty| compact_path(ty.syntax().text().to_string()))
                .filter(|ty| RAW_ID_TYPES.contains(&ty.as_str()))
            else {
                continue;
            };
            by_type.entry(ty).or_default().push(name);
        }
        let identity = ctx
            .function_names
            .get(&func.syntax().text_range())
            .cloned()
            .unwrap_or_else(|| format!("{}:<fn>", ctx.rel_path));
        for (ty, names) in by_type {
            if names.len() < 2 {
                continue;
            }
            let joined = names.join(", ");
            findings.push(Finding {
                issue_type: IssueType::RawIdParams,
                risk: RiskAxis::Evidence,
                condition: function_visibility_condition(&func),
                file: ctx.file.to_path_buf(),
                rel_path: ctx.rel_path.to_string(),
                line: ctx.line(func.syntax().text_range()),
                function: identity.clone(),
                target: format!("{}:{ty}", names.join(",")),
                message: format!(
                    "identifier parameters `{joined}` share raw type `{ty}`; call sites can swap them silently"
                ),
                remediation:
                    "Wrap each identifier in a newtype (e.g. `CustomerId(u64)`, `OrderId(u64)`) so argument order is compiler-checked."
                        .to_string(),
            });
        }
    }
}

fn is_id_name(name: &str) -> bool {
    name == "id" || name.ends_with("_id")
}

#[cfg(test)]
mod tests {
    use super::super::parse_fixture;
    use crate::issue::IssueType;

    #[test]
    fn two_same_typed_id_params_are_reported() {
        let findings = parse_fixture(
            r#"
            pub fn link(customer_id: u64, order_id: u64) -> bool {
                customer_id == order_id
            }
            "#,
        );
        assert!(
            findings
                .iter()
                .any(|finding| finding.issue_type == IssueType::RawIdParams)
        );
    }

    #[test]
    fn single_or_newtyped_id_params_are_not_reported() {
        let findings = parse_fixture(
            r#"
            pub struct CustomerId(u64);
            pub struct OrderId(u64);

            pub fn lookup(customer_id: u64) -> u64 {
                customer_id
            }

            pub fn link(customer_id: CustomerId, order_id: OrderId) -> bool {
                customer_id.0 == order_id.0
            }
            "#,
        );
        assert!(
            !findings
                .iter()
                .any(|finding| finding.issue_type == IssueType::RawIdParams)
        );
    }
}
