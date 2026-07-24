use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use tempfile::TempDir;

fn bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_cargo-agent-context"))
}

fn run(args: &[&str]) -> Output {
    Command::new(bin()).args(args).output().expect("run binary")
}

fn run_with_empty_path(args: &[&str]) -> Output {
    let bin_dir = TempDir::new().expect("bin dir");
    symlink_exe("cargo", &bin_dir.path().join("cargo"));
    symlink_exe("rustc", &bin_dir.path().join("rustc"));
    Command::new(bin())
        .args(args)
        .env("PATH", bin_dir.path())
        .output()
        .expect("run binary")
}

#[cfg(unix)]
fn symlink_exe(name: &str, link: &Path) {
    let target = find_exe(name).expect("find exe");
    std::os::unix::fs::symlink(target, link).expect("symlink exe");
}

#[cfg(not(unix))]
fn symlink_exe(name: &str, link: &Path) {
    fs::copy(find_exe(name).expect("find exe"), link).expect("copy exe");
}

fn find_exe(name: &str) -> Option<PathBuf> {
    std::env::var_os("PATH").and_then(|path| {
        std::env::split_paths(&path)
            .map(|dir| dir.join(name))
            .find(|candidate| candidate.is_file())
    })
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("stdout utf8")
}

fn stderr(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).expect("stderr utf8")
}

fn write_file(path: &Path, text: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create parent");
    }
    fs::write(path, text).expect("write file");
}

fn minimal_crate() -> TempDir {
    let dir = TempDir::new().expect("tempdir");
    write_file(
        &dir.path().join("Cargo.toml"),
        r#"[package]
name = "fixture"
version = "0.1.0"
edition = "2024"
"#,
    );
    write_file(
        &dir.path().join("src/lib.rs"),
        r#"
pub struct Visible;

pub fn visible_api() {}

#[cfg(test)]
pub struct HiddenTestType;

#[cfg(test)]
pub fn hidden_test_fn() {}
"#,
    );
    dir
}

#[test]
fn sibling_json_unknown_fields_are_ok_but_missing_expected_fields_are_schema_mismatch() {
    let krate = minimal_crate();
    let reports = TempDir::new().expect("reports");
    write_file(
        &reports.path().join("boundary.json"),
        r#"{
  "grade": "A",
  "issues": [],
  "unknown_future_field": {"ok": true},
  "blind_spots": {"notes": ["syntax only"]}
}"#,
    );
    write_file(
        &reports.path().join("error-map.json"),
        r#"{"grade": "B", "unknown_future_field": true}"#,
    );

    let output = run(&[
        krate.path().to_str().expect("path"),
        "--from",
        reports.path().to_str().expect("reports"),
    ]);
    assert!(output.status.success(), "{}", stderr(&output));
    let text = stdout(&output);
    assert!(text.contains("`cargo-boundary` | loaded | A | 0"));
    assert!(text.contains("`cargo-error-map` | schema mismatch: missing array field `issues`"));
}

#[test]
fn run_with_no_sibling_binaries_reports_no_tools_and_exits_zero() {
    let krate = minimal_crate();
    let output = run_with_empty_path(&[krate.path().to_str().expect("path"), "--run"]);
    assert!(output.status.success(), "{}", stderr(&output));
    assert!(stdout(&output).contains("no sibling tools available"));
}

#[test]
fn from_directory_integrates_present_json_and_marks_missing_reports() {
    let krate = minimal_crate();
    let reports = TempDir::new().expect("reports");
    write_file(
        &reports.path().join("boundary.json"),
        r#"{"grade": "A", "issues": []}"#,
    );
    let output = run(&[
        krate.path().to_str().expect("path"),
        "--from",
        reports.path().to_str().expect("reports"),
    ]);
    assert!(output.status.success(), "{}", stderr(&output));
    let text = stdout(&output);
    assert!(text.contains("`cargo-boundary` | loaded | A | 0"));
    assert!(text.contains("`cargo-error-map` | not provided"));
}

#[test]
fn workspace_root_lists_members_and_points_to_per_crate_analysis() {
    let dir = TempDir::new().expect("tempdir");
    write_file(
        &dir.path().join("Cargo.toml"),
        r#"[workspace]
members = ["a", "b"]
resolver = "3"
"#,
    );
    for name in ["a", "b"] {
        write_file(
            &dir.path().join(name).join("Cargo.toml"),
            &format!(
                r#"[package]
name = "{name}"
version = "0.1.0"
edition = "2024"
"#
            ),
        );
        write_file(
            &dir.path().join(name).join("src/lib.rs"),
            "pub struct Public;\n",
        );
    }

    let output = run(&[dir.path().to_str().expect("path")]);
    assert!(output.status.success(), "{}", stderr(&output));
    let text = stdout(&output);
    assert!(text.contains("Workspace members: a, b"));
    assert!(text.contains("pass an individual crate path for per-crate module and API analysis"));
}

#[test]
fn output_is_deterministic_for_same_input() {
    let krate = minimal_crate();
    let first = run(&[krate.path().to_str().expect("path")]);
    let second = run(&[krate.path().to_str().expect("path")]);
    assert!(first.status.success(), "{}", stderr(&first));
    assert!(second.status.success(), "{}", stderr(&second));
    assert_eq!(stdout(&first), stdout(&second));
}

#[test]
fn cfg_test_items_are_excluded_from_public_api() {
    let krate = minimal_crate();
    let output = run(&[krate.path().to_str().expect("path")]);
    assert!(output.status.success(), "{}", stderr(&output));
    let text = stdout(&output);
    assert!(text.contains("Visible"));
    assert!(text.contains("visible_api"));
    assert!(!text.contains("HiddenTestType"));
    assert!(!text.contains("hidden_test_fn"));
}

#[test]
fn empty_directory_is_runtime_error() {
    let dir = TempDir::new().expect("tempdir");
    let output = run(&[dir.path().to_str().expect("path")]);
    assert_eq!(output.status.code(), Some(1));
    assert!(stderr(&output).contains("error: empty directory"));
}

#[test]
fn unwritable_output_path_is_runtime_error() {
    let krate = minimal_crate();
    let output = run(&[
        krate.path().to_str().expect("path"),
        "--output",
        krate
            .path()
            .join("missing-dir/out.md")
            .to_str()
            .expect("output path"),
    ]);
    assert_eq!(output.status.code(), Some(1));
    assert!(stderr(&output).contains("error: failed to write"));
}
