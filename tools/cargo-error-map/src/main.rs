use std::io::{Write, stdout};
use std::path::PathBuf;
use std::process;

use cargo_error_map::baseline::{BaselineDiff, diff_against_ref};
use cargo_error_map::config::Config;
use cargo_error_map::error::Result;
use cargo_error_map::issue::Severity;
use cargo_error_map::output::{
    GateReport, OutputOptions, write_ai, write_blind_spots, write_json, write_text,
};
use cargo_error_map::{analyze_path, graph::ErrorGraph};
use clap::{Parser, ValueEnum};
use design_gate_core::{select_mode, warn_ignored_modes};

#[derive(Debug, Clone, Copy, ValueEnum)]
enum GraphFormat {
    Text,
    Dot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OutputMode {
    Human,
    Summary,
    Json,
    Ai,
    BlindSpots,
}

#[derive(Debug, Parser)]
#[command(name = "cargo-error-map")]
#[command(
    author,
    version,
    about = "Diagnose Rust error propagation design risks"
)]
struct Args {
    /// Rust project directory or a single .rs file to analyze.
    #[arg(default_value = ".")]
    path: PathBuf,

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

    /// Render the approximate error propagation graph.
    #[arg(
        long,
        value_enum,
        num_args = 0..=1,
        require_equals = true,
        default_missing_value = "text",
        conflicts_with_all = ["json", "ai", "summary", "blind_spots"]
    )]
    graph: Option<GraphFormat>,

    /// Use Japanese labels in human-oriented output.
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
    let config = Config::load_near(&args.path)?;
    let analysis = analyze_path(&args.path, &config)?;
    let baseline = match &args.baseline {
        Some(git_ref) => Some(diff_against_ref(&args.path, &config, &analysis, git_ref)?),
        None => None,
    };
    let fail_on = args.fail_on.into();
    let gate = args
        .check
        .then(|| gate_report(&analysis.issues, baseline.as_ref(), fail_on));

    let mut out = stdout();
    if let Some(format) = args.graph {
        write_graph(&analysis.graph, format, args.japanese, &mut out)?;
        return Ok(0);
    }
    let mode = output_mode(&args);
    warn_ignored_modes_for_args(&args, mode);
    let options = OutputOptions {
        all: args.all,
        summary: mode == OutputMode::Summary,
        japanese: args.japanese,
        blind_spots: mode == OutputMode::BlindSpots,
        gate: gate.as_ref(),
    };
    if mode == OutputMode::BlindSpots {
        write_blind_spots(&analysis.blind_spots, options, &mut out)?;
        return Ok(0);
    }
    if mode == OutputMode::Json {
        write_json(&analysis, baseline.as_ref(), options, &mut out)?;
    } else if mode == OutputMode::Ai {
        write_ai(&analysis, baseline.as_ref(), options, &mut out)?;
    } else {
        write_text(&analysis, baseline.as_ref(), options, &mut out)?;
    }
    if args.check {
        return Ok(if gate.expect("gate is set when --check").passed {
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
    design_gate_core::absorb_cargo_subcommand(args, "error-map")
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

fn write_graph<W: Write>(
    graph: &ErrorGraph,
    format: GraphFormat,
    japanese: bool,
    writer: &mut W,
) -> std::io::Result<()> {
    match format {
        GraphFormat::Text => writer.write_all(graph.render_text(japanese).as_bytes()),
        GraphFormat::Dot => writer.write_all(graph.render_dot(japanese).as_bytes()),
    }
}

fn gate_report(
    issues: &[cargo_error_map::Issue],
    baseline: Option<&BaselineDiff>,
    fail_on: Severity,
) -> GateReport {
    let issues = if let Some(diff) = baseline {
        diff.new_issues.as_slice()
    } else {
        issues
    };
    design_gate_core::gate_report(issues, fail_on, |issue| issue.severity)
}
