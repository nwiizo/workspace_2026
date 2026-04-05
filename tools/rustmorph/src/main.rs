use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use colored::Colorize;
use rustmorph::graph::GraphBuilder;
use rustmorph::scan::{ScanConfig, ScanEngine, ScanJob};
use rustmorph::simulate::{ChangeKind, ImpactAnalyzer, Transform};
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(
    name = "cargo-rustmorph",
    about = "RustMorph — ownership-aware refactoring impact analysis",
    version
)]
struct Cli {
    /// When invoked as `cargo rustmorph`, cargo passes "rustmorph" as the first arg.
    #[arg(hide = true, default_value = "rustmorph")]
    _cargo: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Analyze a project and show ownership dependency summary.
    Analyze {
        /// Project root directory (defaults to current directory).
        #[arg(short, long, default_value = ".")]
        path: PathBuf,
    },
    /// Preview the impact of a signature change.
    Preview {
        /// The function name to transform (e.g. "module::process").
        #[arg(short, long)]
        function: String,

        /// The parameter index to transform (0-based).
        #[arg(short = 'i', long, default_value = "0")]
        param_index: usize,

        /// The transform to apply.
        /// Options: ref-to-owned, owned-to-ref, ref-to-mut-ref, mut-ref-to-ref,
        ///          string-to-str, str-to-string, vec-to-slice, slice-to-vec, box-to-inline
        #[arg(short, long)]
        transform: String,

        /// Project root directory.
        #[arg(short, long, default_value = ".")]
        path: PathBuf,

        /// Output as JSON.
        #[arg(long)]
        json: bool,
    },
    /// List all functions in the project with their ownership signatures.
    Functions {
        /// Project root directory.
        #[arg(short, long, default_value = ".")]
        path: PathBuf,

        /// Filter functions by name substring.
        #[arg(short, long)]
        filter: Option<String>,
    },
    /// Show available transforms.
    Transforms,
    /// Scan all functions for refactoring opportunities.
    Scan {
        /// Project root directory.
        #[arg(short, long, default_value = ".")]
        path: PathBuf,

        /// Scan job type: full, clone-audit, api-slim.
        #[arg(short, long, default_value = "full")]
        job: String,

        /// Minimum safety score to include (0-100).
        #[arg(long, default_value = "0")]
        min_score: u32,

        /// Maximum candidates to report.
        #[arg(long)]
        max_candidates: Option<usize>,

        /// Filter functions by name substring.
        #[arg(short, long)]
        filter: Option<String>,

        /// Output as JSON.
        #[arg(long)]
        json: bool,

        /// Include self parameters.
        #[arg(long)]
        include_self: bool,

        /// Include test functions.
        #[arg(long)]
        include_tests: bool,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Analyze { path } => cmd_analyze(&path),
        Commands::Preview {
            function,
            param_index,
            transform,
            path,
            json,
        } => cmd_preview(&path, &function, param_index, &transform, json),
        Commands::Functions { path, filter } => cmd_functions(&path, filter.as_deref()),
        Commands::Transforms => cmd_transforms(),
        Commands::Scan {
            path,
            job,
            min_score,
            max_candidates,
            filter,
            json,
            include_self,
            include_tests,
        } => cmd_scan(
            &path,
            &job,
            min_score,
            max_candidates,
            filter.as_deref(),
            json,
            include_self,
            include_tests,
        ),
    }
}

fn cmd_analyze(path: &Path) -> Result<()> {
    println!(
        "{} {}",
        "Analyzing".green().bold(),
        path.canonicalize()
            .unwrap_or_else(|_| path.to_path_buf())
            .display()
    );

    let graph = GraphBuilder::build(path).context("failed to build dependency graph")?;

    println!();
    println!("  関数/メソッド数: {}", graph.node_count());
    println!("  依存エッジ数:    {}", graph.edge_count());

    // Edge kind summary.
    if graph.edge_count() > 0 {
        let counts = graph.edge_kind_counts();
        println!(
            "  内訳: move {} / borrow {} / &mut {} / clone {}",
            counts.owns.to_string().red(),
            counts.borrows.to_string().blue(),
            counts.mut_borrows.to_string().purple(),
            counts.clones.to_string().yellow()
        );
    }
    println!();

    let names = graph.function_names();
    if names.is_empty() {
        println!("  (関数が見つかりませんでした)");
    } else {
        println!("  発見された関数:");
        let mut sorted_names = names;
        sorted_names.sort();
        for name in sorted_names.iter().take(30) {
            if let Some(idx) = graph.find_function(name) {
                let callers = graph.callers(idx);
                let callees = graph.callees(idx);
                println!(
                    "    {} (呼び出し元: {}, 呼び出し先: {})",
                    name.cyan(),
                    callers.len(),
                    callees.len()
                );
            }
        }
        if sorted_names.len() > 30 {
            println!("    ... 他 {} 関数", sorted_names.len() - 30);
        }
    }

    Ok(())
}

