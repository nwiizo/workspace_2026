use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

use cargo_boundary::baseline::BaselineOptions;
use cargo_boundary::output::{OutputMode, print_report};
use cargo_boundary::{AnalysisOptions, BoundaryError, Severity, analyze_path};
use clap::{Parser, ValueEnum};
use design_gate_core::{select_mode as core_select_mode, warn_ignored_modes};

#[derive(Debug, Parser)]
#[command(
    name = "cargo-boundary",
    about = "Detect DDD / Clean Architecture boundary risks in Rust crates",
    version
)]
struct Cli {
    /// Path to a crate or source tree.
    #[arg(default_value = ".")]
    path: PathBuf,

    /// Print only project summary.
    #[arg(long)]
    summary: bool,

    /// Include Low severity issues.
    #[arg(long)]
    all: bool,

    /// Machine-readable JSON output.
    #[arg(long)]
    json: bool,

    /// Markdown output for AI coding agents, including fix steps.
    #[arg(long)]
    ai: bool,

    /// Compare against a git ref and report only the diff context.
    #[arg(long, value_name = "GIT_REF")]
    baseline: Option<String>,

    /// CI gate. Exits 1 when current or ratchet failures meet --fail-on.
    #[arg(long)]
    check: bool,

    /// Minimum severity that fails --check.
    #[arg(long, value_enum, default_value_t = CliSeverity::High)]
    fail_on: CliSeverity,

    /// Show the full blind-spot manifest.
    #[arg(long)]
    blind_spots: bool,

    /// Show declared or inferred layers.
    #[arg(long)]
    layers: bool,

    /// Japanese output.
    #[arg(long, visible_alias = "jp")]
    japanese: bool,
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

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(err) => {
            eprintln!("error: {err}");
            ExitCode::from(1)
        }
    }
}

fn run() -> Result<ExitCode, BoundaryError> {
    let cli = Cli::parse_from(normalized_args());
    let options = AnalysisOptions {
        include_low: cli.all,
    };
    let baseline = cli.baseline.as_deref().map(|git_ref| BaselineOptions {
        git_ref: git_ref.to_string(),
        fail_on: cli.fail_on.into(),
    });
    let mut report = analyze_path(&cli.path, &options)?;
    if let Some(options) = baseline {
        report.baseline = Some(cargo_boundary::baseline::diff_against_ref(
            &cli.path, &options, &report,
        )?);
    }
    if cli.check {
        report.gate = Some(gate_report(&report, cli.fail_on.into()));
    }

    let mode = select_mode(&cli);
    warn_ignored_modes_for_cli(&cli, mode);
    print_report(&report, mode, cli.japanese)?;

    if cli.check && report.gate.as_ref().is_some_and(|gate| !gate.passed) {
        return Ok(ExitCode::from(1));
    }
    Ok(ExitCode::SUCCESS)
}

fn normalized_args() -> Vec<String> {
    design_gate_core::absorb_cargo_subcommand(env::args(), "boundary")
}

fn select_mode(cli: &Cli) -> OutputMode {
    core_select_mode(OutputMode::Human, &mode_flags(cli))
}

fn mode_flags(cli: &Cli) -> [(OutputMode, bool); 5] {
    [
        (OutputMode::Json, cli.json),
        (OutputMode::Ai, cli.ai),
        (OutputMode::Summary, cli.summary),
        (OutputMode::BlindSpots, cli.blind_spots),
        (OutputMode::Layers, cli.layers),
    ]
}

fn warn_ignored_modes_for_cli(cli: &Cli, selected: OutputMode) {
    let flags = [
        (OutputMode::Json, cli.json, "--json"),
        (OutputMode::Ai, cli.ai, "--ai"),
        (OutputMode::Summary, cli.summary, "--summary"),
        (OutputMode::BlindSpots, cli.blind_spots, "--blind-spots"),
        (OutputMode::Layers, cli.layers, "--layers"),
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
        OutputMode::Layers => "--layers",
    }
}

fn gate_report(
    report: &cargo_boundary::BoundaryReport,
    fail_on: Severity,
) -> cargo_boundary::model::GateReport {
    let issues = if let Some(diff) = &report.baseline {
        diff.new_issues.as_slice()
    } else {
        report.issues.as_slice()
    };
    design_gate_core::gate_report(issues, fail_on, |issue| issue.severity)
}
