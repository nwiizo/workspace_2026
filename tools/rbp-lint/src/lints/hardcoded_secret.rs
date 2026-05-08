use ra_ap_syntax::ast::{self, AstNode};

use crate::diagnostic::{Diagnostic, Severity};
use crate::lints::{LintContext, LintRule};

pub struct HardcodedSecret;

/// Heuristic patterns for likely-hardcoded credentials in string literals.
const SUSPICIOUS_PREFIXES: &[&str] = &[
    "sk-",
    "ghp_",
    "gho_",
    "github_pat_",
    "AKIA",
    "AIza",
    "xoxb-",
    "xoxp-",
    "Bearer ",
];

impl LintRule for HardcodedSecret {
    fn id(&self) -> &'static str {
        "hardcoded-secret"
    }

    fn description(&self) -> &'static str {
        "Detect string literals that look like API keys or tokens"
    }

    fn check(&self, ctx: &LintContext<'_>, out: &mut Vec<Diagnostic>) {
        for node in ctx.tree.syntax().descendants() {
            let Some(lit) = ast::Literal::cast(node) else {
                continue;
            };
            let text = lit.syntax().text().to_string();
            // Only string literals.
            let Some(stripped) = text.strip_prefix('"').and_then(|s| s.strip_suffix('"')) else {
                continue;
            };
            if !SUSPICIOUS_PREFIXES.iter().any(|p| stripped.starts_with(p)) {
                continue;
            }
            let range = lit.syntax().text_range();
            out.push(Diagnostic::from_range(
                ctx.file,
                self.id(),
                Severity::Error,
                "string literal looks like a hardcoded secret",
                ctx.source,
                range,
                Some(
                    "load secrets from env vars or `secrecy::SecretString`; never check them in"
                        .into(),
                ),
            ));
        }
    }
}
