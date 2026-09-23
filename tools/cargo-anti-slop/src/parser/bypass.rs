use std::collections::HashMap;

use ra_ap_syntax::SyntaxNode;
use ra_ap_syntax::ast::{self, AstNode, HasAttrs, HasName};

use crate::issue::{IssueType, RiskAxis};

use super::{
    CONDITION_HIGH, CONDITION_MEDIUM, FileContext, Finding, compact_path, excluded_test_context,
};

#[derive(Debug, Default)]
struct ImplSurface {
    smart_constructor: bool,
    from_impls: Vec<(String, usize)>,
}

pub(crate) fn detect(root: &SyntaxNode, ctx: &FileContext<'_>, findings: &mut Vec<Finding>) {
    if ctx.config.is_allowed(IssueType::ConstructorBypass) {
        return;
    }
    let mut surfaces: HashMap<String, ImplSurface> = HashMap::new();
    for imp in root.descendants().filter_map(ast::Impl::cast) {
        if excluded_test_context(imp.syntax()) {
            continue;
        }
        let Some(self_name) = self_type_head(&imp) else {
            continue;
        };
        let surface = surfaces.entry(self_name).or_default();
        match imp.trait_() {
            Some(trait_ty) => {
                let trait_text = compact_path(trait_ty.syntax().text().to_string());
                if trait_text.starts_with("From<") {
                    surface
                        .from_impls
                        .push((trait_text, ctx.line(imp.syntax().text_range())));
                }
            }
            None => {
                if imp
                    .syntax()
                    .descendants()
                    .filter_map(ast::Fn::cast)
                    .any(|func| is_smart_constructor(&func))
                {
                    surface.smart_constructor = true;
                }
            }
        }
    }

    for strukt in root.descendants().filter_map(ast::Struct::cast) {
        if excluded_test_context(strukt.syntax()) || !has_only_private_fields(&strukt) {
            continue;
        }
        let Some(name) = strukt.name().map(|name| name.text().to_string()) else {
            continue;
        };
        let Some(surface) = surfaces.get(&name).filter(|s| s.smart_constructor) else {
            continue;
        };
        let derives = derive_traits(&strukt);
        let line = ctx.line(strukt.syntax().text_range());
        if derives.iter().any(|derive| derive == "Default") {
            push(
                ctx,
                findings,
                &name,
                line,
                CONDITION_MEDIUM,
                "derive(Default)".to_string(),
                format!(
                    "`{name}` has a smart constructor but `derive(Default)` constructs it without validation"
                ),
                "Remove the Default derive, or make the default value provably valid through the constructor.",
            );
        }
        if derives.iter().any(|derive| derive == "Deserialize") && !has_serde_conversion(&strukt) {
            push(
                ctx,
                findings,
                &name,
                line,
                CONDITION_HIGH,
                "derive(Deserialize)".to_string(),
                format!(
                    "`{name}` has a smart constructor but `derive(Deserialize)` deserializes past the validation"
                ),
                "Route serde through the constructor with `#[serde(try_from = \"RawType\")]`, or deserialize a raw DTO and convert with TryFrom.",
            );
        }
        for (trait_text, impl_line) in &surface.from_impls {
            push(
                ctx,
                findings,
                &name,
                *impl_line,
                CONDITION_MEDIUM,
                trait_text.clone(),
                format!(
                    "`{name}` has a smart constructor but `impl {trait_text}` converts infallibly around it"
                ),
                "Replace the infallible From with TryFrom so the conversion carries the validation.",
            );
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn push(
    ctx: &FileContext<'_>,
    findings: &mut Vec<Finding>,
    name: &str,
    line: usize,
    condition: u8,
    target: String,
    message: String,
    remediation: &str,
) {
    findings.push(Finding {
        issue_type: IssueType::ConstructorBypass,
        risk: RiskAxis::Evidence,
        condition,
        file: ctx.file.to_path_buf(),
        rel_path: ctx.rel_path.to_string(),
        line,
        function: ctx.item_identity(name),
        target,
        message,
        remediation: remediation.to_string(),
    });
}

fn self_type_head(imp: &ast::Impl) -> Option<String> {
    let text = compact_path(imp.self_ty()?.syntax().text().to_string());
    let head = text
        .split(['<', ':'])
        .find(|part| !part.is_empty())
        .unwrap_or(&text);
    (!head.is_empty()).then(|| head.to_string())
}

fn is_smart_constructor(func: &ast::Fn) -> bool {
    let Some(name) = func.name().map(|name| name.text().to_string()) else {
        return false;
    };
    let named_like_constructor = name == "new"
        || name == "parse"
        || name == "from_str"
        || name.starts_with("try_")
        || name.starts_with("new_");
    // A constructor builds a value from raw input; methods taking self
    // (try_clone, try_lock, ...) transform an already-validated value.
    let takes_self = func
        .param_list()
        .is_some_and(|params| params.self_param().is_some());
    named_like_constructor
        && !takes_self
        && func
            .ret_type()
            .and_then(|ret| ret.ty())
            .map(|ty| compact_path(ty.syntax().text().to_string()).starts_with("Result<"))
            .unwrap_or(false)
}

fn has_only_private_fields(strukt: &ast::Struct) -> bool {
    use ra_ap_syntax::ast::HasVisibility;
    match strukt.field_list() {
        Some(ast::FieldList::RecordFieldList(fields)) => {
            let mut fields = fields.fields().peekable();
            fields.peek().is_some() && fields.all(|field| field.visibility().is_none())
        }
        Some(ast::FieldList::TupleFieldList(fields)) => {
            let mut fields = fields.fields().peekable();
            fields.peek().is_some() && fields.all(|field| field.visibility().is_none())
        }
        None => false,
    }
}

fn derive_traits(strukt: &ast::Struct) -> Vec<String> {
    let mut traits = Vec::new();
    for attr in strukt.attrs() {
        let text = compact_path(attr.syntax().text().to_string());
        let Some(inner) = text
            .strip_prefix("#[derive(")
            .and_then(|rest| rest.strip_suffix(")]"))
        else {
            continue;
        };
        for entry in inner.split(',') {
            let last = entry.rsplit("::").next().unwrap_or(entry).trim();
            if !last.is_empty() {
                traits.push(last.to_string());
            }
        }
    }
    traits
}

fn has_serde_conversion(strukt: &ast::Struct) -> bool {
    strukt.attrs().any(|attr| {
        let text = compact_path(attr.syntax().text().to_string());
        text.starts_with("#[serde(") && (text.contains("try_from") || text.contains("from="))
    })
}

#[cfg(test)]
mod tests {
    use super::super::parse_fixture;
    use crate::issue::IssueType;

    fn bypass_targets(source: &str) -> Vec<String> {
        parse_fixture(source)
            .into_iter()
            .filter(|finding| finding.issue_type == IssueType::ConstructorBypass)
            .map(|finding| finding.target)
            .collect()
    }

    #[test]
    fn derive_and_from_bypasses_are_reported() {
        let targets = bypass_targets(
            r#"
            use serde::Deserialize;

            #[derive(Debug, Default, Deserialize)]
            pub struct Email(String);

            impl Email {
                pub fn new(raw: &str) -> Result<Self, String> {
                    if raw.contains('@') {
                        Ok(Self(raw.to_string()))
                    } else {
                        Err("invalid".to_string())
                    }
                }
            }

            impl From<String> for Email {
                fn from(raw: String) -> Self {
                    Self(raw)
                }
            }
            "#,
        );
        assert!(
            targets.contains(&"derive(Default)".to_string()),
            "{targets:?}"
        );
        assert!(
            targets.contains(&"derive(Deserialize)".to_string()),
            "{targets:?}"
        );
        assert!(targets.contains(&"From<String>".to_string()), "{targets:?}");
    }

    #[test]
    fn self_taking_try_methods_are_not_smart_constructors() {
        let targets = bypass_targets(
            r#"
            pub struct File {
                inner: u64,
            }

            impl File {
                pub fn try_clone(&self) -> Result<Self, String> {
                    Ok(Self { inner: self.inner })
                }
            }

            impl From<u64> for File {
                fn from(inner: u64) -> Self {
                    Self { inner }
                }
            }
            "#,
        );
        assert!(targets.is_empty(), "{targets:?}");
    }

    #[test]
    fn serde_try_from_and_plain_structs_are_not_reported() {
        let targets = bypass_targets(
            r#"
            use serde::Deserialize;

            #[derive(Debug, Deserialize)]
            #[serde(try_from = "String")]
            pub struct Email(String);

            impl Email {
                pub fn new(raw: &str) -> Result<Self, String> {
                    Ok(Self(raw.to_string()))
                }
            }

            #[derive(Debug, Default, Deserialize)]
            pub struct Options {
                retries: u8,
            }
            "#,
        );
        assert!(targets.is_empty(), "{targets:?}");
    }
}