fn cmd_preview(
    path: &Path,
    function: &str,
    param_index: usize,
    transform_name: &str,
    json: bool,
) -> Result<()> {
    let transform = Transform::from_str_name(transform_name).ok_or_else(|| {
        anyhow::anyhow!(
            "unknown transform: {transform_name}\navailable: {}",
            Transform::all_names().join(", ")
        )
    })?;

    let graph = GraphBuilder::build(path).context("failed to build dependency graph")?;

    // Check if function exists, suggest alternatives if not.
    if graph.find_function(function).is_none() {
        let names = graph.function_names();
        let suggestions: Vec<_> = names
            .iter()
            .filter(|n| {
                n.contains(function)
                    || function.contains(*n)
                    || n.split("::").last() == Some(function)
            })
            .take(5)
            .collect();

        let mut msg = format!("function '{function}' not found in the dependency graph");
        if !suggestions.is_empty() {
            msg.push_str("\n\nDid you mean:");
            for s in &suggestions {
                msg.push_str(&format!("\n  - {s}"));
            }
        }
        bail!(msg);
    }

    let analyzer = ImpactAnalyzer::new(&graph);

    let impact = analyzer
        .analyze(function, param_index, &transform)
        .ok_or_else(|| {
            let sig = graph
                .find_function(function)
                .and_then(|idx| graph.get_signature(idx));
            if let Some(sig) = sig {
                if param_index >= sig.params.len() {
                    anyhow::anyhow!(
                        "parameter index {param_index} out of range (function has {} params)",
                        sig.params.len()
                    )
                } else {
                    let param = &sig.params[param_index];
                    anyhow::anyhow!(
                        "transform '{transform}' requires {} parameter, but '{}' is {}",
                        transform.source_ownership(),
                        param.name,
                        param.type_info.ownership,
                    )
                }
            } else {
                anyhow::anyhow!(
                    "cannot apply transform '{transform}' to parameter {param_index} of '{function}'"
                )
            }
        })?;

    if json {
        println!("{}", serde_json::to_string_pretty(&impact)?);
        return Ok(());
    }

    // Pretty-print the impact.
    println!(
        "シグネチャ変更: {} パラメータ[{}] に {} を適用",
        function.cyan().bold(),
        param_index,
        transform.to_string().yellow()
    );
    println!();

    if impact.changes.is_empty() {
        println!("  影響箇所: なし (安全に変更可能)");
    } else {
        println!(
            "影響箇所: {}ファイル, {}箇所",
            impact.affected_files.to_string().red(),
            impact.changes.len().to_string().red()
        );

        for (i, change) in impact.changes.iter().enumerate() {
            let prefix = if i == 0 {
                "┌"
            } else if i == impact.changes.len() - 1 {
                "└"
            } else {
                "├"
            };
            println!(
                "  {} {:<25} {} → {}  [{}]",
                prefix,
                change.span.to_string().dimmed(),
                change.original,
                change.suggested.green(),
                change.kind.to_string().yellow()
            );
        }
    }

    println!();

    let clone_count = impact.count_by_kind(ChangeKind::AddClone);
    let move_count = impact.count_by_kind(ChangeKind::ConvertToMove);
    let lifetime_count = impact.count_by_kind(ChangeKind::AddLifetime);

    println!(
        "推定コスト: Clone追加 {}箇所 / move変換 {}箇所 / ライフタイム注釈変更 {}箇所",
        clone_count, move_count, lifetime_count
    );
    println!();

    print!("{}", impact.safety_score);

    Ok(())
}

fn cmd_functions(path: &Path, filter: Option<&str>) -> Result<()> {
    let graph = GraphBuilder::build(path).context("failed to build dependency graph")?;

    let mut names = graph.function_names();
    names.sort();

    if let Some(f) = filter {
        names.retain(|n| n.contains(f));
    }

    if names.is_empty() {
        bail!("no functions found");
    }

    for name in &names {
        if let Some(idx) = graph.find_function(name) {
            if let Some(sig) = graph.get_signature(idx) {
                let callers = graph.callers(idx).len();
                let callees = graph.callees(idx).len();
                println!(
                    "  {} {}  [{} callers, {} callees]",
                    name.cyan(),
                    format!("({})", sig).dimmed(),
                    callers,
                    callees,
                );
                println!("     at {}", sig.span.to_string().dimmed());
            }
        }
    }

    Ok(())
}

fn cmd_transforms() -> Result<()> {
    println!("利用可能な変換:");
    println!();

    let transforms = [
        ("ref-to-owned", "&T → T", "借用を所有に変換"),
        ("owned-to-ref", "T → &T", "所有を借用に変換"),
        ("ref-to-mut-ref", "&T → &mut T", "共有借用を可変借用に変換"),
        ("mut-ref-to-ref", "&mut T → &T", "可変借用を共有借用に変換"),
        (
            "string-to-str",
            "String → &str",
            "所有文字列をスライスに変換",
        ),
        (
            "str-to-string",
            "&str → String",
            "文字列スライスを所有に変換",
        ),
        (
            "vec-to-slice",
            "Vec<T> → &[T]",
            "所有ベクタをスライスに変換",
        ),
        (
            "slice-to-vec",
            "&[T] → Vec<T>",
            "スライスを所有ベクタに変換",
        ),
        ("box-to-inline", "Box<T> → T", "ヒープ配置をスタックに変換"),
    ];

    for (name, arrow, desc) in &transforms {
        println!("  {:<20} {:<20} {}", name.cyan(), arrow.yellow(), desc);
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn cmd_scan(
    path: &Path,
    job_name: &str,
    min_score: u32,
    max_candidates: Option<usize>,
    filter: Option<&str>,
    json: bool,
    include_self: bool,
    include_tests: bool,
) -> Result<()> {
    let job = ScanJob::from_str_name(job_name).ok_or_else(|| {
        anyhow::anyhow!("unknown job: {job_name}\navailable: full, clone-audit, api-slim")
    })?;

    let graph = GraphBuilder::build(path).context("failed to build dependency graph")?;

    let config = ScanConfig {
        job,
        min_score,
        max_candidates,
        skip_self_params: !include_self,
        skip_test_functions: !include_tests,
        function_filter: filter.map(|s| s.to_string()),
        ..ScanConfig::default()
    };

    let engine = ScanEngine::new(&graph, config);
    let report = engine.scan();

    if json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        rustmorph::scan::print_report(&report);
    }

    Ok(())
}
