use std::io::{Write, stdout};
use std::path::PathBuf;
use std::process;

use cargo_feature_doctor::baseline::{BaselineDiff, diff_against_ref};
use cargo_feature_doctor::config::Config;
use cargo_feature_doctor::issue::Severity;
use cargo_feature_doctor::output::{
    GateReport, OutputOptions, write_ai, write_blind_spots, write_json, write_matrix,
    write_suggest_hack, write_text,
};
use cargo_feature_doctor::{Issue, analyze_path};
use clap::{Parser, ValueEnum};

#[derive(Debug, Parser)]
#[command(name = "cargo-feature-doctor")]
#[command(
    author,
    version,
    about = "Diagnose Cargo feature design risks without building"
)]
struct Args {
    /// Rust project directory to analyze.
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

    /// Use Japanese labels in human-oriented output.
    #[arg(long, visible_alias = "jp")]
    japanese: bool,

    /// Print the feature by inspection-status matrix.
    #[arg(long, conflicts_with_all = ["json", "ai", "summary", "blind_spots", "suggest_hack"])]
    matrix: bool,

    /// Print cargo-hack commands targeted at the detected risks.
    #[arg(long, conflicts_with_all = ["json", "ai", "summary", "blind_spots", "matrix"])]
    suggest_hack: bool,
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

fn run() -> std::result::Result<i32, Box<dyn std::error::Error>> {
    let args = Args::parse_from(absorb_cargo_subcommand(std::env::args()));
    let config = Config::load_near(&args.path)?;
    let analysis = analyze_path(&args.path, &config)?;
    let needs_baseline = args.baseline.is_some() && !args.matrix && !args.suggest_hack;
    let baseline = if needs_baseline {
        let git_ref = args.baseline.as_deref().expect("checked by needs_baseline");
        Some(diff_against_ref(&args.path, &config, &analysis, git_ref)?)
    } else {
        None
    };
    let fail_on = args.fail_on.into();
    let gate = args
        .check
        .then(|| gate_report(&analysis.issues, baseline.as_ref(), fail_on));

    let mut out = stdout();
    let options = OutputOptions {
        all: args.all,
        summary: args.summary,
        japanese: args.japanese,
        blind_spots: args.blind_spots,
        gate: gate.as_ref(),
    };
    if args.matrix {
        write_matrix(&analysis, args.japanese, &mut out)?;
        write_gate_line(options, &mut out)?;
        out.flush()?;
        return Ok(exit_code(args.check, gate.as_ref()));
    }
    if args.suggest_hack {
        write_suggest_hack(&analysis, args.japanese, &mut out)?;
        write_gate_line(options, &mut out)?;
        out.flush()?;
        return Ok(exit_code(args.check, gate.as_ref()));
    }
    if args.blind_spots && !args.json && !args.ai && !args.summary {
        write_blind_spots(&analysis.blind_spots, options, &mut out)?;
    } else if args.json {
        write_json(&analysis, baseline.as_ref(), options, &mut out)?;
    } else if args.ai {
        write_ai(&analysis, baseline.as_ref(), options, &mut out)?;
    } else {
        write_text(&analysis, baseline.as_ref(), options, &mut out)?;
    }
    out.flush()?;
    Ok(exit_code(args.check, gate.as_ref()))
}

fn absorb_cargo_subcommand<I>(args: I) -> Vec<String>
where
    I: IntoIterator<Item = String>,
{
    design_gate_core::absorb_cargo_subcommand(args, "feature-doctor")
}

fn gate_report(issues: &[Issue], baseline: Option<&BaselineDiff>, fail_on: Severity) -> GateReport {
    let issues = if let Some(diff) = baseline {
        diff.new_issues.as_slice()
    } else {
        issues
    };
    design_gate_core::gate_report(issues, fail_on, |issue| issue.severity)
}

fn exit_code(check: bool, gate: Option<&GateReport>) -> i32 {
    if !check {
        return 0;
    }
    match gate {
        Some(gate) if gate.passed => 0,
        Some(_) | None => 1,
    }
}

fn write_gate_line<W: Write>(options: OutputOptions<'_>, writer: &mut W) -> std::io::Result<()> {
    if let Some(gate) = options.gate {
        if options.japanese {
            writeln!(
                writer,
                "check: {} (fail-on={}, threshold 以上の issue: {})",
                if gate.passed { "PASS" } else { "FAIL" },
                gate.fail_on,
                gate.failing
            )
        } else {
            writeln!(
                writer,
                "check: {} (fail-on={}, {} issue(s) at/above threshold)",
                if gate.passed { "PASS" } else { "FAIL" },
                gate.fail_on,
                gate.failing
            )
        }
    } else {
        Ok(())
    }
}
