use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use faultforge::graph::SystemGraph;
use faultforge::output::OutputFormat;
use faultforge::simulation::{CascadeEngine, SpofEngine};
use faultforge::topology;
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(
    name = "faultforge",
    about = "FaultForge — simulation-based infrastructure resilience analysis",
    version,
    after_help = "Examples:\n  faultforge validate topology.yaml\n  faultforge simulate topology.yaml -c api-gateway\n  faultforge analyze topology.yaml"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Validate a topology file for errors.
    Validate {
        /// Path to topology YAML file.
        topology: PathBuf,
    },
    /// Simulate cascade failure for a specific component.
    Simulate {
        /// Path to topology YAML file.
        topology: PathBuf,
        /// Component ID to simulate failure for.
        #[arg(short, long)]
        component: String,
        /// Propagation probability threshold (0.0-1.0).
        #[arg(long, default_value = "0.5")]
        threshold: f64,
        /// Output format: text, json.
        #[arg(short, long, default_value = "text")]
        format: String,
    },
    /// Run SPOF detection and resilience analysis.
    Analyze {
        /// Path to topology YAML file.
        topology: PathBuf,
        /// Output format: text, json.
        #[arg(short, long, default_value = "text")]
        format: String,
    },
    /// Simulate all components and generate a full resilience report.
    Report {
        /// Path to topology YAML file.
        topology: PathBuf,
        /// Output format: text, json.
        #[arg(short, long, default_value = "text")]
        format: String,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Validate { topology } => cmd_validate(&topology),
        Commands::Simulate {
            topology,
            component,
            threshold,
            format,
        } => cmd_simulate(&topology, &component, threshold, &format),
        Commands::Analyze { topology, format } => cmd_analyze(&topology, &format),
        Commands::Report { topology, format } => cmd_report(&topology, &format),
    }
}

fn load_and_validate(path: &Path) -> Result<faultforge::Topology> {
    let topo = topology::loader::load_yaml(path).context("failed to load topology")?;
    let report = topology::validate(&topo);
    if !report.is_valid() {
        let errors = report.errors.join("\n  ");
        bail!("topology validation failed:\n  {errors}");
    }
    for warn in &report.warnings {
        eprintln!("warning: {warn}");
    }
    Ok(topo)
}

fn parse_format(s: &str) -> Result<OutputFormat> {
    OutputFormat::from_str_name(s)
        .ok_or_else(|| anyhow::anyhow!("unknown format: {s} (available: text, json)"))
}

fn cmd_validate(path: &Path) -> Result<()> {
    let topo = topology::loader::load_yaml(path).context("failed to load topology")?;
    let report = topology::validate(&topo);
    print!("{}", faultforge::output::text::render_validation(&report));
    if report.is_valid() {
        Ok(())
    } else {
        bail!("validation failed with {} error(s)", report.errors.len());
    }
}

fn cmd_simulate(path: &Path, component: &str, threshold: f64, format: &str) -> Result<()> {
    let fmt = parse_format(format)?;
    let topo = load_and_validate(path)?;
    let graph = SystemGraph::from_topology(&topo).context("failed to build graph")?;
    let engine = CascadeEngine::new(&graph, threshold);
    let result = engine
        .simulate(component)
        .context("cascade simulation failed")?;

    match fmt {
        OutputFormat::Text => print!("{}", faultforge::output::text::render_cascade(&result)),
        OutputFormat::Json => println!("{}", faultforge::output::json::render_cascade(&result)?),
    }
    Ok(())
}

fn cmd_analyze(path: &Path, format: &str) -> Result<()> {
    let fmt = parse_format(format)?;
    let topo = load_and_validate(path)?;
    let graph = SystemGraph::from_topology(&topo).context("failed to build graph")?;
    let result = SpofEngine::new(&graph).analyze();

    match fmt {
        OutputFormat::Text => print!("{}", faultforge::output::text::render_spof(&result)),
        OutputFormat::Json => println!("{}", faultforge::output::json::render_spof(&result)?),
    }
    Ok(())
}

fn cmd_report(path: &Path, format: &str) -> Result<()> {
    let fmt = parse_format(format)?;
    let topo = load_and_validate(path)?;
    let graph = SystemGraph::from_topology(&topo).context("failed to build graph")?;

    // SPOF analysis.
    let spof = SpofEngine::new(&graph).analyze();

    // Simulate cascade for every component.
    let cascade_engine = CascadeEngine::new(&graph, 0.5);
    let mut cascades: Vec<faultforge::CascadeResult> = Vec::new();
    for comp in graph.all_components() {
        if let Ok(result) = cascade_engine.simulate(&comp.id) {
            if result.blast_radius.total_affected > 1 {
                cascades.push(result);
            }
        }
    }
    cascades.sort_by(|a, b| {
        b.blast_radius
            .impact_percentage
            .partial_cmp(&a.blast_radius.impact_percentage)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    match fmt {
        OutputFormat::Text => {
            print!("{}", faultforge::output::text::render_spof(&spof));
            println!();
            if cascades.is_empty() {
                println!("No significant cascade failures detected.");
            } else {
                println!("Top Cascade Scenarios ({} total):", cascades.len());
                println!("{}", "-".repeat(60));
                for (i, c) in cascades.iter().take(10).enumerate() {
                    println!(
                        "  {}. {} → {:.1}% impact ({} components), severity: {}",
                        i + 1,
                        c.origin_component,
                        c.blast_radius.impact_percentage,
                        c.blast_radius.total_affected,
                        c.severity,
                    );
                }
            }
        }
        OutputFormat::Json => {
            let report = serde_json::json!({
                "spof_analysis": spof,
                "cascade_scenarios": cascades,
            });
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
    }
    Ok(())
}
