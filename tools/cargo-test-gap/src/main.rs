use std::io::stdout;
use std::path::PathBuf;
use std::process;

use cargo_test_gap::{
    AnalyzeOptions, BaselineDiff, GateReport, Issue, OutputOptions, Result, Severity, analyze_path,
    diff_against_ref, write_ai, write_blind_spots, write_json, write_text,
};
use clap::{Parser, ValueEnum};
use design_gate_core::{select_mode, warn_ignored_modes};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OutputMode {
    Human,
    Summary,
    Json,
    Ai,
    BlindSpots,
}

#[derive(Debug, Parser)]
#[command(name = "cargo-test-gap")]
#[command(
    author,
    version,
    about = "Rank Rust functions by churn, complexity, exposure, and test coverage gap"
)]
struct Args {
    /// Rust project directory or a single .rs file to analyze.
    #[arg(default_value = ".")]
    path: PathBuf,

    /// Print only project-level summary information.
    #[arg(long)]
    summary: bool,

    /// Include low severity candidates in human, JSON, and AI output.
    #[arg(long)]
    all: bool,

    /// Emit a machine-readable JSON report.
    #[arg(long)]
    json: bool,

    /// Emit an AI-agent oriented test prioritization plan.
    #[arg(long)]
    ai: bool,

    /// Compare the current analysis against a git ref.
    #[arg(long)]
    baseline: Option<String>,

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

    /// Limit the number of ranked candidates shown.
    #[arg(long, default_value_t = 10)]
    top: usize,

    /// Read cargo-llvm-cov JSON output from this path.
    #[arg(long)]
    llvm_cov: Option<PathBuf>,
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
            process::exit(1);
        }
    }
}

fn run() -> Result<i32> {
    let mut args = Args::parse_from(absorb_cargo_subcommand(std::env::args()));
    let options = AnalyzeOptions {
        llvm_cov: args.llvm_cov.take(),
    };
    let analysis = analyze_path(&args.path, &options)?;
    let baseline = match &args.baseline {
        Some(git_ref) => Some(diff_against_ref(&args.path, &options, &analysis, git_ref)?),
        None => None,
    };
    let fail_on = args.fail_on.into();
    let gate = args
        .check
        .then(|| gate_report(&analysis.issues, baseline.as_ref(), fail_on));

    let mode = output_mode(&args);
    warn_ignored_modes_for_args(&args, mode);
    let output_options = OutputOptions {
        all: args.all,
        summary: mode == OutputMode::Summary,
        japanese: args.japanese,
        blind_spots: mode == OutputMode::BlindSpots,
        gate: gate.as_ref(),
        top: args.top,
    };
    let mut out = stdout();
    if mode == OutputMode::BlindSpots {
        write_blind_spots(&analysis.blind_spots, output_options, &mut out)?;
    } else if mode == OutputMode::Json {
        write_json(&analysis, baseline.as_ref(), output_options, &mut out)?;
    } else if mode == OutputMode::Ai {
        write_ai(&analysis, baseline.as_ref(), output_options, &mut out)?;
    } else {
        write_text(&analysis, baseline.as_ref(), output_options, &mut out)?;
    }
    if args.check {
        return Ok(if gate.map(|gate| gate.passed).unwrap_or(true) {
            0
        } else {
            1
        });
    }
    Ok(0)
}

fn absorb_cargo_subcommand<I>(args: I) -> Vec<String>
where
    I: IntoIterator<Item = String>,
{
    design_gate_core::absorb_cargo_subcommand(args, "test-gap")
}

fn output_mode(args: &Args) -> OutputMode {
    select_mode(OutputMode::Human, &mode_flags(args))
}

fn mode_flags(args: &Args) -> [(OutputMode, bool); 4] {
    [
        (OutputMode::Json, args.json),
        (OutputMode::Ai, args.ai),
        (OutputMode::Summary, args.summary),
        (OutputMode::BlindSpots, args.blind_spots),
    ]
}

fn warn_ignored_modes_for_args(args: &Args, selected: OutputMode) {
    let flags = [
        (OutputMode::Json, args.json, "--json"),
        (OutputMode::Ai, args.ai, "--ai"),
        (OutputMode::Summary, args.summary, "--summary"),
        (OutputMode::BlindSpots, args.blind_spots, "--blind-spots"),
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
    }
}

fn gate_report(issues: &[Issue], baseline: Option<&BaselineDiff>, fail_on: Severity) -> GateReport {
    let issues = if let Some(diff) = baseline {
        diff.new_issues.as_slice()
    } else {
        issues
    };
    design_gate_core::gate_report(issues, fail_on, |issue| issue.severity)
}
