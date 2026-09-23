use std::collections::HashSet;

use ra_ap_syntax::SyntaxNode;
use ra_ap_syntax::ast::{self, AstNode, HasName, HasVisibility};

use crate::issue::{IssueType, RiskAxis};

use super::{CONDITION_MEDIUM, FileContext, Finding, excluded_test_context, visibility_condition};

pub(crate) fn detect(root: &SyntaxNode, ctx: &FileContext<'_>, findings: &mut Vec<Finding>) {
    if ctx.config.is_allowed(IssueType::VagueSymbolName) {
        return;
    }
    for node in root.descendants() {
        let named: Option<(String, Option<ast::Visibility>)> =
            if let Some(item) = ast::Struct::cast(node.clone()) {
                item.name()
                    .map(|name| (name.text().to_string(), item.visibility()))
            } else if let Some(item) = ast::Enum::cast(node.clone()) {
                item.name()
                    .map(|name| (name.text().to_string(), item.visibility()))
            } else if let Some(item) = ast::Trait::cast(node.clone()) {
                item.name()
                    .map(|name| (name.text().to_string(), item.visibility()))
            } else if let Some(item) = ast::TypeAlias::cast(node.clone()) {
                item.name()
                    .map(|name| (name.text().to_string(), item.visibility()))
            } else {
                None
            };
        let Some((name, visibility)) = named else {
            continue;
        };
        if excluded_test_context(&node) {
            continue;
        }
        let Some(word) = vague_word_hit(&name, &ctx.config.vague_words) else {
            continue;
        };
        findings.push(Finding {
            issue_type: IssueType::VagueSymbolName,
            risk: RiskAxis::Signal,
            condition: visibility_condition(visibility).min(CONDITION_MEDIUM),
            file: ctx.file.to_path_buf(),
            rel_path: ctx.rel_path.to_string(),
            line: ctx.line(node.text_range()),
            function: ctx.item_identity(&name),
            target: name.clone(),
            message: format!("type name `{name}` carries low signal (`{word}`)"),
            remediation:
                "Name the type after its domain meaning, not its shape or role in the code."
                    .to_string(),
        });
    }
}

fn vague_word_hit(name: &str, words: &HashSet<String>) -> Option<String> {
    if words.contains(name) {
        return Some(name.to_string());
    }
    words
        .iter()
        .find(|word| {
            name.len() > word.len()
                && name.ends_with(word.as_str())
                && name[..name.len() - word.len()]
                    .chars()
                    .next_back()
                    .is_some_and(|ch| ch.is_ascii_lowercase())
        })
        .cloned()
}

#[cfg(test)]
mod tests {
    use super::super::parse_fixture;
    use crate::issue::IssueType;

    #[test]
    fn suffix_and_exact_vague_names_are_reported() {
        let findings = parse_fixture(
            r#"
            pub struct UserData {
                pub id: u64,
            }
            struct Helper;
            "#,
        );
        let names: Vec<_> = findings
            .iter()
            .filter(|finding| finding.issue_type == IssueType::VagueSymbolName)
            .map(|finding| finding.target.clone())
            .collect();
        assert!(names.contains(&"UserData".to_string()));
        assert!(names.contains(&"Helper".to_string()));
    }

    #[test]
    fn camel_case_boundary_is_respected() {
        let findings = parse_fixture(
            r#"
            pub struct Metadata {
                pub id: u64,
            }
            "#,
        );
        assert!(
            !findings
                .iter()
                .any(|finding| finding.issue_type == IssueType::VagueSymbolName)
        );
    }
}
