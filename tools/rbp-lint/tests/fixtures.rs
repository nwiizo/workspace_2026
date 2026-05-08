use std::collections::BTreeSet;
use std::path::PathBuf;

use rbp_lint::{all_lints, lint_source};

fn fixture_path(name: &str) -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("tests/fixtures");
    p.push(name);
    p
}

fn run(name: &str) -> Vec<rbp_lint::Diagnostic> {
    let path = fixture_path(name);
    let source = std::fs::read_to_string(&path).expect("fixture exists");
    let lints = all_lints();
    lint_source(&path, &source, &lints)
}

#[test]
fn good_fixture_is_clean() {
    let diags = run("good.rs");
    assert!(
        diags.is_empty(),
        "expected no diagnostics, got {:#?}",
        diags
    );
}

#[test]
fn bad_fixture_triggers_each_rule() {
    let diags = run("bad.rs");
    let rules: BTreeSet<&str> = diags.iter().map(|d| d.rule).collect();
    let expected = [
        "no-unwrap",
        "no-expect",
        "no-panic",
        "dead-code-comment",
        "tracing-format",
        "arc-clone-explicit",
        "hardcoded-secret",
        "unsafe-safety-comment",
        "debug-print",
        "string-as-error",
        "unbounded-channel",
        "unwrap-or-default-call",
        "needless-return",
        "lazy-static-macro",
        "manual-let-else",
        "pub-field-newtype",
        "non-exhaustive-pub-error",
        "raw-id-field",
        "status-string-field",
        "bool-option-pair",
    ];
    for rule in expected {
        assert!(
            rules.contains(rule),
            "expected rule {rule} to fire on bad.rs; got {:?}",
            rules
        );
    }
}
