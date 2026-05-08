#![allow(
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_used,
    reason = "tests keep assertions and fixture setup direct"
)]

use std::path::{Path, PathBuf};
use std::process::Command;

fn fixture_manifest(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
        .join("Cargo.toml")
}

fn cargo_command() -> String {
    std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned())
}

fn run_fixture(name: &str, should_succeed: bool) {
    let manifest = fixture_manifest(name);
    let target_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("ui-fixtures")
        .join(name);

    let output = Command::new(cargo_command())
        .arg("check")
        .arg("--quiet")
        .arg("--offline")
        .arg("--manifest-path")
        .arg(&manifest)
        .env("CARGO_TARGET_DIR", &target_dir)
        .output()
        .expect("fixture cargo check should run");

    if should_succeed {
        assert!(
            output.status.success(),
            "fixture {name} should compile\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    } else {
        assert!(
            !output.status.success(),
            "fixture {name} should fail to compile\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

#[test]
fn compile_fail_examples_enforce_type_walls() {
    run_fixture("sealed_trait_external_impl_fail", false);
    run_fixture("non_exhaustive_match_fail", false);
    run_fixture("non_exhaustive_struct_literal_fail", false);
    run_fixture("non_exhaustive_wildcard_pass", true);
}
