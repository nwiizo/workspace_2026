use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "cargo-rustprobe", version, about)]
struct Cli {
    #[command(subcommand)]
    command: CargoSubcommand,
}

#[derive(Subcommand)]
enum CargoSubcommand {
    Rustprobe(RustprobeArgs),
}

#[derive(Parser)]
struct RustprobeArgs {
    #[command(subcommand)]
    action: Action,
}

#[derive(Subcommand)]
enum Action {
    /// Run MIR analysis on the current crate/workspace.
    Probe {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        cargo_args: Vec<String>,
    },
    /// Display the analysis report from the most recent probe run.
    Report {
        #[arg(short, long, default_value = "target/rustprobe")]
        output_dir: PathBuf,

        #[arg(short, long, default_value = "text")]
        format: OutputFormat,
    },
}

#[derive(Clone, Debug, clap::ValueEnum)]
enum OutputFormat {
    Text,
    Json,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let CargoSubcommand::Rustprobe(args) = cli.command;

    match args.action {
        Action::Probe { cargo_args } => run_probe(&cargo_args),
        Action::Report { output_dir, format } => run_report(&output_dir, &format),
    }
}

fn run_probe(cargo_args: &[String]) -> Result<()> {
    let driver_path = find_driver_binary().context("could not find rustprobe-driver binary")?;

    let sysroot = find_sysroot().context("could not determine rustc sysroot")?;

    let output_dir =
        env::var("RUSTPROBE_OUTPUT_DIR").unwrap_or_else(|_| "target/rustprobe".to_string());

    let _ = std::fs::remove_dir_all(&output_dir);
    std::fs::create_dir_all(&output_dir).context("failed to create output directory")?;

    eprintln!(
        "rustprobe: analyzing with driver at {}",
        driver_path.display()
    );
    eprintln!("rustprobe: output directory: {output_dir}");

    let mut cmd = Command::new("cargo");
    cmd.arg("check");
    cmd.args(cargo_args);

    cmd.env("RUSTC_WORKSPACE_WRAPPER", &driver_path);
    cmd.env("RUSTPROBE_OUTPUT_DIR", &output_dir);

    let lib_path = PathBuf::from(&sysroot).join("lib");
    if cfg!(target_os = "macos") {
        cmd.env("DYLD_FALLBACK_LIBRARY_PATH", &lib_path);
    } else {
        cmd.env("LD_LIBRARY_PATH", &lib_path);
    }

    let status = cmd.status().context("failed to execute cargo check")?;

    if !status.success() {
        bail!("cargo check failed with status: {status}");
    }

    eprintln!("rustprobe: analysis complete. Run `cargo rustprobe report` to see results.");

    run_report(Path::new(&output_dir), &OutputFormat::Text)?;

    Ok(())
}

fn run_report(output_dir: &Path, format: &OutputFormat) -> Result<()> {
    let data = rustprobe_analysis::reader::read_probe_data(output_dir)
        .context("failed to read probe data")?;

    match format {
        OutputFormat::Text => {
            let summary = rustprobe_analysis::summary::generate_summary(&data);
            println!("{summary}");
        }
        OutputFormat::Json => {
            let json =
                serde_json::to_string_pretty(&data).context("failed to serialize probe data")?;
            println!("{json}");
        }
    }

    Ok(())
}

fn find_driver_binary() -> Result<PathBuf> {
    if let Ok(current_exe) = env::current_exe() {
        let sibling = current_exe.with_file_name("rustprobe-driver");
        if sibling.exists() {
            return Ok(sibling);
        }
    }

    if let Ok(output) = Command::new("which").arg("rustprobe-driver").output()
        && output.status.success()
    {
        let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !path.is_empty() {
            return Ok(PathBuf::from(path));
        }
    }

    let target_debug = PathBuf::from("target/debug/rustprobe-driver");
    if target_debug.exists() {
        return Ok(target_debug);
    }

    bail!(
        "rustprobe-driver not found. Build it with:\n\
         cargo build -p rustprobe-driver"
    )
}

fn find_sysroot() -> Result<String> {
    let output = Command::new("rustc")
        .arg("--print")
        .arg("sysroot")
        .output()
        .context("failed to run `rustc --print sysroot`")?;

    if !output.status.success() {
        bail!("rustc --print sysroot failed");
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}
