use std::path::PathBuf;
use std::process::Command;

fn driver_path() -> PathBuf {
    let path = PathBuf::from(env!("CARGO_BIN_EXE_rustlean-driver"));
    // Verify the binary exists
    assert!(
        path.exists(),
        "rustlean-driver binary not found at {path:?}"
    );
    path
}

fn sysroot() -> String {
    let output = Command::new("rustc")
        .args(["--print", "sysroot"])
        .output()
        .expect("failed to get sysroot");
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

fn run_driver_on_fixture(fixture: &str) -> std::process::Output {
    let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(fixture);

    assert!(fixture_path.exists(), "fixture not found: {fixture_path:?}");

    let sysroot = sysroot();
    let lib_dir = PathBuf::from(&sysroot).join("lib");

    let mut cmd = Command::new(driver_path());
    cmd.arg(&fixture_path);
    cmd.args(["--edition", "2021"]);
    cmd.args(["--sysroot", &sysroot]);
    cmd.args(["--crate-type", "bin"]);
    cmd.args(["--crate-name", "test_fixture"]);

    // Output to a temp directory
    let tmp = tempfile::tempdir().expect("create tempdir");
    cmd.args(["-o", &tmp.path().join("output").display().to_string()]);

    // Set format to JSON for parsing
    cmd.env("RUSTLEAN_FORMAT", "json");

    // Set library path for rustc_private
    if cfg!(target_os = "macos") {
        cmd.env("DYLD_LIBRARY_PATH", &lib_dir);
    } else {
        cmd.env("LD_LIBRARY_PATH", &lib_dir);
    }

    cmd.output().expect("failed to run rustlean-driver")
}

#[test]
fn driver_succeeds_on_clone_fixture() {
    let output = run_driver_on_fixture("clone_cases.rs");
    assert!(
        output.status.success(),
        "driver failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn driver_succeeds_on_alloc_fixture() {
    let output = run_driver_on_fixture("alloc_cases.rs");
    assert!(
        output.status.success(),
        "driver failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn driver_succeeds_on_layout_fixture() {
    let output = run_driver_on_fixture("layout_cases.rs");
    assert!(
        output.status.success(),
        "driver failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
