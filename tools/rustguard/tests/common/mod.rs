use std::path::Path;
use std::process::Command;

pub struct TestResult {
    pub exit_code: i32,
    pub stderr: String,
}

/// Run the cargo-rustguard binary on a fixture file in wrapper mode.
pub fn run_on_fixture(fixture_name: &str, format: &str) -> TestResult {
    let fixture_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(fixture_name);

    let binary = env!("CARGO_BIN_EXE_cargo-rustguard");

    // Use tempfile for output isolation between parallel tests
    let out_dir = tempfile::tempdir().expect("failed to create temp dir");
    let out_file = out_dir.path().join("output.rmeta");

    // Invoke in wrapper mode: binary "rustc" <fixture> --edition=2021 --crate-type=lib
    let output = Command::new(binary)
        .arg("rustc") // simulate wrapper mode: argv[1] = "rustc"
        .arg(fixture_path.to_str().expect("valid fixture path"))
        .arg("--edition=2021")
        .arg("--crate-type=lib")
        .arg("--emit=metadata")
        .arg("-o")
        .arg(out_file.to_str().expect("valid output path"))
        .env("RUSTGUARD_FORMAT", format)
        .output()
        .expect("failed to run cargo-rustguard");

    TestResult {
        exit_code: output.status.code().unwrap_or(-1),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
    }
}
