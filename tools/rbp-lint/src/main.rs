use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::Result;
use clap::Parser;
use rbp_lint::{lint_file, Diagnostic, Severity};
use walkdir::WalkDir;

#[derive(Parser, Debug)]
#[command(
    name = "rbp-lint",
    about = "Rust Best Practices Linter (rowan / ra_ap_syntax)",
    version
)]
struct Cli {
    /// Files or directories to lint. Directories are walked recursively for `.rs` files.
    #[arg(required = true)]
    paths: Vec<PathBuf>,

    /// Output format.
    #[arg(long, value_enum, default_value_t = Format::Human)]
    format: Format,

    /// Treat warnings as errors (exit non-zero on any diagnostic).
    #[arg(long)]
    deny_warnings: bool,
}

#[derive(Copy, Clone, Debug, clap::ValueEnum)]
enum Format {
    Human,
    Json,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(cli) {
        Ok(code) => code,
        Err(err) => {
            eprintln!("rbp-lint: {err:#}");
            ExitCode::from(2)
        }
    }
}

fn run(cli: Cli) -> Result<ExitCode> {
    let mut all = Vec::new();
    for path in &cli.paths {
        if path.is_dir() {
            for entry in WalkDir::new(path).follow_links(false) {
                let entry = entry?;
                if entry.file_type().is_file()
                    && entry.path().extension().is_some_and(|e| e == "rs")
                {
                    all.extend(lint_file(entry.path())?);
                }
            }
        } else {
            all.extend(lint_file(path)?);
        }
    }

    let has_error = all.iter().any(|d| matches!(d.severity, Severity::Error));
    let has_any = !all.is_empty();

    match cli.format {
        Format::Human => print_human(&all),
        Format::Json => println!("{}", serde_json::to_string_pretty(&all)?),
    }

    let exit_bad = has_error || (cli.deny_warnings && has_any);
    Ok(if exit_bad {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    })
}

fn print_human(diagnostics: &[Diagnostic]) {
    for d in diagnostics {
        println!("{}", d.render_human());
    }
    let n = diagnostics.len();
    let errs = diagnostics
        .iter()
        .filter(|d| matches!(d.severity, Severity::Error))
        .count();
    eprintln!("rbp-lint: {n} diagnostic(s), {errs} error(s)");
}
