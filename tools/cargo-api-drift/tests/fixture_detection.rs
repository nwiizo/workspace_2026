use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use std::path::Path;
use std::process::Command as StdCommand;
use tempfile::TempDir;

fn fixture(old: &str, new: &str) -> TempDir {
    let dir = TempDir::new().expect("tempdir");
    fs::create_dir_all(dir.path().join("src")).expect("src");
    fs::write(
        dir.path().join("Cargo.toml"),
        r#"[package]
name = "fixture"
version = "0.1.0"
edition = "2024"
publish = false
"#,
    )
    .expect("manifest");
    fs::write(dir.path().join("src/lib.rs"), old).expect("old lib");
    git(dir.path(), &["init"]);
    git(dir.path(), &["add", "."]);
    git(
        dir.path(),
        &[
            "-c",
            "user.email=a@example.com",
            "-c",
            "user.name=A",
            "commit",
            "-m",
            "baseline",
        ],
    );
    let branch = baseline_ref(dir.path());
    git(dir.path(), &["branch", &branch]);
    fs::write(dir.path().join("src/lib.rs"), new).expect("new lib");
    dir
}

fn baseline_ref(path: &Path) -> String {
    format!(
        "baseline-{}",
        path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("fixture")
    )
}

fn git(cwd: &Path, args: &[&str]) {
    let status = StdCommand::new("git")
        .current_dir(cwd)
        .args(args)
        .status()
        .expect("git");
    assert!(status.success(), "git {args:?} failed");
}

