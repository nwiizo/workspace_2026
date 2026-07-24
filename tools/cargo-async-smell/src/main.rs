use std::io::stdout;
use std::process;

use cargo_async_smell::analyze_path;
use cargo_async_smell::analyzer::Runtime;
use cargo_async_smell::baseline::diff_against_ref;
use cargo_async_smell::config::Config;
use cargo_async_smell::error::Result;
use cargo_async_smell::issue::Severity;
use cargo_async_smell::output::{
    OutputOptions, write_ai, write_blind_spots, write_json, write_text,
};
use clap::{Parser, ValueEnum};

#[derive(Debug, Parser)]
#[command(name = "cargo-async-smell")]
#[command(
    author,
    version,
    about = "Diagnose production-risk smells in async Rust"
)]
struct Args {
    /// Rust project directory or a single .rs file to analyze.
    #[arg(default_value = ".")]
    path: std::path::PathBuf,

    /// Print only project-level summary information.
    #[arg(long)]
    summary: bool,

    /// Include low severity issues in human, JSON, and AI output.
    #[arg(long)]
    all: bool,

    /// Emit a machine-readable JSON report.
    #[arg(long)]
    json: bool,

    /// Emit an AI-agent oriented repair plan.
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

    /// Async runtime profile to analyze.
    #[arg(long, value_enum, default_value = "tokio")]
    runtime: CliRuntime,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum CliRuntime {
    Tokio,
    AsyncStd,
    Smol,
}

impl From<CliRuntime> for Runtime {
    fn from(value: CliRuntime) -> Self {
        match value {
            CliRuntime::Tokio => Self::Tokio,
            CliRuntime::AsyncStd => Self::AsyncStd,
            CliRuntime::Smol => Self::Smol,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OutputMode {
    Human,
    Summary,
    Json,
    Ai,
    BlindSpots,
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
    let args = Args::parse_from(absorb_cargo_subcommand(std::env::args()));
    let mode = select_mode(&args);
    warn_ignored_modes(&args, mode);
    let config = Config::load_near(&args.path)?;
    let runtime = args.runtime.into();
    let analysis = analyze_path(&args.path, &config, runtime)?;
    let baseline = match &args.baseline {
        Some(git_ref) => Some(diff_against_ref(
            &args.path, &config, &analysis, git_ref, runtime,
        )?),
        None => None,
    };
    let fail_on = args.fail_on.into();
    let gate = args
        .check
        .then(|| gate_report(&analysis.issues, baseline.as_ref(), fail_on));
    let options = OutputOptions {
        all: args.all,
        summary: args.summary,
        japanese: args.japanese,
        blind_spots: args.blind_spots,
        gate: gate.as_ref(),
    };

    let mut out = stdout();
    match mode {
        OutputMode::BlindSpots => write_blind_spots(&analysis.blind_spots, options, &mut out)?,
        OutputMode::Json => write_json(&analysis, baseline.as_ref(), options, &mut out)?,
        OutputMode::Ai => write_ai(&analysis, baseline.as_ref(), options, &mut out)?,
        OutputMode::Summary | OutputMode::Human => {
            write_text(&analysis, baseline.as_ref(), options, &mut out)?;
        }
    }
    if let Some(gate) = gate {
        return Ok(if gate.passed { 0 } else { 1 });
    }
    Ok(0)
}

fn absorb_cargo_subcommand<I>(args: I) -> Vec<String>
where
    I: IntoIterator<Item = String>,
{
    design_gate_core::absorb_cargo_subcommand(args, "async-smell")
}

fn select_mode(args: &Args) -> OutputMode {
    if args.json {
        OutputMode::Json
    } else if args.ai {
        OutputMode::Ai
    } else if args.summary {
        OutputMode::Summary
    } else if args.blind_spots {
        OutputMode::BlindSpots
    } else {
        OutputMode::Human
    }
}

fn warn_ignored_modes(args: &Args, selected: OutputMode) {
    let flags = [
        (OutputMode::Json, args.json, "--json"),
        (OutputMode::Ai, args.ai, "--ai"),
        (OutputMode::Summary, args.summary, "--summary"),
        (OutputMode::BlindSpots, args.blind_spots, "--blind-spots"),
    ];
    let selected_name = mode_name(selected);
    for (mode, enabled, name) in flags {
        if enabled && mode_name(mode) != selected_name {
            eprintln!("warning: ignoring {name}; using {selected_name}");
        }
    }
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

fn gate_report(
    issues: &[cargo_async_smell::Issue],
    baseline: Option<&cargo_async_smell::baseline::BaselineDiff>,
    fail_on: Severity,
) -> cargo_async_smell::output::GateReport {
    let issues = baseline
        .map(|diff| diff.new_issues.as_slice())
        .unwrap_or(issues);
    design_gate_core::gate_report(issues, fail_on, |issue| issue.severity)
}
