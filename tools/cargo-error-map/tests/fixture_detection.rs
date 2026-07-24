use std::fs;
use std::path::Path;
use std::process::Command;

use cargo_error_map::baseline::diff_against_ref;
use cargo_error_map::{Config, IssueType, analyze_path};
use tempfile::TempDir;

#[test]
fn detects_all_issue_types_and_suppression() {
    let temp = fixture_crate();
    let analysis = analyze_path(temp.path(), &Config::default()).expect("analysis should succeed");
    let issue_types = analysis
        .issues
        .iter()
        .map(|issue| issue.issue_type())
        .collect::<std::collections::HashSet<_>>();

    assert!(issue_types.contains(&IssueType::AnyhowLeak));
    assert!(issue_types.contains(&IssueType::DynErrorExposure));
    assert!(issue_types.contains(&IssueType::ErrorEnumBloat));
    assert!(issue_types.contains(&IssueType::MissingContext));
    assert!(issue_types.contains(&IssueType::BoundaryPanic));
    assert!(
        analysis
            .issues
            .iter()
            .all(|issue| !issue.key.source.contains("allowed_panic")),
        "suppressed panic must not be reported"
    );
}

#[test]
fn cli_json_and_ai_include_blind_spots() {
    let temp = fixture_crate();
    let binary = env!("CARGO_BIN_EXE_cargo-error-map");
    let json = Command::new(binary)
        .arg("--json")
        .arg(temp.path())
        .output()
        .expect("json command should run");
    assert!(json.status.success());
    let stdout = String::from_utf8(json.stdout).expect("json output should be utf8");
    assert!(stdout.contains("blind_spots"));

    let ai = Command::new(binary)
        .arg("--ai")
        .arg(temp.path())
        .output()
        .expect("ai command should run");
    assert!(ai.status.success());
    let stdout = String::from_utf8(ai.stdout).expect("ai output should be utf8");
    assert!(stdout.contains("Blind spot manifest"));
}

#[test]
fn detects_public_struct_and_enum_field_error_types() {
    let temp = basic_crate(
        r#"
pub struct StructLeak {
    source: Box<dyn std::error::Error>,
}

pub enum EnumLeak {
    Any(anyhow::Error),
    Dyn(Box<dyn std::error::Error>),
}
"#,
    );
    let analysis = analyze_path(temp.path(), &Config::default()).expect("analysis should succeed");
    assert!(has_issue(
        &analysis,
        IssueType::DynErrorExposure,
        "src/lib.rs:StructLeak"
    ));
    assert!(has_issue(
        &analysis,
        IssueType::AnyhowLeak,
        "src/lib.rs:EnumLeak"
    ));
    assert!(has_issue(
        &analysis,
        IssueType::DynErrorExposure,
        "src/lib.rs:EnumLeak"
    ));
}

#[test]
fn pub_trait_methods_are_public_api_but_pub_crate_is_internal() {
    let temp = basic_crate(
        r#"
pub trait Runner {
    fn run(&self) -> anyhow::Result<()>;
}

pub struct Job;

impl Runner for Job {
    fn run(&self) -> anyhow::Result<()> {
        Ok(())
    }
}

pub(crate) fn crate_only() -> anyhow::Result<()> {
    Ok(())
}
"#,
    );
    let analysis = analyze_path(temp.path(), &Config::default()).expect("analysis should succeed");
    assert!(has_issue(
        &analysis,
        IssueType::AnyhowLeak,
        "src/lib.rs:run"
    ));
    assert!(has_issue(
        &analysis,
        IssueType::AnyhowLeak,
        "src/lib.rs:run#2"
    ));
    assert!(!has_issue(
        &analysis,
        IssueType::AnyhowLeak,
        "src/lib.rs:crate_only"
    ));
}

