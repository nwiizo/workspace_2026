//! `kata` — CLI entrypoint.
//!
//! ```text
//! kata <eval.yaml> [--task ID] [--model M]
//! kata iterate <eval.yaml> [--max N] [--task ID]
//! kata compare <eval.yaml> --models M1,M2[,M3...] [--task ID]
//! kata variant <eval.yaml> --base SKILL --candidate SKILL [--task ID]
//! ```

use anyhow::{Context, Result, anyhow};
use clap::{Parser, Subcommand};
use kata_eval::iterate;
use kata_eval::runner::{self, EvalSummary, RunOptions};
use std::path::{Path, PathBuf};

#[derive(Parser, Debug)]
#[command(
    name = "kata",
    about = "Skill evaluation CLI for the Claude Code CLI.",
    long_about = "kata-eval — waxa/waza-schema-compatible skill runner with structured \
                  self-report grading, RED/GREEN/REFACTOR iterate loop, model comparison, \
                  and skill A/B variant exploration."
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,

    /// When no subcommand is given, `kata <eval.yaml>` is single-run mode.
    eval: Option<PathBuf>,

    /// Limit to a single task by id.
    #[arg(long)]
    task: Option<String>,

    /// Override the model (otherwise eval.config.model → .kata.yaml defaults).
    #[arg(long)]
    model: Option<String>,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Run the eval once.
    Run {
        eval: PathBuf,
        #[arg(long)]
        task: Option<String>,
        #[arg(long)]
        model: Option<String>,
    },
    /// RED/GREEN/REFACTOR loop with cumulative ledger.
    Iterate {
        eval: PathBuf,
        #[arg(long, default_value_t = 5)]
        max: u32,
        #[arg(long)]
        task: Option<String>,
        #[arg(long)]
        model: Option<String>,
    },
    /// Compare the same eval across multiple models (objective axes only).
    Compare {
        eval: PathBuf,
        /// Comma-separated list of model names.
        #[arg(long)]
        models: String,
        #[arg(long)]
        task: Option<String>,
    },
    /// A/B a base skill against a candidate rewrite.
    Variant {
        eval: PathBuf,
        #[arg(long)]
        base: String,
        #[arg(long)]
        candidate: String,
        #[arg(long)]
        task: Option<String>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Some(Command::Run { eval, task, model }) => {
            run_single(&eval, task.as_deref(), model.as_deref()).await
        }
        Some(Command::Iterate {
            eval,
            max,
            task,
            model,
        }) => run_iterate(&eval, max, task.as_deref(), model.as_deref()).await,
        Some(Command::Compare { eval, models, task }) => {
            let model_list: Vec<String> = models
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            if model_list.len() < 2 {
                return Err(anyhow!(
                    "--models needs at least 2 comma-separated entries (got {})",
                    model_list.len()
                ));
            }
            run_compare(&eval, &model_list, task.as_deref()).await
        }
        Some(Command::Variant {
            eval,
            base,
            candidate,
            task,
        }) => run_variant(&eval, &base, &candidate, task.as_deref()).await,
        None => {
            let eval = cli
                .eval
                .ok_or_else(|| anyhow!("expected `<eval.yaml>` or a subcommand"))?;
            run_single(&eval, cli.task.as_deref(), cli.model.as_deref()).await
        }
    }
}

async fn run_single(eval: &Path, task: Option<&str>, model: Option<&str>) -> Result<()> {
    let loaded = runner::load(eval).with_context(|| format!("loading {}", eval.display()))?;
    let opts = RunOptions {
        only_task: task,
        model_override: model,
        ..Default::default()
    };
    runner::run_eval(&loaded, &opts).await?;
    Ok(())
}

async fn run_iterate(eval: &Path, max: u32, task: Option<&str>, model: Option<&str>) -> Result<()> {
    let loaded = runner::load(eval)?;
    iterate::run(&loaded, max, task, model).await
}

async fn run_compare(eval: &Path, models: &[String], task: Option<&str>) -> Result<()> {
    let loaded = runner::load(eval)?;
    let mut rows = Vec::with_capacity(models.len());
    for m in models {
        println!("\n========== model: {m} ==========");
        let opts = RunOptions {
            only_task: task,
            model_override: Some(m.as_str()),
            ..Default::default()
        };
        let s = runner::run_eval(&loaded, &opts).await?;
        rows.push((m.clone(), s));
    }
    print_summary_table("compare summary", "model", &rows);
    Ok(())
}

async fn run_variant(eval: &Path, base: &str, candidate: &str, task: Option<&str>) -> Result<()> {
    let loaded = runner::load(eval)?;
    let mut rows = Vec::new();
    for skill in [base, candidate] {
        println!("\n========== skill: {skill} ==========");
        let opts = RunOptions {
            only_task: task,
            skill_override: Some(skill),
            ..Default::default()
        };
        let s = runner::run_eval(&loaded, &opts).await?;
        rows.push((skill.to_string(), s));
    }
    print_summary_table("variant summary", "skill", &rows);
    // Recommendation: rank by pass desc → unclear asc → duration asc.
    rows.sort_by(|(_, a), (_, b)| {
        b.mean_pass_rate
            .partial_cmp(&a.mean_pass_rate)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.total_unclear.cmp(&b.total_unclear))
            .then(a.duration_ms.cmp(&b.duration_ms))
    });
    println!("\nrecommendation: {}", rows[0].0);
    Ok(())
}

fn print_summary_table(title: &str, label_col: &str, rows: &[(String, EvalSummary)]) {
    println!("\n========== {title} ==========");
    println!(
        "{:<40} {:>10} {:>10} {:>10}",
        label_col, "pass%", "unclear", "ms"
    );
    for (label, s) in rows {
        println!(
            "{:<40} {:>9.0}% {:>10} {:>10}",
            label,
            s.mean_pass_rate * 100.0,
            s.total_unclear,
            s.duration_ms,
        );
    }
}