fn api_drift(dir: &TempDir, extra: &[&str]) -> String {
    let mut command = Command::cargo_bin("cargo-api-drift").expect("binary");
    let output = command
        .arg(dir.path())
        .args(["--against", &baseline_ref(dir.path())])
        .args(extra)
        .output()
        .expect("run cargo-api-drift");
    assert!(
        output.status.success(),
        "stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).expect("utf8")
}

fn api_drift_output(dir: &TempDir, extra: &[&str]) -> (i32, String, String) {
    let mut command = Command::cargo_bin("cargo-api-drift").expect("binary");
    let output = command
        .arg(dir.path())
        .args(["--against", &baseline_ref(dir.path())])
        .args(extra)
        .output()
        .expect("run cargo-api-drift");
    (
        output.status.code().unwrap_or(255),
        String::from_utf8(output.stdout).expect("stdout utf8"),
        String::from_utf8(output.stderr).expect("stderr utf8"),
    )
}

#[test]
fn non_exhaustive_variant_addition_is_risky_not_breaking() {
    let dir = fixture(
        "#[non_exhaustive]\npub enum Mode { A }\n",
        "#[non_exhaustive]\npub enum Mode { A, B }\n",
    );
    let out = api_drift(&dir, &[]);
    assert!(out.contains("risky variant-added"));
    assert!(!out.contains("breaking variant-added"));
}

#[test]
fn exhaustive_variant_addition_is_breaking() {
    let dir = fixture("pub enum Mode { A }\n", "pub enum Mode { A, B }\n");
    let out = api_drift(&dir, &[]);
    assert!(out.contains("breaking variant-added"));
}

#[test]
fn derive_removal_is_risky_and_addition_is_safe_under_all() {
    let removed = fixture(
        "#[derive(Clone)]\npub struct Thing;\n",
        "pub struct Thing;\n",
    );
    assert!(api_drift(&removed, &[]).contains("risky derive-removed"));

    let added = fixture(
        "pub struct Thing;\n",
        "#[derive(Clone)]\npub struct Thing;\n",
    );
    let default = api_drift(&added, &[]);
    assert!(!default.contains("derive `Clone` was added"));
    assert!(api_drift(&added, &["--all"]).contains("safe added"));
}

#[test]
fn public_fn_argument_addition_breaks_but_new_fn_is_hidden_by_default() {
    let dir = fixture(
        "pub fn parse(input: &str) -> usize { input.len() }\n",
        "pub fn parse(input: &str, radix: u32) -> usize { input.len() + radix as usize }\npub fn helper() {}\n",
    );
    let default = api_drift(&dir, &[]);
    assert!(default.contains("breaking signature-changed"));
    assert!(!default.contains("new public fn `helper` was added"));
    assert!(api_drift(&dir, &["--all"]).contains("new public fn `helper` was added"));
}

#[test]
fn pub_crate_signature_change_is_not_reported() {
    let dir = fixture(
        "pub(crate) fn parse(input: &str) -> usize { input.len() }\n",
        "pub(crate) fn parse(input: &str, radix: u32) -> usize { input.len() + radix as usize }\n",
    );
    let out = api_drift(&dir, &["--summary"]);
    assert!(out.contains("Issues: 0"));
    assert!(out.contains("Breakdown: none"));
}

#[test]
fn doc_comment_and_formatting_only_change_is_not_reported() {
    let dir = fixture(
        "/// old docs\npub fn parse(input: &str) -> usize { input.len() }\n",
        "/// new docs\npub fn parse(\n    input: &str,\n) -> usize {\n    input.len()\n}\n",
    );
    let out = api_drift(&dir, &["--summary"]);
    assert!(out.contains("Issues: 0"));
    assert!(out.contains("Breakdown: none"));
}

#[test]
fn trait_default_method_addition_is_risky_and_required_method_addition_breaks() {
    let risky = fixture(
        "pub trait Service { fn run(&self); }\n",
        "pub trait Service { fn run(&self); fn flush(&self) {} }\n",
    );
    assert!(api_drift(&risky, &[]).contains("risky trait-method-added"));

    let breaking = fixture(
        "pub trait Service { fn run(&self); }\n",
        "pub trait Service { fn run(&self); fn flush(&self); }\n",
    );
    assert!(api_drift(&breaking, &[]).contains("breaking trait-method-added"));
}

#[test]
fn re_export_source_replacement_is_breaking() {
    let dir = fixture(
        "mod inner { pub struct Thing; }\npub use inner::Thing;\n",
        "mod moved { pub struct Thing; }\npub use moved::Thing;\n",
    );
    let out = api_drift(&dir, &[]);
    assert!(out.contains("breaking signature-changed"));
}

#[test]
fn block_comment_that_looks_like_removed_code_is_ignored() {
    let dir = fixture(
        "pub fn alive() {}\n",
        "pub fn alive() {}\n/* pub fn removed() {} */\n",
    );
    let out = api_drift(&dir, &["--summary"]);
    assert!(out.contains("Issues: 0"));
}

#[test]
fn same_named_items_in_different_modules_do_not_collide() {
    let dir = fixture(
        "pub mod a { pub struct Foo; }\npub mod b { pub struct Foo; }\n",
        "pub mod b { pub struct Foo; }\n",
    );
    let out = api_drift(&dir, &[]);
    assert!(out.contains("src/lib.rs:a::Foo"));
    assert!(!out.contains("src/lib.rs:b::Foo"));
}

#[test]
fn changelog_buckets_breaking_risky_and_safe_changes() {
    let dir = fixture(
        "#[non_exhaustive]\npub enum Event { A }\npub fn old() {}\n",
        "#[non_exhaustive]\npub enum Event { A, B }\npub fn new_fn() {}\n",
    );
    let out = api_drift(&dir, &["--changelog"]);
    assert!(out.contains("### Added"));
    assert!(out.contains("**risky** `src/lib.rs:Event::B`"));
    assert!(out.contains("**safe** `src/lib.rs:new_fn`"));
    assert!(out.contains("### Removed"));
    assert!(out.contains("**breaking** `src/lib.rs:old`"));
}

#[test]
fn macro_public_signatures_are_not_tracked_and_declared_blind_spot() {
    let dir = fixture("#[macro_export]\nmacro_rules! exported { () => {}; }\n", "");
    let out = api_drift(&dir, &["--summary"]);
    assert!(out.contains("Issues: 0"));

    let mut command = Command::cargo_bin("cargo-api-drift").expect("binary");
    command
        .arg("--blind-spots")
        .assert()
        .success()
        .stdout(predicate::str::contains("macro-public-api"))
        .stdout(predicate::str::contains("cargo-semver-checks"));
}

#[test]
fn struct_field_addition_breaks_when_exhaustive_and_is_risky_when_non_exhaustive() {
    let exhaustive = fixture(
        "pub struct Config { pub timeout: u64 }\n",
        "pub struct Config { pub timeout: u64, pub retries: u8 }\n",
    );
    assert!(api_drift(&exhaustive, &[]).contains("breaking field-added"));

    let non_exhaustive = fixture(
        "#[non_exhaustive]\npub struct Config { pub timeout: u64 }\n",
        "#[non_exhaustive]\npub struct Config { pub timeout: u64, pub retries: u8 }\n",
    );
    let out = api_drift(&non_exhaustive, &[]);
    assert!(out.contains("risky field-added"));
    assert!(!out.contains("breaking field-added"));
}

#[test]
fn generic_arity_changes_are_detected_for_nominal_items() {
    let strukt = fixture(
        "pub struct Boxed<T>(pub T);\n",
        "pub struct Boxed<T, U>(pub T, pub U);\n",
    );
    assert!(api_drift(&strukt, &[]).contains("breaking signature-changed"));

    let enm = fixture(
        "pub enum Event<T> { One(T) }\n",
        "pub enum Event<T, U> { One(T), Two(U) }\n",
    );
    assert!(api_drift(&enm, &[]).contains("breaking signature-changed"));

    let trait_item = fixture(
        "pub trait Service<T> { fn run(&self, value: T); }\n",
        "pub trait Service<T, U> { fn run(&self, value: T); }\n",
    );
    assert!(api_drift(&trait_item, &[]).contains("breaking signature-changed"));
}

#[test]
fn aliased_re_export_replacement_is_detected() {
    let dir = fixture(
        "mod a { pub struct Foo; }\npub use a::Foo as Thing;\n",
        "mod b { pub struct Bar; }\npub use b::Bar as Thing;\n",
    );
    let out = api_drift(&dir, &[]);
    assert!(out.contains("breaking signature-changed"));
    assert!(out.contains("src/lib.rs:Thing"));
}

#[test]
fn same_named_inherent_associated_functions_do_not_collide() {
    let dir = fixture(
        "pub struct A;\npub struct B;\nimpl A { pub fn new() -> Self { A } }\nimpl B { pub fn new() -> Self { B } }\n",
        "pub struct A;\npub struct B;\nimpl A { pub fn new() -> Self { A } }\n",
    );
    let out = api_drift(&dir, &[]);
    assert!(out.contains("src/lib.rs:B::new"));
    assert!(!out.contains("src/lib.rs:A::new"));
}

#[test]
fn public_const_and_static_are_tracked() {
    let removed = fixture(
        "pub const LIMIT: usize = 10;\npub static NAME: &str = \"x\";\n",
        "",
    );
    let out = api_drift(&removed, &[]);
    assert!(out.contains("public const `LIMIT` was removed"));
    assert!(out.contains("public static `NAME` was removed"));

    let changed = fixture(
        "pub const LIMIT: usize = 10;\npub static NAME: &str = \"x\";\n",
        "pub const LIMIT: u64 = 10;\npub static NAME: &[u8] = b\"x\";\n",
    );
    let out = api_drift(&changed, &[]);
    assert!(out.matches("breaking signature-changed").count() >= 2);
}

#[test]
fn cosmetic_signature_changes_are_ignored_but_cfg_changes_are_reported() {
    let args = fixture(
        "pub fn parse(input: &str) -> usize { input.len() }\n",
        "pub fn parse(value: &str) -> usize { value.len() }\n",
    );
    assert!(api_drift(&args, &["--summary"]).contains("Issues: 0"));

    let where_order = fixture(
        "pub fn merge<T, U>(left: T, right: U) where T: Clone, U: Copy { let _ = (left, right); }\n",
        "pub fn merge<T, U>(left: T, right: U) where U: Copy, T: Clone { let _ = (left, right); }\n",
    );
    assert!(api_drift(&where_order, &["--summary"]).contains("Issues: 0"));

    let inline = fixture(
        "pub fn parse(input: &str) -> usize { input.len() }\n",
        "#[inline]\npub fn parse(input: &str) -> usize { input.len() }\n",
    );
    assert!(api_drift(&inline, &["--summary"]).contains("Issues: 0"));

    let cfg = fixture(
        "pub fn parse(input: &str) -> usize { input.len() }\n",
        "#[cfg(feature = \"fast\")]\npub fn parse(input: &str) -> usize { input.len() }\n",
    );
    assert!(api_drift(&cfg, &[]).contains("risky cfg-changed"));
}

#[test]
fn bound_addition_breaks_and_bound_removal_is_risky() {
    let added = fixture(
        "pub fn parse<T>(value: T) { let _ = value; }\n",
        "pub fn parse<T: Clone>(value: T) { let _ = value; }\n",
    );
    assert!(api_drift(&added, &[]).contains("breaking bound-added"));

    let removed = fixture(
        "pub fn parse<T: Clone>(value: T) { let _ = value; }\n",
        "pub fn parse<T>(value: T) { let _ = value; }\n",
    );
    let out = api_drift(&removed, &[]);
    assert!(out.contains("risky bound-removed"));
    assert!(out.contains("relaxed"));
}

#[test]
fn check_runs_for_blind_spots_and_changelog_prints_gate() {
    let dir = fixture("pub fn removed() {}\n", "");
    let (code, stdout, _) = api_drift_output(&dir, &["--check", "--blind-spots"]);
    assert_eq!(code, 1);
    assert!(stdout.contains("Blind spots"));
    assert!(stdout.contains("check: FAIL"));

    let (code, stdout, _) = api_drift_output(&dir, &["--check", "--changelog"]);
    assert_eq!(code, 1);
    assert!(stdout.contains("check: FAIL"));
    assert!(stdout.contains("### Removed"));
}

#[test]
fn default_against_uses_master_before_head_parent() {
    let dir = fixture(
        "pub fn old() {}\n",
        "pub fn old(arg: u8) { let _ = arg; }\n",
    );
    git(dir.path(), &["branch", "-M", "master"]);

    let mut command = Command::cargo_bin("cargo-api-drift").expect("binary");
    command
        .arg(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("breaking signature-changed"))
        .stderr(predicate::str::contains("HEAD~1").not());
}