#[test]
fn inline_cfg_test_and_test_attrs_are_not_boundary_panic() {
    let temp = basic_crate(
        r#"
#[cfg(test)]
mod tests {
    #[test]
    fn unit_test() {
        let _ = Some(1).unwrap();
    }
}

pub fn real_panic() {
    let _ = Some(1).unwrap();
}
"#,
    );
    let analysis = analyze_path(temp.path(), &Config::default()).expect("analysis should succeed");
    assert!(!has_issue(
        &analysis,
        IssueType::BoundaryPanic,
        "src/lib.rs:unit_test"
    ));
    assert!(has_issue(
        &analysis,
        IssueType::BoundaryPanic,
        "src/lib.rs:real_panic"
    ));
}

#[test]
fn async_fn_item_suppression_is_applied() {
    let temp = basic_crate(
        r#"
// error-map-allow: boundary-panic
pub async fn allowed() {
    let _ = Some(1).unwrap();
}
"#,
    );
    let analysis = analyze_path(temp.path(), &Config::default()).expect("analysis should succeed");
    assert!(
        analysis
            .issues
            .iter()
            .all(|issue| !issue.key.source.contains("allowed")),
        "async function item suppression must apply"
    );
    assert_eq!(analysis.suppressed_issues, 1);
}

#[test]
fn single_file_mode_uses_file_name_in_issue_keys() {
    let temp = basic_crate(
        r#"
pub fn leak_anyhow() -> anyhow::Result<()> {
    Ok(())
}
"#,
    );
    let file = temp.path().join("src/lib.rs");
    let analysis = analyze_path(&file, &Config::default()).expect("analysis should succeed");
    assert!(
        analysis
            .issues
            .iter()
            .any(|issue| issue.key.source == "lib.rs:leak_anyhow"),
        "single-file keys should include the file name, got: {:#?}",
        analysis.issues
    );
    assert!(
        analysis
            .issues
            .iter()
            .all(|issue| !issue.key.source.starts_with(':')),
        "single-file keys must not have an empty path prefix: {:#?}",
        analysis.issues
    );
}

#[test]
fn same_named_error_enums_in_different_files_are_not_deduped() {
    let temp = TempDir::new().expect("tempdir");
    write_manifest(temp.path());
    fs::create_dir_all(temp.path().join("src")).expect("create src");
    write_file(
        temp.path().join("src/lib.rs").as_path(),
        "pub mod a;\npub mod b;\n",
    );
    write_file(temp.path().join("src/a.rs").as_path(), big_error_enum());
    write_file(temp.path().join("src/b.rs").as_path(), big_error_enum());
    let analysis = analyze_path(temp.path(), &Config::default()).expect("analysis should succeed");
    let sources = analysis
        .issues
        .iter()
        .filter(|issue| issue.issue_type() == IssueType::ErrorEnumBloat)
        .map(|issue| issue.key.source.as_str())
        .collect::<std::collections::HashSet<_>>();
    assert!(sources.contains("src/a.rs:Error"));
    assert!(sources.contains("src/b.rs:Error"));
}

#[test]
fn baseline_same_commit_diff_has_only_unchanged_issues() {
    let temp = fixture_crate();
    run_git(temp.path(), &["init"]);
    run_git(temp.path(), &["config", "user.email", "test@example.com"]);
    run_git(temp.path(), &["config", "user.name", "Test User"]);
    run_git(temp.path(), &["add", "."]);
    run_git(temp.path(), &["commit", "-m", "fixture"]);

    let config = Config::default();
    let analysis = analyze_path(temp.path(), &config).expect("analysis should succeed");
    let diff = diff_against_ref(temp.path(), &config, &analysis, "HEAD").expect("diff succeeds");
    assert_eq!(diff.new_issues.len(), 0);
    assert_eq!(diff.resolved_issues.len(), 0);
    assert!(diff.unchanged > 0);
}

