use ra_ap_syntax::ast::{self, AstNode, HasAttrs, HasName};
use ra_ap_syntax::{SyntaxKind, SyntaxNode, SyntaxToken};

/// True if `node` is inside a `#[test]` function, a `#[cfg(test)]` module,
/// or any `tests` named module.
pub fn is_in_test_context(node: &SyntaxNode) -> bool {
    for ancestor in node.ancestors() {
        if let Some(func) = ast::Fn::cast(ancestor.clone()) {
            if has_test_attr(&func) {
                return true;
            }
        }
        if let Some(module) = ast::Module::cast(ancestor.clone()) {
            if has_cfg_test_attr(&module) {
                return true;
            }
            if let Some(name) = module.name() {
                if name.text() == "tests" || name.text() == "test" {
                    return true;
                }
            }
        }
    }
    false
}

fn has_test_attr<N: HasAttrs>(node: &N) -> bool {
    node.attrs().any(|attr| attr_path_string(&attr) == "test")
}

fn has_cfg_test_attr<N: HasAttrs>(node: &N) -> bool {
    node.attrs().any(|attr| {
        if attr_path_string(&attr) != "cfg" {
            return false;
        }
        attr_full_text(&attr).contains("test")
    })
}

/// Return the full syntax text of an attribute, including the `#[..]` wrapper.
/// Used as a coarse substring check for things like `dead_code` or `test`.
pub fn attr_full_text(attr: &ast::Attr) -> String {
    attr.syntax().text().to_string()
}

pub fn attr_path_string(attr: &ast::Attr) -> String {
    attr.path()
        .map(|p| p.syntax().text().to_string())
        .unwrap_or_default()
}

/// Find the previous non-whitespace sibling token of `node`.
pub fn prev_meaningful_token(node: &SyntaxNode) -> Option<SyntaxToken> {
    let mut tok = node.first_token()?.prev_token()?;
    loop {
        match tok.kind() {
            SyntaxKind::WHITESPACE => {
                tok = tok.prev_token()?;
            }
            _ => return Some(tok),
        }
    }
}

/// True if any of `comment_lines` is a `// rbp-lint-allow: <rule>[, <rule>...]`
/// (synonyms `disable` / `ignore`) that names `rule_id` or `all`.
pub fn comment_allows(comment_lines: &[&str], rule_id: &str) -> bool {
    for raw in comment_lines {
        let lower = raw.to_ascii_lowercase();
        let stripped = lower
            .trim_start()
            .trim_start_matches("///")
            .trim_start_matches("//!")
            .trim_start_matches("//")
            .trim();
        for marker in ["rbp-lint-allow:", "rbp-lint-disable:", "rbp-lint-ignore:"] {
            let Some(rest) = stripped.strip_prefix(marker) else {
                continue;
            };
            // strip trailing free-text after `(reason: ...)` or `--`.
            let list = rest.split('(').next().unwrap_or(rest);
            let list = list.split("--").next().unwrap_or(list);
            for tok in list.split(',') {
                let tok = tok.trim();
                if tok == rule_id || tok == "all" {
                    return true;
                }
            }
        }
    }
    false
}
