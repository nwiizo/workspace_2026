use std::ffi::OsString;
use std::path::PathBuf;

use lazyssh::cli::{Cli, Outcome, parse};

fn args(values: &[&str]) -> Vec<OsString> {
    values.iter().map(OsString::from).collect()
}

#[test]
fn accepts_an_explicit_config_path() {
    assert_eq!(
        parse(
            args(&["lazyssh", "--config", "fixtures/ssh_config"]),
            PathBuf::from("default")
        ),
        Ok(Outcome::Run(Cli {
            config: PathBuf::from("fixtures/ssh_config")
        }))
    );
}

#[test]
fn handles_help_and_rejects_unknown_arguments() {
    assert_eq!(
        parse(args(&["lazyssh", "--help"]), PathBuf::from("default")),
        Ok(Outcome::Help)
    );
    assert!(parse(args(&["lazyssh", "--wat"]), PathBuf::from("default")).is_err());
}