#[test]
fn handler_directory_is_boundary_by_default() {
    let temp = TempDir::new().expect("tempdir");
    write_manifest(temp.path());
    fs::create_dir_all(temp.path().join("src/handlers")).expect("create handlers");
    write_file(
        temp.path().join("src/lib.rs").as_path(),
        "pub mod handlers;\n",
    );
    write_file(
        temp.path().join("src/handlers/mod.rs").as_path(),
        r#"
pub fn handler() {
    let _ = Some(1).unwrap();
}
"#,
    );
    let analysis = analyze_path(temp.path(), &Config::default()).expect("analysis should succeed");
    assert!(!has_issue(
        &analysis,
        IssueType::BoundaryPanic,
        "src/handlers/mod.rs:handler"
    ));
}

fn fixture_crate() -> TempDir {
    let temp = TempDir::new().expect("tempdir");
    write_manifest(temp.path());
    fs::create_dir_all(temp.path().join("src")).expect("create src");
    write_file(
        temp.path().join("src/lib.rs").as_path(),
        r#"
pub mod inner;

pub fn leak_anyhow() -> anyhow::Result<()> {
    inner::a()?;
    Ok(())
}

pub fn dyn_error() -> Result<(), Box<dyn std::error::Error>> {
    Ok(())
}

#[derive(thiserror::Error, Debug)]
pub enum BigError {
    #[error("v1")] V1,
    #[error("v2")] V2,
    #[error("v3")] V3,
    #[error("v4")] V4,
    #[error("v5")] V5,
    #[error("v6")] V6,
    #[error("v7")] V7,
    #[error("v8")] V8,
    #[error("v9")] V9,
    #[error("v10")] V10,
    #[error("v11")] V11,
    #[error("v12")] V12,
    #[error("v13")] V13,
}

pub fn chain_entry() -> Result<(), BigError> {
    inner::a()?;
    Ok(())
}
"#,
    );
    write_file(
        temp.path().join("src/inner.rs").as_path(),
        r#"
pub fn a() -> Result<(), crate::BigError> {
    b()?;
    Ok(())
}

pub fn b() -> Result<(), crate::BigError> {
    c()?;
    Ok(())
}

pub fn c() -> Result<(), crate::BigError> {
    Err(crate::BigError::V1)?;
    Ok(())
}

pub fn panics() {
    let _value = "1".parse::<u8>().unwrap();
}

// error-map-allow: boundary-panic
pub fn allowed_panic() {
    panic!("suppressed");
}
"#,
    );
    temp
}

fn basic_crate(lib_rs: &str) -> TempDir {
    let temp = TempDir::new().expect("tempdir");
    write_manifest(temp.path());
    fs::create_dir_all(temp.path().join("src")).expect("create src");
    write_file(temp.path().join("src/lib.rs").as_path(), lib_rs);
    temp
}

fn write_manifest(path: &Path) {
    write_file(
        path.join("Cargo.toml").as_path(),
        r#"
[package]
name = "fixture"
version = "0.1.0"
edition = "2024"
"#,
    );
}

fn write_file(path: &Path, content: &str) {
    fs::write(path, content).expect("write fixture file");
}

fn has_issue(analysis: &cargo_error_map::Analysis, issue_type: IssueType, source: &str) -> bool {
    analysis
        .issues
        .iter()
        .any(|issue| issue.issue_type() == issue_type && issue.key.source == source)
}

fn big_error_enum() -> &'static str {
    r#"
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("v1")] V1,
    #[error("v2")] V2,
    #[error("v3")] V3,
    #[error("v4")] V4,
    #[error("v5")] V5,
    #[error("v6")] V6,
    #[error("v7")] V7,
    #[error("v8")] V8,
    #[error("v9")] V9,
    #[error("v10")] V10,
    #[error("v11")] V11,
    #[error("v12")] V12,
    #[error("v13")] V13,
}
"#
}

fn run_git(cwd: &Path, args: &[&str]) {
    let output = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .expect("git command should run");
    assert!(
        output.status.success(),
        "git {:?} failed: {}",
        args,
        String::from_utf8_lossy(&output.stderr)
    );
}
