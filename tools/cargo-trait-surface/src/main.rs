use std::io::stdout;
use std::path::PathBuf;
use std::process;

use cargo_trait_surface::baseline::{BaselineDiff, diff_against_ref};
use cargo_trait_surface::config::Config;
use cargo_trait_surface::error::Result;
use cargo_trait_surface::issue::Severity;
use cargo_trait_surface::output::{
    GateReport, OutputOptions, write_ai, write_blind_spots, write_json, write_text,
    write_trait_detail,
};
use cargo_trait_surface::{Issue, TraitDetail, analyze_path};
use clap::{Parser, ValueEnum};

#[derive(Debug, Parser)]
#[command(name = "cargo-trait-surface")]
#[command(
    author,
    version,
    about = "Diagnose Rust trait abstraction surface risks"
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

    /// Use Japanese labels in human-oriented output.
    #[arg(long, visible_alias = "jp")]
    japanese: bool,

    /// Print detailed diagnostics for one trait by name.
    #[arg(
        long = "trait",
        conflicts_with_all = ["json", "ai", "summary", "blind_spots", "check", "baseline"]
    )]
    trait_name: Option<String>,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum CliSeverity {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OutputMode {
    Human,
    Summary,
    Json,
    Ai,
    BlindSpots,
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
    if let Some(name) = &args.trait_name {
        let Some(detail) = resolve_trait_detail(&analysis.traits, name) else {
            return Ok(1);
        };
        write_trait_detail(Some(detail), args.japanese, &mut out)?;
        return Ok(0);
    }
    let mode = select_mode(&args);
    warn_ignored_modes(&args, mode);
    let options = OutputOptions {
        all: args.all,
        summary: mode == OutputMode::Summary,
        japanese: args.japanese,
        blind_spots: mode == OutputMode::BlindSpots,
        gate: gate.as_ref(),
    };
    match mode {
        OutputMode::Json => write_json(&analysis, baseline.as_ref(), options, &mut out)?,
        OutputMode::Ai => write_ai(&analysis, baseline.as_ref(), options, &mut out)?,
        OutputMode::BlindSpots => write_blind_spots(&analysis.blind_spots, options, &mut out)?,
        OutputMode::Summary | OutputMode::Human => {
            write_text(&analysis, baseline.as_ref(), options, &mut out)?;
        }
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
    design_gate_core::absorb_cargo_subcommand(args, "trait-surface")
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
        if enabled && mode != selected {
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

fn resolve_trait_detail<'a>(traits: &'a [TraitDetail], name: &str) -> Option<&'a TraitDetail> {
    let matches = traits
        .iter()
        .filter(|detail| detail.name == name)
        .collect::<Vec<_>>();
    if matches.len() == 1 {
        return Some(matches[0]);
    }
    if matches.len() > 1 {
        eprintln!("error: trait '{name}' is ambiguous; matches:");
        for detail in matches {
            eprintln!("  {}:{}", detail.file.display(), detail.line);
        }
        return None;
    }
    eprintln!("error: trait '{name}' not found");
    let suggestions = trait_suggestions(traits, name);
    if !suggestions.is_empty() {
        eprintln!("suggestions:");
        for detail in suggestions {
            eprintln!(
                "  {} at {}:{}",
                detail.name,
                detail.file.display(),
                detail.line
            );
        }
    }
    None
}

fn trait_suggestions<'a>(traits: &'a [TraitDetail], name: &str) -> Vec<&'a TraitDetail> {
    let needle = name.to_ascii_lowercase();
    traits
        .iter()
        .filter(|detail| {
            let candidate = detail.name.to_ascii_lowercase();
            candidate == needle || candidate.contains(&needle) || needle.contains(&candidate)
        })
        .take(5)
        .collect()
}

fn gate_report(issues: &[Issue], baseline: Option<&BaselineDiff>, fail_on: Severity) -> GateReport {
    let issues = if let Some(diff) = baseline {
        diff.new_issues.as_slice()
    } else {
        issues
    };
    design_gate_core::gate_report(issues, fail_on, |issue| issue.severity)
}
