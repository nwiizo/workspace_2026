use std::io::stdout;
use std::path::{Path, PathBuf};
use std::process;

use cargo_api_drift::{
    OutputOptions, Severity, analyze_path, write_ai, write_blind_spots, write_changelog,
    write_json, write_text,
};
use clap::{Parser, ValueEnum};
use design_gate_core::{gate_report, select_mode, warn_ignored_modes};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OutputMode {
    Human,
    Summary,
    Json,
    Ai,
    BlindSpots,
    Changelog,
}

#[derive(Debug, Parser)]
#[command(name = "cargo-api-drift")]
#[command(
    author,
    version,
    about = "Classify Rust public API drift from a git ref"
)]
struct Args {
    /// Rust project directory or a single .rs file to analyze.
    #[arg(default_value = ".")]
    path: PathBuf,

    /// Print only project-level summary information.
    #[arg(long)]
    summary: bool,

    /// Include safe additions and low severity findings in output.
    #[arg(long)]
    all: bool,

    /// Emit a machine-readable JSON report.
    #[arg(long)]
    json: bool,

    /// Emit an AI-agent oriented API change review.
    #[arg(long)]
    ai: bool,

    /// Compare the current working tree against this git ref.
    #[arg(long)]
    against: Option<String>,

    /// Return a non-zero exit when the configured gate fails.
    #[arg(long)]
    check: bool,

    /// Severity threshold used by --check.
    #[arg(long, value_enum, default_value = "high")]
    fail_on: CliSeverity,

    /// Show the full blind spot manifest.
    #[arg(long)]
    blind_spots: bool,

    /// Use Japanese labels in human-oriented output.
    #[arg(long, visible_alias = "jp")]
    japanese: bool,

    /// Emit a Keep a Changelog formatted fragment.
    #[arg(long)]
    changelog: bool,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum CliSeverity {
    Low,
    Medium,
    High,
    Critical,
}

impl From<CliSeverity> for Severity {
    fn from(value: CliSeverity) -> Self {
        match value {
            CliSeverity::Low => Self::Low,
            CliSeverity::Medium => Self::Medium,
            CliSeverity::High => Self::High,
            CliSeverity::Critical => Self::Critical,
        }
    }
}

fn main() {
    match run() {
        Ok(code) => process::exit(code),
        Err(error) => {
            eprintln!("error: {error}");
            process::exit(design_gate_core::cli::RUNTIME_ERROR_EXIT);
        }
    }
}

fn run() -> std::result::Result<i32, Box<dyn std::error::Error>> {
    let args = Args::parse_from(design_gate_core::absorb_cargo_subcommand(
        std::env::args(),
        "api-drift",
    ));
    let mode = output_mode(&args);
    warn_ignored_modes_for_args(&args, mode);
    if mode == OutputMode::BlindSpots && !args.check {
        let mut out = stdout();
        write_blind_spots(args.japanese, &mut out)?;
        return Ok(0);
    }

    let against = args
        .against
        .clone()
        .map(Ok)
        .unwrap_or_else(|| default_against(&args.path))?;
    let analysis = analyze_path(&args.path, &against)?;
    let fail_on = args.fail_on.into();
    let gate = args
        .check
        .then(|| gate_report(&analysis.issues, fail_on, |issue| issue.severity));
    let options = OutputOptions {
        all: args.all,
        summary: mode == OutputMode::Summary,
        japanese: args.japanese,
        gate: gate.as_ref(),
    };
    let mut out = stdout();
    match mode {
        OutputMode::Json => write_json(&analysis, options, &mut out)?,
        OutputMode::Ai => write_ai(&analysis, options, &mut out)?,
        OutputMode::Changelog => write_changelog(&analysis, gate.as_ref(), &mut out)?,
        OutputMode::Summary | OutputMode::Human => write_text(&analysis, options, &mut out)?,
        OutputMode::BlindSpots => {
            write_blind_spots(args.japanese, &mut out)?;
            if let Some(gate) = gate.as_ref() {
                println!(
                    "check: {} (fail-on={}, {} issue(s) at/above threshold)",
                    if gate.passed { "PASS" } else { "FAIL" },
                    gate.fail_on,
                    gate.failing
                );
            }
        }
    }
    if args.check {
        let passed = gate.as_ref().map(|gate| gate.passed).unwrap_or(true);
        return Ok(if passed { 0 } else { 1 });
    }
    Ok(0)
}

fn default_against(path: &Path) -> std::result::Result<String, Box<dyn std::error::Error>> {
    let root = design_gate_core::repo_root(path).unwrap_or_else(|_| path.to_path_buf());
    for candidate in ["main", "master"] {
        if git_ref_exists(&root, candidate) {
            return Ok(candidate.to_string());
        }
    }
    if let Ok(output) =
        design_gate_core::run_git(&root, ["symbolic-ref", "refs/remotes/origin/HEAD"])
        && output.status.success()
    {
        let reference = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !reference.is_empty() {
            eprintln!("info: --against omitted; falling back to {reference}");
            return Ok(reference);
        }
    }
    if git_ref_exists(&root, "HEAD~1") {
        eprintln!("info: --against omitted; falling back to HEAD~1");
        Ok("HEAD~1".to_string())
    } else {
        Err("--against was omitted and no default ref was found; pass --against explicitly (single-commit repositories do not have HEAD~1)".into())
    }
}

fn git_ref_exists(root: &Path, reference: &str) -> bool {
    design_gate_core::run_git(root, ["rev-parse", "--verify", reference])
        .map(|output| output.status.success())
        .unwrap_or(false)
}

fn output_mode(args: &Args) -> OutputMode {
    select_mode(OutputMode::Human, &mode_flags(args))
}

fn mode_flags(args: &Args) -> [(OutputMode, bool); 5] {
    [
        (OutputMode::Json, args.json),
        (OutputMode::Ai, args.ai),
        (OutputMode::Summary, args.summary),
        (OutputMode::BlindSpots, args.blind_spots),
        (OutputMode::Changelog, args.changelog),
    ]
}

fn warn_ignored_modes_for_args(args: &Args, selected: OutputMode) {
    let flags = [
        (OutputMode::Json, args.json, "--json"),
        (OutputMode::Ai, args.ai, "--ai"),
        (OutputMode::Summary, args.summary, "--summary"),
        (OutputMode::BlindSpots, args.blind_spots, "--blind-spots"),
        (OutputMode::Changelog, args.changelog, "--changelog"),
    ];
    warn_ignored_modes(&flags, selected, mode_name);
}

fn mode_name(mode: OutputMode) -> &'static str {
    match mode {
        OutputMode::Human => "human",
        OutputMode::Summary => "--summary",
        OutputMode::Json => "--json",
        OutputMode::Ai => "--ai",
        OutputMode::BlindSpots => "--blind-spots",
        OutputMode::Changelog => "--changelog",
    }
}
