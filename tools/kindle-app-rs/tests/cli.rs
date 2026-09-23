use std::process::Command;

#[test]
fn help_exposes_the_complete_workflow() {
    let output = Command::new(env!("CARGO_BIN_EXE_kindle-capture"))
        .arg("--help")
        .output()
        .unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();

    assert!(output.status.success());
    for command in ["web", "app", "pdf", "trim", "dedupe-tail"] {
        assert!(
            stdout.contains(command),
            "missing {command} in help: {stdout}"
        );
    }
}

#[test]
fn invalid_subcommand_returns_clap_usage_error() {
    let status = Command::new(env!("CARGO_BIN_EXE_kindle-capture"))
        .arg("unknown")
        .status()
        .unwrap();

    assert_eq!(status.code(), Some(2));
}

#[test]
fn web_help_exposes_the_normal_chrome_debug_port() {
    let output = Command::new(env!("CARGO_BIN_EXE_kindle-capture"))
        .args(["web", "--help"])
        .output()
        .unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();

    assert!(output.status.success());
    assert!(stdout.contains("--debug-port"));
    assert!(stdout.contains("9445"));
}

#[test]
fn explicitly_missing_config_is_reported_before_work_starts() {
    let output = Command::new(env!("CARGO_BIN_EXE_kindle-capture"))
        .args([
            "--config",
            "definitely-not-present.yaml",
            "pdf",
            "--input",
            "also-not-present",
        ])
        .output()
        .unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();

    assert_eq!(output.status.code(), Some(1));
    assert!(stderr.contains("config file does not exist"));
}
